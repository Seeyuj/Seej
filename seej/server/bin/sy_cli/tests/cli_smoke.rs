//! CLI smoke tests for `sy_cli` (read-only inspection tool).

use std::fs::OpenOptions;
use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use sy_api::commands::{CreateWorldCmd, SimCommand};
use sy_api::persistence::{IEventLog, IWorldStore};
use sy_infra::snapshot::{decode_world, encode_world};
use sy_infra::{FileEventLog, FilesystemStore, Pcg32Rng, PersistentSimulation, UnlimitedClock};
use sy_types::RngSeed;
use tempfile::TempDir;

fn sy_cli() -> Command {
    Command::cargo_bin("sy_cli").expect("sy_cli binary must be built")
}

fn server_d() -> Command {
    Command::cargo_bin("server_d").expect("server_d binary must be built")
}

fn temp_data_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("seej_sy_cli_")
        .tempdir()
        .expect("tempdir creation failed")
}

fn create_world(dir: &TempDir, seed: u64) {
    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .args([
            "create",
            "--name",
            "SyCliFixture",
            "--seed",
            &seed.to_string(),
        ])
        .assert()
        .success();
}

fn create_world_with_unsnapshoted_ticks(dir: &TempDir, seed: u64, ticks: u64) {
    let world_id = format!("world_{seed}");
    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let wal_path = store.events_dir(&world_id).expect("valid world id");
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::uninitialized();
    let clock = UnlimitedClock::new();
    let mut sim = PersistentSimulation::new(rng, clock, event_log, store);

    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: "SyCliRecoveredFixture".to_string(),
        seed: RngSeed::new(seed),
    }))
    .expect("create world failed");
    sim.save_world().expect("initial save failed");

    for _ in 0..ticks {
        sim.process_command(SimCommand::Tick).expect("tick failed");
    }
}

fn create_world_with_snapshot_but_no_meta(dir: &TempDir, seed: u64) {
    let world_id = format!("world_{seed}");
    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let wal_path = store.events_dir(&world_id).expect("valid world id");
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::uninitialized();
    let clock = UnlimitedClock::new();
    let mut sim = PersistentSimulation::new(rng, clock, event_log, store);

    sim.process_command(SimCommand::CreateWorld(CreateWorldCmd {
        name: "SyCliSnapshotOnlyFixture".to_string(),
        seed: RngSeed::new(seed),
    }))
    .expect("create world failed");

    let mut snapshot_world = sim.world().expect("world must be loaded").clone();
    let (_, event_log, mut store) = sim.into_parts();
    snapshot_world.meta.current_tick = snapshot_world.current_tick;
    snapshot_world.meta.sim_time = snapshot_world.sim_time;
    snapshot_world.meta.snapshot_tick = snapshot_world.current_tick;
    snapshot_world.meta.last_event_id = event_log.last_event_id();
    let snapshot = encode_world(&snapshot_world).expect("snapshot encode failed");
    store
        .save_snapshot(&world_id, &snapshot)
        .expect("snapshot save failed");
}

#[test]
fn status_on_existing_world_succeeds() {
    let dir = temp_data_dir();
    create_world(&dir, 1);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["status", "world_1"])
        .assert()
        .success();
}

#[test]
fn status_uses_recovered_state_after_unsnapshoted_wal_ticks() {
    let dir = temp_data_dir();
    create_world_with_unsnapshoted_ticks(&dir, 88, 5);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["status", "world_88"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current Tick:    T5"));
}

#[test]
fn status_uses_snapshot_when_meta_json_is_missing_after_interrupted_save() {
    let dir = temp_data_dir();
    create_world_with_snapshot_but_no_meta(&dir, 90);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["status", "world_90"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID:              world_90"));
}

#[test]
fn dump_uses_recovered_state_after_unsnapshoted_wal_ticks() {
    let dir = temp_data_dir();
    create_world_with_unsnapshoted_ticks(&dir, 89, 7);

    let output = sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["dump", "world_89"])
        .output()
        .expect("dump must run");
    assert!(output.status.success(), "dump must succeed");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("dump stdout must be valid JSON");
    assert_eq!(value["current_tick"], serde_json::json!(7));

    let decoded = decode_world(stdout.as_bytes()).expect("dump must be a valid snapshot");
    assert_eq!(decoded.meta.current_tick, decoded.current_tick);
    assert_eq!(decoded.meta.snapshot_tick, decoded.current_tick);
}

#[test]
fn status_on_missing_world_fails_gracefully() {
    let dir = temp_data_dir();

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["status", "world_does_not_exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("World not found"));
}

#[test]
fn dump_to_stdout_emits_valid_json() {
    let dir = temp_data_dir();
    create_world(&dir, 2);

    let output = sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["dump", "world_2"])
        .output()
        .expect("dump must run");
    assert!(output.status.success(), "dump must succeed");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let _: serde_json::Value =
        serde_json::from_str(&stdout).expect("dump stdout must be valid JSON");
}

#[test]
fn dump_to_file_writes_valid_json() {
    let dir = temp_data_dir();
    create_world(&dir, 3);
    let out_path = dir.path().join("dump.json");

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["dump", "world_3", "--output"])
        .arg(&out_path)
        .assert()
        .success();

    let contents = std::fs::read_to_string(&out_path).expect("dump file must exist");
    let _: serde_json::Value =
        serde_json::from_str(&contents).expect("dump file must contain valid JSON");
}

#[test]
fn events_on_existing_world_succeeds() {
    let dir = temp_data_dir();
    create_world(&dir, 4);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["events", "world_4", "--count", "5"])
        .assert()
        .success();
}

#[test]
fn events_on_corrupt_wal_fails_without_truncating() {
    let dir = temp_data_dir();
    create_world(&dir, 44);
    let store = FilesystemStore::new(dir.path()).expect("store creation failed");
    let wal_path = store.events_dir("world_44").expect("valid world id");
    let valid_len = std::fs::metadata(&wal_path)
        .expect("WAL metadata failed")
        .len();

    let mut file = OpenOptions::new()
        .append(true)
        .open(&wal_path)
        .expect("WAL open failed");
    file.write_all(b"corrupt-tail")
        .expect("WAL corrupt tail write failed");
    file.sync_all().expect("WAL sync failed");
    let corrupt_len = std::fs::metadata(&wal_path)
        .expect("WAL metadata failed")
        .len();
    assert!(corrupt_len > valid_len);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["events", "world_44"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read events"));

    assert_eq!(
        std::fs::metadata(&wal_path)
            .expect("WAL metadata failed")
            .len(),
        corrupt_len
    );
}

#[test]
fn entities_on_existing_world_succeeds() {
    let dir = temp_data_dir();
    create_world(&dir, 5);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["entities", "world_5"])
        .assert()
        .success();
}

#[test]
fn zones_on_existing_world_succeeds() {
    let dir = temp_data_dir();
    create_world(&dir, 6);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["zones", "world_6"])
        .assert()
        .success();
}

#[test]
fn entity_lookup_unknown_id_fails_gracefully() {
    let dir = temp_data_dir();
    create_world(&dir, 7);

    sy_cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["entity", "world_7", "999999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Entity not found"));
}
