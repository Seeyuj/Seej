//! # Ports
//!
//! Interfaces (traits) defining the needs of the core.
//! These are implemented by sy_infra or sy_testkit.
//!
//! ## Architecture
//! The core defines what it NEEDS (ports/interfaces).
//! Infrastructure provides IMPLEMENTATIONS of those needs.
//! This allows:
//! - Testing with mocks
//! - Swapping implementations
//! - Keeping core pure and I/O-free

pub mod event_log;
pub mod hasher;
pub mod rng;
pub mod sim_clock;
pub mod store;

// Re-exports
pub use event_log::IEventLog;
pub use hasher::{IStateHasher, StateHash};
pub use rng::IRng;
pub use sim_clock::ISimClock;
pub use store::{IWorldStore, WorldSnapshot};
