//! Concrete hashing implementations for infrastructure and tools.

use sy_core::ports::{IStateHasher, StateHash};
use xxhash_rust::xxh64::Xxh64;

/// State hasher using xxHash64 for speed.
pub struct XxHasher {
    hasher: Xxh64,
}

impl XxHasher {
    pub fn new() -> Self {
        Self {
            hasher: Xxh64::new(0),
        }
    }
}

impl Default for XxHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl IStateHasher for XxHasher {
    fn reset(&mut self) {
        self.hasher.reset(0);
    }

    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(&self) -> StateHash {
        StateHash(self.hasher.digest())
    }
}
