//! # Clock
//!
//! ISimClock implementations for the simulation.
//!
//! ## Implementations
//! - `UnlimitedClock`: Runs as fast as possible (for headless/batch)
//! - `FixedStepClock`: Runs at a fixed tick rate (for real-time)

use std::time::{Duration, Instant};

use sy_core::ports::ISimClock;
use sy_types::{SimTime, Tick};

/// Clock that runs as fast as possible (no rate limiting).
/// Used for headless simulation and testing.
pub struct UnlimitedClock {
    current_tick: Tick,
}

impl UnlimitedClock {
    pub fn new() -> Self {
        UnlimitedClock {
            current_tick: Tick::ZERO,
        }
    }
}

impl Default for UnlimitedClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ISimClock for UnlimitedClock {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }

    fn sim_time(&self) -> SimTime {
        SimTime::from_ticks(self.current_tick)
    }

    fn advance(&mut self) -> Tick {
        self.current_tick = self.current_tick.next();
        self.current_tick
    }

    fn set_tick(&mut self, tick: Tick) {
        self.current_tick = tick;
    }

    fn should_tick(&self) -> bool {
        true // Always ready
    }
}

/// Clock that runs at a fixed tick rate.
/// Useful for real-time simulation or debugging.
///
/// ## Note on `Instant::now()`
/// This clock uses `Instant::now()` for **throttling only** (rate limiting).
/// The simulation tick itself is always incremented deterministically.
/// This does NOT break determinism because:
/// - The tick value is not derived from real time
/// - `should_tick()` and `wait_for_next_tick()` are optional rate-limiting helpers
/// - The core simulation never depends on wall-clock time
pub struct FixedStepClock {
    current_tick: Tick,
    tick_duration: Duration,
    last_tick_time: Option<Instant>,
}

impl FixedStepClock {
    /// Create a new fixed-step clock with the given ticks per second.
    pub fn new(ticks_per_second: u32) -> Self {
        let tick_duration = Duration::from_secs_f64(1.0 / ticks_per_second as f64);
        FixedStepClock {
            current_tick: Tick::ZERO,
            tick_duration,
            last_tick_time: None,
        }
    }

    /// Create a clock that runs at 20 TPS (typical game tick rate).
    pub fn default_rate() -> Self {
        Self::new(20)
    }
}

impl ISimClock for FixedStepClock {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }

    fn sim_time(&self) -> SimTime {
        SimTime::from_ticks(self.current_tick)
    }

    fn advance(&mut self) -> Tick {
        self.current_tick = self.current_tick.next();
        self.last_tick_time = Some(Instant::now());
        self.current_tick
    }

    fn set_tick(&mut self, tick: Tick) {
        self.current_tick = tick;
        self.last_tick_time = None;
    }

    fn should_tick(&self) -> bool {
        match self.last_tick_time {
            None => true,
            Some(last) => last.elapsed() >= self.tick_duration,
        }
    }

    fn wait_for_next_tick(&self) {
        if let Some(last) = self.last_tick_time {
            let elapsed = last.elapsed();
            if elapsed < self.tick_duration {
                std::thread::sleep(self.tick_duration - elapsed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_clock_advances() {
        let mut clock = UnlimitedClock::new();
        assert_eq!(clock.current_tick(), Tick::ZERO);

        clock.advance();
        assert_eq!(clock.current_tick(), Tick(1));

        clock.advance();
        assert_eq!(clock.current_tick(), Tick(2));
    }

    #[test]
    fn clock_set_tick() {
        let mut clock = UnlimitedClock::new();
        clock.set_tick(Tick(100));
        assert_eq!(clock.current_tick(), Tick(100));
    }

    #[test]
    fn fixed_step_creation() {
        let clock = FixedStepClock::new(60);
        assert_eq!(clock.current_tick(), Tick::ZERO);
    }
}
