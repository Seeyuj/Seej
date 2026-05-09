//! # Simulation
//!
//! Core simulation logic: apply(commands) -> events + tick loop.
//!
//! ## Design
//! - Deterministic: same inputs = same outputs
//! - Event-sourced: all changes produce events
//! - Pure logic: no I/O (uses injected ports)
//!
//! ## Crash Recovery
//! On LoadWorld:
//! 1. Load snapshot (state at snapshot_tick)
//! 2. Read events with event_id > last_event_id
//! 3. Replay events using apply_event()

use sy_api::commands::{Command, CreateWorldCmd, CreateZoneCmd, SpawnEntityCmd};
use sy_api::errors::{ApiError, ApiResult};
use sy_api::events::{DespawnReason, EventData, SimEvent};
use sy_types::{EntityId, EntityKind, EntityState, Tick, WorldMeta, ZoneId};
use tracing::{debug, info};

use crate::ports::{IEventLog, IRng, ISimClock, IWorldStore};
use crate::replay::apply_event;
use crate::world::{Entity, World, Zone};

/// The simulation engine.
/// Processes commands, runs tick logic, emits events.
pub struct Simulation<R: IRng, C: ISimClock, E: IEventLog, S: IWorldStore> {
    /// The world state (may be None if no world loaded)
    world: Option<World>,
    /// Injected RNG
    rng: R,
    /// Injected clock
    clock: C,
    /// Injected event log
    event_log: E,
    /// Injected world store
    store: S,
    /// Events pending to be recorded
    pending_events: Vec<SimEvent>,
}

impl<R: IRng, C: ISimClock, E: IEventLog, S: IWorldStore> Simulation<R, C, E, S> {
    /// Create a new simulation with injected dependencies.
    pub fn new(rng: R, clock: C, event_log: E, store: S) -> Self {
        Simulation {
            world: None,
            rng,
            clock,
            event_log,
            store,
            pending_events: Vec::new(),
        }
    }

    /// Check if a world is loaded.
    pub fn has_world(&self) -> bool {
        self.world.is_some()
    }

    /// Get a reference to the current world (if loaded).
    pub fn world(&self) -> Option<&World> {
        self.world.as_ref()
    }

    /// Get a mutable reference to the current world.
    pub fn world_mut(&mut self) -> Option<&mut World> {
        self.world.as_mut()
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> Tick {
        self.world
            .as_ref()
            .map(|w| w.current_tick)
            .unwrap_or(Tick::ZERO)
    }

    /// Get access to the RNG.
    pub fn rng(&mut self) -> &mut R {
        &mut self.rng
    }

    // ========================================================================
    // Command processing
    // ========================================================================

    /// Process a command and return resulting events.
    pub fn process_command(&mut self, cmd: Command) -> ApiResult<Vec<SimEvent>> {
        // Validate first
        if let Err(errors) = sy_api::validation::validate_command(&cmd) {
            return Err(ApiError::ValidationFailed(errors));
        }

        self.pending_events.clear();
        let world_before = self.world.clone();
        let rng_seed_before = self.rng.seed();
        let rng_state_before = self.rng.state();
        let clock_tick_before = self.clock.current_tick();

        let command_result = match cmd {
            Command::CreateWorld(c) => self.cmd_create_world(c),
            Command::LoadWorld(c) => self.cmd_load_world(&c.world_id),
            Command::SaveWorld => self.cmd_save_world(),
            Command::Tick => self.cmd_tick(),
            Command::TickN(n) => {
                let mut result = Ok(());
                for _ in 0..n {
                    if let Err(err) = self.cmd_tick() {
                        result = Err(err);
                        break;
                    }
                }
                result
            }
            Command::SpawnEntity(c) => self.cmd_spawn_entity(c),
            Command::DespawnEntity(id) => self.cmd_despawn_entity(id),
            Command::CreateZone(c) => self.cmd_create_zone(c),
            Command::Shutdown => {
                // Save before shutdown
                if self.world.is_some() {
                    self.cmd_save_world()
                } else {
                    Ok(())
                }
            }
        };

        if let Err(err) = command_result {
            self.world = world_before;
            self.rng.restore_seeded(rng_seed_before, rng_state_before);
            self.clock.set_tick(clock_tick_before);
            self.pending_events.clear();
            return Err(err);
        }

        // Record events to log (assigns event_id to each event)
        let persisted = if !self.pending_events.is_empty() {
            let events = std::mem::take(&mut self.pending_events);
            match self.event_log.append_batch(events) {
                Ok(persisted) => persisted,
                Err(e) => {
                    self.world = world_before;
                    self.rng.restore_seeded(rng_seed_before, rng_state_before);
                    self.clock.set_tick(clock_tick_before);
                    self.pending_events.clear();
                    return Err(ApiError::StorageError(e.to_string()));
                }
            }
        } else {
            Vec::new()
        };

        Ok(persisted)
    }

    // ========================================================================
    // Command implementations
    // ========================================================================

    fn cmd_create_world(&mut self, cmd: CreateWorldCmd) -> ApiResult<()> {
        let world = World::new(cmd.name.clone(), cmd.seed);
        let world_id = world.id().to_string();

        // Check if already exists
        if self.store.exists(&world_id) {
            return Err(ApiError::WorldAlreadyExists(world_id));
        }

        // Initialize RNG with world seed
        self.rng.restore_seeded(cmd.seed, cmd.seed.as_u64());

        // Set clock to tick 0
        self.clock.set_tick(Tick::ZERO);

        self.emit(EventData::WorldCreated {
            world_id: world_id.clone(),
            name: cmd.name,
            seed: cmd.seed,
        });

        // Also emit zone created for origin zone
        self.emit(EventData::ZoneCreated {
            zone_id: ZoneId::ORIGIN,
            name: Some("Origin".to_string()),
        });

        self.world = Some(world);

        // Save initial state
        self.cmd_save_world()?;

        Ok(())
    }

    fn cmd_load_world(&mut self, world_id: &str) -> ApiResult<()> {
        if !self.store.exists(world_id) {
            return Err(ApiError::WorldNotFound(world_id.to_string()));
        }

        // Step 1: Load snapshot
        let snapshot = self
            .store
            .load_snapshot(world_id)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        let mut world = World::from_bytes(&snapshot)
            .map_err(|e| ApiError::StorageError(format!("Failed to deserialize world: {}", e)))?;

        let snapshot_tick = world.meta.snapshot_tick;
        let last_event_id = world.meta.last_event_id;

        if world.meta.format_version != WorldMeta::CURRENT_FORMAT_VERSION {
            return Err(ApiError::StorageError(format!(
                "Unsupported world format {} (current {}). No implicit migration is available.",
                world.meta.format_version,
                WorldMeta::CURRENT_FORMAT_VERSION
            )));
        }

        info!(
            "Loaded snapshot at tick {}, last_event_id={}",
            snapshot_tick, last_event_id
        );

        // Step 2: Read events since last_event_id for crash recovery
        let events_to_replay = self
            .event_log
            .read_from_event_id(last_event_id)
            .map_err(|e| ApiError::StorageError(format!("Failed to read WAL: {}", e)))?;

        // Step 3: Replay events
        if !events_to_replay.is_empty() {
            info!(
                "Replaying {} events for crash recovery (from event_id {} to {})",
                events_to_replay.len(),
                events_to_replay
                    .first()
                    .map(|e| e.event_id.as_u64())
                    .unwrap_or(0),
                events_to_replay
                    .last()
                    .map(|e| e.event_id.as_u64())
                    .unwrap_or(0)
            );

            for event in &events_to_replay {
                if let Err(e) = apply_event(&mut world, event) {
                    return Err(ApiError::StorageError(format!(
                        "Replay failed at event {}: {}",
                        event.event_id, e
                    )));
                }
            }

            // Update world tick to the latest event tick
            if let Some(last_event) = events_to_replay.last() {
                if last_event.tick > world.current_tick {
                    world.current_tick = last_event.tick;
                    world.sim_time = sy_types::SimTime::from_ticks(last_event.tick);
                    world.meta.current_tick = last_event.tick;
                    world.meta.sim_time = world.sim_time;
                }
            }

            info!(
                "Crash recovery complete. World now at tick {}",
                world.current_tick
            );
        }

        // Restore RNG state using persisted seed + state.
        // This is done after replay so the RNG continues exactly from the recovered world.
        self.rng.restore_seeded(world.meta.seed, world.rng_state);

        // Restore clock
        self.clock.set_tick(world.current_tick);

        let tick = world.current_tick;
        self.world = Some(world);

        self.emit(EventData::WorldLoaded {
            world_id: world_id.to_string(),
            tick,
        });

        Ok(())
    }

    fn cmd_save_world(&mut self) -> ApiResult<()> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;

        // Update RNG state in world
        world.rng_state = self.rng.state();

        // Update snapshot metadata for crash recovery
        world.meta.snapshot_tick = world.current_tick;
        world.meta.last_event_id = self.event_log.last_event_id();

        debug!(
            "Saving world at tick {}, last_event_id={}",
            world.meta.snapshot_tick, world.meta.last_event_id
        );

        let snapshot = world
            .to_bytes()
            .map_err(|e| ApiError::StorageError(format!("Failed to serialize world: {}", e)))?;

        let world_id = world.id().to_string();

        self.store
            .save_snapshot(&world_id, &snapshot)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        self.store
            .save_meta(&world.meta)
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        // Sync event log
        self.event_log
            .sync()
            .map_err(|e| ApiError::StorageError(e.to_string()))?;

        let tick = world.current_tick;
        self.emit(EventData::WorldSaved { tick });

        Ok(())
    }

    fn cmd_tick(&mut self) -> ApiResult<()> {
        // Advance tick
        {
            let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;
            world.advance_tick();
            self.clock.set_tick(world.current_tick);
        }

        let (tick, sim_time) = {
            let world = self.world.as_ref().ok_or(ApiError::NoWorldLoaded)?;
            (world.current_tick, world.sim_time)
        };

        // Run systemic rules
        let entities_processed = self.run_tick_systems()?;

        // Checkpoint RNG state every tick so replay can restore the exact future stream.
        let rng_state_after = self.rng.state();
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;
        world.rng_state = rng_state_after;

        self.emit(EventData::TickProcessed {
            tick,
            sim_time,
            entities_processed,
            rng_state_after: Some(rng_state_after),
        });

        Ok(())
    }

    fn cmd_spawn_entity(&mut self, cmd: SpawnEntityCmd) -> ApiResult<()> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;

        // Check zone exists
        if !world.has_zone(cmd.position.zone) {
            return Err(ApiError::ZoneNotFound(cmd.position.zone));
        }

        let id = world.allocate_entity_id();
        let tick = world.current_tick;

        let entity = Entity::new(id, cmd.kind, cmd.position, tick, cmd.properties.clone());

        world.add_entity(entity);

        self.emit(EventData::EntitySpawned {
            entity_id: id,
            kind: cmd.kind,
            position: cmd.position,
            properties: cmd.properties,
        });

        Ok(())
    }

    fn cmd_despawn_entity(&mut self, id: EntityId) -> ApiResult<()> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;

        if world.remove_entity(id).is_none() {
            return Err(ApiError::EntityNotFound(id));
        }

        self.emit(EventData::EntityDespawned {
            entity_id: id,
            reason: DespawnReason::Command,
        });

        Ok(())
    }

    fn cmd_create_zone(&mut self, cmd: CreateZoneCmd) -> ApiResult<()> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;

        if world.has_zone(cmd.zone_id) {
            return Err(ApiError::ZoneAlreadyExists(cmd.zone_id));
        }

        let zone = Zone::new(cmd.zone_id, cmd.name.clone());
        world.add_zone(zone);

        self.emit(EventData::ZoneCreated {
            zone_id: cmd.zone_id,
            name: cmd.name,
        });

        Ok(())
    }

    // ========================================================================
    // Tick systems (Phase 1: minimal rules)
    // ========================================================================

    /// Run all tick-based systems. Returns number of entities processed.
    fn run_tick_systems(&mut self) -> ApiResult<u32> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;
        let mut processed = 0u32;

        // Collect entity IDs to process (avoid borrow issues)
        let entity_ids: Vec<EntityId> = world
            .entities
            .values()
            .filter(|e| e.is_active())
            .map(|e| e.id)
            .collect();

        for entity_id in entity_ids {
            // Get entity data
            let (kind, health, amount) = {
                let entity = match world.entities.get(&entity_id) {
                    Some(e) => e,
                    None => continue,
                };
                (
                    entity.kind,
                    entity.properties.health,
                    entity.properties.amount,
                )
            };

            // Apply rules based on entity kind
            match kind {
                EntityKind::Resource => {
                    // Resources degrade over time (simple rule)
                    if let Some(amt) = amount {
                        if amt > 0 && self.rng.chance(0.01) {
                            // 1% chance per tick to lose 1 unit
                            let new_amount = amt.saturating_sub(1);

                            // Update entity
                            if let Some(entity) = world.entities.get_mut(&entity_id) {
                                entity.properties.amount = Some(new_amount);
                            }

                            self.pending_events.push(SimEvent::new(
                                world.current_tick,
                                EventData::ResourceDepleted {
                                    entity_id,
                                    amount: 1,
                                    remaining: new_amount,
                                },
                            ));

                            // If depleted, mark as dead
                            if new_amount == 0 {
                                if let Some(entity) = world.entities.get_mut(&entity_id) {
                                    let old_state = entity.state;
                                    entity.state = EntityState::Dead;

                                    self.pending_events.push(SimEvent::new(
                                        world.current_tick,
                                        EventData::EntityStateChanged {
                                            entity_id,
                                            old_state,
                                            new_state: EntityState::Dead,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
                EntityKind::Creature => {
                    // Creatures degrade health over time (hunger/decay)
                    if let Some(hp) = health {
                        if hp > 0 && self.rng.chance(0.005) {
                            // 0.5% chance per tick
                            let new_health = hp.saturating_sub(1);

                            if let Some(entity) = world.entities.get_mut(&entity_id) {
                                let old_health = hp;
                                entity.properties.health = Some(new_health);

                                self.pending_events.push(SimEvent::new(
                                    world.current_tick,
                                    EventData::EntityDegraded {
                                        entity_id,
                                        old_health,
                                        new_health,
                                    },
                                ));
                            }

                            // If dead, mark as dead
                            if new_health == 0 {
                                if let Some(entity) = world.entities.get_mut(&entity_id) {
                                    let old_state = entity.state;
                                    entity.state = EntityState::Dead;

                                    self.pending_events.push(SimEvent::new(
                                        world.current_tick,
                                        EventData::EntityStateChanged {
                                            entity_id,
                                            old_state,
                                            new_state: EntityState::Dead,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            processed += 1;
        }

        // Clean up dead entities periodically (every 100 ticks)
        if world.current_tick.as_u64() % 100 == 0 {
            let dead_ids: Vec<EntityId> = world
                .entities
                .values()
                .filter(|e| e.is_dead())
                .map(|e| e.id)
                .collect();

            for id in dead_ids {
                if world.remove_entity(id).is_some() {
                    self.pending_events.push(SimEvent::new(
                        world.current_tick,
                        EventData::EntityDespawned {
                            entity_id: id,
                            reason: DespawnReason::Death,
                        },
                    ));
                }
            }
        }

        Ok(processed)
    }

    // ========================================================================
    // Event emission
    // ========================================================================

    fn emit(&mut self, data: EventData) {
        let tick = self
            .world
            .as_ref()
            .map(|w| w.current_tick)
            .unwrap_or(Tick::ZERO);
        self.pending_events.push(SimEvent::new(tick, data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{IEventLog, IRng, ISimClock, IWorldStore, WorldSnapshot};
    use std::collections::HashMap;
    use sy_api::commands::{EntityProperties, SpawnEntityCmd};
    use sy_types::{
        EntityId, EventId, Position, RngSeed, SimError, SimResult, SimTime, WorldMeta, WorldPos,
    };

    struct TestRng {
        seed: RngSeed,
        state: u64,
    }

    impl TestRng {
        fn new(seed: RngSeed) -> Self {
            Self {
                seed,
                state: seed.as_u64(),
            }
        }
    }

    impl IRng for TestRng {
        fn seed(&self) -> RngSeed {
            self.seed
        }

        fn state(&self) -> u64 {
            self.state
        }

        fn restore(&mut self, state: u64) {
            self.state = state;
        }

        fn restore_seeded(&mut self, seed: RngSeed, state: u64) {
            self.seed = seed;
            self.state = state;
        }

        fn next_u32(&mut self) -> u32 {
            self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (self.state >> 32) as u32
        }

        fn next_u64(&mut self) -> u64 {
            let hi = self.next_u32() as u64;
            let lo = self.next_u32() as u64;
            (hi << 32) | lo
        }
    }

    struct TestClock {
        tick: Tick,
    }

    impl TestClock {
        fn new() -> Self {
            Self { tick: Tick::ZERO }
        }
    }

    impl ISimClock for TestClock {
        fn current_tick(&self) -> Tick {
            self.tick
        }

        fn sim_time(&self) -> SimTime {
            SimTime::from_ticks(self.tick)
        }

        fn advance(&mut self) -> Tick {
            self.tick = self.tick.next();
            self.tick
        }

        fn set_tick(&mut self, tick: Tick) {
            self.tick = tick;
        }

        fn should_tick(&self) -> bool {
            true
        }
    }

    struct TestWorldStore {
        metas: HashMap<String, WorldMeta>,
        snapshots: HashMap<String, WorldSnapshot>,
    }

    impl TestWorldStore {
        fn new() -> Self {
            Self {
                metas: HashMap::new(),
                snapshots: HashMap::new(),
            }
        }
    }

    impl IWorldStore for TestWorldStore {
        fn exists(&self, world_id: &str) -> bool {
            self.metas.contains_key(world_id)
        }

        fn list_worlds(&self) -> SimResult<Vec<String>> {
            Ok(self.metas.keys().cloned().collect())
        }

        fn load_meta(&self, world_id: &str) -> SimResult<WorldMeta> {
            self.metas
                .get(world_id)
                .cloned()
                .ok_or_else(|| SimError::PersistenceError(format!("World not found: {}", world_id)))
        }

        fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()> {
            self.metas.insert(meta.world_id.clone(), meta.clone());
            Ok(())
        }

        fn load_snapshot(&self, world_id: &str) -> SimResult<WorldSnapshot> {
            self.snapshots.get(world_id).cloned().ok_or_else(|| {
                SimError::PersistenceError(format!("Snapshot not found: {}", world_id))
            })
        }

        fn save_snapshot(&mut self, world_id: &str, snapshot: &WorldSnapshot) -> SimResult<()> {
            self.snapshots
                .insert(world_id.to_string(), snapshot.clone());
            Ok(())
        }

        fn delete_world(&mut self, world_id: &str) -> SimResult<()> {
            self.metas.remove(world_id);
            self.snapshots.remove(world_id);
            Ok(())
        }

        fn world_path(&self, world_id: &str) -> String {
            format!("test://{}", world_id)
        }
    }

    struct FailOnNthBatchLog {
        events: Vec<SimEvent>,
        next_event_id: u64,
        batch_calls: usize,
        fail_on_batch: usize,
    }

    impl FailOnNthBatchLog {
        fn new(fail_on_batch: usize) -> Self {
            Self {
                events: Vec::new(),
                next_event_id: 1,
                batch_calls: 0,
                fail_on_batch,
            }
        }
    }

    impl IEventLog for FailOnNthBatchLog {
        fn append(&mut self, event: SimEvent) -> SimResult<SimEvent> {
            let mut persisted = self.append_batch(vec![event])?;
            Ok(persisted.remove(0))
        }

        fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>> {
            self.batch_calls += 1;
            if self.batch_calls == self.fail_on_batch {
                return Err(SimError::PersistenceError(
                    "forced append failure".to_string(),
                ));
            }

            let mut persisted = Vec::with_capacity(events.len());
            for mut event in events {
                event.event_id = EventId::new(self.next_event_id);
                self.next_event_id += 1;
                self.events.push(event.clone());
                persisted.push(event);
            }
            Ok(persisted)
        }

        fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>> {
            Ok(self
                .events
                .iter()
                .filter(|event| event.event_id > from_id)
                .cloned()
                .collect())
        }

        fn read_all_valid(&self) -> SimResult<Vec<SimEvent>> {
            Ok(self.events.clone())
        }

        fn last_event_id(&self) -> EventId {
            self.events
                .last()
                .map(|event| event.event_id)
                .unwrap_or(EventId::ZERO)
        }

        fn last_tick(&self) -> Option<Tick> {
            self.events.last().map(|event| event.tick)
        }

        fn truncate_after(&mut self, event_id: EventId) -> SimResult<()> {
            self.events.retain(|event| event.event_id <= event_id);
            self.next_event_id = self
                .events
                .last()
                .map(|event| event.event_id.as_u64() + 1)
                .unwrap_or(1);
            Ok(())
        }

        fn sync(&mut self) -> SimResult<()> {
            Ok(())
        }

        fn len(&self) -> usize {
            self.events.len()
        }
    }

    #[test]
    fn append_failure_rolls_back_world_rng_and_clock() {
        let seed = RngSeed::new(42);
        let mut sim = Simulation::new(
            TestRng::new(seed),
            TestClock::new(),
            FailOnNthBatchLog::new(3),
            TestWorldStore::new(),
        );

        sim.process_command(Command::CreateWorld(CreateWorldCmd {
            name: "Rollback".to_string(),
            seed,
        }))
        .unwrap();
        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(0, 0, 0)),
            kind: EntityKind::Resource,
            properties: EntityProperties {
                name: Some("ore".to_string()),
                amount: Some(10),
                health: None,
            },
        }))
        .unwrap();

        let rng_before = sim.rng().state();
        let err = sim.process_command(Command::Tick).unwrap_err();
        assert!(matches!(err, ApiError::StorageError(_)));

        let world = sim.world().unwrap();
        assert_eq!(world.current_tick, Tick::ZERO);
        assert_eq!(world.sim_time, sy_types::SimTime::ZERO);
        assert_eq!(
            world
                .get_entity(EntityId::new(1))
                .unwrap()
                .properties
                .amount,
            Some(10)
        );
        assert_eq!(sim.current_tick(), Tick::ZERO);
        assert_eq!(sim.rng().state(), rng_before);
    }

    #[test]
    fn command_error_rolls_back_world_rng_clock_and_pending_events() {
        let seed = RngSeed::new(43);
        let mut sim = Simulation::new(
            TestRng::new(seed),
            TestClock::new(),
            FailOnNthBatchLog::new(usize::MAX),
            TestWorldStore::new(),
        );

        sim.process_command(Command::CreateWorld(CreateWorldCmd {
            name: "CommandRollback".to_string(),
            seed,
        }))
        .unwrap();

        let world_before = sim.world().unwrap().clone();
        let rng_before = sim.rng().state();
        let clock_before = sim.clock.current_tick();

        let err = sim
            .process_command(Command::SpawnEntity(SpawnEntityCmd {
                position: WorldPos::new(ZoneId::new(99), Position::new(0, 0, 0)),
                kind: EntityKind::Resource,
                properties: EntityProperties::default(),
            }))
            .unwrap_err();

        assert!(matches!(err, ApiError::ZoneNotFound(_)));
        let world_after = sim.world().unwrap();
        assert_eq!(world_after.current_tick, world_before.current_tick);
        assert_eq!(world_after.next_entity_id, world_before.next_entity_id);
        assert_eq!(world_after.entity_count(), world_before.entity_count());
        assert_eq!(sim.rng().state(), rng_before);
        assert_eq!(sim.clock.current_tick(), clock_before);
        assert!(sim.pending_events.is_empty());
    }

    #[test]
    fn load_world_rejects_unsupported_snapshot_format() {
        let seed = RngSeed::new(44);
        let mut world = World::new("OldFormat".to_string(), seed);
        world.meta.format_version = WorldMeta::CURRENT_FORMAT_VERSION - 1;
        let world_id = world.id().to_string();
        let snapshot = world.to_bytes().unwrap();

        let mut store = TestWorldStore::new();
        store.metas.insert(world_id.clone(), world.meta.clone());
        store.snapshots.insert(world_id.clone(), snapshot);

        let mut sim = Simulation::new(
            TestRng::new(seed),
            TestClock::new(),
            FailOnNthBatchLog::new(usize::MAX),
            store,
        );

        let err = sim
            .process_command(Command::LoadWorld(sy_api::commands::LoadWorldCmd {
                world_id,
            }))
            .unwrap_err();

        assert!(
            matches!(err, ApiError::StorageError(msg) if msg.contains("Unsupported world format"))
        );
        assert!(sim.world().is_none());
        assert!(sim.pending_events.is_empty());
    }
}
