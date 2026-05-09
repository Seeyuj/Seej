//! Persistent runtime orchestration for snapshots, WAL, and recovery.

use sy_api::commands::SimCommand;
use sy_api::errors::{ApiError, ApiResult};
use sy_api::events::SimEvent;
use sy_api::persistence::{IEventLog, IWorldStore, WorldStorageStatus};
use sy_api::validation::validate_world_id_result;
use sy_core::ports::{IRng, ISimClock};
use sy_core::{replay_events, Simulation, World};
use sy_types::{EventId, SimError, SimResult, WorldMeta};

use crate::snapshot::{decode_world, encode_world};

/// Persistent runtime that commits core events to WAL before accepting state.
pub struct PersistentSimulation<R: IRng, C: ISimClock, E: IEventLog, S: IWorldStore> {
    sim: Simulation<R, C>,
    event_log: E,
    store: S,
}

impl<R, C, E, S> PersistentSimulation<R, C, E, S>
where
    R: IRng,
    C: ISimClock,
    E: IEventLog,
    S: IWorldStore,
{
    pub fn new(rng: R, clock: C, event_log: E, store: S) -> Self {
        Self {
            sim: Simulation::new(rng, clock),
            event_log,
            store,
        }
    }

    pub fn simulation(&self) -> &Simulation<R, C> {
        &self.sim
    }

    pub fn world(&self) -> Option<&World> {
        self.sim.world()
    }

    pub fn current_tick(&self) -> sy_types::Tick {
        self.sim.current_tick()
    }

    pub fn process_command(&mut self, command: SimCommand) -> ApiResult<Vec<SimEvent>> {
        if let SimCommand::CreateWorld(c) = &command {
            let world_id = format!("world_{}", c.seed.as_u64());
            match self
                .store
                .storage_status(&world_id)
                .map_err(|e| ApiError::StorageError(e.to_string()))?
            {
                WorldStorageStatus::Absent => {}
                WorldStorageStatus::Complete => {
                    return Err(ApiError::WorldAlreadyExists(world_id));
                }
                WorldStorageStatus::Incomplete { reason } => {
                    return Err(ApiError::StorageError(format!(
                        "Cannot create world {}: incomplete persistent storage exists ({})",
                        world_id, reason
                    )));
                }
            }

            if !self.event_log.is_empty() {
                return Err(ApiError::StorageError(format!(
                    "Cannot create world {}: WAL already contains {} durable events without a coherent snapshot/meta pair",
                    world_id,
                    self.event_log.len()
                )));
            }
        }

        let checkpoint = self.sim.checkpoint();
        let events = self.sim.process_command(command)?;

        match self.event_log.append_batch(events) {
            Ok(persisted) => Ok(persisted),
            Err(e) => {
                self.sim.restore_checkpoint(checkpoint);
                Err(ApiError::StorageError(e.to_string()))
            }
        }
    }

    pub fn save_world(&mut self) -> ApiResult<()> {
        let mut world = self.sim.world().cloned().ok_or(ApiError::NoWorldLoaded)?;
        let wal_last_event_id = self.event_log.last_event_id();

        validate_wal_covers_snapshot(world.meta.last_event_id, wal_last_event_id)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        world.meta.current_tick = world.current_tick;
        world.meta.sim_time = world.sim_time;
        world.meta.snapshot_tick = world.current_tick;
        world.meta.last_event_id = wal_last_event_id;

        let snapshot = encode_world(&world).map_err(|e| ApiError::StorageError(e.to_string()))?;
        let world_id = world.id().to_string();

        self.event_log
            .sync()
            .map_err(|e| ApiError::StorageError(e.to_string()))?;
        self.store
            .save_snapshot(&world_id, &snapshot)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;
        self.store
            .save_meta(&world.meta)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        self.sim.load_world_state(world);
        Ok(())
    }

    pub fn load_world(&mut self, world_id: &str) -> ApiResult<Vec<SimEvent>> {
        let (world, replayed) = load_recovered_world(&self.store, &self.event_log, world_id)
            .map_err(|e| match e {
                SimError::NotFound(_) => ApiError::WorldNotFound(world_id.to_string()),
                other => ApiError::StorageError(other.to_string()),
            })?;
        self.sim.load_world_state(world);
        Ok(replayed)
    }

    pub fn into_parts(self) -> (Simulation<R, C>, E, S) {
        (self.sim, self.event_log, self.store)
    }
}

/// Load a snapshot and replay durable WAL events after the snapshot cursor.
pub fn load_recovered_world<S, E>(
    store: &S,
    event_log: &E,
    world_id: &str,
) -> SimResult<(World, Vec<SimEvent>)>
where
    S: IWorldStore,
    E: IEventLog,
{
    validate_world_id_result(world_id)?;

    let snapshot = store.load_snapshot(world_id)?;
    let mut world = decode_world(&snapshot)?;
    validate_snapshot_world_id(&world, world_id)?;

    match store.load_meta(world_id) {
        Ok(meta) => validate_recovered_meta_mirror(&world.meta, &meta)?,
        Err(err) if is_missing_meta_error(&err) => {}
        Err(err) => return Err(err),
    }

    let last_event_id = world.meta.last_event_id;
    let announced_wal_last_event_id = event_log.last_event_id();
    let all_events = event_log.read_all_valid()?;
    let durable_wal_last_event_id = all_events
        .last()
        .map(|event| event.event_id)
        .unwrap_or(EventId::ZERO);

    validate_wal_cursor_matches_durable_tail(
        announced_wal_last_event_id,
        durable_wal_last_event_id,
    )?;
    validate_wal_prefix_is_contiguous(&all_events)?;
    validate_wal_covers_snapshot(last_event_id, durable_wal_last_event_id)?;
    let replayed: Vec<_> = all_events
        .into_iter()
        .filter(|event| event.event_id > last_event_id)
        .collect();
    validate_replay_covers_wal_tail(last_event_id, durable_wal_last_event_id, &replayed)?;
    replay_events(&mut world, &replayed)
        .map_err(|e| SimError::CorruptedState(format!("Replay failed: {}", e)))?;

    Ok((world, replayed))
}

pub fn load_snapshot_world<S: IWorldStore>(store: &S, world_id: &str) -> SimResult<World> {
    validate_world_id_result(world_id)?;
    let snapshot = store.load_snapshot(world_id)?;
    let world = decode_world(&snapshot)?;
    validate_snapshot_world_id(&world, world_id)?;
    let meta = store.load_meta(world_id)?;
    validate_meta_mirror(&world.meta, &meta)?;
    Ok(world)
}

fn validate_snapshot_world_id(world: &World, requested_world_id: &str) -> SimResult<()> {
    if world.id() != requested_world_id {
        return Err(SimError::CorruptedState(format!(
            "Snapshot world_id mismatch: requested {}, snapshot {}",
            requested_world_id,
            world.id()
        )));
    }

    Ok(())
}

fn validate_meta_mirror(snapshot_meta: &WorldMeta, file_meta: &WorldMeta) -> SimResult<()> {
    if snapshot_meta != file_meta {
        return Err(SimError::CorruptedState(
            "meta.json does not match snapshot metadata".to_string(),
        ));
    }
    Ok(())
}

fn validate_recovered_meta_mirror(
    snapshot_meta: &WorldMeta,
    file_meta: &WorldMeta,
) -> SimResult<()> {
    if snapshot_meta == file_meta {
        return Ok(());
    }

    if is_stale_meta_from_interrupted_save(snapshot_meta, file_meta) {
        return Ok(());
    }

    Err(SimError::CorruptedState(
        "meta.json does not match snapshot metadata".to_string(),
    ))
}

fn is_stale_meta_from_interrupted_save(snapshot_meta: &WorldMeta, file_meta: &WorldMeta) -> bool {
    snapshot_meta.world_id == file_meta.world_id
        && snapshot_meta.seed == file_meta.seed
        && snapshot_meta.format_version == file_meta.format_version
        && snapshot_meta.created_tick == file_meta.created_tick
        && file_meta.last_event_id < snapshot_meta.last_event_id
        && file_meta.snapshot_tick <= snapshot_meta.snapshot_tick
        && file_meta.current_tick <= snapshot_meta.current_tick
        && file_meta.sim_time <= snapshot_meta.sim_time
}

fn is_missing_meta_error(err: &SimError) -> bool {
    matches!(err, SimError::NotFound(_))
}

fn validate_wal_covers_snapshot(
    snapshot_last_event_id: EventId,
    wal_last_event_id: EventId,
) -> SimResult<()> {
    if wal_last_event_id < snapshot_last_event_id {
        return Err(SimError::CorruptedState(format!(
            "WAL cursor is behind snapshot cursor: wal last_event_id {} < snapshot last_event_id {}",
            wal_last_event_id, snapshot_last_event_id
        )));
    }

    Ok(())
}

fn validate_wal_cursor_matches_durable_tail(
    announced_wal_last_event_id: EventId,
    durable_wal_last_event_id: EventId,
) -> SimResult<()> {
    if announced_wal_last_event_id != durable_wal_last_event_id {
        return Err(SimError::CorruptedState(format!(
            "WAL cursor does not match durable tail: announced last_event_id {} != durable last_event_id {}",
            announced_wal_last_event_id, durable_wal_last_event_id
        )));
    }

    Ok(())
}

fn validate_wal_prefix_is_contiguous(events: &[SimEvent]) -> SimResult<()> {
    let mut expected_event_id = EventId::new(1);

    for event in events {
        if event.event_id != expected_event_id {
            return Err(SimError::CorruptedState(format!(
                "WAL prefix is not contiguous: expected event_id {}, found {}",
                expected_event_id, event.event_id
            )));
        }
        expected_event_id = expected_event_id.next();
    }

    Ok(())
}

fn validate_replay_covers_wal_tail(
    snapshot_last_event_id: EventId,
    wal_last_event_id: EventId,
    replayed: &[SimEvent],
) -> SimResult<()> {
    let mut expected_event_id = snapshot_last_event_id.next();

    for event in replayed {
        if event.event_id != expected_event_id {
            return Err(SimError::CorruptedState(format!(
                "WAL replay range is incomplete: expected event_id {}, found {}",
                expected_event_id, event.event_id
            )));
        }
        expected_event_id = expected_event_id.next();
    }

    if expected_event_id != wal_last_event_id.next() {
        return Err(SimError::CorruptedState(format!(
            "WAL replay range ended before WAL cursor: next expected event_id {}, wal last_event_id {}",
            expected_event_id, wal_last_event_id
        )));
    }

    Ok(())
}
