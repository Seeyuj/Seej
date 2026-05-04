//! # RNG
//!
//! IRng implementations with deterministic seeding.
//!
//! ## PCG32
//! A fast, deterministic PRNG suitable for games.
//! - Period: 2^64
//! - State: 64 bits
//! - Output: 32 bits per call

use sy_core::ports::IRng;
use sy_types::RngSeed;

/// PCG32 random number generator.
/// Deterministic and suitable for simulation use.
pub struct Pcg32Rng {
    seed: RngSeed,
    state: u64,
    increment: u64,
}

impl Pcg32Rng {
    /// PCG multiplier constant
    const MULTIPLIER: u64 = 6364136223846793005;

    /// Create a new PCG32 RNG with the given seed.
    pub fn new(seed: RngSeed) -> Self {
        let mut rng = Pcg32Rng {
            seed,
            state: 0,
            increment: (seed.as_u64() << 1) | 1, // Must be odd
        };
        // Warm up the generator
        rng.state = rng.state.wrapping_add(seed.as_u64());
        rng.next_u32();
        rng
    }

    /// Advance the internal state
    fn advance(&mut self) {
        self.state = self
            .state
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.increment);
    }
}

impl IRng for Pcg32Rng {
    fn seed(&self) -> RngSeed {
        self.seed
    }

    fn state(&self) -> u64 {
        self.state
    }

    fn restore(&mut self, state: u64) {
        self.state = state;
    }

    fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.advance();

        // XSH-RR output function
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sequence() {
        let mut rng1 = Pcg32Rng::new(RngSeed::new(42));
        let mut rng2 = Pcg32Rng::new(RngSeed::new(42));

        for _ in 0..100 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn different_seeds_different_sequences() {
        let mut rng1 = Pcg32Rng::new(RngSeed::new(42));
        let mut rng2 = Pcg32Rng::new(RngSeed::new(43));

        let seq1: Vec<u32> = (0..10).map(|_| rng1.next_u32()).collect();
        let seq2: Vec<u32> = (0..10).map(|_| rng2.next_u32()).collect();

        assert_ne!(seq1, seq2);
    }

    #[test]
    fn state_save_restore() {
        let mut rng = Pcg32Rng::new(RngSeed::new(42));

        // Generate some values
        for _ in 0..50 {
            rng.next_u32();
        }

        // Save state
        let saved_state = rng.state();

        // Generate more values
        let expected: Vec<u32> = (0..10).map(|_| rng.next_u32()).collect();

        // Restore and regenerate
        rng.restore(saved_state);
        let actual: Vec<u32> = (0..10).map(|_| rng.next_u32()).collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn range_bounds() {
        let mut rng = Pcg32Rng::new(RngSeed::new(12345));

        for _ in 0..1000 {
            let val = rng.range_i32(-10, 10);
            assert!((-10..=10).contains(&val));
        }
    }
}
