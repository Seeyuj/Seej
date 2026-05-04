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
// pub mod net;  // Phase 2 - Network layer
pub mod observability;
pub mod rng;
pub mod store;

// Re-exports
pub use clock::{FixedStepClock, UnlimitedClock};
pub use rng::Pcg32Rng;
pub use store::{FileEventLog, FilesystemStore};
