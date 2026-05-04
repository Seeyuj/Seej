//! # IWorldStore
//!
//! Interface for world state snapshots and persistence.
//!
//! ## Purpose
//! - Save/load complete world snapshots
//! - Manage world metadata
//! - Support crash recovery

use sy_types::{SimResult, WorldMeta};

/// Serialized world state (opaque bytes).
pub type WorldSnapshot = Vec<u8>;

/// World persistence interface.
pub trait IWorldStore: Send {
    /// Check if a world exists in storage.
    fn exists(&self, world_id: &str) -> bool;

    /// List all available world IDs.
    fn list_worlds(&self) -> SimResult<Vec<String>>;

    /// Load world metadata (without loading full state).
    fn load_meta(&self, world_id: &str) -> SimResult<WorldMeta>;

    /// Save world metadata.
    fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()>;

    /// Load a complete world snapshot.
    fn load_snapshot(&self, world_id: &str) -> SimResult<WorldSnapshot>;

    /// Save a complete world snapshot.
    fn save_snapshot(&mut self, world_id: &str, snapshot: &WorldSnapshot) -> SimResult<()>;

    /// Delete a world from storage.
    fn delete_world(&mut self, world_id: &str) -> SimResult<()>;

    /// Get the path/location of a world's data (for logging/debugging).
    fn world_path(&self, world_id: &str) -> String;
}
