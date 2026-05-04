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

use sy_api::events::SimEvent;
use sy_core::ports::IEventLog;
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
}

impl FileEventLog {
    /// Create or open a WAL file at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> SimResult<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SimError::PersistenceError(format!("Failed to create WAL dir: {}", e))
            })?;
        }

        let mut log = FileEventLog {
            path,
            writer: None,
            next_event_id: 1,
            last_tick: None,
            total_events: 0,
        };

        // Scan existing WAL to recover state
        log.recover()?;

        info!(
            "Initialized WAL with {} events, next_event_id={}",
            log.total_events, log.next_event_id
        );

        Ok(log)
    }

    /// Scan existing WAL file and recover state.
    /// Stops at first invalid/partial record.
    fn recover(&mut self) -> SimResult<()> {
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
                Ok(event) => {
                    self.next_event_id = event.event_id.as_u64() + 1;
                    self.last_tick = Some(event.tick);
                    self.total_events += 1;
                    last_valid_offset = reader.stream_position().map_err(|e| {
                        SimError::PersistenceError(format!("Stream position error: {}", e))
                    })?;
                    offset = last_valid_offset;
                }
                Err(e) => {
                    warn!("WAL recovery stopped at offset {}: {}", offset, e);
                    break;
                }
            }
        }

        // If there's garbage at the end, truncate it
        if last_valid_offset < file_len && last_valid_offset > 0 {
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

    /// Read a single record at the given offset.
    fn read_record_at(&self, reader: &mut BufReader<File>, offset: u64) -> SimResult<SimEvent> {
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

        // Deserialize event data
        let data: sy_api::events::EventData = serde_json::from_slice(&payload)
            .map_err(|e| SimError::PersistenceError(format!("Deserialize event failed: {}", e)))?;

        Ok(SimEvent::with_id(EventId::new(event_id), Tick(tick), data))
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

    /// Write a single event to the WAL.
    fn write_event(&mut self, mut event: SimEvent) -> SimResult<SimEvent> {
        // Assign event_id
        event.event_id = EventId::new(self.next_event_id);
        self.next_event_id += 1;

        // Serialize payload
        let payload = serde_json::to_vec(&event.data)
            .map_err(|e| SimError::PersistenceError(format!("Serialize event failed: {}", e)))?;

        let payload_len = payload.len() as u32;
        let event_id = event.event_id.as_u64();
        let tick = event.tick.as_u64();

        // Compute CRC
        let crc = self.compute_crc(WAL_VERSION, payload_len, event_id, tick, &payload);

        // Ensure writer is open
        if self.writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|e| SimError::PersistenceError(format!("Failed to open WAL: {}", e)))?;
            self.writer = Some(BufWriter::new(file));
        }

        let writer = self.writer.as_mut().unwrap();

        // Write record
        writer
            .write_u32::<LittleEndian>(WAL_MAGIC)
            .map_err(|e| SimError::PersistenceError(format!("Write magic failed: {}", e)))?;
        writer
            .write_u16::<LittleEndian>(WAL_VERSION)
            .map_err(|e| SimError::PersistenceError(format!("Write version failed: {}", e)))?;
        writer
            .write_u32::<LittleEndian>(payload_len)
            .map_err(|e| SimError::PersistenceError(format!("Write length failed: {}", e)))?;
        writer
            .write_u64::<LittleEndian>(event_id)
            .map_err(|e| SimError::PersistenceError(format!("Write event_id failed: {}", e)))?;
        writer
            .write_u64::<LittleEndian>(tick)
            .map_err(|e| SimError::PersistenceError(format!("Write tick failed: {}", e)))?;
        writer
            .write_all(&payload)
            .map_err(|e| SimError::PersistenceError(format!("Write payload failed: {}", e)))?;
        writer
            .write_u32::<LittleEndian>(crc)
            .map_err(|e| SimError::PersistenceError(format!("Write CRC failed: {}", e)))?;

        // Flush and sync
        writer
            .flush()
            .map_err(|e| SimError::PersistenceError(format!("Flush failed: {}", e)))?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|e| SimError::PersistenceError(format!("Sync failed: {}", e)))?;

        self.last_tick = Some(event.tick);
        self.total_events += 1;

        Ok(event)
    }

    /// Read all valid events from the WAL file.
    fn read_all_events(&self) -> SimResult<Vec<SimEvent>> {
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
                Ok(event) => {
                    offset = reader.stream_position().map_err(|e| {
                        SimError::PersistenceError(format!("Stream position error: {}", e))
                    })?;
                    events.push(event);
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
        for event in events {
            persisted.push(self.write_event(event)?);
        }
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
    use std::env::temp_dir;
    use std::sync::atomic::{AtomicU64, Ordering};
    use sy_api::events::EventData;
    use sy_types::RngSeed;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_wal() -> FileEventLog {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = temp_dir().join(format!("seeyuj_wal_test_{}_{}.wal", std::process::id(), id));
        // Clean up any existing file
        let _ = fs::remove_file(&path);
        FileEventLog::new(&path).unwrap()
    }

    #[test]
    fn append_and_read() {
        let mut log = temp_wal();

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
    fn event_id_is_monotonic() {
        let mut log = temp_wal();

        for i in 1..=10 {
            let event = SimEvent::new(
                Tick(i),
                EventData::TickProcessed {
                    tick: Tick(i),
                    sim_time: sy_types::SimTime { units: i },
                    entities_processed: 0,
                },
            );
            let persisted = log.append(event).unwrap();
            assert_eq!(persisted.event_id.as_u64(), i);
        }

        assert_eq!(log.last_event_id(), EventId::new(10));
    }

    #[test]
    fn read_from_event_id() {
        let mut log = temp_wal();

        for i in 1..=10 {
            let event = SimEvent::new(
                Tick(i),
                EventData::TickProcessed {
                    tick: Tick(i),
                    sim_time: sy_types::SimTime { units: i },
                    entities_processed: 0,
                },
            );
            log.append(event).unwrap();
        }

        let events = log.read_from_event_id(EventId::new(5)).unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].event_id, EventId::new(6));
    }

    #[test]
    fn recovery_after_reopen() {
        let path = temp_dir().join(format!("seeyuj_wal_recovery_{}.wal", std::process::id()));
        let _ = fs::remove_file(&path);

        // Write some events
        {
            let mut log = FileEventLog::new(&path).unwrap();
            for i in 1..=5 {
                let event = SimEvent::new(
                    Tick(i),
                    EventData::TickProcessed {
                        tick: Tick(i),
                        sim_time: sy_types::SimTime { units: i },
                        entities_processed: 0,
                    },
                );
                log.append(event).unwrap();
            }
        }

        // Reopen and verify
        {
            let log = FileEventLog::new(&path).unwrap();
            assert_eq!(log.len(), 5);
            assert_eq!(log.last_event_id(), EventId::new(5));

            let events = log.read_all_valid().unwrap();
            assert_eq!(events.len(), 5);
        }

        // Clean up
        let _ = fs::remove_file(&path);
    }
}
