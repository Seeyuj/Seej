//! Core simulation logic: apply simulation commands -> events + state.

use sy_api::commands::{CreateWorldCmd, CreateZoneCmd, SimCommand, SpawnEntityCmd};
use sy_api::errors::{ApiError, ApiResult};
use sy_api::events::{DespawnReason, EventData, SimEvent};
use sy_types::{EntityId, EntityKind, EntityState, RngSeed, Tick, ZoneId};

use crate::ports::{IRng, ISimClock};
use crate::world::{Entity, World, Zone};

/// Snapshot of mutable engine state used by infra to roll back failed commits.
pub struct SimulationCheckpoint {
    world: Option<World>,
    rng_seed: RngSeed,
    rng_state: u64,
    clock_tick: Tick,
}

/// Pure simulation engine.
///
/// It owns no storage, WAL, filesystem, networking, tracing, or serialization.
/// Persistence layers may commit returned events and then keep or roll back the
/// in-memory state using `checkpoint`/`restore_checkpoint`.
pub struct Simulation<R: IRng, C: ISimClock> {
    world: Option<World>,
    rng: R,
    clock: C,
    pending_events: Vec<SimEvent>,
}

impl<R: IRng, C: ISimClock> Simulation<R, C> {
    /// Create a new simulation with injected deterministic dependencies.
    pub fn new(rng: R, clock: C) -> Self {
        Self {
            world: None,
            rng,
            clock,
            pending_events: Vec::new(),
        }
    }

    /// Replace the loaded world and restore deterministic runtime state.
    pub fn load_world_state(&mut self, world: World) {
        self.rng.restore_seeded(world.meta.seed, world.rng_state);
        self.clock.set_tick(world.current_tick);
        self.world = Some(world);
        self.pending_events.clear();
    }

    /// Remove the loaded world.
    pub fn clear_world(&mut self) {
        self.world = None;
        self.pending_events.clear();
        self.clock.set_tick(Tick::ZERO);
    }

    pub fn has_world(&self) -> bool {
        self.world.is_some()
    }

    pub fn world(&self) -> Option<&World> {
        self.world.as_ref()
    }

    pub fn world_mut(&mut self) -> Option<&mut World> {
        self.world.as_mut()
    }

    pub fn current_tick(&self) -> Tick {
        self.world
            .as_ref()
            .map(|w| w.current_tick)
            .unwrap_or(Tick::ZERO)
    }

    pub fn rng(&mut self) -> &mut R {
        &mut self.rng
    }

    pub fn clock(&self) -> &C {
        &self.clock
    }

    pub fn checkpoint(&self) -> SimulationCheckpoint {
        SimulationCheckpoint {
            world: self.world.clone(),
            rng_seed: self.rng.seed(),
            rng_state: self.rng.state(),
            clock_tick: self.clock.current_tick(),
        }
    }

    pub fn restore_checkpoint(&mut self, checkpoint: SimulationCheckpoint) {
        self.world = checkpoint.world;
        self.rng
            .restore_seeded(checkpoint.rng_seed, checkpoint.rng_state);
        self.clock.set_tick(checkpoint.clock_tick);
        self.pending_events.clear();
    }

    /// Process a simulation-only command and return the generated events.
    pub fn process_command(&mut self, cmd: SimCommand) -> ApiResult<Vec<SimEvent>> {
        if let Err(errors) = sy_api::validation::validate_sim_command(&cmd) {
            return Err(ApiError::ValidationFailed(errors));
        }

        self.pending_events.clear();
        let checkpoint = self.checkpoint();

        let command_result = match cmd {
            SimCommand::CreateWorld(c) => self.cmd_create_world(c),
            SimCommand::Tick => self.cmd_tick(),
            SimCommand::TickN(n) => {
                for _ in 0..n {
                    self.cmd_tick()?;
                }
                Ok(())
            }
            SimCommand::SpawnEntity(c) => self.cmd_spawn_entity(c),
            SimCommand::DespawnEntity(id) => self.cmd_despawn_entity(id),
            SimCommand::CreateZone(c) => self.cmd_create_zone(c),
        };

        if let Err(err) = command_result {
            self.restore_checkpoint(checkpoint);
            return Err(err);
        }

        Ok(std::mem::take(&mut self.pending_events))
    }

    fn cmd_create_world(&mut self, cmd: CreateWorldCmd) -> ApiResult<()> {
        if let Some(world) = &self.world {
            return Err(ApiError::WorldAlreadyExists(world.id().to_string()));
        }

        let world = World::new(cmd.name.clone(), cmd.seed);
        let world_id = world.id().to_string();

        self.rng.restore_seeded(cmd.seed, cmd.seed.as_u64());
        self.clock.set_tick(Tick::ZERO);

        self.emit(EventData::WorldCreated {
            world_id,
            name: cmd.name,
            seed: cmd.seed,
        });
        self.emit(EventData::ZoneCreated {
            zone_id: ZoneId::ORIGIN,
            name: Some("Origin".to_string()),
        });

        self.world = Some(world);
        Ok(())
    }

    fn cmd_tick(&mut self) -> ApiResult<()> {
        {
            let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;
            world.advance_tick();
            self.clock.set_tick(world.current_tick);
        }

        let (tick, sim_time) = {
            let world = self.world.as_ref().ok_or(ApiError::NoWorldLoaded)?;
            (world.current_tick, world.sim_time)
        };

        let entities_processed = self.run_tick_systems()?;
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

    fn run_tick_systems(&mut self) -> ApiResult<u32> {
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;
        let mut processed = 0u32;

        let entity_ids: Vec<EntityId> = world
            .entities
            .values()
            .filter(|e| e.is_active())
            .map(|e| e.id)
            .collect();

        for entity_id in entity_ids {
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

            match kind {
                EntityKind::Resource => {
                    if let Some(amt) = amount {
                        if amt > 0 && self.rng.chance(0.01) {
                            let new_amount = amt.saturating_sub(1);

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
                    if let Some(hp) = health {
                        if hp > 0 && self.rng.chance(0.005) {
                            let new_health = hp.saturating_sub(1);

                            if let Some(entity) = world.entities.get_mut(&entity_id) {
                                entity.properties.health = Some(new_health);
                                self.pending_events.push(SimEvent::new(
                                    world.current_tick,
                                    EventData::EntityDegraded {
                                        entity_id,
                                        old_health: hp,
                                        new_health,
                                    },
                                ));
                            }

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
    use crate::ports::{IRng, ISimClock};
    use sy_api::commands::{EntityProperties, SpawnEntityCmd};
    use sy_types::{Position, SimTime, WorldPos};

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

    #[test]
    fn command_error_rolls_back_world_rng_clock_and_pending_events() {
        let seed = RngSeed::new(43);
        let mut sim = Simulation::new(TestRng::new(seed), TestClock::new());

        sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
            name: "CommandRollback".to_string(),
            seed,
        }))
        .unwrap();

        let world_before = sim.world().unwrap().clone();
        let rng_before = sim.rng().state();
        let clock_before = sim.clock().current_tick();

        let err = sim
            .process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
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
        assert_eq!(sim.clock().current_tick(), clock_before);
    }
}
