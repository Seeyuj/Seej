//! # Replay
//!
//! Event replay for crash recovery.
//!
//! ## Purpose
//! Apply recorded events to reconstruct world state after a crash.
//! This is the core of the crash recovery mechanism.
//!
//! ## Contract
//! - `apply_event` is deterministic and total
//! - No I/O, no RNG, no system time
//! - Idempotent when event_id is checked by caller

use sy_api::events::{EventData, SimEvent};
use sy_types::{EntityState, SimTime};

use crate::world::{Entity, World, Zone};

/// Apply a single event to the world state.
///
/// ## Invariants
/// - Deterministic: same event + same state = same result
/// - Total: handles all event types (no panics)
/// - Pure: no I/O, no side effects outside of world mutation
///
/// ## Returns
/// - `Ok(())` if event was applied successfully
/// - `Err(reason)` if event could not be applied (e.g., entity not found)
///   In crash recovery, errors are typically logged but not fatal.
pub fn apply_event(world: &mut World, event: &SimEvent) -> Result<(), String> {
    // Update world tick to max of current and event tick
    if event.tick > world.current_tick {
        world.current_tick = event.tick;
        world.sim_time = SimTime::from_ticks(event.tick);
        world.meta.current_tick = event.tick;
        world.meta.sim_time = world.sim_time;
    }

    match &event.data {
        // ====================================================================
        // World lifecycle events (mostly no-op on replay)
        // ====================================================================
        EventData::WorldCreated { .. } => {
            // World already exists, this is just recording
            Ok(())
        }
        EventData::WorldLoaded { .. } => {
            // No state change needed
            Ok(())
        }
        EventData::WorldSaved { .. } => {
            // No state change needed
            Ok(())
        }

        // ====================================================================
        // Tick events
        // ====================================================================
        EventData::TickProcessed { tick, sim_time, .. } => {
            // Ensure world is at this tick
            if *tick > world.current_tick {
                world.current_tick = *tick;
                world.sim_time = *sim_time;
                world.meta.current_tick = *tick;
                world.meta.sim_time = *sim_time;
            }
            Ok(())
        }

        // ====================================================================
        // Zone events
        // ====================================================================
        EventData::ZoneCreated { zone_id, name } => {
            if !world.zones.contains_key(zone_id) {
                let zone = Zone::new(*zone_id, name.clone());
                world.zones.insert(*zone_id, zone);
            }
            Ok(())
        }
        EventData::ZoneLoaded { zone_id } => {
            if let Some(zone) = world.zones.get_mut(zone_id) {
                zone.loaded = true;
            }
            Ok(())
        }
        EventData::ZoneUnloaded { zone_id } => {
            if let Some(zone) = world.zones.get_mut(zone_id) {
                zone.loaded = false;
            }
            Ok(())
        }

        // ====================================================================
        // Entity events
        // ====================================================================
        EventData::EntitySpawned {
            entity_id,
            kind,
            position,
            properties,
        } => {
            // Don't re-spawn if entity already exists
            if world.entities.contains_key(entity_id) {
                return Ok(()); // Idempotent
            }

            // Ensure next_entity_id is updated
            if entity_id.as_u64() >= world.next_entity_id {
                world.next_entity_id = entity_id.as_u64() + 1;
            }

            let entity = Entity::new(*entity_id, *kind, *position, event.tick, properties.clone());
            world.add_entity(entity);
            Ok(())
        }

        EventData::EntityDespawned { entity_id, .. } => {
            world.remove_entity(*entity_id);
            Ok(())
        }

        EventData::EntityMoved {
            entity_id,
            from,
            to,
        } => {
            if let Some(entity) = world.entities.get_mut(entity_id) {
                // Update zone membership
                if from.zone != to.zone {
                    if let Some(old_zone) = world.zones.get_mut(&from.zone) {
                        old_zone.remove_entity(*entity_id);
                    }
                    if let Some(new_zone) = world.zones.get_mut(&to.zone) {
                        new_zone.add_entity(*entity_id);
                    }
                }
                entity.position = *to;
            }
            Ok(())
        }

        EventData::EntityStateChanged {
            entity_id,
            new_state,
            ..
        } => {
            if let Some(entity) = world.entities.get_mut(entity_id) {
                entity.state = *new_state;
            }
            Ok(())
        }

        EventData::EntityPropertyChanged {
            entity_id,
            property,
            new_value,
            ..
        } => {
            if let Some(entity) = world.entities.get_mut(entity_id) {
                // Apply property change based on property name
                match property.as_str() {
                    "name" => {
                        if let sy_api::events::PropertyValue::String(s) = new_value {
                            entity.properties.name = Some(s.clone());
                        }
                    }
                    "amount" => {
                        if let sy_api::events::PropertyValue::UInt(v) = new_value {
                            entity.properties.amount = Some(*v as u32);
                        }
                    }
                    "health" => {
                        if let sy_api::events::PropertyValue::UInt(v) = new_value {
                            entity.properties.health = Some(*v as u32);
                        }
                    }
                    _ => {} // Unknown property, ignore
                }
            }
            Ok(())
        }

        // ====================================================================
        // Systemic events
        // ====================================================================
        EventData::ResourceDepleted {
            entity_id,
            remaining,
            ..
        } => {
            if let Some(entity) = world.entities.get_mut(entity_id) {
                entity.properties.amount = Some(*remaining);
                if *remaining == 0 {
                    entity.state = EntityState::Dead;
                }
            }
            Ok(())
        }

        EventData::EntityDegraded {
            entity_id,
            new_health,
            ..
        } => {
            if let Some(entity) = world.entities.get_mut(entity_id) {
                entity.properties.health = Some(*new_health);
                if *new_health == 0 {
                    entity.state = EntityState::Dead;
                }
            }
            Ok(())
        }
    }
}

/// Replay multiple events in order.
/// Returns the number of events successfully applied.
pub fn replay_events(world: &mut World, events: &[SimEvent]) -> usize {
    let mut applied = 0;
    for event in events {
        if apply_event(world, event).is_ok() {
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use sy_api::commands::EntityProperties;
    use sy_types::{EntityId, EntityKind, EventId, RngSeed, Tick, WorldPos};

    #[test]
    fn replay_entity_spawn() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::origin(),
                properties: EntityProperties::default(),
            },
        );

        apply_event(&mut world, &event).unwrap();
        assert_eq!(world.entity_count(), 1);
        assert!(world.get_entity(EntityId::new(1)).is_some());
    }

    #[test]
    fn replay_is_idempotent() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::origin(),
                properties: EntityProperties::default(),
            },
        );

        // Apply twice
        apply_event(&mut world, &event).unwrap();
        apply_event(&mut world, &event).unwrap();

        // Should still have only 1 entity
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn replay_updates_tick() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        assert_eq!(world.current_tick, Tick::ZERO);

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(100),
            EventData::TickProcessed {
                tick: Tick(100),
                sim_time: SimTime { units: 100 },
                entities_processed: 0,
            },
        );

        apply_event(&mut world, &event).unwrap();
        assert_eq!(world.current_tick, Tick(100));
    }
}
