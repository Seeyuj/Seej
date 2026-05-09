//! Snapshot encoding/decoding owned by infrastructure.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sy_api::commands::EntityProperties;
use sy_core::{Entity, World, Zone};
use sy_types::{
    EntityId, EntityKind, EntityState, SimError, SimResult, SimTime, Tick, WorldMeta, WorldPos,
    ZoneId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityDto {
    id: EntityId,
    kind: EntityKind,
    state: EntityState,
    position: WorldPos,
    created_at: Tick,
    properties: EntityProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZoneDto {
    id: ZoneId,
    name: Option<String>,
    loaded: bool,
    entities: Vec<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorldSnapshotDto {
    meta: WorldMeta,
    current_tick: Tick,
    sim_time: SimTime,
    rng_state: u64,
    next_entity_id: u64,
    entities: BTreeMap<EntityId, EntityDto>,
    zones: BTreeMap<ZoneId, ZoneDto>,
}

impl From<&Entity> for EntityDto {
    fn from(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            kind: entity.kind,
            state: entity.state,
            position: entity.position,
            created_at: entity.created_at,
            properties: entity.properties.clone(),
        }
    }
}

impl From<EntityDto> for Entity {
    fn from(entity: EntityDto) -> Self {
        Self {
            id: entity.id,
            kind: entity.kind,
            state: entity.state,
            position: entity.position,
            created_at: entity.created_at,
            properties: entity.properties,
        }
    }
}

impl From<&Zone> for ZoneDto {
    fn from(zone: &Zone) -> Self {
        Self {
            id: zone.id,
            name: zone.name.clone(),
            loaded: zone.loaded,
            entities: zone.entities.clone(),
        }
    }
}

impl From<ZoneDto> for Zone {
    fn from(zone: ZoneDto) -> Self {
        Self {
            id: zone.id,
            name: zone.name,
            loaded: zone.loaded,
            entities: zone.entities,
        }
    }
}

impl From<&World> for WorldSnapshotDto {
    fn from(world: &World) -> Self {
        Self {
            meta: world.meta.clone(),
            current_tick: world.current_tick,
            sim_time: world.sim_time,
            rng_state: world.rng_state,
            next_entity_id: world.next_entity_id,
            entities: world
                .entities
                .iter()
                .map(|(id, entity)| (*id, EntityDto::from(entity)))
                .collect(),
            zones: world
                .zones
                .iter()
                .map(|(id, zone)| (*id, ZoneDto::from(zone)))
                .collect(),
        }
    }
}

impl From<WorldSnapshotDto> for World {
    fn from(snapshot: WorldSnapshotDto) -> Self {
        Self {
            meta: snapshot.meta,
            current_tick: snapshot.current_tick,
            sim_time: snapshot.sim_time,
            rng_state: snapshot.rng_state,
            next_entity_id: snapshot.next_entity_id,
            entities: snapshot
                .entities
                .into_iter()
                .map(|(id, entity)| (id, Entity::from(entity)))
                .collect(),
            zones: snapshot
                .zones
                .into_iter()
                .map(|(id, zone)| (id, Zone::from(zone)))
                .collect(),
        }
    }
}

/// Serialize a world snapshot.
pub fn encode_world(world: &World) -> SimResult<Vec<u8>> {
    serde_json::to_vec(&WorldSnapshotDto::from(world))
        .map_err(|e| SimError::PersistenceError(format!("Failed to serialize world: {}", e)))
}

/// Deserialize a world snapshot.
pub fn decode_world(data: &[u8]) -> SimResult<World> {
    let snapshot: WorldSnapshotDto = serde_json::from_slice(data)
        .map_err(|e| SimError::PersistenceError(format!("Failed to deserialize world: {}", e)))?;

    validate_snapshot(&snapshot)?;

    Ok(World::from(snapshot))
}

fn validate_snapshot(snapshot: &WorldSnapshotDto) -> SimResult<()> {
    if snapshot.meta.format_version != WorldMeta::CURRENT_FORMAT_VERSION {
        return Err(SimError::PersistenceError(format!(
            "Unsupported world format {} (current {}). No implicit migration is available.",
            snapshot.meta.format_version,
            WorldMeta::CURRENT_FORMAT_VERSION
        )));
    }

    if snapshot.meta.current_tick != snapshot.current_tick {
        return Err(SimError::CorruptedState(format!(
            "Snapshot metadata tick mismatch: meta current_tick {} != snapshot current_tick {}",
            snapshot.meta.current_tick, snapshot.current_tick
        )));
    }

    if snapshot.meta.snapshot_tick != snapshot.current_tick {
        return Err(SimError::CorruptedState(format!(
            "Snapshot tick mismatch: meta snapshot_tick {} != snapshot current_tick {}",
            snapshot.meta.snapshot_tick, snapshot.current_tick
        )));
    }

    if snapshot.meta.sim_time != snapshot.sim_time {
        return Err(SimError::CorruptedState(format!(
            "Snapshot metadata sim_time mismatch: meta sim_time {} != snapshot sim_time {}",
            snapshot.meta.sim_time, snapshot.sim_time
        )));
    }

    let expected_sim_time = SimTime::from_ticks(snapshot.current_tick);
    if snapshot.sim_time != expected_sim_time {
        return Err(SimError::CorruptedState(format!(
            "Snapshot sim_time mismatch: snapshot sim_time {} != expected {} for tick {}",
            snapshot.sim_time, expected_sim_time, snapshot.current_tick
        )));
    }

    validate_entity_ids(snapshot)?;
    validate_zone_ids(snapshot)?;
    validate_zone_entity_index(snapshot)?;

    Ok(())
}

fn validate_entity_ids(snapshot: &WorldSnapshotDto) -> SimResult<()> {
    let max_entity_id = snapshot
        .entities
        .keys()
        .map(|id| id.as_u64())
        .max()
        .unwrap_or(0);

    if snapshot.next_entity_id == 0 {
        return Err(SimError::CorruptedState(
            "Snapshot next_entity_id must not be zero".to_string(),
        ));
    }

    if snapshot.next_entity_id <= max_entity_id {
        return Err(SimError::CorruptedState(format!(
            "Snapshot next_entity_id {} is not greater than max entity id {}",
            snapshot.next_entity_id, max_entity_id
        )));
    }

    for (id, entity) in &snapshot.entities {
        if *id != entity.id {
            return Err(SimError::CorruptedState(format!(
                "Snapshot entity key {} does not match entity id {}",
                id, entity.id
            )));
        }
        if !id.is_valid() {
            return Err(SimError::CorruptedState(
                "Snapshot contains invalid entity id E0".to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_zone_ids(snapshot: &WorldSnapshotDto) -> SimResult<()> {
    for (id, zone) in &snapshot.zones {
        if *id != zone.id {
            return Err(SimError::CorruptedState(format!(
                "Snapshot zone key {} does not match zone id {}",
                id, zone.id
            )));
        }
    }

    Ok(())
}

fn validate_zone_entity_index(snapshot: &WorldSnapshotDto) -> SimResult<()> {
    let mut expected_by_zone: BTreeMap<ZoneId, BTreeSet<EntityId>> = BTreeMap::new();

    for (entity_id, entity) in &snapshot.entities {
        if !snapshot.zones.contains_key(&entity.position.zone) {
            return Err(SimError::CorruptedState(format!(
                "Snapshot entity {} references missing zone {}",
                entity_id, entity.position.zone
            )));
        }
        expected_by_zone
            .entry(entity.position.zone)
            .or_default()
            .insert(*entity_id);
    }

    for (zone_id, zone) in &snapshot.zones {
        let mut actual = BTreeSet::new();
        for entity_id in &zone.entities {
            if !actual.insert(*entity_id) {
                return Err(SimError::CorruptedState(format!(
                    "Snapshot zone {} contains duplicate entity {}",
                    zone_id, entity_id
                )));
            }

            let entity = snapshot.entities.get(entity_id).ok_or_else(|| {
                SimError::CorruptedState(format!(
                    "Snapshot zone {} references missing entity {}",
                    zone_id, entity_id
                ))
            })?;

            if entity.position.zone != *zone_id {
                return Err(SimError::CorruptedState(format!(
                    "Snapshot zone {} references entity {} whose position is in zone {}",
                    zone_id, entity_id, entity.position.zone
                )));
            }
        }

        let expected = expected_by_zone.remove(zone_id).unwrap_or_default();
        if actual != expected {
            return Err(SimError::CorruptedState(format!(
                "Snapshot zone {} entity index does not match entity positions",
                zone_id
            )));
        }
    }

    if let Some((zone_id, _)) = expected_by_zone.into_iter().next() {
        return Err(SimError::CorruptedState(format!(
            "Snapshot has entities indexed to missing zone {}",
            zone_id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sy_types::{EntityKind, RngSeed};

    fn snapshot_value() -> serde_json::Value {
        let mut world = World::new("Snapshot Test".to_string(), RngSeed::new(1));
        let entity = Entity::new(
            EntityId::new(1),
            EntityKind::Resource,
            WorldPos::origin(),
            Tick::ZERO,
            EntityProperties::default(),
        );
        world.add_entity(entity);
        world.next_entity_id = 2;

        let bytes = encode_world(&world).expect("snapshot encode failed");
        serde_json::from_slice(&bytes).expect("snapshot JSON must parse")
    }

    fn decode_value(value: serde_json::Value) -> SimResult<World> {
        let bytes = serde_json::to_vec(&value).expect("snapshot JSON must encode");
        decode_world(&bytes)
    }

    #[test]
    fn decode_rejects_meta_current_tick_mismatch() {
        let mut value = snapshot_value();
        value["meta"]["current_tick"] = serde_json::json!(1);

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("metadata tick mismatch"));
    }

    #[test]
    fn decode_rejects_next_entity_id_behind_existing_entity() {
        let mut value = snapshot_value();
        value["next_entity_id"] = serde_json::json!(1);

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("next_entity_id"));
    }

    #[test]
    fn decode_rejects_entity_referencing_missing_zone() {
        let mut value = snapshot_value();
        value["entities"]["1"]["position"] = serde_json::json!({
            "zone": 99,
            "pos": { "x": 0, "y": 0, "z": 0 }
        });

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("references missing zone"));
    }

    #[test]
    fn decode_rejects_zone_entity_index_mismatch() {
        let mut value = snapshot_value();
        value["zones"]["0"]["entities"] = serde_json::json!([]);

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("entity index"));
    }

    #[test]
    fn decode_accepts_valid_snapshot() {
        let value = snapshot_value();

        let world = decode_value(value).expect("valid snapshot must decode");

        assert_eq!(world.entity_count(), 1);
        assert_eq!(
            world
                .get_zone(ZoneId::ORIGIN)
                .expect("origin zone must exist")
                .entities,
            vec![EntityId::new(1)]
        );
    }

    #[test]
    fn decode_rejects_entity_in_wrong_zone_index() {
        let mut value = snapshot_value();
        value["entities"]["1"]["position"] = serde_json::json!({
            "zone": 0,
            "pos": { "x": 1, "y": 0, "z": 0 }
        });
        value["zones"]["0"]["entities"] = serde_json::json!([1, 1]);

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("duplicate entity"));
    }

    #[test]
    fn decode_rejects_entity_position_zone_not_in_index() {
        let mut value = snapshot_value();
        value["zones"]["1"] = serde_json::json!({
            "id": 1,
            "name": null,
            "loaded": true,
            "entities": [1]
        });

        let err = decode_value(value).unwrap_err();

        assert!(err.to_string().contains("whose position is in zone"));
    }
}
