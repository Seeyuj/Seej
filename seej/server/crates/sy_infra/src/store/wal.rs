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
//! - Recovery repairs only a compatible fragment of a v1 record header
//! - Incomplete records with a full header, corruption, and I/O errors are fatal
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
/// Record header size (magic + version + length + event_id + tick)
const RECORD_HEADER_SIZE: usize = 4 + 2 + 4 + 8 + 8; // 26 bytes
/// CRC size
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
    /// First record error observed while opening in read-only inspection mode.
    read_error: Option<SimError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenMode {
    Repair,
    ReadOnly,
}

#[derive(Debug)]
enum RecordError {
    ShortHeader(SimError),
    Rejected(SimError),
}

impl RecordError {
    fn into_sim_error(self) -> SimError {
        match self {
            Self::ShortHeader(err) | Self::Rejected(err) => err,
        }
    }
}

fn wal_io_error(offset: u64, operation: &str, err: std::io::Error) -> SimError {
    SimError::PersistenceError(format!("WAL at offset {offset}: {operation}: {err}"))
}

impl FileEventLog {
    /// Create or open a WAL file at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> SimResult<Self> {
        Self::open(path, OpenMode::Repair)
    }

    /// Open a WAL file for read-only inspection.
    ///
    /// Unlike `new`, this never truncates a compatible short header. If a
    /// record error is found, reads fail explicitly rather than report a prefix.
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

    fn open_reader(&self) -> SimResult<Option<(BufReader<File>, u64)>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(wal_io_error(0, "Failed to open WAL", e)),
        };
        let file_len = file
            .metadata()
            .map_err(|e| wal_io_error(0, "Failed to get WAL metadata", e))?
            .len();
        Ok(Some((BufReader::new(file), file_len)))
    }

    /// Scan existing WAL records and repair only a compatible short header.
    fn recover(&mut self, mode: OpenMode) -> SimResult<()> {
        let Some((mut reader, file_len)) = self.open_reader()? else {
            return Ok(());
        };
        let mut offset = 0;

        while offset < file_len {
            match self.read_record_at(&mut reader, offset, file_len) {
                Ok(events) => {
                    let last = events.last().expect("decoded batches are nonempty");
                    self.next_event_id = last.event_id.as_u64() + 1;
                    self.last_tick = Some(last.tick);
                    self.total_events += events.len();
                    offset = reader
                        .stream_position()
                        .map_err(|e| wal_io_error(offset, "Failed to get stream position", e))?;
                }
                Err(RecordError::ShortHeader(err)) if mode == OpenMode::Repair => {
                    warn!("Removing partial WAL header: {err}");
                    let file = OpenOptions::new()
                        .write(true)
                        .open(&self.path)
                        .map_err(|e| wal_io_error(offset, "Failed to open WAL for truncate", e))?;
                    file.set_len(offset)
                        .map_err(|e| wal_io_error(offset, "Failed to truncate WAL", e))?;
                    file.sync_all()
                        .map_err(|e| wal_io_error(offset, "Failed to sync truncated WAL", e))?;
                    break;
                }
                Err(err) if mode == OpenMode::ReadOnly => {
                    let err = err.into_sim_error();
                    warn!("WAL inspection stopped: {err}");
                    self.read_error = Some(err);
                    break;
                }
                Err(err) => return Err(err.into_sim_error()),
            }
        }

        debug!(
            "WAL recovery complete: {} events, last_event_id={}, last_tick={:?}",
            self.total_events,
            self.next_event_id - 1,
            self.last_tick
        );

        Ok(())
    }

    /// Read a single WAL record at a known boundary before the observed EOF.
    fn read_record_at<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: u64,
        file_len: u64,
    ) -> Result<Vec<SimEvent>, RecordError> {
        let invalid = |cause| {
            RecordError::Rejected(SimError::CorruptedState(format!(
                "WAL record at offset {offset}: {cause}"
            )))
        };
        let io_error = |operation, e| RecordError::Rejected(wal_io_error(offset, operation, e));

        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|e| io_error("Seek failed", e))?;

        let remaining = file_len - offset;
        let header_len = remaining.min(RECORD_HEADER_SIZE as u64) as usize;
        let mut header = [0; RECORD_HEADER_SIZE];
        reader
            .read_exact(&mut header[..header_len])
            .map_err(|e| io_error("Read WAL header failed", e))?;

        let magic_len = header_len.min(4);
        if header[..magic_len] != WAL_MAGIC.to_le_bytes()[..magic_len] {
            return Err(invalid(format!(
                "Invalid magic: expected {:02x?}, got {:02x?}",
                &WAL_MAGIC.to_le_bytes()[..magic_len],
                &header[..magic_len]
            )));
        }

        let version = u16::from_le_bytes(header[4..6].try_into().unwrap());
        let version_len = header_len.saturating_sub(4).min(2);
        if header[4..4 + version_len] != WAL_VERSION.to_le_bytes()[..version_len] {
            return Err(invalid(if version_len == 2 {
                format!("Unsupported WAL version: {version}")
            } else {
                format!("Unsupported WAL version prefix: 0x{:02x}", header[4])
            }));
        }

        let length_bytes = header_len.saturating_sub(6).min(4);
        let payload_len = u32::from_le_bytes(header[6..10].try_into().unwrap());
        // If the prefix is all zeros, the smallest positive completion has a 1
        // in the first missing byte. A complete all-zero length has no such completion.
        let min_positive_len = if payload_len == 0 && length_bytes < 4 {
            1 << (8 * length_bytes)
        } else {
            payload_len
        };
        if min_positive_len == 0 {
            return Err(invalid("WAL payload length is zero".into()));
        }
        if min_positive_len > MAX_WAL_PAYLOAD_LEN {
            return Err(invalid(format!(
                "WAL payload length {min_positive_len} exceeds max record payload {MAX_WAL_PAYLOAD_LEN}"
            )));
        }

        let event_id = u64::from_le_bytes(header[10..18].try_into().unwrap());
        if header_len >= 18 && event_id == 0 {
            return Err(invalid("WAL event_id is zero".into()));
        }
        if header_len < RECORD_HEADER_SIZE {
            return Err(RecordError::ShortHeader(SimError::CorruptedState(format!(
                "WAL record at offset {offset}: partial header ({header_len} of {RECORD_HEADER_SIZE} bytes)"
            ))));
        }

        let tick = u64::from_le_bytes(header[18..26].try_into().unwrap());
        let required_remaining = u64::from(payload_len) + CRC_SIZE as u64;
        let available_remaining = remaining - RECORD_HEADER_SIZE as u64;
        if required_remaining > available_remaining {
            return Err(invalid(format!(
                "WAL record length {required_remaining} exceeds remaining file bytes {available_remaining}"
            )));
        }

        // Read payload
        let mut payload = vec![0u8; payload_len as usize];
        reader
            .read_exact(&mut payload)
            .map_err(|e| io_error("Read payload failed", e))?;

        // Read and verify CRC
        let stored_crc = reader
            .read_u32::<LittleEndian>()
            .map_err(|e| io_error("Read CRC failed", e))?;

        let computed_crc = self.compute_crc(version, payload_len, event_id, tick, &payload);

        if stored_crc != computed_crc {
            return Err(invalid(format!(
                "CRC mismatch: stored={:08x}, computed={:08x}",
                stored_crc, computed_crc
            )));
        }

        let WalPayload::Batch { events } = serde_json::from_slice::<WalPayload>(&payload)
            .map_err(|e| invalid(format!("Invalid WAL batch payload: {e}")))?;

        let last = events
            .last()
            .ok_or_else(|| invalid("WAL batch record contains no events".into()))?;
        if last.event_id.as_u64() != event_id || last.tick.as_u64() != tick {
            return Err(invalid(format!(
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

    /// Read all events, or return the first scan error.
    fn read_all_events(&self) -> SimResult<Vec<SimEvent>> {
        if let Some(err) = &self.read_error {
            return Err(err.clone());
        }

        let Some((mut reader, file_len)) = self.open_reader()? else {
            return Ok(Vec::new());
        };
        let mut events = Vec::new();
        let mut offset = 0u64;

        while offset < file_len {
            let record_events = self
                .read_record_at(&mut reader, offset, file_len)
                .map_err(RecordError::into_sim_error)?;
            offset = reader
                .stream_position()
                .map_err(|e| wal_io_error(offset, "Failed to get stream position", e))?;
            events.extend(record_events);
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
    use proptest::prelude::*;
    use std::io::{Cursor, Error, ErrorKind, Read, Seek, SeekFrom, Write};
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

    fn append_bytes(path: &Path, bytes: &[u8]) {
        OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    fn header(version: u16, payload_len: u32, event_id: u64) -> [u8; RECORD_HEADER_SIZE] {
        let mut bytes = [0; RECORD_HEADER_SIZE];
        bytes[..4].copy_from_slice(&WAL_MAGIC.to_le_bytes());
        bytes[4..6].copy_from_slice(&version.to_le_bytes());
        bytes[6..10].copy_from_slice(&payload_len.to_le_bytes());
        bytes[10..18].copy_from_slice(&event_id.to_le_bytes());
        bytes[18..].copy_from_slice(&1u64.to_le_bytes());
        bytes
    }

    fn assert_rejected_unchanged(path: &Path, offset: u64, cause: &str) {
        let before = fs::read(path).unwrap();
        let Err(SimError::CorruptedState(message)) = FileEventLog::new(path) else {
            panic!("opening must report WAL corruption at offset {offset}");
        };
        assert!(message.contains(&format!("offset {offset}:")), "{message}");
        assert!(message.contains(cause), "{message}");
        assert_eq!(fs::read(path).unwrap(), before);
    }

    fn assert_reads_reject(log: &FileEventLog, cause: &str) {
        for error in [
            log.read_all_valid().unwrap_err(),
            log.read_from_event_id(EventId::ZERO).unwrap_err(),
        ] {
            assert!(error.to_string().contains(cause), "{error}");
        }
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
        let (_dir, path, log) = temp_wal();
        assert_eq!(log.len(), 0);
        assert!(log.read_all_valid().unwrap().is_empty());
        assert!(!path.exists(), "opening an absent WAL must be lazy");

        fs::write(&path, []).unwrap();
        let reopened = FileEventLog::new(&path).unwrap();
        assert_eq!(reopened.len(), 0);
        assert_eq!(reopened.last_event_id(), EventId::ZERO);
        assert!(fs::read(&path).unwrap().is_empty());
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
    fn recovery_rejects_invalid_magic_at_offset_zero() {
        let (_dir, path, _log) = temp_wal();
        fs::write(&path, b"not-a-valid-wal").unwrap();
        assert_rejected_unchanged(&path, 0, "Invalid magic");
    }

    #[test]
    fn recovery_repairs_compatible_fragment_at_offset_zero() {
        let (_dir, path, _log) = temp_wal();
        // Three zero length bytes can be completed to the maximum payload length.
        let bytes = header(WAL_VERSION, MAX_WAL_PAYLOAD_LEN, 1);
        fs::write(&path, &bytes[..9]).unwrap();
        let mut log = FileEventLog::new(&path).unwrap();
        assert_eq!(log.len(), 0);
        assert!(fs::read(&path).unwrap().is_empty());
        assert_eq!(log.append(tick_event(1)).unwrap().event_id, EventId::new(1));
    }

    #[test]
    fn recovery_rejects_unknown_version_at_any_record_boundary() {
        for with_prefix in [false, true] {
            let (_dir, path, mut log) = temp_wal();
            if with_prefix {
                log.append(tick_event(1)).unwrap();
            } else {
                fs::write(&path, []).unwrap();
            }
            drop(log);
            let offset = fs::metadata(&path).unwrap().len();
            append_bytes(&path, &header(2, 1, 2));
            assert_rejected_unchanged(&path, offset, "Unsupported WAL version: 2");
        }
    }

    #[test]
    fn recovery_rejects_incompatible_short_header_fields() {
        let (_dir, path, _log) = temp_wal();
        for (bytes, len, cause) in [
            (header(2, 1, 1), 5, "Unsupported WAL version prefix"),
            (header(WAL_VERSION, 0, 1), 10, "WAL payload length is zero"),
            (header(WAL_VERSION, 1, 0), 18, "WAL event_id is zero"),
        ] {
            fs::write(&path, &bytes[..len]).unwrap();
            assert_rejected_unchanged(&path, 0, cause);
        }
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

        assert_rejected_unchanged(&path, 0, "Invalid WAL batch payload");
    }

    #[test]
    fn recovery_rejects_invalid_batch_with_valid_crc() {
        let (_dir, path, log) = temp_wal();
        for (events, cause) in [
            (Vec::new(), "WAL batch record contains no events"),
            (vec![tick_event(1)], "WAL batch header mismatch"),
        ] {
            let payload = serde_json::to_vec(&WalPayload::Batch { events }).unwrap();
            write_raw_record(&log, &path, 1, 1, &payload);
            assert_rejected_unchanged(&path, 0, cause);
        }
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
    fn recovery_rejects_partially_written_batch_record() {
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

        assert_rejected_unchanged(&path, 0, "exceeds remaining file bytes");
    }

    #[test]
    fn recovery_rejects_incomplete_record_after_valid_prefix() {
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

        assert_rejected_unchanged(&path, valid_prefix_len, "exceeds remaining file bytes");
    }

    #[test]
    fn recovery_rejects_crc_invalid_tail_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let valid_prefix_len = fs::metadata(&path).unwrap().len();
        log.append(tick_event(2)).unwrap();
        drop(log);
        let full_len = fs::metadata(&path).unwrap().len();
        flip_byte(&path, full_len - 1);

        assert_rejected_unchanged(&path, valid_prefix_len, "CRC mismatch");
    }

    #[test]
    fn read_only_open_reports_bad_tail_without_truncating() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        drop(log);
        let prefix = fs::read(&path).unwrap();

        for (tail, cause) in [
            (b"corrupt-tail".as_slice(), "Invalid magic"),
            (&header(WAL_VERSION, 100, 2)[..10], "partial header"),
            (&header(2, 100, 2), "Unsupported WAL version: 2"),
        ] {
            fs::write(&path, &prefix).unwrap();
            append_bytes(&path, tail);
            let before = fs::read(&path).unwrap();
            let log = FileEventLog::open_read_only(&path).unwrap();
            assert_reads_reject(&log, cause);
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn recovery_rejects_magic_invalid_tail_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let valid_prefix_len = fs::metadata(&path).unwrap().len();
        log.append(tick_event(2)).unwrap();
        drop(log);
        flip_byte(&path, valid_prefix_len);

        assert_rejected_unchanged(&path, valid_prefix_len, "Invalid magic");
    }

    #[test]
    fn recovery_rejects_short_header_with_payload_length_above_limit() {
        let (_dir, path, _log) = temp_wal();
        let bytes = header(WAL_VERSION, MAX_WAL_PAYLOAD_LEN + 1, 1);
        fs::write(&path, &bytes[..10]).unwrap();
        assert_rejected_unchanged(&path, 0, "exceeds max record payload");
    }

    #[test]
    fn recovery_rejects_header_with_length_beyond_remaining_file_without_allocating() {
        let (_dir, path, _log) = temp_wal();
        fs::write(&path, header(WAL_VERSION, 1024, 1)).unwrap();
        assert_rejected_unchanged(&path, 0, "exceeds remaining file bytes");
    }

    #[test]
    fn recovery_rejects_length_increase_in_middle_or_last_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let middle = fs::metadata(&path).unwrap().len() as usize;
        log.append(tick_event(2)).unwrap();
        let last = fs::metadata(&path).unwrap().len() as usize;
        log.append(tick_event(3)).unwrap();
        drop(log);
        let original = fs::read(&path).unwrap();

        for (offset, cause) in [
            (middle, "CRC mismatch"),
            (last, "exceeds remaining file bytes"),
        ] {
            let mut corrupt = original.clone();
            let field = &mut corrupt[offset + 6..offset + 10];
            let length = u32::from_le_bytes(field.try_into().unwrap());
            field.copy_from_slice(&(length + 1).to_le_bytes());
            fs::write(&path, corrupt).unwrap();
            assert_rejected_unchanged(&path, offset as u64, cause);
        }
    }

    #[test]
    fn recovery_classifies_every_cut_of_a_real_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let prefix_len = fs::metadata(&path).unwrap().len() as usize;
        let second_event = log.append(tick_event(2)).unwrap();
        drop(log);
        let full = fs::read(&path).unwrap();
        let prefix = &full[..prefix_len];
        let second_len = full.len() - prefix_len;

        for cut in 0..=second_len {
            let before = &full[..prefix_len + cut];
            fs::write(&path, before).unwrap();
            match cut {
                0 => {
                    let log = FileEventLog::new(&path).unwrap();
                    assert_eq!(log.read_all_valid().unwrap().len(), 1);
                    assert_eq!(fs::read(&path).unwrap(), before);
                }
                1..RECORD_HEADER_SIZE => {
                    let mut log = FileEventLog::new(&path).unwrap();
                    assert_eq!(fs::read(&path).unwrap(), prefix);
                    assert_eq!(log.append(tick_event(2)).unwrap().event_id, EventId::new(2));
                    assert_eq!(fs::read(&path).unwrap(), full);
                }
                cut if cut == second_len => {
                    let log = FileEventLog::new(&path).unwrap();
                    let events = log.read_all_valid().unwrap();
                    assert_eq!(events.len(), 2);
                    assert_eq!(events[1], second_event);
                    assert_eq!(fs::read(&path).unwrap(), full);
                }
                _ => assert_rejected_unchanged(
                    &path,
                    prefix_len as u64,
                    "exceeds remaining file bytes",
                ),
            }
        }
    }

    #[test]
    fn recovery_preserves_a_complete_appended_record() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let prefix_len = fs::metadata(&path).unwrap().len() as usize;
        let second = log.append(tick_event(2)).unwrap();
        drop(log);

        let full = fs::read(&path).unwrap();
        fs::write(&path, &full[..prefix_len]).unwrap();
        append_bytes(&path, &full[prefix_len..]);
        let log = FileEventLog::new(&path).unwrap();
        assert_eq!(
            log.read_from_event_id(EventId::new(1)).unwrap(),
            vec![second]
        );
        assert_eq!(fs::read(&path).unwrap(), full);
    }

    #[test]
    fn reads_propagate_corruption_after_a_healthy_open() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        log.append(tick_event(2)).unwrap();
        drop(log);
        let log = FileEventLog::open_read_only(&path).unwrap();
        assert_eq!(log.read_all_valid().unwrap().len(), 2);

        let len = fs::metadata(&path).unwrap().len();
        flip_byte(&path, len - 1);
        let before = fs::read(&path).unwrap();
        assert_reads_reject(&log, "CRC mismatch");
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    struct FailingReader<'a> {
        data: Cursor<&'a [u8]>,
        fail_at: u64,
        kind: ErrorKind,
    }

    impl Read for FailingReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let remaining = self.fail_at.saturating_sub(self.data.position()) as usize;
            if remaining == 0 {
                return Err(Error::new(self.kind, "injected read failure"));
            }
            let len = buf.len().min(remaining);
            self.data.read(&mut buf[..len])
        }
    }

    impl Seek for FailingReader<'_> {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.data.seek(pos)
        }
    }

    #[test]
    fn read_failures_are_not_repairable() {
        let (_dir, path, mut log) = temp_wal();
        log.append(tick_event(1)).unwrap();
        let bytes = fs::read(path).unwrap();
        for (file_len, fail_at, kind, stage) in [
            (10, 3, ErrorKind::UnexpectedEof, "Read WAL header failed"),
            (
                bytes.len(),
                5,
                ErrorKind::PermissionDenied,
                "Read WAL header failed",
            ),
            (
                bytes.len(),
                RECORD_HEADER_SIZE + 1,
                ErrorKind::UnexpectedEof,
                "Read payload failed",
            ),
            (
                bytes.len(),
                RECORD_HEADER_SIZE + 1,
                ErrorKind::PermissionDenied,
                "Read payload failed",
            ),
        ] {
            let mut reader = FailingReader {
                data: Cursor::new(bytes.as_slice()),
                fail_at: fail_at as u64,
                kind,
            };
            let error = log
                .read_record_at(&mut reader, 0, file_len as u64)
                .unwrap_err();
            let RecordError::Rejected(SimError::PersistenceError(message)) = error else {
                panic!("expected rejected persistence error, got {error:?}");
            };
            assert!(message.contains(stage), "{message}");
            assert!(message.contains("offset 0"), "{message}");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn three_record_recovery_is_fail_closed(
            byte in any::<usize>(),
            mask in 1u8..=255,
            cut in any::<usize>(),
            tail in prop::collection::vec(any::<u8>(), 1..RECORD_HEADER_SIZE),
        ) {
            let (_dir, path, log) = temp_wal();
            let mut full = Vec::new();
            let mut boundaries = [0; 4];
            for id in 1..=3 {
                let mut event = tick_event(id);
                event.event_id = EventId::new(id);
                full.extend(log.encode_event_record(&[event]).unwrap());
                boundaries[id as usize] = full.len();
            }

            let mut flipped = full.clone();
            let byte = byte % flipped.len();
            flipped[byte] ^= mask;
            fs::write(&path, &flipped).unwrap();
            prop_assert!(FileEventLog::new(&path).is_err());
            prop_assert_eq!(fs::read(&path).unwrap(), flipped);

            let cut = cut % (full.len() + 1);
            let complete = boundaries.partition_point(|&boundary| boundary <= cut) - 1;
            let boundary = boundaries[complete];
            fs::write(&path, &full[..cut]).unwrap();
            let result = FileEventLog::new(&path);
            if cut - boundary < RECORD_HEADER_SIZE {
                let recovered = result.unwrap();
                prop_assert_eq!(recovered.read_all_valid().unwrap().len(), complete);
                prop_assert_eq!(fs::read(&path).unwrap(), &full[..boundary]);
            } else {
                prop_assert!(result.is_err());
                prop_assert_eq!(fs::read(&path).unwrap(), &full[..cut]);
            }

            let with_tail = [&full[..], &tail[..]].concat();
            fs::write(&path, &with_tail).unwrap();
            // With this size limit every incomplete length field is extendable;
            // a zero prefix in three bytes can be completed to 1 << 24.
            let tag = [&WAL_MAGIC.to_le_bytes()[..], &WAL_VERSION.to_le_bytes()[..]].concat();
            let expected_tag = &tag[..tail.len().min(tag.len())];
            let compatible = tail.starts_with(expected_tag)
                && (tail.len() < 10
                    || (1..=MAX_WAL_PAYLOAD_LEN)
                        .contains(&u32::from_le_bytes(tail[6..10].try_into().unwrap())))
                && (tail.len() < 18 || tail[10..18] != [0; 8]);
            let result = FileEventLog::new(&path);
            if compatible {
                prop_assert_eq!(result.unwrap().read_all_valid().unwrap().len(), 3);
                prop_assert_eq!(fs::read(&path).unwrap(), full);
            } else {
                prop_assert!(result.is_err());
                prop_assert_eq!(fs::read(&path).unwrap(), with_tail);
            }
        }
    }
}
