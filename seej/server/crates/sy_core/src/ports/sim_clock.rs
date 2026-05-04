//! # ISimClock
//!
//! Tick source interface for the simulation.
//!
//! ## Rules
//! - The simulation NEVER accesses real system time directly
//! - Time is controlled entirely through this interface
//! - This enables deterministic replay and time manipulation

use sy_types::{SimTime, Tick};

/// Clock/tick source for the simulation.
/// Controls when and how fast simulation time progresses.
pub trait ISimClock: Send {
    /// Get the current simulation tick
    fn current_tick(&self) -> Tick;

    /// Get the current simulated time
    fn sim_time(&self) -> SimTime;

    /// Advance to the next tick, returns the new tick number
    fn advance(&mut self) -> Tick;

    /// Set the tick directly (for loading saved state)
    fn set_tick(&mut self, tick: Tick);

    /// Check if we should process another tick (for rate limiting)
    /// Returns true if enough "real" time has passed for the next tick.
    /// In headless/fast mode, this always returns true.
    fn should_tick(&self) -> bool;

    /// Wait until we should tick again (optional, for rate limiting)
    /// Default implementation does nothing (runs as fast as possible).
    fn wait_for_next_tick(&self) {}
}
