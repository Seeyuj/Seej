//! # Commands
//!
//! Intentions/requests from internal systems.
//! Commands represent deterministic simulation intentions.
//!
//! Note: Phase 1 has NO player commands and no persistence/admin commands here.

use serde::{Deserialize, Serialize};
use sy_types::{EntityId, RngSeed, WorldPos, ZoneId};

/// Commands accepted by the pure simulation core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimCommand {
    /// Create a new world with the given seed
    CreateWorld(CreateWorldCmd),
    /// Advance simulation by one tick
    Tick,
    /// Advance simulation by N ticks
    TickN(u32),
    /// Spawn an entity in the world
    SpawnEntity(SpawnEntityCmd),
    /// Remove an entity from the world
    DespawnEntity(EntityId),
    /// Create a new zone
    CreateZone(CreateZoneCmd),
}

/// Command to create a new world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorldCmd {
    /// Human-readable name
    pub name: String,
    /// RNG seed for deterministic generation
    pub seed: RngSeed,
}

/// Command to spawn an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEntityCmd {
    /// Where to spawn
    pub position: WorldPos,
    /// What kind of entity
    pub kind: sy_types::EntityKind,
    /// Initial properties (key-value for flexibility)
    pub properties: EntityProperties,
}

/// Entity properties (simple key-value for Phase 1)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityProperties {
    /// Display name (optional)
    pub name: Option<String>,
    /// Amount/quantity (for resources)
    pub amount: Option<u32>,
    /// Health/durability (for creatures/structures)
    pub health: Option<u32>,
}

/// Command to create a new zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateZoneCmd {
    /// Zone identifier
    pub zone_id: ZoneId,
    /// Optional name
    pub name: Option<String>,
}
