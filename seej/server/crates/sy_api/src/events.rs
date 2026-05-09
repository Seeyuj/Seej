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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        /// RNG state after processing this tick.
        /// Optional only so older WAL payloads can deserialize; Phase 1 replay
        /// refuses missing checkpoints unless an explicit migration rewrites them.
        #[serde(default)]
        rng_state_after: Option<u64>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PropertyValue {
    #[default]
    None,
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use sy_types::{EntityKind, EntityState, Position};

    fn round_trip_event_data(data: EventData) {
        let encoded = serde_json::to_string(&data).expect("event data must serialize");
        let decoded: EventData =
            serde_json::from_str(&encoded).expect("event data must deserialize");
        assert_eq!(decoded, data);
    }

    #[test]
    fn all_event_data_variants_round_trip_through_json() {
        let entity_id = EntityId::new(42);
        let zone_id = ZoneId::new(7);
        let origin = WorldPos::origin();
        let target = WorldPos::new(zone_id, Position::new(1, -2, 3));

        let cases = [
            EventData::WorldCreated {
                world_id: "world-1".to_string(),
                name: "deterministic test world".to_string(),
                seed: RngSeed::new(123),
            },
            EventData::WorldLoaded {
                world_id: "world-1".to_string(),
                tick: Tick(5),
            },
            EventData::WorldSaved { tick: Tick(8) },
            EventData::TickProcessed {
                tick: Tick(9),
                sim_time: SimTime::from_ticks(Tick(9)),
                entities_processed: 3,
                rng_state_after: Some(0x5eed),
            },
            EventData::ZoneCreated {
                zone_id,
                name: Some("origin-adjacent".to_string()),
            },
            EventData::ZoneLoaded { zone_id },
            EventData::ZoneUnloaded { zone_id },
            EventData::EntitySpawned {
                entity_id,
                kind: EntityKind::Resource,
                position: origin,
                properties: EntityProperties {
                    name: Some("resource-node".to_string()),
                    amount: Some(100),
                    health: Some(10),
                },
            },
            EventData::EntityDespawned {
                entity_id,
                reason: DespawnReason::Depleted,
            },
            EventData::EntityMoved {
                entity_id,
                from: origin,
                to: target,
            },
            EventData::EntityStateChanged {
                entity_id,
                old_state: EntityState::Active,
                new_state: EntityState::Dormant,
            },
            EventData::EntityPropertyChanged {
                entity_id,
                property: "health".to_string(),
                old_value: PropertyValue::UInt(10),
                new_value: PropertyValue::UInt(9),
            },
            EventData::ResourceDepleted {
                entity_id,
                amount: 1,
                remaining: 99,
            },
            EventData::EntityDegraded {
                entity_id,
                old_health: 10,
                new_health: 9,
            },
        ];

        for data in cases {
            round_trip_event_data(data);
        }
    }

    #[test]
    fn sim_event_round_trip_preserves_replay_cursor_and_payload() {
        let event = SimEvent::with_id(
            EventId::new(12),
            Tick(34),
            EventData::TickProcessed {
                tick: Tick(34),
                sim_time: SimTime::from_ticks(Tick(34)),
                entities_processed: 2,
                rng_state_after: Some(987_654),
            },
        );

        let encoded = serde_json::to_string(&event).expect("sim event must serialize");
        let decoded: SimEvent = serde_json::from_str(&encoded).expect("sim event must deserialize");

        assert_eq!(decoded, event);
    }

    #[test]
    fn tick_processed_decodes_missing_rng_state_for_legacy_wal_payloads() {
        let json = r#"{
            "TickProcessed": {
                "tick": 3,
                "sim_time": { "units": 3 },
                "entities_processed": 4
            }
        }"#;

        let decoded: EventData =
            serde_json::from_str(json).expect("legacy tick event must deserialize");

        assert_eq!(
            decoded,
            EventData::TickProcessed {
                tick: Tick(3),
                sim_time: SimTime::from_ticks(Tick(3)),
                entities_processed: 4,
                rng_state_after: None,
            }
        );
    }
}
