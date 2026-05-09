use std::path::Path;

use proptest::prelude::*;
use sy_api::commands::{CreateWorldCmd, EntityProperties, SimCommand, SpawnEntityCmd};
use sy_api::persistence::{IEventLog, IWorldStore};
use sy_core::{compute_canonical_hash, World};
use sy_infra::{
    load_recovered_world,
    snapshot::{decode_world, encode_world},
    FileEventLog, FilesystemStore, Pcg32Rng, PersistentSimulation, UnlimitedClock, XxHasher,
};
use sy_types::{EntityKind, EventId, Position, RngSeed, SimError, Tick, WorldPos, ZoneId};
use tempfile::TempDir;

type InfraSim = PersistentSimulation<Pcg32Rng, UnlimitedClock, FileEventLog, FilesystemStore>;

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
    let wal_path = store.events_dir(world_id).expect("valid world id");
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::uninitialized();
    let clock = UnlimitedClock::new();
    PersistentSimulation::new(rng, clock, event_log, store)
}

fn create_world_with_entities(sim: &mut InfraSim, seed: RngSeed) {
    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: "Recovery Determinism".to_string(),
        seed,
    }))
    .expect("create world failed");

    for i in 0..5 {
        sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
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
        sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
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
        sim.process_command(SimCommand::Tick).expect("tick failed");
    }
}

fn load_world(sim: &mut InfraSim, world_id: &str) -> Vec<sy_api::events::SimEvent> {
    sim.load_world(world_id).expect("load world failed")
}

fn hash_and_tick(sim: &mut InfraSim) -> (u64, Tick) {
    sim.save_world().expect("save before hash failed");
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
    let (continuous_hash_200, continuous_tick_200) = hash_and_tick(&mut continuous);
    assert_eq!(continuous_tick_200, Tick(200));

    let dir_restart = temp_data_dir("restart_100_100");
    let mut phase_a = make_sim(dir_restart.path(), &wid);
    create_world_with_entities(&mut phase_a, seed);
    run_ticks(&mut phase_a, 100);
    phase_a
        .save_world()
        .expect("snapshot save at tick 100 failed");
    drop(phase_a);

    let mut phase_b = make_sim(dir_restart.path(), &wid);
    load_world(&mut phase_b, &wid);
    run_ticks(&mut phase_b, 100);
    let (restart_hash_200, restart_tick_200) = hash_and_tick(&mut phase_b);
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
    let (continuous_hash_140, continuous_tick_140) = hash_and_tick(&mut continuous);
    assert_eq!(continuous_tick_140, Tick(140));
    run_ticks(&mut continuous, 60);
    let (continuous_hash_200, continuous_tick_200) = hash_and_tick(&mut continuous);
    assert_eq!(continuous_tick_200, Tick(200));

    let dir_recovery = temp_data_dir("crash_replay");
    let mut phase_a = make_sim(dir_recovery.path(), &wid);
    create_world_with_entities(&mut phase_a, seed);
    run_ticks(&mut phase_a, 100);
    phase_a
        .save_world()
        .expect("snapshot save at tick 100 failed");
    run_ticks(&mut phase_a, 40);
    // Simulate a crash by dropping without a final save.
    drop(phase_a);

    let mut recovered = make_sim(dir_recovery.path(), &wid);
    load_world(&mut recovered, &wid);
    let (recovered_hash_140, recovered_tick_140) = hash_and_tick(&mut recovered);
    assert_eq!(recovered_tick_140, Tick(140));
    assert_eq!(recovered_hash_140, continuous_hash_140);

    run_ticks(&mut recovered, 60);
    let (recovered_hash_200, recovered_tick_200) = hash_and_tick(&mut recovered);
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
        .save_world()
        .expect("snapshot save at tick 10 failed");
    run_ticks(&mut phase_a, 5);
    drop(phase_a);

    let mut recovered = make_sim(dir.path(), &wid);
    let events = load_world(&mut recovered, &wid);
    assert_eq!(recovered.current_tick(), Tick(15));
    assert_eq!(events.last().map(|event| event.tick), Some(Tick(15)));
}

#[test]
fn snapshot_meta_cursor_points_to_last_event_included_in_snapshot() {
    let seed = RngSeed::new(424245);
    let wid = world_id(seed);
    let dir = temp_data_dir("snapshot_cursor");

    let mut sim = make_sim(dir.path(), &wid);
    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: "Cursor".to_string(),
        seed,
    }))
    .expect("create world failed");
    sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
        position: WorldPos::new(ZoneId::ORIGIN, Position::new(0, 0, 0)),
        kind: EntityKind::Resource,
        properties: EntityProperties {
            name: Some("Resource".to_string()),
            amount: Some(10),
            health: None,
        },
    }))
    .expect("spawn failed");
    sim.save_world().expect("save failed");
    drop(sim);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let meta = store.load_meta(&wid).expect("meta load failed");
    assert_ne!(meta.last_event_id, EventId::ZERO);

    let snapshot_world =
        decode_world(&store.load_snapshot(&wid).unwrap()).expect("snapshot deserialize failed");
    let snapshot_hash = hash_world(&snapshot_world);

    let event_log = FileEventLog::new(store.events_dir(&wid).expect("valid world id"))
        .expect("event log open failed");
    let events_after_cursor = event_log
        .read_from_event_id(meta.last_event_id)
        .expect("wal read failed");

    assert!(events_after_cursor.is_empty());
    assert_eq!(hash_world(&snapshot_world), snapshot_hash);
    assert_eq!(snapshot_world.meta.last_event_id, meta.last_event_id);
}

#[test]
fn create_rejects_orphan_wal_without_snapshot_or_meta() {
    let seed = RngSeed::new(424246);
    let wid = world_id(seed);
    let dir = temp_data_dir("orphan_wal_create");

    let mut phase_a = make_sim(dir.path(), &wid);
    phase_a
        .process_command(SimCommand::CreateWorld(CreateWorldCmd {
            name: "Interrupted Create".to_string(),
            seed,
        }))
        .expect("initial create should persist WAL event");
    drop(phase_a);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let wal_path = store.events_dir(&wid).expect("valid world id");
    assert!(wal_path.exists(), "WAL must exist after interrupted create");
    assert!(
        std::fs::metadata(&wal_path)
            .expect("WAL metadata failed")
            .len()
            > 0,
        "WAL must be non-empty after interrupted create"
    );
    assert!(
        !store.exists(&wid),
        "meta.json is intentionally absent in interrupted create"
    );
    drop(store);

    let mut phase_b = make_sim(dir.path(), &wid);
    let err = phase_b
        .process_command(SimCommand::CreateWorld(CreateWorldCmd {
            name: "Interrupted Create".to_string(),
            seed,
        }))
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("incomplete persistent storage exists"),
        "unexpected error: {err}"
    );
}

#[test]
fn recovery_rejects_meta_json_that_disagrees_with_snapshot() {
    let seed = RngSeed::new(424247);
    let wid = world_id(seed);
    let dir = temp_data_dir("meta_mismatch");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    run_ticks(&mut sim, 3);
    sim.save_world().expect("save failed");
    drop(sim);

    let mut store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let mut meta = store.load_meta(&wid).expect("meta load failed");
    meta.snapshot_tick = Tick(meta.snapshot_tick.as_u64() + 1);
    store.save_meta(&meta).expect("meta rewrite failed");

    let event_log = FileEventLog::new(store.events_dir(&wid).expect("valid world id"))
        .expect("event log open failed");
    let err = load_recovered_world(&store, &event_log, &wid).unwrap_err();

    assert!(
        err.to_string()
            .contains("meta.json does not match snapshot"),
        "unexpected error: {err}"
    );
}

#[test]
fn recovery_accepts_snapshot_written_before_first_meta_after_interrupted_save() {
    let seed = RngSeed::new(4242471);
    let wid = world_id(seed);
    let dir = temp_data_dir("snapshot_without_meta");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    run_ticks(&mut sim, 3);

    let mut snapshot_world = sim.world().expect("world must be loaded").clone();
    let (_, event_log, mut store) = sim.into_parts();
    snapshot_world.meta.current_tick = snapshot_world.current_tick;
    snapshot_world.meta.sim_time = snapshot_world.sim_time;
    snapshot_world.meta.snapshot_tick = snapshot_world.current_tick;
    snapshot_world.meta.last_event_id = event_log.last_event_id();
    let snapshot = encode_world(&snapshot_world).expect("snapshot encode failed");
    store
        .save_snapshot(&wid, &snapshot)
        .expect("snapshot-only save failed");
    drop(store);
    drop(event_log);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    assert!(store.load_meta(&wid).is_err(), "meta must be absent");
    let event_log = FileEventLog::new(store.events_dir(&wid).expect("valid world id"))
        .expect("event log open failed");

    let (recovered, replayed) =
        load_recovered_world(&store, &event_log, &wid).expect("recovery should use snapshot meta");

    assert!(replayed.is_empty());
    assert_eq!(recovered.current_tick, Tick(3));
    assert_eq!(hash_world(&recovered), hash_world(&snapshot_world));
}

#[test]
fn recovery_rejects_snapshot_world_id_that_disagrees_with_requested_world() {
    let seed = RngSeed::new(4242473);
    let wid = world_id(seed);
    let dir = temp_data_dir("snapshot_world_id_mismatch");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    sim.save_world().expect("save failed");
    let snapshot = {
        let world = sim.world().expect("world must be loaded");
        encode_world(world).expect("snapshot encode failed")
    };
    drop(sim);

    let mut store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let wrong_world_id = "world_4242474";
    store
        .save_snapshot(wrong_world_id, &snapshot)
        .expect("wrong snapshot save failed");
    let event_log = FileEventLog::new(store.events_dir(wrong_world_id).expect("valid world id"))
        .expect("event log open failed");

    let err = load_recovered_world(&store, &event_log, wrong_world_id).unwrap_err();

    assert!(
        err.to_string().contains("Snapshot world_id mismatch"),
        "unexpected error: {err}"
    );
}

#[test]
fn recovery_accepts_stale_meta_after_snapshot_then_crash_before_meta() {
    let seed = RngSeed::new(4242472);
    let wid = world_id(seed);
    let dir = temp_data_dir("stale_meta_after_snapshot");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    run_ticks(&mut sim, 3);
    sim.save_world().expect("initial save failed");
    run_ticks(&mut sim, 2);

    let mut snapshot_world = sim.world().expect("world must be loaded").clone();
    let (_, event_log, mut store) = sim.into_parts();
    let stale_meta = store.load_meta(&wid).expect("stale meta load failed");
    snapshot_world.meta.current_tick = snapshot_world.current_tick;
    snapshot_world.meta.sim_time = snapshot_world.sim_time;
    snapshot_world.meta.snapshot_tick = snapshot_world.current_tick;
    snapshot_world.meta.last_event_id = event_log.last_event_id();
    assert!(stale_meta.last_event_id < snapshot_world.meta.last_event_id);

    let snapshot = encode_world(&snapshot_world).expect("snapshot encode failed");
    store
        .save_snapshot(&wid, &snapshot)
        .expect("snapshot overwrite failed");
    drop(store);
    drop(event_log);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let event_log = FileEventLog::new(store.events_dir(&wid).expect("valid world id"))
        .expect("event log open failed");

    let (recovered, replayed) =
        load_recovered_world(&store, &event_log, &wid).expect("stale meta should be recoverable");

    assert!(replayed.is_empty());
    assert_eq!(recovered.current_tick, Tick(5));
    assert_eq!(hash_world(&recovered), hash_world(&snapshot_world));
}

#[test]
fn recovery_rejects_wal_cursor_behind_snapshot_cursor() {
    let seed = RngSeed::new(424248);
    let wid = world_id(seed);
    let dir = temp_data_dir("wal_cursor_behind_snapshot");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    run_ticks(&mut sim, 3);
    sim.save_world().expect("save failed");
    drop(sim);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let meta = store.load_meta(&wid).expect("meta load failed");
    assert_ne!(meta.last_event_id, EventId::ZERO);

    let wal_path = store.events_dir(&wid).expect("valid world id");
    std::fs::remove_file(&wal_path).expect("WAL delete failed");
    let event_log = FileEventLog::new(&wal_path).expect("event log reopen failed");

    let err = load_recovered_world(&store, &event_log, &wid).unwrap_err();
    assert!(
        matches!(err, SimError::CorruptedState(_)),
        "unexpected error: {err:?}"
    );
    assert!(
        err.to_string().contains("WAL cursor is behind snapshot"),
        "unexpected error: {err}"
    );
}

#[test]
fn recovery_rejects_missing_wal_after_log_opened() {
    let seed = RngSeed::new(424249);
    let wid = world_id(seed);
    let dir = temp_data_dir("wal_deleted_after_open");

    let mut sim = make_sim(dir.path(), &wid);
    create_world_with_entities(&mut sim, seed);
    run_ticks(&mut sim, 3);
    sim.save_world().expect("save failed");
    drop(sim);

    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let meta = store.load_meta(&wid).expect("meta load failed");
    let wal_path = store.events_dir(&wid).expect("valid world id");
    let event_log = FileEventLog::new(&wal_path).expect("event log open failed");
    assert!(event_log.last_event_id() >= meta.last_event_id);

    std::fs::remove_file(&wal_path).expect("WAL delete failed");

    let err = load_recovered_world(&store, &event_log, &wid).unwrap_err();
    assert!(
        matches!(err, SimError::CorruptedState(_)),
        "unexpected error: {err:?}"
    );
    assert!(
        err.to_string()
            .contains("WAL cursor does not match durable tail"),
        "unexpected error: {err}"
    );
}

#[test]
fn filesystem_store_rejects_path_like_world_ids() {
    let dir = temp_data_dir("invalid_world_id");
    let store = FilesystemStore::new(dir.path()).expect("store creation failed");

    for id in [
        "..",
        "../world_1",
        r"..\world_1",
        "/world_1",
        r"C:\world_1",
        "world/1",
    ] {
        assert!(!store.exists(id));
        assert!(store.events_dir(id).is_err(), "{id} must be rejected");
        assert!(store.load_meta(id).is_err(), "{id} must be rejected");
    }
}

fn run_burn_in(
    data_dir: &Path,
    seed: RngSeed,
    world_id: &str,
) -> Vec<(u64, u64, u64, Option<u64>)> {
    const TOTAL_TICKS: u64 = 100_000;

    let mut sim = make_sim(data_dir, world_id);
    create_world_with_entities(&mut sim, seed);

    let wal_path = FilesystemStore::new(data_dir)
        .expect("store creation failed")
        .events_dir(world_id)
        .expect("valid world id");
    let mut checkpoints = Vec::new();
    let mut last_wal_len = 0;
    let mut tick = 0u64;

    while tick < TOTAL_TICKS {
        let next_special_tick = ((tick + 1)..=TOTAL_TICKS)
            .find(|candidate| {
                candidate.is_multiple_of(1_000)
                    || candidate.is_multiple_of(2_500)
                    || candidate.is_multiple_of(10_000)
            })
            .unwrap_or(TOTAL_TICKS);

        let quiet_ticks = next_special_tick - tick - 1;
        if quiet_ticks > 0 {
            sim.process_command(SimCommand::TickN(quiet_ticks as u32))
                .expect("burn-in tick batch failed");
            tick += quiet_ticks;
        }

        let next_tick = tick + 1;
        if next_tick.is_multiple_of(1_000) {
            sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
                position: WorldPos::new(
                    ZoneId::ORIGIN,
                    Position::new((next_tick % 97) as i32, (next_tick % 53) as i32, 0),
                ),
                kind: EntityKind::Resource,
                properties: EntityProperties {
                    name: Some(format!("BurnInResource_{next_tick}")),
                    amount: Some(100),
                    health: None,
                },
            }))
            .expect("burn-in spawn failed");
        }

        if next_tick.is_multiple_of(2_500) {
            if let Some(id) = sim
                .world()
                .and_then(|world| world.entities.keys().copied().next())
            {
                let _ = sim.process_command(SimCommand::DespawnEntity(id));
            }
        }

        sim.process_command(SimCommand::Tick)
            .expect("burn-in tick failed");
        tick = next_tick;

        if tick.is_multiple_of(10_000) {
            let (hash, current_tick) = hash_and_tick(&mut sim);
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
    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: "Parity Property".to_string(),
        seed,
    }))
    .map_err(|err| format!("create world failed: {err:?}"))?;

    for i in 0..resources {
        sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
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
        sim.process_command(SimCommand::SpawnEntity(SpawnEntityCmd {
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
    let (continuous_hash, continuous_tick) = hash_and_tick(&mut continuous);
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
        .save_world()
        .map_err(|err| TestCaseError::fail(format!("snapshot save failed: {err:?}")))?;
    drop(phase_a);

    let mut phase_b = make_sim(dir_split.path(), &wid);
    load_world(&mut phase_b, &wid);
    run_ticks(&mut phase_b, u64::from(case.total_ticks - case.snapshot_at));
    let (replay_hash, replay_tick) = hash_and_tick(&mut phase_b);
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
