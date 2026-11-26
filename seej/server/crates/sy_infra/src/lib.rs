//! # sy_infra (NIV 3)
//!
//! Real I/O implementations for the prototype's port interfaces.
//!
//! ## Modules
//! - `rng`: Deterministic RNG (PCG32)
//! - `clock`: Simulation clock implementations
//! - `store`: Persistence (filesystem, WAL)
//! - `observability`: Logging and metrics

pub mod clock;
pub mod hash;
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
