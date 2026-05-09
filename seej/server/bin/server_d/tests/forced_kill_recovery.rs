use std::path::Path;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sy_api::commands::{Command as SimCommand, LoadWorldCmd};
use sy_core::{compute_canonical_hash, Simulation, XxHasher};
use sy_infra::{FileEventLog, FilesystemStore, Pcg32Rng, UnlimitedClock};
use sy_types::{RngSeed, Tick};

type InfraSim = Simulation<Pcg32Rng, UnlimitedClock, FileEventLog, FilesystemStore>;

fn server_bin() -> &'static str {
    env!("CARGO_BIN_EXE_server_d")
}

fn world_id(seed: RngSeed) -> String {
    format!("world_{}", seed.as_u64())
}

fn run_server(args: &[&str]) {
    let output = StdCommand::new(server_bin())
        .args(args)
        .output()
        .expect("server_d command failed to spawn");
    assert!(
        output.status.success(),
        "server_d failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_world(data_dir: &Path, seed: RngSeed) {
    run_server(&[
        "--data-dir",
        data_dir.to_str().unwrap(),
        "create",
        "--name",
        "ForcedKill",
        "--seed",
        &seed.as_u64().to_string(),
    ]);
}

fn make_sim(data_dir: &Path, world_id: &str) -> InfraSim {
    let store = FilesystemStore::new(data_dir).expect("store creation failed");
    let wal_path = store.events_dir(world_id);
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::new(RngSeed::new(0));
    let clock = UnlimitedClock::new();
    Simulation::new(rng, clock, event_log, store)
}

fn load_hash_and_tick(data_dir: &Path, world_id: &str) -> (u64, Tick) {
    let mut sim = make_sim(data_dir, world_id);
    sim.process_command(SimCommand::LoadWorld(LoadWorldCmd {
        world_id: world_id.to_string(),
    }))
    .expect("load world failed");
    let world = sim.world().expect("world must be loaded");
    let mut hasher = XxHasher::new();
    (
        compute_canonical_hash(world, &mut hasher).as_u64(),
        world.current_tick,
    )
}

#[cfg(windows)]
fn force_kill(child: &mut Child) {
    let status = StdCommand::new("taskkill")
        .args(["/F", "/PID", &child.id().to_string()])
        .status()
        .expect("taskkill failed to spawn");
    assert!(status.success(), "taskkill /F failed with {status}");
}

#[cfg(unix)]
fn force_kill(child: &mut Child) {
    child.kill().expect("SIGKILL failed");
}

fn wait_for_wal_growth(child: &mut Child, wal_path: &Path, initial_len: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            panic!("server_d exited before forced kill: {status}");
        }

        let current_len = std::fs::metadata(wal_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        if current_len > initial_len {
            return;
        }

        thread::sleep(Duration::from_millis(25));
    }

    panic!("server_d did not durably append to the WAL before forced kill deadline");
}

#[test]
#[ignore = "force-kills server_d; run in Phase 1 closure/nightly job"]
fn forced_process_kill_recovers_to_continuous_run_hash() {
    let seed = RngSeed::new(777_001);
    let wid = world_id(seed);
    let crashed_dir = tempfile::tempdir().unwrap();
    let baseline_dir = tempfile::tempdir().unwrap();

    create_world(crashed_dir.path(), seed);
    let store = FilesystemStore::new(crashed_dir.path()).expect("store creation failed");
    let wal_path = store.events_dir(&wid);
    let initial_wal_len = std::fs::metadata(&wal_path)
        .expect("WAL must exist after world creation")
        .len();

    let mut child = StdCommand::new(server_bin())
        .args([
            "--data-dir",
            crashed_dir.path().to_str().unwrap(),
            "run",
            "--world",
            &wid,
            "--ticks",
            "0",
            "--save-interval",
            "1000000",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("server_d run failed to spawn");

    wait_for_wal_growth(&mut child, &wal_path, initial_wal_len);
    force_kill(&mut child);
    let _ = child.wait();

    let (crashed_hash, recovered_tick) = load_hash_and_tick(crashed_dir.path(), &wid);
    assert!(
        recovered_tick.as_u64() > 0,
        "forced kill happened before any tick was durably recovered"
    );

    create_world(baseline_dir.path(), seed);
    run_server(&[
        "--data-dir",
        baseline_dir.path().to_str().unwrap(),
        "run",
        "--world",
        &wid,
        "--ticks",
        &recovered_tick.as_u64().to_string(),
        "--save-interval",
        "0",
    ]);
    let (baseline_hash, baseline_tick) = load_hash_and_tick(baseline_dir.path(), &wid);

    assert_eq!(baseline_tick, recovered_tick);
    assert_eq!(baseline_hash, crashed_hash);
}
