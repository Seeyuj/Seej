//! # World
//!
//! Root type representing the simulation state in memory.
//!
//! ## Design
//! - World is the complete simulation state
//! - Fully serializable (for snapshots)
//! - Contains: metadata, time, entities, zones
//! - NO I/O or side effects - pure data
//!
//! ## Determinism
//! Uses BTreeMap (not HashMap) for deterministic iteration order.
//! This is critical for reproducible hash computation and simulation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use sy_api::commands::EntityProperties;
use sy_types::{
    EntityId, EntityKind, EntityState, EventId, RngSeed, SimTime, Tick, WorldMeta, WorldPos, ZoneId,
};

// ============================================================================
// Entity
// ============================================================================

/// A complete entity in the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier
    pub id: EntityId,
    /// Entity type
    pub kind: EntityKind,
    /// Lifecycle state
    pub state: EntityState,
    /// World position
    pub position: WorldPos,
    /// Tick when created
    pub created_at: Tick,
    /// Properties
    pub properties: EntityProperties,
}

impl Entity {
    pub fn new(
        id: EntityId,
        kind: EntityKind,
        position: WorldPos,
        created_at: Tick,
        properties: EntityProperties,
    ) -> Self {
        Entity {
            id,
            kind,
            state: EntityState::Active,
            position,
            created_at,
            properties,
        }
    }

    pub fn is_active(&self) -> bool {
        self.state == EntityState::Active
    }

    pub fn is_dead(&self) -> bool {
        self.state == EntityState::Dead
    }
}

// ============================================================================
// Zone
// ============================================================================

/// A zone/region in the world.
/// Zones are the unit of spatial partitioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    /// Zone identifier
    pub id: ZoneId,
    /// Optional name
    pub name: Option<String>,
    /// Whether this zone is currently loaded/active
    pub loaded: bool,
    /// Entity IDs in this zone
    pub entities: Vec<EntityId>,
}

impl Zone {
    pub fn new(id: ZoneId, name: Option<String>) -> Self {
        Zone {
            id,
            name,
            loaded: true,
            entities: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, id: EntityId) {
        if !self.entities.contains(&id) {
            self.entities.push(id);
        }
    }

    pub fn remove_entity(&mut self, id: EntityId) {
        self.entities.retain(|&e| e != id);
    }
}

// ============================================================================
// World
// ============================================================================

/// The complete world state.
/// This is the root of all simulation data.
///
/// ## Determinism Invariant
/// Uses BTreeMap to guarantee iteration order by key.
/// Never use HashMap in simulation state!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    /// World metadata
    pub meta: WorldMeta,
    /// Current simulation tick
    pub current_tick: Tick,
    /// Current simulated time
    pub sim_time: SimTime,
    /// RNG state (for determinism)
    pub rng_state: u64,
    /// Next entity ID to assign
    pub next_entity_id: u64,
    /// All entities, indexed by ID (BTreeMap for deterministic order)
    pub entities: BTreeMap<EntityId, Entity>,
    /// All zones, indexed by ID (BTreeMap for deterministic order)
    pub zones: BTreeMap<ZoneId, Zone>,
}

impl World {
    /// Create a new empty world.
    ///
    /// ## Note
    /// No system clock access - all timestamps are in simulated ticks.
    pub fn new(name: String, seed: RngSeed) -> Self {
        let world_id = format!("world_{}", seed.as_u64());

        let meta = WorldMeta {
            world_id,
            name,
            seed,
            current_tick: Tick::ZERO,
            sim_time: SimTime::ZERO,
            created_tick: Tick::ZERO,     // World created at tick 0
            snapshot_tick: Tick::ZERO,    // Will be updated on save
            last_event_id: EventId::ZERO, // No events yet
            format_version: WorldMeta::CURRENT_FORMAT_VERSION,
        };

        let mut world = World {
            meta,
            current_tick: Tick::ZERO,
            sim_time: SimTime::ZERO,
            rng_state: seed.as_u64(),
            next_entity_id: 1, // 0 is reserved for INVALID
            entities: BTreeMap::new(),
            zones: BTreeMap::new(),
        };

        // Create the origin zone by default
        world.zones.insert(
            ZoneId::ORIGIN,
            Zone::new(ZoneId::ORIGIN, Some("Origin".to_string())),
        );

        world
    }

    /// Get the world ID.
    pub fn id(&self) -> &str {
        &self.meta.world_id
    }

    /// Get the world name.
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    /// Get the RNG seed.
    pub fn seed(&self) -> RngSeed {
        self.meta.seed
    }

    // ========================================================================
    // Entity management
    // ========================================================================

    /// Allocate a new entity ID.
    pub fn allocate_entity_id(&mut self) -> EntityId {
        let id = EntityId::new(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    /// Add an entity to the world.
    pub fn add_entity(&mut self, entity: Entity) {
        let zone_id = entity.position.zone;
        let entity_id = entity.id;

        self.entities.insert(entity_id, entity);

        // Add to zone
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.add_entity(entity_id);
        }
    }

    /// Remove an entity from the world.
    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity> {
        if let Some(entity) = self.entities.remove(&id) {
            // Remove from zone
            if let Some(zone) = self.zones.get_mut(&entity.position.zone) {
                zone.remove_entity(id);
            }
            Some(entity)
        } else {
            None
        }
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Get a mutable entity by ID.
    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// Get all active entities.
    pub fn active_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values().filter(|e| e.is_active())
    }

    /// Count entities by state.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn active_entity_count(&self) -> usize {
        self.entities.values().filter(|e| e.is_active()).count()
    }

    // ========================================================================
    // Zone management
    // ========================================================================

    /// Add a zone to the world.
    pub fn add_zone(&mut self, zone: Zone) {
        self.zones.insert(zone.id, zone);
    }

    /// Get a zone by ID.
    pub fn get_zone(&self, id: ZoneId) -> Option<&Zone> {
        self.zones.get(&id)
    }

    /// Get a mutable zone by ID.
    pub fn get_zone_mut(&mut self, id: ZoneId) -> Option<&mut Zone> {
        self.zones.get_mut(&id)
    }

    /// Check if a zone exists.
    pub fn has_zone(&self, id: ZoneId) -> bool {
        self.zones.contains_key(&id)
    }

    /// Get all zone IDs.
    pub fn zone_ids(&self) -> impl Iterator<Item = ZoneId> + '_ {
        self.zones.keys().copied()
    }

    /// Count zones.
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    // ========================================================================
    // Time
    // ========================================================================

    /// Advance to the next tick.
    pub fn advance_tick(&mut self) {
        self.current_tick = self.current_tick.next();
        self.sim_time = SimTime::from_ticks(self.current_tick);
        self.meta.current_tick = self.current_tick;
        self.meta.sim_time = self.sim_time;
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Serialize to bytes (for snapshots).
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode_serialize(self).map_err(|e| e.to_string())
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        bincode_deserialize(data).map_err(|e| e.to_string())
    }
}

// Simple bincode-like serialization using serde_json for now
// (In production, we'd use actual bincode for efficiency)
fn bincode_serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, serde::de::value::Error> {
    // Using JSON as a simple serialization format for Phase 1
    // Can be replaced with bincode later for efficiency
    serde_json::to_vec(value).map_err(|_| serde::de::Error::custom("serialization failed"))
}

fn bincode_deserialize<'a, T: Deserialize<'a>>(
    data: &'a [u8],
) -> Result<T, serde::de::value::Error> {
    serde_json::from_slice(data).map_err(|_| serde::de::Error::custom("deserialization failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world() {
        let world = World::new("Test World".to_string(), RngSeed::new(42));
        assert_eq!(world.name(), "Test World");
        assert_eq!(world.current_tick, Tick::ZERO);
        assert!(world.has_zone(ZoneId::ORIGIN));
    }

    #[test]
    fn add_remove_entity() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let id = world.allocate_entity_id();
        let entity = Entity::new(
            id,
            EntityKind::Resource,
            WorldPos::origin(),
            Tick::ZERO,
            EntityProperties::default(),
        );

        world.add_entity(entity);
        assert_eq!(world.entity_count(), 1);
        assert!(world.get_entity(id).is_some());

        world.remove_entity(id);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn world_serialization() {
        let world = World::new("Serialize Test".to_string(), RngSeed::new(123));
        let bytes = world.to_bytes().unwrap();
        let restored = World::from_bytes(&bytes).unwrap();
        assert_eq!(restored.name(), world.name());
        assert_eq!(restored.seed().as_u64(), world.seed().as_u64());
    }
}
