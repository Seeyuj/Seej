//! # sy_infra (NIV 3)
//!
//! Real I/O implementations for the core's port interfaces.
//!
//! ## Phase 1 Modules
//! - `rng`: Deterministic RNG (PCG32)
//! - `clock`: Simulation clock implementations
//! - `store`: Persistence (filesystem, WAL)
//! - `observability`: Logging and metrics
//!
//! ## Phase 2+ Modules (disabled)
//! - `net`: Network - wire protocol mapping

pub mod clock;
pub mod hash;
// pub mod net;  // Phase 2 - Network layer
pub mod observability;
pub mod rng;
pub mod runtime;
pub mod snapshot;
pub mod store;

// Re-exports
pub use clock::{FixedStepClock, UnlimitedClock};
pub use hash::XxHasher;
pub use rng::Pcg32Rng;
pub use runtime::{load_recovered_world, load_snapshot_world, PersistentSimulation};
pub use store::{FileEventLog, FilesystemStore};
