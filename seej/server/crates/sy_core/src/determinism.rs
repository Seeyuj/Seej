//! # Determinism
//!
//! Tools for verifying simulation determinism.
//!
//! ## Contract
//! Same genesis (seed + config) + same inputs (commands in order) = same outputs (state hash)
//!
//! ## Usage
//! ```ignore
//! let hashes_a = run_deterministic(&genesis, &inputs, steps, checkpoint_every);
//! let hashes_b = run_deterministic(&genesis, &inputs, steps, checkpoint_every);
//! assert_eq!(hashes_a, hashes_b);
//! ```

use byteorder::{LittleEndian, WriteBytesExt};
use xxhash_rust::xxh64::Xxh64;

use sy_api::commands::Command;
use sy_types::{RngSeed, Tick};

use crate::ports::{IEventLog, IRng, ISimClock, IStateHasher, IWorldStore, StateHash};
use crate::world::World;
use crate::Simulation;

// ============================================================================
// Canonical State Hasher
// ============================================================================

/// State hasher using xxHash64 for speed.
/// Computes a deterministic hash of the world state.
pub struct XxHasher {
    hasher: Xxh64,
}

impl XxHasher {
    pub fn new() -> Self {
        XxHasher {
            hasher: Xxh64::new(0),
        }
    }
}

impl Default for XxHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl IStateHasher for XxHasher {
    fn reset(&mut self) {
        self.hasher.reset(0);
    }

    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(&self) -> StateHash {
        StateHash(self.hasher.digest())
    }
}

// ============================================================================
// Canonical Hash Computation
// ============================================================================

/// Compute a canonical hash of the world state.
///
/// ## Canonical Encoding
/// The hash covers (in order):
/// 1. Tick (u64 LE)
/// 2. SimTime (u64 LE)
/// 3. RNG state (u64 LE)
/// 4. Number of entities (u64 LE)
/// 5. For each entity (sorted by EntityId):
///    - EntityId (u64 LE)
///    - Kind (u8)
///    - State (u8)
///    - Position: zone (u32 LE), x (i32 LE), y (i32 LE), z (i32 LE)
///    - Properties: name length + bytes, amount (u32 LE), health (u32 LE)
/// 6. Number of zones (u64 LE)
/// 7. For each zone (sorted by ZoneId):
///    - ZoneId (u32 LE)
///    - loaded (u8)
///    - Number of entities in zone (u64 LE)
///
/// This encoding is stable across runs.
pub fn compute_canonical_hash(world: &World, hasher: &mut dyn IStateHasher) -> StateHash {
    hasher.reset();

    let mut buf = Vec::with_capacity(1024);

    // 1. Tick
    buf.write_u64::<LittleEndian>(world.current_tick.as_u64())
        .unwrap();

    // 2. SimTime
    buf.write_u64::<LittleEndian>(world.sim_time.units).unwrap();

    // 3. RNG state
    buf.write_u64::<LittleEndian>(world.rng_state).unwrap();

    // 4. Next entity ID
    buf.write_u64::<LittleEndian>(world.next_entity_id).unwrap();

    // 5. Number of entities
    buf.write_u64::<LittleEndian>(world.entities.len() as u64)
        .unwrap();

    // 6. Entities (BTreeMap guarantees sorted order by EntityId)
    for (id, entity) in &world.entities {
        buf.write_u64::<LittleEndian>(id.as_u64()).unwrap();

        // Kind as u8 (EntityKind is non_exhaustive, so we need a wildcard)
        let kind_byte = match entity.kind {
            sy_types::EntityKind::Resource => 0u8,
            sy_types::EntityKind::Creature => 1u8,
            sy_types::EntityKind::Item => 2u8,
            sy_types::EntityKind::Structure => 3u8,
            _ => 255u8, // Unknown kind (future-proofing)
        };
        buf.push(kind_byte);

        // State as u8
        let state_byte = match entity.state {
            sy_types::EntityState::Active => 0u8,
            sy_types::EntityState::Dormant => 1u8,
            sy_types::EntityState::Dead => 2u8,
        };
        buf.push(state_byte);

        // Position
        buf.write_u32::<LittleEndian>(entity.position.zone.as_u32())
            .unwrap();
        buf.write_i32::<LittleEndian>(entity.position.pos.x)
            .unwrap();
        buf.write_i32::<LittleEndian>(entity.position.pos.y)
            .unwrap();
        buf.write_i32::<LittleEndian>(entity.position.pos.z)
            .unwrap();

        // Created at
        buf.write_u64::<LittleEndian>(entity.created_at.as_u64())
            .unwrap();

        // Properties
        if let Some(ref name) = entity.properties.name {
            buf.write_u32::<LittleEndian>(name.len() as u32).unwrap();
            buf.extend_from_slice(name.as_bytes());
        } else {
            buf.write_u32::<LittleEndian>(0).unwrap();
        }
        buf.write_u32::<LittleEndian>(entity.properties.amount.unwrap_or(0))
            .unwrap();
        buf.write_u32::<LittleEndian>(entity.properties.health.unwrap_or(0))
            .unwrap();
    }

    // 7. Number of zones
    buf.write_u64::<LittleEndian>(world.zones.len() as u64)
        .unwrap();

    // 8. Zones (BTreeMap guarantees sorted order by ZoneId)
    for (id, zone) in &world.zones {
        buf.write_u32::<LittleEndian>(id.as_u32()).unwrap();
        buf.push(if zone.loaded { 1u8 } else { 0u8 });
        buf.write_u64::<LittleEndian>(zone.entities.len() as u64)
            .unwrap();
    }

    hasher.update(&buf);
    hasher.finalize()
}

// ============================================================================
// Checkpoint
// ============================================================================

/// A checkpoint containing tick and state hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub tick: Tick,
    pub hash: StateHash,
}

// ============================================================================
// Deterministic Runner
// ============================================================================

/// Input for the deterministic runner.
#[derive(Debug, Clone)]
pub struct ScheduledCommand {
    /// Tick at which to execute the command
    pub tick: Tick,
    /// The command to execute
    pub command: Command,
}

/// Configuration for a deterministic run.
pub struct DeterministicRunConfig {
    /// RNG seed for the world
    pub seed: RngSeed,
    /// World name
    pub world_name: String,
    /// Commands to execute (must be sorted by tick)
    pub inputs: Vec<ScheduledCommand>,
    /// Total ticks to simulate
    pub total_ticks: u64,
    /// Checkpoint interval (0 = only final)
    pub checkpoint_every: u64,
}

/// Result of a deterministic run.
pub struct DeterministicRunResult {
    /// Checkpoints collected during the run
    pub checkpoints: Vec<Checkpoint>,
    /// Final tick reached
    pub final_tick: Tick,
}

/// Run a deterministic simulation and collect state hashes at checkpoints.
///
/// ## Parameters
/// - `config`: Run configuration (seed, inputs, steps, checkpoint interval)
/// - `rng`: RNG implementation
/// - `clock`: Clock implementation
/// - `event_log`: Event log (can be a mock for testing)
/// - `store`: World store (can be a mock for testing)
///
/// ## Returns
/// Checkpoints with (tick, hash) pairs.
pub fn run_deterministic<R, C, E, S>(
    config: &DeterministicRunConfig,
    rng: R,
    clock: C,
    event_log: E,
    store: S,
) -> DeterministicRunResult
where
    R: IRng,
    C: ISimClock,
    E: IEventLog,
    S: IWorldStore,
{
    let mut sim = Simulation::new(rng, clock, event_log, store);
    let mut hasher = XxHasher::new();
    let mut checkpoints = Vec::new();

    // Create world
    let create_cmd = Command::CreateWorld(sy_api::commands::CreateWorldCmd {
        name: config.world_name.clone(),
        seed: config.seed,
    });
    sim.process_command(create_cmd)
        .expect("Failed to create world");

    // Sort inputs by tick (defensive)
    let mut inputs = config.inputs.clone();
    inputs.sort_by_key(|s| s.tick.as_u64());

    let mut input_idx = 0;

    // Run simulation
    for tick_num in 0..config.total_ticks {
        let current_tick = Tick(tick_num);

        // Execute all commands scheduled for this tick
        while input_idx < inputs.len() && inputs[input_idx].tick <= current_tick {
            sim.process_command(inputs[input_idx].command.clone())
                .expect("Command execution failed");
            input_idx += 1;
        }

        // Execute tick
        sim.process_command(Command::Tick).expect("Tick failed");

        // Checkpoint?
        let should_checkpoint =
            config.checkpoint_every > 0 && (tick_num + 1) % config.checkpoint_every == 0;

        if should_checkpoint || tick_num + 1 == config.total_ticks {
            if let Some(world) = sim.world() {
                let hash = compute_canonical_hash(world, &mut hasher);
                checkpoints.push(Checkpoint {
                    tick: world.current_tick,
                    hash,
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
/// Returns Ok(()) if identical, Err with first divergence tick otherwise.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{IEventLog, IRng, ISimClock, IWorldStore};
    use std::collections::HashMap;
    use sy_api::commands::{EntityProperties, SpawnEntityCmd};
    use sy_api::events::SimEvent;
    use sy_types::{
        EntityKind, EventId, Position, SimError, SimResult, SimTime, WorldMeta, WorldPos, ZoneId,
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

        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn next_u64(&mut self) -> u64 {
            // Simple deterministic LCG for tests
            self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.state
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

    struct TestEventLog {
        events: Vec<SimEvent>,
        next_event_id: u64,
        last_tick: Option<Tick>,
    }

    impl TestEventLog {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                next_event_id: 1,
                last_tick: None,
            }
        }
    }

    impl IEventLog for TestEventLog {
        fn append(&mut self, mut event: SimEvent) -> SimResult<SimEvent> {
            event.event_id = EventId::new(self.next_event_id);
            self.next_event_id += 1;
            self.last_tick = Some(event.tick);
            self.events.push(event.clone());
            Ok(event)
        }

        fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>> {
            let mut out = Vec::with_capacity(events.len());
            for event in events {
                out.push(self.append(event)?);
            }
            Ok(out)
        }

        fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>> {
            Ok(self
                .events
                .iter()
                .filter(|e| e.event_id > from_id)
                .cloned()
                .collect())
        }

        fn read_all_valid(&self) -> SimResult<Vec<SimEvent>> {
            Ok(self.events.clone())
        }

        fn last_event_id(&self) -> EventId {
            if self.next_event_id > 1 {
                EventId::new(self.next_event_id - 1)
            } else {
                EventId::ZERO
            }
        }

        fn last_tick(&self) -> Option<Tick> {
            self.last_tick
        }

        fn truncate_after(&mut self, event_id: EventId) -> SimResult<()> {
            self.events.retain(|e| e.event_id <= event_id);
            self.next_event_id = self
                .events
                .last()
                .map(|e| e.event_id.as_u64() + 1)
                .unwrap_or(1);
            self.last_tick = self.events.last().map(|e| e.tick);
            Ok(())
        }

        fn sync(&mut self) -> SimResult<()> {
            Ok(())
        }

        fn len(&self) -> usize {
            self.events.len()
        }
    }

    struct TestWorldStore {
        meta: Option<WorldMeta>,
        snapshots: HashMap<String, Vec<u8>>,
    }

    impl TestWorldStore {
        fn new() -> Self {
            Self {
                meta: None,
                snapshots: HashMap::new(),
            }
        }
    }

    impl IWorldStore for TestWorldStore {
        fn exists(&self, world_id: &str) -> bool {
            self.snapshots.contains_key(world_id)
        }

        fn list_worlds(&self) -> SimResult<Vec<String>> {
            Ok(self.snapshots.keys().cloned().collect())
        }

        fn load_meta(&self, _world_id: &str) -> SimResult<WorldMeta> {
            self.meta
                .clone()
                .ok_or_else(|| SimError::PersistenceError("Meta not found".to_string()))
        }

        fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()> {
            self.meta = Some(meta.clone());
            Ok(())
        }

        fn load_snapshot(&self, world_id: &str) -> SimResult<Vec<u8>> {
            self.snapshots
                .get(world_id)
                .cloned()
                .ok_or_else(|| SimError::PersistenceError("Snapshot not found".to_string()))
        }

        fn save_snapshot(&mut self, world_id: &str, snapshot: &Vec<u8>) -> SimResult<()> {
            self.snapshots
                .insert(world_id.to_string(), snapshot.clone());
            Ok(())
        }

        fn delete_world(&mut self, world_id: &str) -> SimResult<()> {
            self.snapshots.remove(world_id);
            Ok(())
        }

        fn world_path(&self, world_id: &str) -> String {
            format!("mem://{}", world_id)
        }
    }

    /// Create a fixed input stream for testing.
    fn fixed_input_stream() -> Vec<ScheduledCommand> {
        let mut inputs = Vec::new();

        // Spawn some entities at tick 5
        for i in 0..5 {
            inputs.push(ScheduledCommand {
                tick: Tick(5),
                command: Command::SpawnEntity(SpawnEntityCmd {
                    position: WorldPos::new(ZoneId::ORIGIN, Position::new(i * 10, 0, 0)),
                    kind: EntityKind::Resource,
                    properties: EntityProperties {
                        name: Some(format!("Resource_{}", i)),
                        amount: Some(100),
                        health: None,
                    },
                }),
            });
        }

        // Spawn creatures at tick 10
        for i in 0..3 {
            inputs.push(ScheduledCommand {
                tick: Tick(10),
                command: Command::SpawnEntity(SpawnEntityCmd {
                    position: WorldPos::new(ZoneId::ORIGIN, Position::new(i * 10, 10, 0)),
                    kind: EntityKind::Creature,
                    properties: EntityProperties {
                        name: Some(format!("Creature_{}", i)),
                        amount: None,
                        health: Some(100),
                    },
                }),
            });
        }

        inputs
    }

    #[test]
    fn deterministic_replay_same_inputs_same_hashes() {
        let seed = RngSeed::new(42);
        let inputs = fixed_input_stream();
        let steps = 100;
        let checkpoint_every = 10;

        let config = DeterministicRunConfig {
            seed,
            world_name: "Test World".to_string(),
            inputs,
            total_ticks: steps,
            checkpoint_every,
        };

        // Run A
        let result_a = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        // Run B (fresh instances, same config)
        let result_b = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        // Verify identical
        assert_eq!(result_a.checkpoints.len(), result_b.checkpoints.len());

        for (a, b) in result_a.checkpoints.iter().zip(result_b.checkpoints.iter()) {
            assert_eq!(a.tick, b.tick, "Tick mismatch");
            assert_eq!(
                a.hash, b.hash,
                "Hash mismatch at tick {}: {} vs {}",
                a.tick, a.hash, b.hash
            );
        }

        // Use the helper function too
        verify_determinism(&result_a, &result_b).expect("Determinism verification failed");
    }

    #[test]
    fn deterministic_with_many_ticks() {
        let seed = RngSeed::new(12345);
        let inputs = fixed_input_stream();
        let steps = 1000;
        let checkpoint_every = 100;

        let config = DeterministicRunConfig {
            seed,
            world_name: "Long Test".to_string(),
            inputs,
            total_ticks: steps,
            checkpoint_every,
        };

        let result_a = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        let result_b = run_deterministic(
            &config,
            TestRng::new(seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        verify_determinism(&result_a, &result_b)
            .expect("Determinism verification failed for long run");

        // Check we got the expected number of checkpoints
        // 1000 ticks / 100 = 10 checkpoints
        assert_eq!(result_a.checkpoints.len(), 10);
    }

    #[test]
    fn different_seeds_produce_different_hashes() {
        let inputs = fixed_input_stream();
        let steps = 50;
        let checkpoint_every = 50;

        let config_a = DeterministicRunConfig {
            seed: RngSeed::new(1),
            world_name: "World A".to_string(),
            inputs: inputs.clone(),
            total_ticks: steps,
            checkpoint_every,
        };

        let config_b = DeterministicRunConfig {
            seed: RngSeed::new(2),
            world_name: "World B".to_string(),
            inputs,
            total_ticks: steps,
            checkpoint_every,
        };

        let result_a = run_deterministic(
            &config_a,
            TestRng::new(config_a.seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        let result_b = run_deterministic(
            &config_b,
            TestRng::new(config_b.seed),
            TestClock::new(),
            TestEventLog::new(),
            TestWorldStore::new(),
        );

        // Different seeds should produce different final hashes
        assert_ne!(
            result_a.checkpoints.last().unwrap().hash,
            result_b.checkpoints.last().unwrap().hash,
            "Different seeds should produce different hashes"
        );
    }

    #[test]
    fn canonical_hash_is_stable() {
        let world = World::new("Hash Test".to_string(), RngSeed::new(42));

        let mut hasher1 = XxHasher::new();
        let mut hasher2 = XxHasher::new();

        let hash1 = compute_canonical_hash(&world, &mut hasher1);
        let hash2 = compute_canonical_hash(&world, &mut hasher2);

        assert_eq!(hash1, hash2, "Same world should produce same hash");
    }
}
