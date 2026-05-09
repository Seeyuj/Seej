//! # Ports
//!
//! Interfaces defining deterministic runtime needs of the core.

pub mod hasher;
pub mod rng;
pub mod sim_clock;

pub use hasher::{IStateHasher, StateHash};
pub use rng::IRng;
pub use sim_clock::ISimClock;
