//! # IEventLog
//!
//! Write-Ahead Log interface for recording state transitions.
//!
//! ## Purpose
//! - Record all events for persistence and replay
//! - Enable crash recovery by replaying events
//! - Support deterministic replay for debugging
//!
//! ## Crash Recovery Contract
//! - Events are assigned monotonic `event_id` on append
//! - Recovery reads events with `event_id > snapshot.last_event_id`
//! - Invalid/partial records at end of WAL are detected and ignored

use sy_api::events::SimEvent;
use sy_types::{EventId, SimResult, Tick};

/// Event log interface for recording and replaying events.
pub trait IEventLog: Send {
    /// Append an event to the log.
    /// Assigns a monotonic event_id and returns the persisted event.
    /// Must be durable before returning (for crash recovery).
    fn append(&mut self, event: SimEvent) -> SimResult<SimEvent>;

    /// Append multiple events atomically.
    /// Assigns event_ids and returns the persisted events.
    fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>>;

    /// Read all valid events with event_id > from_id.
    /// Used for crash recovery.
    fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>>;

    /// Read all valid events in the log.
    /// Stops at first invalid/partial record (crash recovery).
    fn read_all_valid(&self) -> SimResult<Vec<SimEvent>>;

    /// Get the last event_id recorded in the log.
    fn last_event_id(&self) -> EventId;

    /// Get the last tick recorded in the log.
    fn last_tick(&self) -> Option<Tick>;

    /// Truncate the log after a given event_id (for recovery/branching).
    fn truncate_after(&mut self, event_id: EventId) -> SimResult<()>;

    /// Sync to disk (if buffered).
    fn sync(&mut self) -> SimResult<()>;

    /// Get the total number of valid events in the log.
    fn len(&self) -> usize;

    /// Check if the log is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
