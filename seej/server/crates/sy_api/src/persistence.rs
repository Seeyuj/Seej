//! Persistence interfaces shared by infrastructure and tools.

use crate::events::SimEvent;
use sy_types::{EventId, SimResult, Tick, WorldMeta};

/// Serialized world state (opaque bytes).
pub type WorldSnapshot = Vec<u8>;

/// Persistent storage state for a world id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldStorageStatus {
    /// No durable storage exists for this world id.
    Absent,
    /// Snapshot and metadata are present. The world id must not be created again.
    Complete,
    /// Some durable storage exists, but it is not a coherent persisted world.
    Incomplete { reason: String },
}

/// World persistence interface.
pub trait IWorldStore: Send {
    fn exists(&self, world_id: &str) -> bool;
    fn storage_status(&self, world_id: &str) -> SimResult<WorldStorageStatus> {
        if self.exists(world_id) {
            Ok(WorldStorageStatus::Complete)
        } else {
            Ok(WorldStorageStatus::Absent)
        }
    }
    fn list_worlds(&self) -> SimResult<Vec<String>>;
    fn load_meta(&self, world_id: &str) -> SimResult<WorldMeta>;
    fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()>;
    fn load_snapshot(&self, world_id: &str) -> SimResult<WorldSnapshot>;
    fn save_snapshot(&mut self, world_id: &str, snapshot: &WorldSnapshot) -> SimResult<()>;
    fn delete_world(&mut self, world_id: &str) -> SimResult<()>;
    fn world_path(&self, world_id: &str) -> String;
}

/// Event log interface for recording and replaying state-transition events.
pub trait IEventLog: Send {
    fn append(&mut self, event: SimEvent) -> SimResult<SimEvent>;
    fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>>;
    fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>>;
    fn read_all_valid(&self) -> SimResult<Vec<SimEvent>>;
    fn last_event_id(&self) -> EventId;
    fn last_tick(&self) -> Option<Tick>;

    #[deprecated(
        note = "Phase 1 rewrite helper: it may reassign event_id values. Do not use for automated compaction."
    )]
    fn truncate_after(&mut self, event_id: EventId) -> SimResult<()>;

    fn sync(&mut self) -> SimResult<()>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
