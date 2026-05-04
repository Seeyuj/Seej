//! # sy_testkit (NIV 3)
//!
//! Deterministic testing harness and mock implementations.
//!
//! ## Usage
//! ```ignore
//! use sy_testkit::scenarios::TestScenario;
//!
//! let sim = TestScenario::empty_world(42)
//!     .with_resource(0, 0, 100)
//!     .build();
//! ```

pub mod mocks;
pub mod scenarios;

// Re-exports
pub use mocks::{MockClock, MockEventLog, MockHasher, MockRng, MockWorldStore};
pub use scenarios::TestScenario;
