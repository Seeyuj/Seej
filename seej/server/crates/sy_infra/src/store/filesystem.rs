//! # Filesystem Store
//!
//! Simple filesystem-based world persistence.
//! Stores world snapshots as JSON files.
//!
//! ## Crash Safety
//! - Snapshots use atomic write (tmp + fsync + rename)
//! - Directory is synced after rename (POSIX)

#[cfg(unix)]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sy_core::ports::{IWorldStore, WorldSnapshot};
use sy_types::{SimError, SimResult, WorldMeta};
use tracing::{debug, info, warn};

/// Filesystem-based world store.
///
/// Directory structure:
/// ```text
/// {base_path}/
///   worlds/
///     {world_id}/
///       meta.json      - World metadata
///       snapshot.json  - World state snapshot
///       events/        - Event log directory
/// ```
pub struct FilesystemStore {
    base_path: PathBuf,
}

impl FilesystemStore {
    /// Create a new filesystem store at the given base path.
    pub fn new<P: AsRef<Path>>(base_path: P) -> SimResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Create base directories
        let worlds_dir = base_path.join("worlds");
        fs::create_dir_all(&worlds_dir).map_err(|e| {
            SimError::PersistenceError(format!("Failed to create worlds dir: {}", e))
        })?;

        info!("Initialized filesystem store at {:?}", base_path);

        Ok(FilesystemStore { base_path })
    }

    /// Get the directory for a specific world.
    fn world_dir(&self, world_id: &str) -> PathBuf {
        self.base_path.join("worlds").join(world_id)
    }

    /// Get the metadata file path for a world.
    fn meta_path(&self, world_id: &str) -> PathBuf {
        self.world_dir(world_id).join("meta.json")
    }

    /// Get the snapshot file path for a world.
    fn snapshot_path(&self, world_id: &str) -> PathBuf {
        self.world_dir(world_id).join("snapshot.json")
    }

    /// Ensure the world directory exists.
    fn ensure_world_dir(&self, world_id: &str) -> SimResult<()> {
        let dir = self.world_dir(world_id);
        fs::create_dir_all(&dir).map_err(|e| {
            SimError::PersistenceError(format!("Failed to create world dir: {}", e))
        })?;
        Ok(())
    }

    /// Get the base path.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get the events directory for a world.
    pub fn events_dir(&self, world_id: &str) -> PathBuf {
        self.world_dir(world_id).join("events")
    }
}

impl IWorldStore for FilesystemStore {
    fn exists(&self, world_id: &str) -> bool {
        self.meta_path(world_id).exists()
    }

    fn list_worlds(&self) -> SimResult<Vec<String>> {
        let worlds_dir = self.base_path.join("worlds");

        if !worlds_dir.exists() {
            return Ok(Vec::new());
        }

        let mut worlds = Vec::new();

        let entries = fs::read_dir(&worlds_dir)
            .map_err(|e| SimError::PersistenceError(format!("Failed to read worlds dir: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                SimError::PersistenceError(format!("Failed to read dir entry: {}", e))
            })?;

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    // Check if it has a meta.json
                    if self.exists(name) {
                        worlds.push(name.to_string());
                    }
                }
            }
        }

        Ok(worlds)
    }

    fn load_meta(&self, world_id: &str) -> SimResult<WorldMeta> {
        let path = self.meta_path(world_id);

        if !path.exists() {
            return Err(SimError::PersistenceError(format!(
                "World not found: {}",
                world_id
            )));
        }

        let mut file = File::open(&path)
            .map_err(|e| SimError::PersistenceError(format!("Failed to open meta: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| SimError::PersistenceError(format!("Failed to read meta: {}", e)))?;

        let meta: WorldMeta = serde_json::from_str(&contents)
            .map_err(|e| SimError::PersistenceError(format!("Failed to parse meta: {}", e)))?;

        debug!("Loaded metadata for world {}", world_id);
        Ok(meta)
    }

    fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()> {
        self.ensure_world_dir(&meta.world_id)?;

        let path = self.meta_path(&meta.world_id);

        let contents = serde_json::to_string_pretty(meta)
            .map_err(|e| SimError::PersistenceError(format!("Failed to serialize meta: {}", e)))?;

        let mut file = File::create(&path).map_err(|e| {
            SimError::PersistenceError(format!("Failed to create meta file: {}", e))
        })?;

        file.write_all(contents.as_bytes())
            .map_err(|e| SimError::PersistenceError(format!("Failed to write meta: {}", e)))?;

        file.sync_all()
            .map_err(|e| SimError::PersistenceError(format!("Failed to sync meta: {}", e)))?;

        debug!("Saved metadata for world {}", meta.world_id);
        Ok(())
    }

    fn load_snapshot(&self, world_id: &str) -> SimResult<WorldSnapshot> {
        let path = self.snapshot_path(world_id);

        if !path.exists() {
            return Err(SimError::PersistenceError(format!(
                "Snapshot not found: {}",
                world_id
            )));
        }

        let mut file = File::open(&path)
            .map_err(|e| SimError::PersistenceError(format!("Failed to open snapshot: {}", e)))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| SimError::PersistenceError(format!("Failed to read snapshot: {}", e)))?;

        info!(
            "Loaded snapshot for world {} ({} bytes)",
            world_id,
            contents.len()
        );
        Ok(contents)
    }

    fn save_snapshot(&mut self, world_id: &str, snapshot: &WorldSnapshot) -> SimResult<()> {
        self.ensure_world_dir(world_id)?;

        let path = self.snapshot_path(world_id);

        // Step 1: Write to temp file
        let temp_path = path.with_extension("json.tmp");

        let mut file = File::create(&temp_path).map_err(|e| {
            SimError::PersistenceError(format!("Failed to create temp file: {}", e))
        })?;

        file.write_all(snapshot)
            .map_err(|e| SimError::PersistenceError(format!("Failed to write snapshot: {}", e)))?;

        // Step 2: fsync the temp file
        file.sync_all()
            .map_err(|e| SimError::PersistenceError(format!("Failed to sync snapshot: {}", e)))?;

        // Step 3: Atomic rename (atomic on POSIX, best-effort on Windows)
        fs::rename(&temp_path, &path)
            .map_err(|e| SimError::PersistenceError(format!("Failed to rename snapshot: {}", e)))?;

        // Step 4: fsync the directory (ensures rename is durable on POSIX)
        #[cfg(unix)]
        {
            let dir = self.world_dir(world_id);
            if let Ok(dir_file) = OpenOptions::new().read(true).open(&dir) {
                let _ = dir_file.sync_all();
            }
        }

        info!(
            "Saved snapshot for world {} ({} bytes)",
            world_id,
            snapshot.len()
        );
        Ok(())
    }

    fn delete_world(&mut self, world_id: &str) -> SimResult<()> {
        let dir = self.world_dir(world_id);

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| {
                SimError::PersistenceError(format!("Failed to delete world: {}", e))
            })?;
            info!("Deleted world {}", world_id);
        } else {
            warn!("World {} not found for deletion", world_id);
        }

        Ok(())
    }

    fn world_path(&self, world_id: &str) -> String {
        self.world_dir(world_id).to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use sy_types::{EventId, RngSeed, SimTime, Tick};

    fn temp_store() -> FilesystemStore {
        let path = temp_dir().join(format!("seeyuj_test_{}", std::process::id()));
        FilesystemStore::new(&path).unwrap()
    }

    #[test]
    fn create_store() {
        let store = temp_store();
        assert!(store.base_path().exists());
    }

    #[test]
    fn save_load_meta() {
        let mut store = temp_store();

        let meta = WorldMeta {
            world_id: "test_world".to_string(),
            name: "Test World".to_string(),
            seed: RngSeed::new(42),
            current_tick: Tick(100),
            sim_time: SimTime { units: 100 },
            created_tick: Tick::ZERO,
            snapshot_tick: Tick(100),
            last_event_id: EventId::new(50),
            format_version: WorldMeta::CURRENT_FORMAT_VERSION,
        };

        store.save_meta(&meta).unwrap();
        assert!(store.exists("test_world"));

        let loaded = store.load_meta("test_world").unwrap();
        assert_eq!(loaded.world_id, meta.world_id);
        assert_eq!(loaded.name, meta.name);
        assert_eq!(loaded.snapshot_tick, meta.snapshot_tick);
        assert_eq!(loaded.last_event_id, meta.last_event_id);
    }

    #[test]
    fn save_load_snapshot() {
        let mut store = temp_store();

        // First save meta
        let meta = WorldMeta {
            world_id: "snapshot_test".to_string(),
            name: "Snapshot Test".to_string(),
            seed: RngSeed::new(1),
            current_tick: Tick::ZERO,
            sim_time: SimTime::ZERO,
            created_tick: Tick::ZERO,
            snapshot_tick: Tick::ZERO,
            last_event_id: EventId::ZERO,
            format_version: WorldMeta::CURRENT_FORMAT_VERSION,
        };
        store.save_meta(&meta).unwrap();

        // Save snapshot
        let snapshot = b"test snapshot data".to_vec();
        store.save_snapshot("snapshot_test", &snapshot).unwrap();

        // Load snapshot
        let loaded = store.load_snapshot("snapshot_test").unwrap();
        assert_eq!(loaded, snapshot);
    }
}
