use std::path::Path;

use proptest::prelude::*;
use sy_api::commands::{Command, CreateWorldCmd, EntityProperties, LoadWorldCmd, SpawnEntityCmd};
use sy_api::events::EventData;
use sy_core::ports::{IEventLog, IWorldStore};
use sy_core::{apply_event, compute_canonical_hash, Simulation, World, XxHasher};
use sy_infra::{FileEventLog, FilesystemStore, Pcg32Rng, UnlimitedClock};
use sy_types::{EntityKind, EventId, Position, RngSeed, Tick, WorldPos, ZoneId};
use tempfile::TempDir;

type InfraSim = Simulation<Pcg32Rng, UnlimitedClock, FileEventLog, FilesystemStore>;

fn temp_data_dir(label: &str) -> TempDir {
    tempfile::Builder::new()
        .prefix(&format!("seej_phase1_recovery_{}_", label))
        .tempdir()
        .unwrap()
}

fn world_id(seed: RngSeed) -> String {
    format!("world_{}", seed.as_u64())
}

fn make_sim(data_dir: &Path, world_id: &str) -> InfraSim {
    let store = FilesystemStore::new(data_dir).expect("store creation failed");
    let wal_path = store.events_dir(world_id);
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::new(RngSeed::new(0));
    let clock = UnlimitedClock::new();
    Simulation::new(rng, clock, event_log, store)
}

fn create_world_with_entities(sim: &mut InfraSim, seed: RngSeed) {
    sim.process_command(Command::CreateWorld(CreateWorldCmd {
        name: "Recovery Determinism".to_string(),
        seed,
    }))
    .expect("create world failed");

    for i in 0..5 {
        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(i * 10, 0, 0)),
            kind: EntityKind::Resource,
            properties: EntityProperties {
                name: Some(format!("Resource_{}", i)),
                amount: Some(100),
                health: None,
            },
        }))
        .expect("spawn resource failed");
    }

    for i in 0..3 {
        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new(i * 10, 10, 0)),
            kind: EntityKind::Creature,
            properties: EntityProperties {
                name: Some(format!("Creature_{}", i)),
                amount: None,
                health: Some(100),
            },
        }))
        .expect("spawn creature failed");
    }
}

fn run_ticks(sim: &mut InfraSim, n: u64) {
    for _ in 0..n {
        sim.process_command(Command::Tick).expect("tick failed");
    }
}

fn load_world(sim: &mut InfraSim, world_id: &str) -> Vec<sy_api::events::SimEvent> {
    sim.process_command(Command::LoadWorld(LoadWorldCmd {
        world_id: world_id.to_string(),
    }))
    .expect("load world failed")
}

fn hash_and_tick(sim: &InfraSim) -> (u64, Tick) {
    let world = sim.world().expect("world must be loaded");
    (hash_world(world), world.current_tick)
}

fn hash_world(world: &World) -> u64 {
    let mut hasher = XxHasher::new();
    compute_canonical_hash(world, &mut hasher).as_u64()
}

#[cfg(windows)]
fn current_process_rss_bytes() -> Option<u64> {
    use std::ffi::c_void;
    use std::mem::size_of;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCounters {
        cb: size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };

    // SAFETY: GetCurrentProcess returns a pseudo-handle valid in this process,
    // and counters points to a properly sized PROCESS_MEMORY_COUNTERS buffer.
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    (ok != 0).then_some(counters.working_set_size as u64)
}

#[cfg(target_os = "linux")]
fn current_process_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb = line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())?;
    Some(kb * 1024)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn current_process_rss_bytes() -> Option<u64> {
    None
}

#[test]
fn clean_restart_matches_continuous_run_hash() {
    let seed = RngSeed::new(424242);
    let wid = world_id(seed);

    let dir_continuous = temp_data_dir("continuous_200");
    let mut continuous = make_sim(dir_continuous.path(), &wid);
    create_world_with_entities(&mut continuous, seed);
    run_ticks(&mut continuous, 200);
    let (continuous_hash_200, continuous_tick_200) = hash_and_tick(&continuous);
    assert_eq!(continuous_tick_200, Tick(200));

    let dir_restart = temp_data_dir("restart_100_100");
    let mut phase_a = make_sim(dir_restart.path(), &wid);
    create_world_with_entities(&mut phase_a, seed);
    run_ticks(&mut phase_a, 100);
    phase_a
        .process_command(Command::SaveWorld)
        .expect("snapshot save at tick 100 failed");
    drop(phase_a);

    let mut phase_b = make_sim(dir_restart.path(), &wid);
    load_world(&mut phase_b, &wid);
    run_ticks(&mut phase_b, 100);
    let (restart_hash_200, restart_tick_200) = hash_and_tick(&phase_b);
    assert_eq!(restart_tick_200, Tick(200));

    assert_eq!(continuous_hash_200, restart_hash_200);
}

#[test]
fn crash_replay_matches_continuous_run_hash() {
    let seed = RngSeed::new(424243);
    let wid = world_id(seed);

    let dir_continuous = temp_data_dir("continuous_140_200");
    let mut continuous = make_sim(dir_continuous.path(), &wid);
    create_world_with_entities(&mut continuous, seed);
    run_ticks(&mut continuous, 140);
    let (continuous_hash_140, continuous_tick_140) = hash_and_tick(&continuous);
    assert_eq!(continuous_tick_140, Tick(140));
    run_ticks(&mut continuous, 60);
    let (continuous_hash_200, continuous_tick_200) = hash_and_tick(&continuous);
    assert_eq!(continuous_tick_200, Tick(200));

    let dir_recovery = temp_data_dir("crash_replay");
    let mut phase_a = make_sim(dir_recovery.path(), &wid);
    create_world_with_entities(&mut phase_a, seed);
    run_ticks(&mut phase_a, 100);
    phase_a
        .process_command(Command::SaveWorld)
        .expect("snapshot save at tick 100 failed");
    run_ticks(&mut phase_a, 40);
    // Simulate a crash by dropping without a final save.
    drop(phase_a);

    let mut recovered = make_sim(dir_recovery.path(), &wid);
    load_world(&mut recovered, &wid);
    let (recovered_hash_140, recovered_tick_140) = hash_and_tick(&recovered);
    assert_eq!(recovered_tick_140, Tick(140));
    assert_eq!(recovered_hash_140, continuous_hash_140);

    run_ticks(&mut recovered, 60);
    let (recovered_hash_200, recovered_tick_200) = hash_and_tick(&recovered);
    assert_eq!(recovered_tick_200, Tick(200));
    assert_eq!(recovered_hash_200, continuous_hash_200);
}

#[test]
fn load_world_event_uses_recovered_tick() {
    let seed = RngSeed::new(424244);
    let wid = world_id(seed);
    let dir = temp_data_dir("load_event_tick");

    let mut phase_a = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut phase_a, seed);
    run_ticks(&mut phase_a, 10);
    phase_a
        .process_command(Command::SaveWorld)
        .expect("snapshot save at tick 10 failed");
    run_ticks(&mut phase_a, 5);
    drop(phase_a);

    let mut recovered = make_sim(dir.path(), &wid);
    let events = load_world(&mut recovered, &wid);
    assert_eq!(recovered.current_tick(), Tick(15));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tick, Tick(15));
}

#[test]
fn snapshot_meta_cursor_points_to_last_event_included_in_snapshot() {
    let seed = RngSeed::new(424245);
    let wid = world_id(seed);
    let dir = temp_data_dir("snapshot_cursor");

    let mut sim = make_sim(dir.path(), &wid);
    sim.process_command(Command::CreateWorld(CreateWorldCmd {
        name: "Cursor".to_string(),
        seed,
    }))
    .expect("create world failed");
    sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
        position: WorldPos::new(ZoneId::ORIGIN, Position::new(0, 0, 0)),
        kind: EntityKind::Resource,
        properties: EntityProperties {
            name: Some("Resource".to_string()),
            amount: Some(10),
            health: None,
        },
    }))
    .expect("spawn failed");
    sim.process_command(Command::SaveWorld)
        .expect("save failed");
    drop(sim);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let meta = store.load_meta(&wid).expect("meta load failed");
    assert_ne!(meta.last_event_id, EventId::ZERO);

    let mut snapshot_world = World::from_bytes(&store.load_snapshot(&wid).unwrap())
        .expect("snapshot deserialize failed");
    let snapshot_hash = hash_world(&snapshot_world);

    let event_log = FileEventLog::new(store.events_dir(&wid)).expect("event log open failed");
    let events_after_cursor = event_log
        .read_from_event_id(meta.last_event_id)
        .expect("wal read failed");

    assert_eq!(events_after_cursor.len(), 1);
    assert!(matches!(
        events_after_cursor[0].data,
        EventData::WorldSaved { .. }
    ));

    apply_event(&mut snapshot_world, &events_after_cursor[0]).expect("WorldSaved must be no-op");
    assert_eq!(hash_world(&snapshot_world), snapshot_hash);
    assert_eq!(snapshot_world.meta.last_event_id, meta.last_event_id);
}

fn run_burn_in(
    data_dir: &Path,
    seed: RngSeed,
    world_id: &str,
) -> Vec<(u64, u64, u64, Option<u64>)> {
    let mut sim = make_sim(data_dir, world_id);
    create_world_with_entities(&mut sim, seed);

    let wal_path = FilesystemStore::new(data_dir)
        .expect("store creation failed")
        .events_dir(world_id);
    let mut checkpoints = Vec::new();
    let mut last_wal_len = 0;

    for tick in 1..=100_000u64 {
        if tick % 1_000 == 0 {
            sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
                position: WorldPos::new(
                    ZoneId::ORIGIN,
                    Position::new((tick % 97) as i32, (tick % 53) as i32, 0),
                ),
                kind: EntityKind::Resource,
                properties: EntityProperties {
                    name: Some(format!("BurnInResource_{tick}")),
                    amount: Some(100),
                    health: None,
                },
            }))
            .expect("burn-in spawn failed");
        }

        if tick % 2_500 == 0 {
            if let Some(id) = sim
                .world()
                .and_then(|world| world.entities.keys().copied().next())
            {
                let _ = sim.process_command(Command::DespawnEntity(id));
            }
        }

        sim.process_command(Command::Tick)
            .expect("burn-in tick failed");

        if tick % 10_000 == 0 {
            let (hash, current_tick) = hash_and_tick(&sim);
            let wal_len = std::fs::metadata(&wal_path).unwrap().len();
            assert!(
                wal_len > last_wal_len,
                "WAL size must grow during Phase 1 burn-in"
            );
            last_wal_len = wal_len;
            checkpoints.push((
                current_tick.as_u64(),
                hash,
                wal_len,
                current_process_rss_bytes(),
            ));
        }
    }

    checkpoints
}

#[test]
#[ignore = "long burn-in; run in Phase 1 closure/nightly job"]
fn burn_in_100k_ticks_keeps_deterministic_hashes_and_bounded_growth() {
    let seed = RngSeed::new(424246);
    let wid = world_id(seed);
    let first_dir = temp_data_dir("burn_in_a");
    let second_dir = temp_data_dir("burn_in_b");

    let first = run_burn_in(first_dir.path(), seed, &wid);
    let second = run_burn_in(second_dir.path(), seed, &wid);

    assert_eq!(first.len(), 10);
    assert_eq!(second.len(), 10);
    for ((tick_a, hash_a, _, _), (tick_b, hash_b, _, _)) in first.iter().zip(second.iter()) {
        assert_eq!(tick_a, tick_b);
        assert_eq!(hash_a, hash_b, "hash diverged at tick {tick_a}");
    }

    let memory_samples: Vec<u64> = first.iter().filter_map(|(_, _, _, rss)| *rss).collect();
    if let (Some(first_sample), Some(last_sample)) = (memory_samples.first(), memory_samples.last())
    {
        assert!(
            last_sample.saturating_sub(*first_sample) < 512 * 1024 * 1024,
            "burn-in RSS grew by more than 512 MiB"
        );
    }
}

#[derive(Debug, Clone)]
struct ParityCase {
    seed: u64,
    total_ticks: u32,
    snapshot_at: u32,
    initial_resources: u32,
    initial_creatures: u32,
}

fn parity_case_strategy() -> impl Strategy<Value = ParityCase> {
    (any::<u64>(), 2u32..=20u32, 0u32..=4u32, 0u32..=3u32).prop_flat_map(
        |(seed, total_ticks, initial_resources, initial_creatures)| {
            (1u32..total_ticks).prop_map(move |snapshot_at| ParityCase {
                seed,
                total_ticks,
                snapshot_at,
                initial_resources,
                initial_creatures,
            })
        },
    )
}

fn create_world_with_counts(
    sim: &mut InfraSim,
    seed: RngSeed,
    resources: u32,
    creatures: u32,
) -> Result<(), String> {
    sim.process_command(Command::CreateWorld(CreateWorldCmd {
        name: "Parity Property".to_string(),
        seed,
    }))
    .map_err(|err| format!("create world failed: {err:?}"))?;

    for i in 0..resources {
        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new((i as i32) * 10, 0, 0)),
            kind: EntityKind::Resource,
            properties: EntityProperties {
                name: Some(format!("Resource_{i}")),
                amount: Some(100),
                health: None,
            },
        }))
        .map_err(|err| format!("spawn resource failed: {err:?}"))?;
    }

    for i in 0..creatures {
        sim.process_command(Command::SpawnEntity(SpawnEntityCmd {
            position: WorldPos::new(ZoneId::ORIGIN, Position::new((i as i32) * 10, 10, 0)),
            kind: EntityKind::Creature,
            properties: EntityProperties {
                name: Some(format!("Creature_{i}")),
                amount: None,
                health: Some(100),
            },
        }))
        .map_err(|err| format!("spawn creature failed: {err:?}"))?;
    }

    Ok(())
}

fn snapshot_replay_parity_for(case: &ParityCase) -> Result<(), TestCaseError> {
    prop_assert!(case.snapshot_at >= 1);
    prop_assert!(case.snapshot_at < case.total_ticks);

    let seed = RngSeed::new(case.seed);
    let wid = world_id(seed);

    let dir_continuous = temp_data_dir(&format!("prop_continuous_{}", case.seed));
    let mut continuous = make_sim(dir_continuous.path(), &wid);
    create_world_with_counts(
        &mut continuous,
        seed,
        case.initial_resources,
        case.initial_creatures,
    )
    .map_err(TestCaseError::fail)?;
    run_ticks(&mut continuous, u64::from(case.total_ticks));
    let (continuous_hash, continuous_tick) = hash_and_tick(&continuous);
    prop_assert_eq!(continuous_tick, Tick(u64::from(case.total_ticks)));
    drop(continuous);

    let dir_split = temp_data_dir(&format!("prop_split_{}", case.seed));
    let mut phase_a = make_sim(dir_split.path(), &wid);
    create_world_with_counts(
        &mut phase_a,
        seed,
        case.initial_resources,
        case.initial_creatures,
    )
    .map_err(TestCaseError::fail)?;
    run_ticks(&mut phase_a, u64::from(case.snapshot_at));
    phase_a
        .process_command(Command::SaveWorld)
        .map_err(|err| TestCaseError::fail(format!("snapshot save failed: {err:?}")))?;
    drop(phase_a);

    let mut phase_b = make_sim(dir_split.path(), &wid);
    load_world(&mut phase_b, &wid);
    run_ticks(&mut phase_b, u64::from(case.total_ticks - case.snapshot_at));
    let (replay_hash, replay_tick) = hash_and_tick(&phase_b);
    prop_assert_eq!(replay_tick, Tick(u64::from(case.total_ticks)));
    prop_assert_eq!(replay_hash, continuous_hash);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_shrink_iters: 32,
        ..ProptestConfig::default()
    })]

    #[test]
    fn snapshot_replay_parity_holds_for_random_seeds_and_split_points(
        case in parity_case_strategy(),
    ) {
        snapshot_replay_parity_for(&case)?;
    }
}
