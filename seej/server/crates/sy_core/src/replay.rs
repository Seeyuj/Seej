//! # Replay
//!
//! Event replay for crash recovery.
//!
//! ## Purpose
//! Apply recorded events to reconstruct world state after a crash.
//! This is the core of the crash recovery mechanism.
//!
//! ## Contract
//! - `apply_event` is deterministic and explicit about invalid references
//! - No I/O, no RNG, no system time
//! - Idempotent when event_id is checked by caller

use sy_api::events::{EventData, SimEvent};
use sy_types::{EntityState, SimTime};

use crate::world::{Entity, World, Zone};

/// Apply a single event to the world state.
///
/// ## Invariants
/// - Deterministic: same event + same state = same result
/// - Explicit: invalid references return errors instead of being silently ignored
/// - Pure: no I/O, no side effects outside of world mutation
///
/// ## Returns
/// - `Ok(())` if event was applied successfully
/// - `Err(reason)` if event could not be applied (e.g., entity not found)
pub fn apply_event(world: &mut World, event: &SimEvent) -> Result<(), String> {
    let mut next = world.clone();
    apply_event_in_place(&mut next, event)?;
    *world = next;
    Ok(())
}

fn apply_event_in_place(world: &mut World, event: &SimEvent) -> Result<(), String> {
    if let EventData::WorldSaved { tick } = &event.data {
        if event.tick != world.current_tick || *tick != world.current_tick {
            return Err(format!(
                "WorldSaved tick mismatch: event tick {}, payload tick {}, current world tick {}",
                event.tick, tick, world.current_tick
            ));
        }
        return Ok(());
    }

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
        EventData::WorldSaved { .. } => unreachable!("WorldSaved handled before tick updates"),

        // ====================================================================
        // Tick events
        // ====================================================================
        EventData::TickProcessed {
            tick,
            sim_time,
            rng_state_after,
            ..
        } => {
            // Ensure world is at this tick
            if *tick > world.current_tick {
                world.current_tick = *tick;
                world.sim_time = *sim_time;
                world.meta.current_tick = *tick;
                world.meta.sim_time = *sim_time;
            }
            if let Some(state) = rng_state_after {
                world.rng_state = *state;
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
            let zone = world
                .zones
                .get_mut(zone_id)
                .ok_or_else(|| format!("ZoneLoaded references missing zone {}", zone_id))?;
            zone.loaded = true;
            Ok(())
        }
        EventData::ZoneUnloaded { zone_id } => {
            let zone = world
                .zones
                .get_mut(zone_id)
                .ok_or_else(|| format!("ZoneUnloaded references missing zone {}", zone_id))?;
            zone.loaded = false;
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
            world.remove_entity(*entity_id).ok_or_else(|| {
                format!("EntityDespawned references missing entity {}", entity_id)
            })?;
            Ok(())
        }

        EventData::EntityMoved {
            entity_id,
            from,
            to,
        } => {
            if !world.entities.contains_key(entity_id) {
                return Err(format!(
                    "EntityMoved references missing entity {}",
                    entity_id
                ));
            }
            if !world.zones.contains_key(&from.zone) {
                return Err(format!(
                    "EntityMoved references missing source zone {}",
                    from.zone
                ));
            }
            if !world.zones.contains_key(&to.zone) {
                return Err(format!(
                    "EntityMoved references missing destination zone {}",
                    to.zone
                ));
            }

            if from.zone != to.zone {
                if let Some(old_zone) = world.zones.get_mut(&from.zone) {
                    old_zone.remove_entity(*entity_id);
                }
                if let Some(new_zone) = world.zones.get_mut(&to.zone) {
                    new_zone.add_entity(*entity_id);
                }
            }
            let entity = world
                .entities
                .get_mut(entity_id)
                .ok_or_else(|| format!("EntityMoved references missing entity {}", entity_id))?;
            entity.position = *to;
            Ok(())
        }

        EventData::EntityStateChanged {
            entity_id,
            new_state,
            ..
        } => {
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!("EntityStateChanged references missing entity {}", entity_id)
            })?;
            entity.state = *new_state;
            Ok(())
        }

        EventData::EntityPropertyChanged {
            entity_id,
            property,
            new_value,
            ..
        } => {
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!(
                    "EntityPropertyChanged references missing entity {}",
                    entity_id
                )
            })?;
            match property.as_str() {
                "name" => {
                    if let sy_api::events::PropertyValue::String(s) = new_value {
                        entity.properties.name = Some(s.clone());
                    } else {
                        return Err("EntityPropertyChanged name requires string value".to_string());
                    }
                }
                "amount" => {
                    if let sy_api::events::PropertyValue::UInt(v) = new_value {
                        entity.properties.amount = Some(*v as u32);
                    } else {
                        return Err("EntityPropertyChanged amount requires uint value".to_string());
                    }
                }
                "health" => {
                    if let sy_api::events::PropertyValue::UInt(v) = new_value {
                        entity.properties.health = Some(*v as u32);
                    } else {
                        return Err("EntityPropertyChanged health requires uint value".to_string());
                    }
                }
                _ => {
                    return Err(format!(
                        "EntityPropertyChanged references unknown property '{}'",
                        property
                    ));
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
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!("ResourceDepleted references missing entity {}", entity_id)
            })?;
            entity.properties.amount = Some(*remaining);
            if *remaining == 0 {
                entity.state = EntityState::Dead;
            }
            Ok(())
        }

        EventData::EntityDegraded {
            entity_id,
            new_health,
            ..
        } => {
            let entity = world
                .entities
                .get_mut(entity_id)
                .ok_or_else(|| format!("EntityDegraded references missing entity {}", entity_id))?;
            entity.properties.health = Some(*new_health);
            if *new_health == 0 {
                entity.state = EntityState::Dead;
            }
            Ok(())
        }
    }
}

/// Replay multiple events in order.
/// Returns the number of events applied, or the first invalid replay error.
pub fn replay_events(world: &mut World, events: &[SimEvent]) -> Result<usize, String> {
    let mut applied = 0;
    for event in events {
        apply_event(world, event)?;
        applied += 1;
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sy_api::commands::EntityProperties;
    use sy_api::events::PropertyValue;
    use sy_types::{EntityId, EntityKind, EventId, Position, RngSeed, Tick, WorldPos, ZoneId};

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
                rng_state_after: Some(999),
            },
        );

        apply_event(&mut world, &event).unwrap();
        assert_eq!(world.current_tick, Tick(100));
        assert_eq!(world.rng_state, 999);
    }

    #[test]
    fn replay_errors_on_missing_entity_reference() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::EntityDespawned {
                entity_id: EntityId::new(404),
                reason: sy_api::events::DespawnReason::Command,
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();
        assert!(err.contains("missing entity"));
        assert_eq!(world.current_tick, Tick::ZERO);
    }

    #[test]
    fn replay_errors_on_missing_zone_reference_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(5),
            EventData::ZoneLoaded {
                zone_id: ZoneId::new(404),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();
        assert!(err.contains("missing zone"));
        assert_eq!(world.current_tick, Tick::ZERO);
        assert_eq!(world.zone_count(), 1);
    }

    #[test]
    fn replay_rejects_property_type_mismatch_without_mutating_entity() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::origin(),
                properties: EntityProperties {
                    name: Some("ore".to_string()),
                    amount: Some(10),
                    health: None,
                },
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid = SimEvent::with_id(
            EventId::new(2),
            Tick(2),
            EventData::EntityPropertyChanged {
                entity_id: EntityId::new(1),
                property: "amount".to_string(),
                old_value: PropertyValue::UInt(10),
                new_value: PropertyValue::String("bad".to_string()),
            },
        );

        let err = apply_event(&mut world, &invalid).unwrap_err();
        assert!(err.contains("amount requires uint"));
        let entity = world.get_entity(EntityId::new(1)).unwrap();
        assert_eq!(entity.properties.amount, Some(10));
        assert_eq!(world.current_tick, Tick(1));
    }

    #[test]
    fn replay_rejects_move_to_missing_destination_zone() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Creature,
                position: WorldPos::origin(),
                properties: EntityProperties {
                    name: None,
                    amount: None,
                    health: Some(5),
                },
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid_move = SimEvent::with_id(
            EventId::new(2),
            Tick(2),
            EventData::EntityMoved {
                entity_id: EntityId::new(1),
                from: WorldPos::origin(),
                to: WorldPos::new(ZoneId::new(99), Position::new(1, 0, 0)),
            },
        );

        let err = apply_event(&mut world, &invalid_move).unwrap_err();
        assert!(err.contains("missing destination zone"));
        let entity = world.get_entity(EntityId::new(1)).unwrap();
        assert_eq!(entity.position, WorldPos::origin());
        assert_eq!(world.current_tick, Tick(1));
    }

    #[test]
    fn replay_world_saved_is_strict_noop() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let tick_event = SimEvent::with_id(
            EventId::new(1),
            Tick(7),
            EventData::TickProcessed {
                tick: Tick(7),
                sim_time: SimTime::from_ticks(Tick(7)),
                entities_processed: 0,
                rng_state_after: Some(123),
            },
        );
        apply_event(&mut world, &tick_event).unwrap();
        let before = world.clone();

        let saved = SimEvent::with_id(
            EventId::new(2),
            Tick(7),
            EventData::WorldSaved { tick: Tick(7) },
        );
        apply_event(&mut world, &saved).unwrap();

        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.meta.current_tick, before.meta.current_tick);
        assert_eq!(world.meta.sim_time, before.meta.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_rejects_incoherent_world_saved_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let saved = SimEvent::with_id(
            EventId::new(1),
            Tick(10),
            EventData::WorldSaved { tick: Tick(10) },
        );

        let err = apply_event(&mut world, &saved).unwrap_err();

        assert!(err.contains("WorldSaved tick mismatch"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.meta.current_tick, before.meta.current_tick);
        assert_eq!(world.meta.sim_time, before.meta.sim_time);
    }
}
