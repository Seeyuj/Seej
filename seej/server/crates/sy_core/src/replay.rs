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

use sy_api::events::{EventData, PropertyValue, SimEvent};
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
    if let EventData::WorldLoaded { tick, .. } = &event.data {
        if event.tick != world.current_tick || *tick != world.current_tick {
            return Err(format!(
                "WorldLoaded tick mismatch: event tick {}, payload tick {}, current world tick {}",
                event.tick, tick, world.current_tick
            ));
        }
        return Ok(());
    }

    if let EventData::WorldSaved { tick } = &event.data {
        if event.tick != world.current_tick || *tick != world.current_tick {
            return Err(format!(
                "WorldSaved tick mismatch: event tick {}, payload tick {}, current world tick {}",
                event.tick, tick, world.current_tick
            ));
        }
        return Ok(());
    }

    if let EventData::TickProcessed {
        tick,
        sim_time,
        rng_state_after,
        ..
    } = &event.data
    {
        let expected_sim_time = SimTime::from_ticks(*tick);
        let expected_tick = world.current_tick.next();
        if rng_state_after.is_none() {
            return Err(format!(
                "TickProcessed missing rng_state_after at tick {}; legacy WAL requires explicit migration before Phase 1 recovery",
                tick
            ));
        }
        if event.tick != *tick
            || *sim_time != expected_sim_time
            || (*tick != world.current_tick && *tick != expected_tick)
        {
            return Err(format!(
                "TickProcessed mismatch: event tick {}, payload tick {}, payload sim_time {}, expected sim_time {}, expected next tick {}, current world tick {}",
                event.tick, tick, sim_time, expected_sim_time, expected_tick, world.current_tick
            ));
        }
    } else if event.tick != world.current_tick && event.tick != world.current_tick.next() {
        return Err(format!(
            "Replay event tick mismatch: event tick {}, current world tick {}",
            event.tick, world.current_tick
        ));
    }

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
        EventData::WorldCreated {
            world_id,
            name,
            seed,
        } => {
            if world.id() != world_id || world.name() != name || world.seed() != *seed {
                return Err(format!(
                    "WorldCreated mismatch: payload=({}, {}, {}), world=({}, {}, {})",
                    world_id,
                    name,
                    seed.as_u64(),
                    world.id(),
                    world.name(),
                    world.seed().as_u64()
                ));
            }
            Ok(())
        }
        EventData::WorldLoaded { .. } => {
            unreachable!("WorldLoaded handled before tick updates")
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
            let state = rng_state_after
                .ok_or_else(|| "TickProcessed missing rng_state_after".to_string())?;
            world.rng_state = state;
            Ok(())
        }

        // ====================================================================
        // Zone events
        // ====================================================================
        EventData::ZoneCreated { zone_id, name } => {
            if let Some(zone) = world.zones.get(zone_id) {
                if zone.name != *name || !zone.loaded || !zone.entities.is_empty() {
                    return Err(format!(
                        "ZoneCreated conflicts with existing zone {}",
                        zone_id
                    ));
                }
            } else {
                world
                    .zones
                    .insert(*zone_id, Zone::new(*zone_id, name.clone()));
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
            if !world.zones.contains_key(&position.zone) {
                return Err(format!(
                    "EntitySpawned references missing zone {}",
                    position.zone
                ));
            }

            if let Some(existing) = world.entities.get(entity_id) {
                if existing.kind != *kind
                    || existing.position != *position
                    || existing.created_at != event.tick
                    || existing.state != EntityState::Active
                    || existing.properties != *properties
                {
                    return Err(format!(
                        "EntitySpawned conflicts with existing entity {}",
                        entity_id
                    ));
                }

                let zone = world.zones.get(&position.zone).ok_or_else(|| {
                    format!("EntitySpawned references missing zone {}", position.zone)
                })?;
                if !zone.entities.contains(entity_id) {
                    return Err(format!(
                        "EntitySpawned existing entity {} is missing from zone index {}",
                        entity_id, position.zone
                    ));
                }
                return Ok(());
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
            let entity = world
                .entities
                .get(entity_id)
                .ok_or_else(|| format!("EntityMoved references missing entity {}", entity_id))?;
            if entity.position != *from {
                return Err(format!(
                    "EntityMoved source mismatch for {}: event from {}, current {}",
                    entity_id, from, entity.position
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
            let source_zone = world.zones.get(&from.zone).ok_or_else(|| {
                format!("EntityMoved references missing source zone {}", from.zone)
            })?;
            if !source_zone.entities.contains(entity_id) {
                return Err(format!(
                    "EntityMoved source zone {} is missing entity {}",
                    from.zone, entity_id
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
            old_state,
            new_state,
        } => {
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!("EntityStateChanged references missing entity {}", entity_id)
            })?;
            if entity.state != *old_state {
                return Err(format!(
                    "EntityStateChanged old_state mismatch for {}: event old {:?}, current {:?}",
                    entity_id, old_state, entity.state
                ));
            }
            entity.state = *new_state;
            Ok(())
        }

        EventData::EntityPropertyChanged {
            entity_id,
            property,
            old_value,
            new_value,
        } => {
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!(
                    "EntityPropertyChanged references missing entity {}",
                    entity_id
                )
            })?;
            let current_value = entity_property_value(entity, property)?;
            if current_value != *old_value {
                return Err(format!(
                    "EntityPropertyChanged old_value mismatch for {}.{}: event old {:?}, current {:?}",
                    entity_id, property, old_value, current_value
                ));
            }
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
            amount,
            remaining,
        } => {
            let entity = world.entities.get_mut(entity_id).ok_or_else(|| {
                format!("ResourceDepleted references missing entity {}", entity_id)
            })?;
            let current_amount = entity.properties.amount.ok_or_else(|| {
                format!(
                    "ResourceDepleted references entity {} without amount",
                    entity_id
                )
            })?;
            let expected_before = remaining.checked_add(*amount).ok_or_else(|| {
                format!(
                    "ResourceDepleted amount overflow for {}: amount {}, remaining {}",
                    entity_id, amount, remaining
                )
            })?;
            if current_amount != expected_before {
                return Err(format!(
                    "ResourceDepleted amount mismatch for {}: event amount {}, remaining {}, current {}",
                    entity_id, amount, remaining, current_amount
                ));
            }
            entity.properties.amount = Some(*remaining);
            Ok(())
        }

        EventData::EntityDegraded {
            entity_id,
            old_health,
            new_health,
        } => {
            let entity = world
                .entities
                .get_mut(entity_id)
                .ok_or_else(|| format!("EntityDegraded references missing entity {}", entity_id))?;
            let current_health = entity.properties.health.ok_or_else(|| {
                format!(
                    "EntityDegraded references entity {} without health",
                    entity_id
                )
            })?;
            if current_health != *old_health {
                return Err(format!(
                    "EntityDegraded old_health mismatch for {}: event old {}, current {}",
                    entity_id, old_health, current_health
                ));
            }
            entity.properties.health = Some(*new_health);
            Ok(())
        }
    }
}

fn entity_property_value(entity: &Entity, property: &str) -> Result<PropertyValue, String> {
    match property {
        "name" => Ok(entity
            .properties
            .name
            .clone()
            .map(PropertyValue::String)
            .unwrap_or(PropertyValue::None)),
        "amount" => Ok(entity
            .properties
            .amount
            .map(|value| PropertyValue::UInt(u64::from(value)))
            .unwrap_or(PropertyValue::None)),
        "health" => Ok(entity
            .properties
            .health
            .map(|value| PropertyValue::UInt(u64::from(value)))
            .unwrap_or(PropertyValue::None)),
        _ => Err(format!(
            "EntityPropertyChanged references unknown property '{}'",
            property
        )),
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
            Tick::ZERO,
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
    fn replay_spawn_is_idempotent_for_identical_existing_entity() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
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
    fn replay_rejects_conflicting_existing_spawn_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
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
        let before = world.clone();

        let conflicting = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Creature,
                position: WorldPos::origin(),
                properties: EntityProperties::default(),
            },
        );

        let err = apply_event(&mut world, &conflicting).unwrap_err();

        assert!(err.contains("conflicts with existing entity"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(
            world.get_entity(EntityId::new(1)).unwrap().kind,
            EntityKind::Resource
        );
    }

    #[test]
    fn replay_rejects_spawn_into_missing_zone_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::new(ZoneId::new(404), Position::ORIGIN),
                properties: EntityProperties::default(),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("missing zone"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn replay_rejects_conflicting_existing_zone_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::ZoneCreated {
                zone_id: ZoneId::ORIGIN,
                name: Some("Different".to_string()),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("conflicts with existing zone"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(
            world.get_zone(ZoneId::ORIGIN).unwrap().name.as_deref(),
            Some("Origin")
        );
    }

    #[test]
    fn replay_updates_tick() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        assert_eq!(world.current_tick, Tick::ZERO);

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::TickProcessed {
                tick: Tick(1),
                sim_time: SimTime { units: 1 },
                entities_processed: 0,
                rng_state_after: Some(999),
            },
        );

        apply_event(&mut world, &event).unwrap();
        assert_eq!(world.current_tick, Tick(1));
        assert_eq!(world.rng_state, 999);
    }

    #[test]
    fn replay_rejects_legacy_tick_without_rng_state_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::TickProcessed {
                tick: Tick(1),
                sim_time: SimTime::from_ticks(Tick(1)),
                entities_processed: 0,
                rng_state_after: None,
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("missing rng_state_after"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_rejects_tick_jump_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();

        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(100),
            EventData::TickProcessed {
                tick: Tick(100),
                sim_time: SimTime::from_ticks(Tick(100)),
                entities_processed: 0,
                rng_state_after: Some(999),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("TickProcessed mismatch"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_rejects_tick_processed_event_tick_mismatch_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(100),
            EventData::TickProcessed {
                tick: Tick(99),
                sim_time: SimTime::from_ticks(Tick(99)),
                entities_processed: 0,
                rng_state_after: Some(999),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("TickProcessed mismatch"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_rejects_tick_processed_sim_time_mismatch_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick(10),
            EventData::TickProcessed {
                tick: Tick(10),
                sim_time: SimTime { units: 11 },
                entities_processed: 0,
                rng_state_after: Some(999),
            },
        );

        let err = apply_event(&mut world, &event).unwrap_err();

        assert!(err.contains("TickProcessed mismatch"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_errors_on_missing_entity_reference() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let event = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
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
            Tick::ZERO,
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
            Tick::ZERO,
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
            Tick::ZERO,
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
        assert_eq!(world.current_tick, Tick::ZERO);
    }

    #[test]
    fn replay_rejects_move_to_missing_destination_zone() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
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
            Tick::ZERO,
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
        assert_eq!(world.current_tick, Tick::ZERO);
    }

    #[test]
    fn replay_rejects_move_with_wrong_source_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Creature,
                position: WorldPos::origin(),
                properties: EntityProperties::default(),
            },
        );
        apply_event(&mut world, &spawn).unwrap();
        let before = world.clone();

        let invalid_move = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::EntityMoved {
                entity_id: EntityId::new(1),
                from: WorldPos::new(ZoneId::ORIGIN, Position::new(9, 0, 0)),
                to: WorldPos::new(ZoneId::ORIGIN, Position::new(1, 0, 0)),
            },
        );

        let err = apply_event(&mut world, &invalid_move).unwrap_err();

        assert!(err.contains("source mismatch"));
        assert_eq!(
            world.get_entity(EntityId::new(1)).unwrap().position,
            before.get_entity(EntityId::new(1)).unwrap().position
        );
    }

    #[test]
    fn replay_rejects_state_change_with_wrong_old_state() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Creature,
                position: WorldPos::origin(),
                properties: EntityProperties::default(),
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::EntityStateChanged {
                entity_id: EntityId::new(1),
                old_state: EntityState::Dormant,
                new_state: EntityState::Dead,
            },
        );

        let err = apply_event(&mut world, &invalid).unwrap_err();

        assert!(err.contains("old_state mismatch"));
        assert_eq!(
            world.get_entity(EntityId::new(1)).unwrap().state,
            EntityState::Active
        );
    }

    #[test]
    fn replay_rejects_property_change_with_wrong_old_value() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::origin(),
                properties: EntityProperties {
                    name: None,
                    amount: Some(10),
                    health: None,
                },
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::EntityPropertyChanged {
                entity_id: EntityId::new(1),
                property: "amount".to_string(),
                old_value: PropertyValue::UInt(9),
                new_value: PropertyValue::UInt(8),
            },
        );

        let err = apply_event(&mut world, &invalid).unwrap_err();

        assert!(err.contains("old_value mismatch"));
        assert_eq!(
            world
                .get_entity(EntityId::new(1))
                .unwrap()
                .properties
                .amount,
            Some(10)
        );
    }

    #[test]
    fn replay_rejects_resource_depletion_with_wrong_current_amount() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Resource,
                position: WorldPos::origin(),
                properties: EntityProperties {
                    name: None,
                    amount: Some(10),
                    health: None,
                },
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::ResourceDepleted {
                entity_id: EntityId::new(1),
                amount: 1,
                remaining: 7,
            },
        );

        let err = apply_event(&mut world, &invalid).unwrap_err();

        assert!(err.contains("amount mismatch"));
        assert_eq!(
            world
                .get_entity(EntityId::new(1))
                .unwrap()
                .properties
                .amount,
            Some(10)
        );
    }

    #[test]
    fn replay_resource_depletion_then_state_change() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let events = vec![
            SimEvent::with_id(
                EventId::new(1),
                Tick::ZERO,
                EventData::EntitySpawned {
                    entity_id: EntityId::new(1),
                    kind: EntityKind::Resource,
                    position: WorldPos::origin(),
                    properties: EntityProperties {
                        name: None,
                        amount: Some(1),
                        health: None,
                    },
                },
            ),
            SimEvent::with_id(
                EventId::new(2),
                Tick::ZERO,
                EventData::ResourceDepleted {
                    entity_id: EntityId::new(1),
                    amount: 1,
                    remaining: 0,
                },
            ),
            SimEvent::with_id(
                EventId::new(3),
                Tick::ZERO,
                EventData::EntityStateChanged {
                    entity_id: EntityId::new(1),
                    old_state: EntityState::Active,
                    new_state: EntityState::Dead,
                },
            ),
        ];

        replay_events(&mut world, &events).unwrap();

        let entity = world.get_entity(EntityId::new(1)).unwrap();
        assert_eq!(entity.properties.amount, Some(0));
        assert_eq!(entity.state, EntityState::Dead);
    }

    #[test]
    fn replay_rejects_entity_degradation_with_wrong_old_health() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let spawn = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::EntitySpawned {
                entity_id: EntityId::new(1),
                kind: EntityKind::Creature,
                position: WorldPos::origin(),
                properties: EntityProperties {
                    name: None,
                    amount: None,
                    health: Some(10),
                },
            },
        );
        apply_event(&mut world, &spawn).unwrap();

        let invalid = SimEvent::with_id(
            EventId::new(2),
            Tick::ZERO,
            EventData::EntityDegraded {
                entity_id: EntityId::new(1),
                old_health: 9,
                new_health: 8,
            },
        );

        let err = apply_event(&mut world, &invalid).unwrap_err();

        assert!(err.contains("old_health mismatch"));
        assert_eq!(
            world
                .get_entity(EntityId::new(1))
                .unwrap()
                .properties
                .health,
            Some(10)
        );
    }

    #[test]
    fn replay_entity_degradation_then_state_change() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let events = vec![
            SimEvent::with_id(
                EventId::new(1),
                Tick::ZERO,
                EventData::EntitySpawned {
                    entity_id: EntityId::new(1),
                    kind: EntityKind::Creature,
                    position: WorldPos::origin(),
                    properties: EntityProperties {
                        name: None,
                        amount: None,
                        health: Some(1),
                    },
                },
            ),
            SimEvent::with_id(
                EventId::new(2),
                Tick::ZERO,
                EventData::EntityDegraded {
                    entity_id: EntityId::new(1),
                    old_health: 1,
                    new_health: 0,
                },
            ),
            SimEvent::with_id(
                EventId::new(3),
                Tick::ZERO,
                EventData::EntityStateChanged {
                    entity_id: EntityId::new(1),
                    old_state: EntityState::Active,
                    new_state: EntityState::Dead,
                },
            ),
        ];

        replay_events(&mut world, &events).unwrap();

        let entity = world.get_entity(EntityId::new(1)).unwrap();
        assert_eq!(entity.properties.health, Some(0));
        assert_eq!(entity.state, EntityState::Dead);
    }

    #[test]
    fn replay_world_saved_is_strict_noop() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let tick_event = SimEvent::with_id(
            EventId::new(1),
            Tick(1),
            EventData::TickProcessed {
                tick: Tick(1),
                sim_time: SimTime::from_ticks(Tick(1)),
                entities_processed: 0,
                rng_state_after: Some(123),
            },
        );
        apply_event(&mut world, &tick_event).unwrap();
        let before = world.clone();

        let saved = SimEvent::with_id(
            EventId::new(2),
            Tick(1),
            EventData::WorldSaved { tick: Tick(1) },
        );
        apply_event(&mut world, &saved).unwrap();

        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.meta.current_tick, before.meta.current_tick);
        assert_eq!(world.meta.sim_time, before.meta.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_world_loaded_is_strict_noop() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();

        let loaded = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::WorldLoaded {
                world_id: world.id().to_string(),
                tick: Tick::ZERO,
            },
        );
        apply_event(&mut world, &loaded).unwrap();

        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.meta.current_tick, before.meta.current_tick);
        assert_eq!(world.meta.sim_time, before.meta.sim_time);
        assert_eq!(world.rng_state, before.rng_state);
    }

    #[test]
    fn replay_rejects_incoherent_world_loaded_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();

        let loaded = SimEvent::with_id(
            EventId::new(1),
            Tick(10),
            EventData::WorldLoaded {
                world_id: world.id().to_string(),
                tick: Tick(10),
            },
        );
        let err = apply_event(&mut world, &loaded).unwrap_err();

        assert!(err.contains("WorldLoaded tick mismatch"));
        assert_eq!(world.current_tick, before.current_tick);
        assert_eq!(world.sim_time, before.sim_time);
        assert_eq!(world.meta.current_tick, before.meta.current_tick);
        assert_eq!(world.meta.sim_time, before.meta.sim_time);
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

    #[test]
    fn replay_rejects_incoherent_world_created_without_mutating_world() {
        let mut world = World::new("Test".to_string(), RngSeed::new(1));
        let before = world.clone();
        let created = SimEvent::with_id(
            EventId::new(1),
            Tick::ZERO,
            EventData::WorldCreated {
                world_id: "world_2".to_string(),
                name: "Wrong".to_string(),
                seed: RngSeed::new(2),
            },
        );

        let err = apply_event(&mut world, &created).unwrap_err();

        assert!(err.contains("WorldCreated mismatch"));
        assert_eq!(world.meta.world_id, before.meta.world_id);
        assert_eq!(world.meta.name, before.meta.name);
        assert_eq!(world.meta.seed, before.meta.seed);
    }
}
