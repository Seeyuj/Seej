//! # IRng
//!
//! Deterministic random number generator interface.
//! Must be injected to ensure reproducibility.
//!
//! ## Rules
//! - NEVER use std::rand or any non-injected RNG
//! - RNG state must be serializable for replay
//! - Same seed + same sequence of calls = same results

use sy_types::RngSeed;

/// Deterministic RNG interface.
/// Implementations must be fully deterministic given the same seed.
pub trait IRng: Send {
    /// Get the current seed (for serialization)
    fn seed(&self) -> RngSeed;

    /// Get current internal state for checkpointing
    fn state(&self) -> u64;

    /// Restore from a saved state
    fn restore(&mut self, state: u64);

    /// Generate a random u32
    fn next_u32(&mut self) -> u32;

    /// Generate a random u64
    fn next_u64(&mut self) -> u64;

    /// Generate a random f32 in [0.0, 1.0)
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Generate a random f64 in [0.0, 1.0)
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Generate a random i32 in [min, max] (inclusive)
    fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u32 + 1;
        let val = self.next_u32() % range;
        min + val as i32
    }

    /// Generate a random u32 in [min, max] (inclusive)
    fn range_u32(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = max - min + 1;
        min + (self.next_u32() % range)
    }

    /// Return true with probability `chance` (0.0 to 1.0)
    fn chance(&mut self, probability: f32) -> bool {
        self.next_f32() < probability
    }

    /// Pick a random element from a slice
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            let idx = self.next_u32() as usize % slice.len();
            Some(&slice[idx])
        }
    }
}
