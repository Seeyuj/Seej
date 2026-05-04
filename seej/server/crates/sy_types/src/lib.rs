//! # sy_types (NIV 0)
//!
//! Stable primitive types used across the entire platform.
//! This crate has minimal dependencies and defines the fundamental building blocks.
//!
//! ## Design principles
//! - All types are serializable (serde)
//! - All types are deterministic (no hidden state)
//! - Copy types where sensible for performance

use serde::{Deserialize, Serialize};

// ============================================================================
// Time & Tick
// ============================================================================

/// A simulation tick - the fundamental unit of time in the simulation.
/// The simulation advances tick by tick, deterministically.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Tick = Tick(0);

    #[inline]
    pub fn next(self) -> Tick {
        Tick(self.0.saturating_add(1))
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

/// Simulated time in the world (abstract units, not real-world seconds).
/// SimTime is derived from ticks but may have different granularity.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct SimTime {
    /// Total simulated time units elapsed
    pub units: u64,
}

impl SimTime {
    pub const ZERO: SimTime = SimTime { units: 0 };

    /// Create SimTime from a tick count (1 tick = 1 time unit by default)
    #[inline]
    pub fn from_ticks(tick: Tick) -> Self {
        SimTime { units: tick.0 }
    }

    #[inline]
    pub fn advance(&mut self, delta: u64) {
        self.units = self.units.saturating_add(delta);
    }
}

impl std::fmt::Display for SimTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ST:{}", self.units)
    }
}

// ============================================================================
// Identifiers
// ============================================================================

/// Unique identifier for an entity in the simulation.
/// Guaranteed stable across restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

impl EntityId {
    pub const INVALID: EntityId = EntityId(0);

    #[inline]
    pub fn new(id: u64) -> Self {
        EntityId(id)
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E{}", self.0)
    }
}

/// Unique identifier for a zone/region in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub u32);

impl ZoneId {
    pub const ORIGIN: ZoneId = ZoneId(0);

    #[inline]
    pub fn new(id: u32) -> Self {
        ZoneId(id)
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ZoneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Z{}", self.0)
    }
}

/// Seed for deterministic RNG. Must be explicitly provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RngSeed(pub u64);

impl RngSeed {
    #[inline]
    pub fn new(seed: u64) -> Self {
        RngSeed(seed)
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Unique identifier for an event in the WAL.
/// Monotonically increasing within a world. Used for crash recovery.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct EventId(pub u64);

impl EventId {
    pub const ZERO: EventId = EventId(0);

    #[inline]
    pub fn new(id: u64) -> Self {
        EventId(id)
    }

    #[inline]
    pub fn next(self) -> EventId {
        EventId(self.0.saturating_add(1))
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EV{}", self.0)
    }
}

// ============================================================================
// Spatial types
// ============================================================================

/// A position within a zone (local coordinates).
/// Uses i32 to allow negative coordinates if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub const ORIGIN: Position = Position { x: 0, y: 0, z: 0 };

    #[inline]
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Position { x, y, z }
    }

    /// Manhattan distance to another position
    #[inline]
    pub fn manhattan_distance(&self, other: &Position) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()) as u32
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// World coordinates: zone + local position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldPos {
    pub zone: ZoneId,
    pub pos: Position,
}

impl WorldPos {
    #[inline]
    pub fn new(zone: ZoneId, pos: Position) -> Self {
        WorldPos { zone, pos }
    }

    #[inline]
    pub fn origin() -> Self {
        WorldPos {
            zone: ZoneId::ORIGIN,
            pos: Position::ORIGIN,
        }
    }
}

impl std::fmt::Display for WorldPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.zone, self.pos)
    }
}

// ============================================================================
// Entity state
// ============================================================================

/// The kind/type of an entity (extensible via modules later)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EntityKind {
    /// A resource node (e.g., tree, rock, ore)
    Resource,
    /// A creature/NPC
    Creature,
    /// An item on the ground
    Item,
    /// A structure (building, etc.)
    Structure,
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityKind::Resource => write!(f, "Resource"),
            EntityKind::Creature => write!(f, "Creature"),
            EntityKind::Item => write!(f, "Item"),
            EntityKind::Structure => write!(f, "Structure"),
        }
    }
}

/// Lifecycle state of an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EntityState {
    /// Entity is active and will be processed
    #[default]
    Active,
    /// Entity is dormant (not processed but persists)
    Dormant,
    /// Entity is marked for removal
    Dead,
}

// ============================================================================
// World metadata
// ============================================================================

/// Metadata about a persisted world.
///
/// ## Invariant
/// All time references are in **simulated time** (Tick), never real-world time.
/// This ensures determinism: the core never accesses the system clock.
///
/// ## Crash Recovery
/// `last_event_id` is the cursor into the WAL. On recovery:
/// 1. Load snapshot (which contains state at `last_saved_tick`)
/// 2. Replay all events with `event_id > last_event_id`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMeta {
    /// Unique world identifier (derived from seed)
    pub world_id: String,
    /// Human-readable name
    pub name: String,
    /// RNG seed used for this world
    pub seed: RngSeed,
    /// Current tick
    pub current_tick: Tick,
    /// Current simulated time
    pub sim_time: SimTime,
    /// Tick at which the world was created (always Tick::ZERO for new worlds)
    pub created_tick: Tick,
    /// Tick at which the snapshot was taken
    pub snapshot_tick: Tick,
    /// Last event ID included in this snapshot (WAL cursor for recovery)
    pub last_event_id: EventId,
    /// Version of the format (for migrations)
    pub format_version: u32,
}

impl WorldMeta {
    pub const CURRENT_FORMAT_VERSION: u32 = 2; // Bumped for crash recovery support
}

// ============================================================================
// Result types
// ============================================================================

/// Standard result type for simulation operations
pub type SimResult<T> = Result<T, SimError>;

/// Errors that can occur in the simulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimError {
    /// Entity not found
    EntityNotFound(EntityId),
    /// Zone not found
    ZoneNotFound(ZoneId),
    /// Invalid operation
    InvalidOperation(String),
    /// Persistence error
    PersistenceError(String),
    /// World state is corrupted
    CorruptedState(String),
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimError::EntityNotFound(id) => write!(f, "Entity not found: {}", id),
            SimError::ZoneNotFound(id) => write!(f, "Zone not found: {}", id),
            SimError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            SimError::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            SimError::CorruptedState(msg) => write!(f, "Corrupted state: {}", msg),
        }
    }
}

impl std::error::Error for SimError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_ordering() {
        assert!(Tick(1) < Tick(2));
        assert_eq!(Tick(5).next(), Tick(6));
    }

    #[test]
    fn entity_id_validity() {
        assert!(!EntityId::INVALID.is_valid());
        assert!(EntityId::new(1).is_valid());
    }

    #[test]
    fn position_distance() {
        let a = Position::new(0, 0, 0);
        let b = Position::new(3, 4, 0);
        assert_eq!(a.manhattan_distance(&b), 7);
    }
}
