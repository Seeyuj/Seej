//! # IStateHasher
//!
//! Interface for computing state checksums to verify determinism.
//!
//! ## Purpose
//! - Compute deterministic hashes of world state
//! - Verify that two runs with same inputs produce same outputs
//! - Detect state drift or corruption

/// State hash (64-bit for speed, not cryptographic security).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateHash(pub u64);

impl StateHash {
    pub const ZERO: StateHash = StateHash(0);

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for StateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Interface for hashing world state.
pub trait IStateHasher: Send {
    /// Start a new hash computation.
    fn reset(&mut self);

    /// Feed bytes into the hash.
    fn update(&mut self, data: &[u8]);

    /// Finalize and return the hash.
    fn finalize(&self) -> StateHash;

    /// Convenience: hash a single byte slice.
    fn hash_bytes(&mut self, data: &[u8]) -> StateHash {
        self.reset();
        self.update(data);
        self.finalize()
    }
}
