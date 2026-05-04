//! # Store
//!
//! Persistence implementations for world state and event logs.
//!
//! ## Phase 1 Implementation
//! - Filesystem-based storage (simple, no external deps)
//! - JSON serialization (readable, debuggable)
//! - WAL for crash recovery

pub mod filesystem;
pub mod migrations;
pub mod wal;

// Re-exports
pub use filesystem::FilesystemStore;
pub use wal::FileEventLog;
