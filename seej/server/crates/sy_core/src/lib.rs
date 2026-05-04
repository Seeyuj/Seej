//! # sy_core (NIV 2)
//!
//! Pure simulation logic - the sanctuary.
//! This crate contains deterministic game logic with no I/O.
//!
//! ## Rules
//! - No async runtime
//! - No filesystem or network access
//! - No randomness from std (use injected IRng)
//! - No time from std (use injected ISimClock)
//! - No HashMap (use BTreeMap for deterministic iteration)
//!
//! ## Main types
//! - `World`: Complete simulation state
//! - `Simulation`: The engine that processes commands and runs ticks
//! - `replay`: Event replay for crash recovery
//! - `determinism`: Determinism verification tools
//! - `ports::*`: Interfaces for external dependencies

pub mod determinism;
pub mod ports;
pub mod replay;
pub mod sim;
pub mod world;

// Re-exports
pub use determinism::{
    compute_canonical_hash, run_deterministic, verify_determinism, Checkpoint,
    DeterministicRunConfig, DeterministicRunResult, ScheduledCommand, XxHasher,
};
pub use replay::{apply_event, replay_events};
pub use sim::Simulation;
pub use world::{Entity, World, Zone};
