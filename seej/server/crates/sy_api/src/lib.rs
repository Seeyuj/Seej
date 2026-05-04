//! # sy_api (NIV 1)
//!
//! Internal API definitions: commands, events, and errors.
//! This is the stable language used within the platform.
//!
//! ## Modules
//! - `commands`: Intentions/requests to the simulation
//! - `events`: Facts representing state changes
//! - `errors`: Typed API errors
//! - `validation`: Input validation

pub mod commands;
pub mod errors;
pub mod events;
pub mod validation;

// Re-exports for convenience
pub use commands::*;
pub use errors::*;
pub use events::*;
