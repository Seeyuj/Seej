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
use sy_types::{EntityId, EntityKind, EntityState, Tick, ZoneId};
use tracing::{debug, info, warn};

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

        match cmd {
            Command::CreateWorld(c) => self.cmd_create_world(c)?,
            Command::LoadWorld(c) => self.cmd_load_world(&c.world_id)?,
            Command::SaveWorld => self.cmd_save_world()?,
            Command::Tick => self.cmd_tick()?,
            Command::TickN(n) => {
                for _ in 0..n {
                    self.cmd_tick()?;
                }
            }
            Command::SpawnEntity(c) => self.cmd_spawn_entity(c)?,
            Command::DespawnEntity(id) => self.cmd_despawn_entity(id)?,
            Command::CreateZone(c) => self.cmd_create_zone(c)?,
            Command::Shutdown => {
                // Save before shutdown
                if self.world.is_some() {
                    self.cmd_save_world()?;
                }
            }
        }

        // Record events to log (assigns event_id to each event)
        let persisted = if !self.pending_events.is_empty() {
            let events = std::mem::take(&mut self.pending_events);
            self.event_log
                .append_batch(events)
                .map_err(|e| ApiError::StorageError(e.to_string()))?
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
        self.rng.restore(cmd.seed.as_u64());

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
                    warn!("Failed to replay event {}: {}", event.event_id, e);
                    // Continue anyway - events may reference entities that no longer exist
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

        // Restore RNG state
        self.rng.restore(world.rng_state);

        // Restore clock
        self.clock.set_tick(world.current_tick);

        let tick = world.current_tick;

        self.emit(EventData::WorldLoaded {
            world_id: world_id.to_string(),
            tick,
        });

        self.world = Some(world);

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
        let world = self.world.as_mut().ok_or(ApiError::NoWorldLoaded)?;

        // Advance tick
        world.advance_tick();
        self.clock.set_tick(world.current_tick);

        let tick = world.current_tick;
        let sim_time = world.sim_time;

        // Run systemic rules
        let entities_processed = self.run_tick_systems()?;

        self.emit(EventData::TickProcessed {
            tick,
            sim_time,
            entities_processed,
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
    // Tests will use sy_testkit mocks
}
