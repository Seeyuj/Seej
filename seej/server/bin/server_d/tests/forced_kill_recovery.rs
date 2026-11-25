use std::fs;
use std::path::Path;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sy_api::commands::SimCommand;
use sy_core::compute_canonical_hash;
use sy_infra::{
    FileEventLog, FilesystemStore, Pcg32Rng, PersistentSimulation, UnlimitedClock, XxHasher,
};
use sy_types::{RngSeed, SimError, Tick};

type InfraSim = PersistentSimulation<Pcg32Rng, UnlimitedClock, FileEventLog, FilesystemStore>;

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
    let wal_path = store.events_dir(world_id).expect("valid world id");
    let event_log = FileEventLog::new(&wal_path).expect("event log creation failed");
    let rng = Pcg32Rng::uninitialized();
    let clock = UnlimitedClock::new();
    PersistentSimulation::new(rng, clock, event_log, store)
}

fn load_hash_and_tick(data_dir: &Path, world_id: &str) -> (u64, Tick) {
    let mut sim = make_sim(data_dir, world_id);
    sim.load_world(world_id).expect("load world failed");
    sim.save_world()
        .expect("normalize recovered snapshot failed");
    let world = sim.world().expect("world must be loaded");
    let mut hasher = XxHasher::new();
    (
        compute_canonical_hash(world, &mut hasher).as_u64(),
        world.current_tick,
    )
}

fn wal_len(path: &Path) -> u64 {
    fs::metadata(path)
        .expect("WAL metadata must be readable")
        .len()
}

fn wait_for_wal_len(child: &mut Child, wal_path: &Path, min_len: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("server_d exited before forced kill: {status}"));
        }
        let len = fs::metadata(wal_path)
            .map_err(|e| format!("Cannot observe child WAL: {e}"))?
            .len();
        if len >= min_len {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(format!(
        "server_d WAL did not reach {min_len} bytes before the forced kill deadline"
    ))
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
    let wal_path = store.events_dir(&wid).expect("valid world id");
    create_world(baseline_dir.path(), seed);
    let baseline_store = FilesystemStore::new(baseline_dir.path()).expect("store creation failed");
    let baseline_wal = baseline_store.events_dir(&wid).expect("valid world id");
    let mut witness = make_sim(baseline_dir.path(), &wid);
    witness
        .load_world(&wid)
        .expect("load witness genesis failed");
    witness
        .process_command(SimCommand::Tick)
        .expect("first witness tick failed");
    let mut witness_end = wal_len(&baseline_wal);
    let first_tick_end = witness_end;

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

    // Observed length is a kill threshold, not evidence that sync_all returned.
    let ready = wait_for_wal_len(&mut child, &wal_path, first_tick_end);
    let killed = child.kill();
    let status = child.wait().expect("waiting for the killed child failed");
    ready.expect("child failed to reach the first complete tick");
    killed.expect("forced child kill failed");
    assert!(
        !status.success(),
        "child exited successfully instead of being killed"
    );
    let captured = fs::read(&wal_path).expect("read child WAL before recovery");
    let captured_len = captured.len() as u64;
    assert!(captured_len >= first_tick_end);

    // Successful live appends establish boundaries without parsing the child WAL.
    let mut complete_tick = Tick(1);
    let mut complete_end = first_tick_end;
    while witness_end < captured_len {
        witness
            .process_command(SimCommand::Tick)
            .expect("witness tick failed");
        let previous_end = witness_end;
        witness_end = wal_len(&baseline_wal);
        assert!(
            witness_end > previous_end,
            "witness tick must append a frame"
        );
        if witness_end <= captured_len {
            complete_tick = witness.current_tick();
            complete_end = witness_end;
        }
    }
    let witness_bytes = fs::read(&baseline_wal).expect("read continuous witness WAL");
    assert!(
        witness_bytes.starts_with(&captured),
        "child WAL is not a prefix of the continuous witness WAL"
    );

    let suffix_len = captured_len - complete_end;
    let opened = FileEventLog::new(&wal_path);
    if suffix_len >= 26 {
        let Err(SimError::CorruptedState(message)) = opened else {
            panic!("recovery must refuse an incomplete frame with a full header");
        };
        assert!(
            message.contains(&format!("offset {complete_end}:")),
            "{message}"
        );
        assert!(
            message.contains("exceeds remaining file bytes"),
            "{message}"
        );
        assert_eq!(fs::read(&wal_path).unwrap(), captured);
        return;
    }

    drop(opened.expect("recovery must accept an exact boundary or repair a short header"));
    assert_eq!(
        fs::read(&wal_path).unwrap(),
        captured[..complete_end as usize]
    );
    let (crashed_hash, recovered_tick) = load_hash_and_tick(crashed_dir.path(), &wid);
    assert!(recovered_tick.as_u64() > 0);
    assert_eq!(recovered_tick, complete_tick);

    // The boundary witness may be one tick ahead. Normalize a continuous run
    // stopped exactly at the identified tick, not that witness's later state.
    let aligned_dir = tempfile::tempdir().unwrap();
    create_world(aligned_dir.path(), seed);
    run_server(&[
        "--data-dir",
        aligned_dir.path().to_str().unwrap(),
        "run",
        "--world",
        &wid,
        "--ticks",
        &complete_tick.as_u64().to_string(),
        "--save-interval",
        "0",
    ]);
    let (baseline_hash, baseline_tick) = load_hash_and_tick(aligned_dir.path(), &wid);

    assert_eq!(baseline_tick, recovered_tick);
    assert_eq!(baseline_hash, crashed_hash);
}
