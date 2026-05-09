//! Deterministic state hashing and simulation-run helpers.

use sy_api::commands::{CreateWorldCmd, SimCommand};
use sy_types::{RngSeed, Tick};

use crate::ports::{IRng, ISimClock, IStateHasher, StateHash};
use crate::world::World;
use crate::Simulation;

/// Compute a canonical hash of the complete persistent world state.
///
/// This includes recovery cursor metadata such as `snapshot_tick` and
/// `last_event_id`. Compare hashes only after both worlds have the same
/// persistence cursor normalization, for example after `save_world()`.
pub fn compute_canonical_hash(world: &World, hasher: &mut dyn IStateHasher) -> StateHash {
    hasher.reset();

    write_str(hasher, &world.meta.world_id);
    write_str(hasher, &world.meta.name);
    write_u64(hasher, world.meta.seed.as_u64());
    write_u64(hasher, world.meta.current_tick.as_u64());
    write_u64(hasher, world.meta.sim_time.units);
    write_u64(hasher, world.meta.created_tick.as_u64());
    write_u64(hasher, world.meta.snapshot_tick.as_u64());
    write_u64(hasher, world.meta.last_event_id.as_u64());
    write_u32(hasher, world.meta.format_version);

    write_u64(hasher, world.current_tick.as_u64());
    write_u64(hasher, world.sim_time.units);
    write_u64(hasher, world.rng_state);
    write_u64(hasher, world.next_entity_id);

    write_u64(hasher, world.entities.len() as u64);
    for (id, entity) in &world.entities {
        write_u64(hasher, id.as_u64());
        write_u64(hasher, entity.id.as_u64());
        write_u8(hasher, entity_kind_byte(entity.kind));
        write_u8(hasher, entity_state_byte(entity.state));
        write_u32(hasher, entity.position.zone.as_u32());
        write_i32(hasher, entity.position.pos.x);
        write_i32(hasher, entity.position.pos.y);
        write_i32(hasher, entity.position.pos.z);
        write_u64(hasher, entity.created_at.as_u64());
        write_opt_str(hasher, entity.properties.name.as_deref());
        write_opt_u32(hasher, entity.properties.amount);
        write_opt_u32(hasher, entity.properties.health);
    }

    write_u64(hasher, world.zones.len() as u64);
    for (id, zone) in &world.zones {
        write_u32(hasher, id.as_u32());
        write_u32(hasher, zone.id.as_u32());
        write_opt_str(hasher, zone.name.as_deref());
        write_u8(hasher, u8::from(zone.loaded));
        write_u64(hasher, zone.entities.len() as u64);
        for entity_id in &zone.entities {
            write_u64(hasher, entity_id.as_u64());
        }
    }

    hasher.finalize()
}

fn entity_kind_byte(kind: sy_types::EntityKind) -> u8 {
    match kind {
        sy_types::EntityKind::Resource => 0,
        sy_types::EntityKind::Creature => 1,
        sy_types::EntityKind::Item => 2,
        sy_types::EntityKind::Structure => 3,
        _ => 255,
    }
}

fn entity_state_byte(state: sy_types::EntityState) -> u8 {
    match state {
        sy_types::EntityState::Active => 0,
        sy_types::EntityState::Dormant => 1,
        sy_types::EntityState::Dead => 2,
    }
}

fn write_u8(hasher: &mut dyn IStateHasher, value: u8) {
    hasher.update(&[value]);
}

fn write_u32(hasher: &mut dyn IStateHasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

fn write_i32(hasher: &mut dyn IStateHasher, value: i32) {
    hasher.update(&value.to_le_bytes());
}

fn write_u64(hasher: &mut dyn IStateHasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

fn write_str(hasher: &mut dyn IStateHasher, value: &str) {
    write_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn write_opt_str(hasher: &mut dyn IStateHasher, value: Option<&str>) {
    match value {
        Some(value) => {
            write_u8(hasher, 1);
            write_str(hasher, value);
        }
        None => write_u8(hasher, 0),
    }
}

fn write_opt_u32(hasher: &mut dyn IStateHasher, value: Option<u32>) {
    match value {
        Some(value) => {
            write_u8(hasher, 1);
            write_u32(hasher, value);
        }
        None => write_u8(hasher, 0),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub tick: Tick,
    pub hash: StateHash,
}

#[derive(Debug, Clone)]
pub struct ScheduledCommand {
    pub tick: Tick,
    pub command: SimCommand,
}

pub struct DeterministicRunConfig {
    pub seed: RngSeed,
    pub world_name: String,
    pub inputs: Vec<ScheduledCommand>,
    pub total_ticks: u64,
    pub checkpoint_every: u64,
}

pub struct DeterministicRunResult {
    pub checkpoints: Vec<Checkpoint>,
    pub final_tick: Tick,
}

pub fn run_deterministic<R, C>(
    config: &DeterministicRunConfig,
    rng: R,
    clock: C,
    hasher: &mut dyn IStateHasher,
) -> DeterministicRunResult
where
    R: IRng,
    C: ISimClock,
{
    let mut sim = Simulation::new(rng, clock);
    let mut checkpoints = Vec::new();

    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: config.world_name.clone(),
        seed: config.seed,
    }))
    .expect("Failed to create world");

    let mut inputs = config.inputs.clone();
    inputs.sort_by_key(|s| s.tick.as_u64());
    let mut input_idx = 0;

    for tick_num in 0..config.total_ticks {
        let current_tick = Tick(tick_num);

        while input_idx < inputs.len() && inputs[input_idx].tick <= current_tick {
            sim.process_command(inputs[input_idx].command.clone())
                .expect("Command execution failed");
            input_idx += 1;
        }

        sim.process_command(SimCommand::Tick).expect("Tick failed");

        let should_checkpoint =
            config.checkpoint_every > 0 && (tick_num + 1) % config.checkpoint_every == 0;

        if should_checkpoint || tick_num + 1 == config.total_ticks {
            if let Some(world) = sim.world() {
                checkpoints.push(Checkpoint {
                    tick: world.current_tick,
                    hash: compute_canonical_hash(world, hasher),
                });
            }
        }
    }

    let final_tick = sim.world().map(|w| w.current_tick).unwrap_or(Tick::ZERO);
    DeterministicRunResult {
        checkpoints,
        final_tick,
    }
}

/// Compare two run results for determinism.
pub fn verify_determinism(
    run_a: &DeterministicRunResult,
    run_b: &DeterministicRunResult,
) -> Result<(), Tick> {
    if run_a.checkpoints.len() != run_b.checkpoints.len() {
        return Err(Tick::ZERO);
    }

    for (a, b) in run_a.checkpoints.iter().zip(run_b.checkpoints.iter()) {
        if a.tick != b.tick {
            return Err(a.tick.min(b.tick));
        }
        if a.hash != b.hash {
            return Err(a.tick);
        }
    }

    Ok(())
}

impl From<SimCommand> for ScheduledCommand {
    fn from(command: SimCommand) -> Self {
        Self {
            tick: Tick::ZERO,
            command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{IRng, ISimClock};
    use sy_types::{SimTime, ZoneId};

    struct TestHasher(u64);

    impl IStateHasher for TestHasher {
        fn reset(&mut self) {
            self.0 = 0;
        }

        fn update(&mut self, data: &[u8]) {
            for byte in data {
                self.0 = self
                    .0
                    .wrapping_mul(1099511628211)
                    .wrapping_add(*byte as u64);
            }
        }

        fn finalize(&self) -> StateHash {
            StateHash(self.0)
        }
    }

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

    struct TestClock(Tick);

    impl ISimClock for TestClock {
        fn current_tick(&self) -> Tick {
            self.0
        }

        fn sim_time(&self) -> SimTime {
            SimTime::from_ticks(self.0)
        }

        fn advance(&mut self) -> Tick {
            self.0 = self.0.next();
            self.0
        }

        fn set_tick(&mut self, tick: Tick) {
            self.0 = tick;
        }

        fn should_tick(&self) -> bool {
            true
        }
    }

    #[test]
    fn canonical_hash_changes_when_zone_membership_changes() {
        let seed = RngSeed::new(42);
        let mut a = World::new("Hash".to_string(), seed);
        let mut b = a.clone();
        a.zones
            .get_mut(&ZoneId::ORIGIN)
            .unwrap()
            .entities
            .push(sy_types::EntityId::new(1));
        b.zones
            .get_mut(&ZoneId::ORIGIN)
            .unwrap()
            .entities
            .push(sy_types::EntityId::new(2));

        let mut hasher = TestHasher(0);
        let hash_a = compute_canonical_hash(&a, &mut hasher);
        let hash_b = compute_canonical_hash(&b, &mut hasher);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn deterministic_replay_same_inputs_same_hashes() {
        let seed = RngSeed::new(42);
        let config = DeterministicRunConfig {
            seed,
            world_name: "Test World".to_string(),
            inputs: Vec::new(),
            total_ticks: 10,
            checkpoint_every: 5,
        };

        let mut hasher_a = TestHasher(0);
        let mut hasher_b = TestHasher(0);
        let a = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock(Tick::ZERO),
            &mut hasher_a,
        );
        let b = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock(Tick::ZERO),
            &mut hasher_b,
        );

        verify_determinism(&a, &b).expect("runs must match");
    }
}
