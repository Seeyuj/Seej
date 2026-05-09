//! # Write-Ahead Log
//!
//! Durable event log implementation for crash recovery and replay.
//!
//! ## Binary Record Format
//! ```text
//! +--------+--------+--------+----------+----------+---------+--------+
//! | MAGIC  | VERSION| LENGTH | EVENT_ID |   TICK   | PAYLOAD |  CRC32 |
//! | 4 bytes| 2 bytes| 4 bytes| 8 bytes  | 8 bytes  | N bytes | 4 bytes|
//! +--------+--------+--------+----------+----------+---------+--------+
//! ```
//!
//! ## Crash Safety
//! - CRC32 validates record integrity
//! - Partial writes detected by length mismatch or CRC failure
//! - Recovery stops at first invalid record
//! - fsync after each write for durability

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};

use sy_api::events::SimEvent;
use sy_api::persistence::IEventLog;
use sy_types::{EventId, SimError, SimResult, Tick};
use tracing::{debug, info, warn};

/// Magic number to identify WAL files
const WAL_MAGIC: u32 = 0x57414C31; // "WAL1" in ASCII
/// Current WAL format version
const WAL_VERSION: u16 = 1;
/// Record header size (magic + version + length + event_id + tick) - kept for documentation
#[allow(dead_code)]
const RECORD_HEADER_SIZE: usize = 4 + 2 + 4 + 8 + 8; // 26 bytes
/// CRC size - kept for documentation
#[allow(dead_code)]
const CRC_SIZE: usize = 4;
/// Maximum serialized payload accepted for a single WAL record.
pub const MAX_WAL_PAYLOAD_LEN: u32 = 16 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum WalPayload {
    Batch { events: Vec<SimEvent> },
}

/// File-based event log with binary format and CRC validation.
pub struct FileEventLog {
    /// Path to the WAL file
    path: PathBuf,
    /// File handle for writing
    writer: Option<BufWriter<File>>,
    /// Next event_id to assign (monotonic)
    next_event_id: u64,
    /// Last tick written
    last_tick: Option<Tick>,
    /// Total valid events
    total_events: usize,
    /// First corruption observed while opening in read-only inspection mode.
    read_error: Option<SimError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenMode {
    Repair,
    ReadOnly,
}

impl FileEventLog {
    /// Create or open a WAL file at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> SimResult<Self> {
        Self::open(path, OpenMode::Repair)
    }

    /// Open a WAL file for read-only inspection.
    ///
    /// Unlike `new`, this never truncates a corrupt or partial tail. If a corrupt
    /// record is found, reads fail explicitly so inspection commands cannot
    /// silently report a repaired prefix.
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> SimResult<Self> {
        Self::open(path, OpenMode::ReadOnly)
    }

    fn open<P: AsRef<Path>>(path: P, mode: OpenMode) -> SimResult<Self> {
        let path = path.as_ref().to_path_buf();

        let mut log = FileEventLog {
            path,
            writer: None,
            next_event_id: 1,
            last_tick: None,
            total_events: 0,
            read_error: None,
        };

        // Scan existing WAL to recover state
        log.recover(mode)?;

        info!(
            "Initialized WAL with {} events, next_event_id={}",
            log.total_events, log.next_event_id
        );

        Ok(log)
    }

    /// Scan existing WAL file and recover state.
    /// Stops at first invalid/partial record.
    fn recover(&mut self, mode: OpenMode) -> SimResult<()> {
        if !self.path.exists() {
            return Ok(());
        }

        let file = File::open(&self.path)
            .map_err(|e| SimError::PersistenceError(format!("Failed to open WAL: {}", e)))?;

        let file_len = file
            .metadata()
            .map_err(|e| SimError::PersistenceError(format!("Failed to get WAL metadata: {}", e)))?
            .len();

        let mut reader = BufReader::new(file);
        let mut offset = 0u64;
        let mut last_valid_offset = 0u64;

        while offset < file_len {
            match self.read_record_at(&mut reader, offset) {
                Ok(events) => {
                    if let Some(last) = events.last() {
                        self.next_event_id = last.event_id.as_u64() + 1;
                        self.last_tick = Some(last.tick);
                    }
                    self.total_events += events.len();
                    last_valid_offset = reader.stream_position().map_err(|e| {
                        SimError::PersistenceError(format!("Stream position error: {}", e))
                    })?;
                    offset = last_valid_offset;
                }
                Err(e) => {
                    warn!("WAL recovery stopped at offset {}: {}", offset, e);
                    if mode == OpenMode::ReadOnly {
                        self.read_error = Some(e);
                    }
                    break;
                }
            }
        }

        // If there's garbage at the end, truncate it. This includes offset 0:
        // a corrupt first record must not poison all future appends.
        if mode == OpenMode::Repair && last_valid_offset < file_len {
            warn!(
                "Truncating WAL from {} to {} bytes (removing partial record)",
                file_len, last_valid_offset
            );
            let file = OpenOptions::new()
                .write(true)
                .open(&self.path)
                .map_err(|e| {
                    SimError::PersistenceError(format!("Failed to open WAL for truncate: {}", e))
                })?;
            file.set_len(last_valid_offset).map_err(|e| {
                SimError::PersistenceError(format!("Failed to truncate WAL: {}", e))
            })?;
        }

        debug!(
            "WAL recovery complete: {} events, last_event_id={}, last_tick={:?}",
            self.total_events,
            self.next_event_id - 1,
            self.last_tick
        );

        Ok(())
    }

    /// Read a single WAL record at the given offset.
    fn read_record_at(
        &self,
        reader: &mut BufReader<File>,
        offset: u64,
    ) -> SimResult<Vec<SimEvent>> {
        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|e| SimError::PersistenceError(format!("Seek failed: {}", e)))?;

        // Read header
        let magic = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read magic failed: {}", e)))?;

        if magic != WAL_MAGIC {
            return Err(SimError::CorruptedState(format!(
                "Invalid magic: expected {:08x}, got {:08x}",
                WAL_MAGIC, magic
            )));
        }

        let version = reader
            .read_u16::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read version failed: {}", e)))?;

        if version != WAL_VERSION {
            return Err(SimError::CorruptedState(format!(
                "Unsupported WAL version: {}",
                version
            )));
        }

        let payload_len = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read length failed: {}", e)))?;

        let event_id = reader
            .read_u64::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read event_id failed: {}", e)))?;

        let tick = reader
            .read_u64::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read tick failed: {}", e)))?;

        if payload_len > MAX_WAL_PAYLOAD_LEN {
            return Err(SimError::CorruptedState(format!(
                "WAL payload length {} exceeds max record payload {}",
                payload_len, MAX_WAL_PAYLOAD_LEN
            )));
        }

        let payload_start = reader
            .stream_position()
            .map_err(|e| SimError::PersistenceError(format!("Stream position error: {}", e)))?;
        let file_len = reader
            .get_ref()
            .metadata()
            .map_err(|e| SimError::PersistenceError(format!("Failed to get WAL metadata: {}", e)))?
            .len();
        let required_remaining = u64::from(payload_len) + CRC_SIZE as u64;
        let available_remaining = file_len.saturating_sub(payload_start);
        if required_remaining > available_remaining {
            return Err(SimError::PersistenceError(format!(
                "WAL record length {} exceeds remaining file bytes {}",
                required_remaining, available_remaining
            )));
        }

        // Read payload
        let mut payload = vec![0u8; payload_len as usize];
        reader
            .read_exact(&mut payload)
            .map_err(|e| SimError::PersistenceError(format!("Read payload failed: {}", e)))?;

        // Read and verify CRC
        let stored_crc = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| SimError::PersistenceError(format!("Read CRC failed: {}", e)))?;

        let computed_crc = self.compute_crc(version, payload_len, event_id, tick, &payload);

        if stored_crc != computed_crc {
            return Err(SimError::CorruptedState(format!(
                "CRC mismatch: stored={:08x}, computed={:08x}",
                stored_crc, computed_crc
            )));
        }

        let WalPayload::Batch { events } = serde_json::from_slice::<WalPayload>(&payload)
            .map_err(|e| SimError::CorruptedState(format!("Invalid WAL batch payload: {}", e)))?;

        let last = events.last().ok_or_else(|| {
            SimError::CorruptedState("WAL batch record contains no events".to_string())
        })?;
        if last.event_id.as_u64() != event_id || last.tick.as_u64() != tick {
            return Err(SimError::CorruptedState(format!(
                "WAL batch header mismatch: header=({}, {}), last=({}, {})",
                event_id,
                tick,
                last.event_id.as_u64(),
                last.tick.as_u64()
            )));
        }

        Ok(events)
    }

    /// Compute CRC32 over record contents (excluding CRC field itself).
    fn compute_crc(
        &self,
        version: u16,
        length: u32,
        event_id: u64,
        tick: u64,
        payload: &[u8],
    ) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&WAL_MAGIC.to_le_bytes());
        hasher.update(&version.to_le_bytes());
        hasher.update(&length.to_le_bytes());
        hasher.update(&event_id.to_le_bytes());
        hasher.update(&tick.to_le_bytes());
        hasher.update(payload);
        hasher.finalize()
    }

    fn encode_event_record(&self, events: &[SimEvent]) -> SimResult<Vec<u8>> {
        let last = events.last().ok_or_else(|| {
            SimError::PersistenceError("Cannot encode empty WAL batch".to_string())
        })?;
        let payload = serde_json::to_vec(&WalPayload::Batch {
            events: events.to_vec(),
        })
        .map_err(|e| SimError::PersistenceError(format!("Serialize WAL batch failed: {}", e)))?;

        if payload.len() > MAX_WAL_PAYLOAD_LEN as usize {
            return Err(SimError::PersistenceError(format!(
                "WAL payload length {} exceeds max record payload {}",
                payload.len(),
                MAX_WAL_PAYLOAD_LEN
            )));
        }
        let payload_len = payload.len() as u32;
        let event_id = last.event_id.as_u64();
        let tick = last.tick.as_u64();
        let crc = self.compute_crc(WAL_VERSION, payload_len, event_id, tick, &payload);

        let mut record = Vec::with_capacity(RECORD_HEADER_SIZE + payload.len() + CRC_SIZE);
        record
            .write_u32::<LittleEndian>(WAL_MAGIC)
            .map_err(|e| SimError::PersistenceError(format!("Encode magic failed: {}", e)))?;
        record
            .write_u16::<LittleEndian>(WAL_VERSION)
            .map_err(|e| SimError::PersistenceError(format!("Encode version failed: {}", e)))?;
        record
            .write_u32::<LittleEndian>(payload_len)
            .map_err(|e| SimError::PersistenceError(format!("Encode length failed: {}", e)))?;
        record
            .write_u64::<LittleEndian>(event_id)
            .map_err(|e| SimError::PersistenceError(format!("Encode event_id failed: {}", e)))?;
        record
            .write_u64::<LittleEndian>(tick)
            .map_err(|e| SimError::PersistenceError(format!("Encode tick failed: {}", e)))?;
        record.extend_from_slice(&payload);
        record
            .write_u32::<LittleEndian>(crc)
            .map_err(|e| SimError::PersistenceError(format!("Encode CRC failed: {}", e)))?;

        Ok(record)
    }

    fn ensure_writer(&mut self) -> SimResult<()> {
        if self.writer.is_none() {
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    SimError::PersistenceError(format!("Failed to create WAL dir: {}", e))
                })?;
            }

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|e| SimError::PersistenceError(format!("Failed to open WAL: {}", e)))?;
            self.writer = Some(BufWriter::new(file));
        }
        Ok(())
    }

    fn write_records(&mut self, bytes: &[u8]) -> SimResult<()> {
        self.ensure_writer()?;

        let writer = self.writer.as_mut().unwrap();

        writer
            .write_all(bytes)
            .map_err(|e| SimError::PersistenceError(format!("Write WAL records failed: {}", e)))?;

        writer
            .flush()
            .map_err(|e| SimError::PersistenceError(format!("Flush failed: {}", e)))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|e| SimError::PersistenceError(format!("Sync failed: {}", e)))?;

        Ok(())
    }

    /// Write a single event to the WAL.
    fn write_event(&mut self, mut event: SimEvent) -> SimResult<SimEvent> {
        event.event_id = EventId::new(self.next_event_id);

        let record = self.encode_event_record(std::slice::from_ref(&event))?;
        self.write_records(&record)?;

        self.next_event_id += 1;
        self.last_tick = Some(event.tick);
        self.total_events += 1;

        Ok(event)
    }

    /// Read all valid events from the WAL file.
    fn read_all_events(&self) -> SimResult<Vec<SimEvent>> {
        if let Some(err) = &self.read_error {
            return Err(err.clone());
        }

        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)
            .map_err(|e| SimError::PersistenceError(format!("Failed to open WAL: {}", e)))?;

        let file_len = file
            .metadata()
            .map_err(|e| SimError::PersistenceError(format!("Failed to get WAL metadata: {}", e)))?
            .len();

        let mut reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut offset = 0u64;

        while offset < file_len {
            match self.read_record_at(&mut reader, offset) {
                Ok(record_events) => {
                    offset = reader.stream_position().map_err(|e| {
                        SimError::PersistenceError(format!("Stream position error: {}", e))
                    })?;
                    events.extend(record_events);
                }
                Err(_) => break, // Stop at first invalid record
            }
        }

        Ok(events)
    }
}

impl IEventLog for FileEventLog {
    fn append(&mut self, event: SimEvent) -> SimResult<SimEvent> {
        self.write_event(event)
    }

    fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>> {
        let mut persisted = Vec::with_capacity(events.len());
        if events.is_empty() {
            return Ok(persisted);
        }

        let mut next_event_id = self.next_event_id;

        for mut event in events {
            event.event_id = EventId::new(next_event_id);
            next_event_id += 1;
            persisted.push(event);
        }

        let record = self.encode_event_record(&persisted)?;
        self.write_records(&record)?;

        self.next_event_id = next_event_id;
        if let Some(last) = persisted.last() {
            self.last_tick = Some(last.tick);
        }
        self.total_events += persisted.len();

        Ok(persisted)
    }

    fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>> {
        let all = self.read_all_events()?;
        Ok(all.into_iter().filter(|e| e.event_id > from_id).collect())
    }

    fn read_all_valid(&self) -> SimResult<Vec<SimEvent>> {
        self.read_all_events()
    }

    fn last_event_id(&self) -> EventId {
        if self.next_event_id > 1 {
            EventId::new(self.next_event_id - 1)
        } else {
            EventId::ZERO
        }
    }

    fn last_tick(&self) -> Option<Tick> {
        self.last_tick
    }

    fn truncate_after(&mut self, event_id: EventId) -> SimResult<()> {
        warn!("Truncating WAL after event_id {}", event_id);

        // Close writer
        self.writer = None;

        // Read events up to event_id
        let events_to_keep: Vec<_> = self
            .read_all_events()?
            .into_iter()
            .filter(|e| e.event_id <= event_id)
            .collect();

        // Delete file
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|e| SimError::PersistenceError(format!("Failed to delete WAL: {}", e)))?;
        }

        // Reset state
        self.next_event_id = 1;
        self.last_tick = None;
        self.total_events = 0;

        // Rewrite events
        for event in events_to_keep {
            // Re-use the same event_id
            let mut e = event.clone();
            e.event_id = EventId::ZERO; // Will be reassigned
            self.write_event(e)?;
        }

        Ok(())
    }

    fn sync(&mut self) -> SimResult<()> {
        if let Some(writer) = &mut self.writer {
            writer
                .flush()
                .map_err(|e| SimError::PersistenceError(format!("Flush failed: {}", e)))?;
            writer
                .get_ref()
                .sync_all()
                .map_err(|e| SimError::PersistenceError(format!("Sync failed: {}", e)))?;
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.total_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};
    use sy_api::events::EventData;
    use sy_types::RngSeed;
    use tempfile::TempDir;

    fn temp_wal() -> (TempDir, PathBuf, FileEventLog) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let log = FileEventLog::new(&path).unwrap();
        (dir, path, log)
    }

    fn tick_event(tick: u64) -> SimEvent {
        SimEvent::new(
            Tick(tick),
            EventData::TickProcessed {
                tick: Tick(tick),
                sim_time: sy_types::SimTime { units: tick },
                entities_processed: 0,
                rng_state_after: Some(tick),
            },
        )
    }

    fn flip_byte(path: &Path, offset: u64) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        file.seek(SeekFrom::Start(offset)).unwrap();
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).unwrap();
        byte[0] ^= 0xff;
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(&byte).unwrap();
        file.sync_all().unwrap();
    }

    fn write_raw_record(log: &FileEventLog, path: &Path, event_id: u64, tick: u64, payload: &[u8]) {
        let payload_len = payload.len() as u32;
        let crc = log.compute_crc(WAL_VERSION, payload_len, event_id, tick, payload);
        let mut record = Vec::new();
        record.write_u32::<LittleEndian>(WAL_MAGIC).unwrap();
        record.write_u16::<LittleEndian>(WAL_VERSION).unwrap();
        record.write_u32::<LittleEndian>(payload_len).unwrap();
        record.write_u64::<LittleEndian>(event_id).unwrap();
        record.write_u64::<LittleEndian>(tick).unwrap();
        record.extend_from_slice(payload);
        record.write_u32::<LittleEndian>(crc).unwrap();
        fs::write(path, record).unwrap();
    }

    #[test]
    fn append_and_read() {
        let (_dir, _path, mut log) = temp_wal();

        let event = SimEvent::new(
            Tick(1),
            EventData::WorldCreated {
                world_id: "test".to_string(),
                name: "Test".to_string(),
                seed: RngSeed::new(42),
            },
        );

        let persisted = log.append(event).unwrap();
        assert_eq!(persisted.event_id, EventId::new(1));
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));

        let events = log.read_all_valid().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, EventId::new(1));
    }

    #[test]
    fn empty_wal_recovers_as_empty() {
        let (dir, _path, log) = temp_wal();
        assert_eq!(log.len(), 0);
        drop(log);

        let reopened = FileEventLog::new(dir.path().join("events")).unwrap();
        assert_eq!(reopened.len(), 0);
        assert_eq!(reopened.last_event_id(), EventId::ZERO);
    }

    #[test]
    fn event_id_is_monotonic() {
        let (_dir, _path, mut log) = temp_wal();

        for i in 1..=10 {
            let persisted = log.append(tick_event(i)).unwrap();
            assert_eq!(persisted.event_id.as_u64(), i);
        }

        assert_eq!(log.last_event_id(), EventId::new(10));
    }

    #[test]
    fn read_from_event_id() {
        let (_dir, _path, mut log) = temp_wal();

        for i in 1..=10 {
            log.append(tick_event(i)).unwrap();
        }

        let events = log.read_from_event_id(EventId::new(5)).unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].event_id, EventId::new(6));
    }

    #[test]
    fn recovery_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");

        {
            let mut log = FileEventLog::new(&path).unwrap();
            for i in 1..=5 {
                log.append(tick_event(i)).unwrap();
            }
        }

        {
            let log = FileEventLog::new(&path).unwrap();
            assert_eq!(log.len(), 5);
            assert_eq!(log.last_event_id(), EventId::new(5));

            let events = log.read_all_valid().unwrap();
            assert_eq!(events.len(), 5);
        }
    }

    #[test]
    fn recovery_truncates_corrupt_first_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        fs::write(&path, b"not-a-valid-wal").unwrap();

        let mut log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 0);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);

        log.append(tick_event(1)).unwrap();
        drop(log);

        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));
    }

    #[test]
    fn recovery_rejects_legacy_single_event_payload() {
        let (_dir, path, log) = temp_wal();
        let legacy_payload = serde_json::to_vec(&EventData::WorldCreated {
            world_id: "legacy".to_string(),
            name: "Legacy".to_string(),
            seed: RngSeed::new(7),
        })
        .unwrap();
        write_raw_record(&log, &path, 1, 0, &legacy_payload);
        drop(log);

        let recovered = FileEventLog::new(&path).unwrap();

        assert_eq!(recovered.len(), 0);
        assert_eq!(recovered.last_event_id(), EventId::ZERO);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn append_batch_assigns_contiguous_ids() {
        let (_dir, _path, mut log) = temp_wal();
        let events: Vec<_> = (1..=3).map(tick_event).collect();

        let persisted = log.append_batch(events).unwrap();
        assert_eq!(persisted.len(), 3);
        assert_eq!(persisted[0].event_id, EventId::new(1));
        assert_eq!(persisted[2].event_id, EventId::new(3));
        assert_eq!(log.last_event_id(), EventId::new(3));
        assert_eq!(log.read_all_valid().unwrap().len(), 3);
    }

    #[test]
    fn recovery_discards_partially_written_batch_record() {
        let (_dir, path, mut log) = temp_wal();
        let events = vec![tick_event(10), tick_event(10), tick_event(10)];
        log.append_batch(events).unwrap();
        drop(log);

        let len = fs::metadata(&path).unwrap().len();
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(len - 1)
            .unwrap();

        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 0);
        assert_eq!(log.read_all_valid().unwrap().len(), 0);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn recovery_keeps_prefix_before_truncated_tail_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        drop(log);
        let valid_prefix_len = fs::metadata(&path).unwrap().len();

        let mut log = FileEventLog::new(&path).unwrap();
        log.append(tick_event(2)).unwrap();
        drop(log);
        let full_len = fs::metadata(&path).unwrap().len();
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(full_len - 1)
            .unwrap();

        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));
        assert_eq!(fs::metadata(&path).unwrap().len(), valid_prefix_len);
    }

    #[test]
    fn recovery_keeps_prefix_before_crc_invalid_tail_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        drop(log);
        let valid_prefix_len = fs::metadata(&path).unwrap().len();

        let mut log = FileEventLog::new(&path).unwrap();
        log.append(tick_event(2)).unwrap();
        drop(log);
        let full_len = fs::metadata(&path).unwrap().len();
        flip_byte(&path, full_len - 1);

        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));
        assert_eq!(fs::metadata(&path).unwrap().len(), valid_prefix_len);
    }

    #[test]
    fn read_only_open_reports_corrupt_tail_without_truncating() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        drop(log);
        let valid_prefix_len = fs::metadata(&path).unwrap().len();

        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"corrupt-tail").unwrap();
        file.sync_all().unwrap();
        let corrupt_len = fs::metadata(&path).unwrap().len();
        assert!(corrupt_len > valid_prefix_len);

        let log = FileEventLog::open_read_only(&path).unwrap();
        let err = log.read_all_valid().unwrap_err();

        assert!(err.to_string().contains("Invalid magic"));
        assert_eq!(fs::metadata(&path).unwrap().len(), corrupt_len);
    }

    #[test]
    fn recovery_keeps_prefix_before_magic_invalid_tail_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        drop(log);
        let valid_prefix_len = fs::metadata(&path).unwrap().len();

        let mut log = FileEventLog::new(&path).unwrap();
        log.append(tick_event(2)).unwrap();
        drop(log);
        flip_byte(&path, valid_prefix_len);

        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));
        assert_eq!(fs::metadata(&path).unwrap().len(), valid_prefix_len);
    }

    #[test]
    fn recovery_truncates_record_with_payload_length_above_limit_without_allocating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let mut record = Vec::new();
        record.write_u32::<LittleEndian>(WAL_MAGIC).unwrap();
        record.write_u16::<LittleEndian>(WAL_VERSION).unwrap();
        record
            .write_u32::<LittleEndian>(MAX_WAL_PAYLOAD_LEN + 1)
            .unwrap();
        record.write_u64::<LittleEndian>(1).unwrap();
        record.write_u64::<LittleEndian>(1).unwrap();
        fs::write(&path, record).unwrap();

        let log = FileEventLog::new(&path).unwrap();

        assert_eq!(log.len(), 0);
        assert_eq!(log.last_event_id(), EventId::ZERO);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    }

    #[test]
    fn recovery_truncates_record_with_length_beyond_remaining_file_without_allocating() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events");
        let mut record = Vec::new();
        record.write_u32::<LittleEndian>(WAL_MAGIC).unwrap();
        record.write_u16::<LittleEndian>(WAL_VERSION).unwrap();
        record.write_u32::<LittleEndian>(1024).unwrap();
        record.write_u64::<LittleEndian>(1).unwrap();
        record.write_u64::<LittleEndian>(1).unwrap();
        fs::write(&path, record).unwrap();

        let log = FileEventLog::new(&path).unwrap();

        assert_eq!(log.len(), 0);
        assert_eq!(log.last_event_id(), EventId::ZERO);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    }
}
