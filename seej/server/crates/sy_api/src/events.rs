//! # Events
//!
//! Facts representing state changes.
//! Events are neutral descriptions of what happened.
//!
//! All events are:
//! - Immutable facts (what happened)
//! - Serializable (for WAL/replay)
//! - Timestamped with the tick when they occurred
//! - Identified by a monotonic event_id (for crash recovery)

use serde::{Deserialize, Serialize};
use sy_types::{
    EntityId, EntityKind, EntityState, EventId, RngSeed, SimTime, Tick, WorldPos, ZoneId,
};

use crate::commands::EntityProperties;

/// An event that occurred in the simulation.
/// Events are the source of truth for state changes.
///
/// ## Crash Recovery
/// `event_id` is assigned by the WAL when the event is persisted.
/// On replay, events with `event_id > snapshot.last_event_id` are replayed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimEvent {
    /// Unique monotonic ID (assigned by WAL on persist)
    pub event_id: EventId,
    /// Tick when this event occurred
    pub tick: Tick,
    /// The actual event data
    pub data: EventData,
}

impl SimEvent {
    /// Create a new event (event_id will be assigned by WAL)
    pub fn new(tick: Tick, data: EventData) -> Self {
        SimEvent {
            event_id: EventId::ZERO, // Placeholder, WAL assigns real ID
            tick,
            data,
        }
    }

    /// Create an event with a specific event_id (for replay)
    pub fn with_id(event_id: EventId, tick: Tick, data: EventData) -> Self {
        SimEvent {
            event_id,
            tick,
            data,
        }
    }
}

/// Event data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    // ========================================================================
    // World lifecycle events
    // ========================================================================
    /// World was created
    WorldCreated {
        world_id: String,
        name: String,
        seed: RngSeed,
    },
    /// World was loaded from storage
    WorldLoaded { world_id: String, tick: Tick },
    /// World was saved
    WorldSaved { tick: Tick },

    // ========================================================================
    // Simulation events
    // ========================================================================
    /// A tick was processed
    TickProcessed {
        tick: Tick,
        sim_time: SimTime,
        entities_processed: u32,
    },

    // ========================================================================
    // Zone events
    // ========================================================================
    /// Zone was created
    ZoneCreated {
        zone_id: ZoneId,
        name: Option<String>,
    },
    /// Zone was loaded into active simulation
    ZoneLoaded { zone_id: ZoneId },
    /// Zone was unloaded from active simulation
    ZoneUnloaded { zone_id: ZoneId },

    // ========================================================================
    // Entity events
    // ========================================================================
    /// Entity was spawned
    EntitySpawned {
        entity_id: EntityId,
        kind: EntityKind,
        position: WorldPos,
        properties: EntityProperties,
    },
    /// Entity was despawned (removed)
    EntityDespawned {
        entity_id: EntityId,
        reason: DespawnReason,
    },
    /// Entity moved
    EntityMoved {
        entity_id: EntityId,
        from: WorldPos,
        to: WorldPos,
    },
    /// Entity state changed
    EntityStateChanged {
        entity_id: EntityId,
        old_state: EntityState,
        new_state: EntityState,
    },
    /// Entity property changed (generic for flexibility)
    EntityPropertyChanged {
        entity_id: EntityId,
        property: String,
        old_value: PropertyValue,
        new_value: PropertyValue,
    },

    // ========================================================================
    // Systemic rule events (Phase 1: minimal rules)
    // ========================================================================
    /// Resource was consumed/depleted
    ResourceDepleted {
        entity_id: EntityId,
        amount: u32,
        remaining: u32,
    },
    /// Entity degraded over time
    EntityDegraded {
        entity_id: EntityId,
        old_health: u32,
        new_health: u32,
    },
}

/// Reason for entity despawn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DespawnReason {
    /// Removed by system command
    Command,
    /// Died/destroyed
    Death,
    /// Resource fully depleted
    Depleted,
    /// Expired (time-based)
    Expired,
}

/// Generic property value for flexible property changes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PropertyValue {
    #[default]
    None,
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
}
