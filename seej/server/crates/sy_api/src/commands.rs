//! # Commands
//!
//! Intentions/requests from internal systems.
//! Commands represent "what the system wants to do".
//!
//! Note: Phase 1 has NO player commands. Only internal/admin commands.

use serde::{Deserialize, Serialize};
use sy_types::{EntityId, RngSeed, WorldPos, ZoneId};

/// Commands that can be issued to the simulation.
/// These are internal commands, not player-facing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Create a new world with the given seed
    CreateWorld(CreateWorldCmd),
    /// Load an existing world from storage
    LoadWorld(LoadWorldCmd),
    /// Save the current world state
    SaveWorld,
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
    /// Shutdown the server gracefully
    Shutdown,
}

/// Command to create a new world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorldCmd {
    /// Human-readable name
    pub name: String,
    /// RNG seed for deterministic generation
    pub seed: RngSeed,
}

/// Command to load an existing world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadWorldCmd {
    /// World identifier (path or UUID)
    pub world_id: String,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
