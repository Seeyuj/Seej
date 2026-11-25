//! CLI smoke tests for `server_d`.
//!
//! These tests drive the binary like an operator would, asserting on
//! exit codes, stderr/stdout, and the on-disk state after the command.

use std::fs::{self, OpenOptions};
use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use sy_api::events::EventData;
use sy_api::persistence::IEventLog;
use sy_infra::FileEventLog;
use sy_types::Tick;
use tempfile::TempDir;

fn server_d() -> Command {
    Command::cargo_bin("server_d").expect("server_d binary must be built")
}

fn temp_data_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("seej_server_d_cli_")
        .tempdir()
        .expect("tempdir creation failed")
}

#[test]
fn create_requires_seed_argument() {
    let dir = temp_data_dir();

    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["create", "--name", "NoSeed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--seed").or(predicate::str::contains("seed")));
}

#[test]
fn create_then_list_shows_the_world() {
    let dir = temp_data_dir();

    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["create", "--name", "SmokeWorld", "--seed", "42"])
        .assert()
        .success();

    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("world_42"));
}

#[test]
fn create_writes_world_directory_layout() {
    let dir = temp_data_dir();

    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["create", "--name", "Layout", "--seed", "7"])
        .assert()
        .success();

    let world_dir = dir.path().join("worlds").join("world_7");
    assert!(world_dir.exists(), "world dir must be created");
    assert!(
        world_dir.is_dir(),
        "world entry must be a directory, not a file"
    );
    assert!(world_dir.join("meta.json").exists(), "meta must be saved");
    assert!(
        world_dir.join("snapshot.json").exists(),
        "snapshot must be saved"
    );
    assert!(world_dir.join("events").exists(), "WAL must be saved");
}

#[test]
fn invalid_log_level_does_not_panic() {
    let dir = temp_data_dir();

    let _assert = server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .args(["--log-level", "not-a-level", "list"])
        .assert();
}

#[test]
fn list_on_empty_data_dir_succeeds() {
    let dir = temp_data_dir();

    server_d()
        .args(["--data-dir"])
        .arg(dir.path())
        .arg("list")
        .assert()
        .success();
}

#[test]
fn run_rejects_unknown_wal_version_without_mutation() {
    let dir = temp_data_dir();
    server_d()
        .arg("--data-dir")
        .arg(dir.path())
        .args(["create", "--name", "V", "--seed", "91"])
        .assert()
        .success();

    let wal = dir.path().join("worlds/world_91/events");
    let prefix = fs::read(&wal).unwrap();
    let mut unknown_header = prefix[..26].to_vec();
    unknown_header[4..6].copy_from_slice(&2u16.to_le_bytes());
    OpenOptions::new()
        .append(true)
        .open(&wal)
        .unwrap()
        .write_all(&unknown_header)
        .unwrap();
    let before = fs::read(&wal).unwrap();

    let result = server_d()
        .arg("--data-dir")
        .arg(dir.path())
        .args(["run", "--world", "world_91", "--ticks", "1"])
        .assert()
        .code(1);
    let output = result.get_output();
    let diagnostic = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostic.contains("Unsupported WAL version: 2"),
        "{diagnostic}"
    );
    assert_eq!(fs::read(&wal).unwrap(), before);
}

#[test]
fn run_repairs_short_wal_header_then_appends_tick() {
    let dir = temp_data_dir();
    server_d()
        .arg("--data-dir")
        .arg(dir.path())
        .args([
            "create",
            "--name",
            "T",
            "--seed",
            "92",
            "--resources",
            "0",
            "--creatures",
            "0",
        ])
        .assert()
        .success();

    let wal = dir.path().join("worlds/world_92/events");
    let prefix = fs::read(&wal).unwrap();
    let genesis_events = FileEventLog::open_read_only(&wal)
        .unwrap()
        .read_all_valid()
        .unwrap();
    OpenOptions::new()
        .append(true)
        .open(&wal)
        .unwrap()
        .write_all(&prefix[..10])
        .unwrap();

    server_d()
        .arg("--data-dir")
        .arg(dir.path())
        .args(["run", "--world", "world_92", "--ticks", "1"])
        .assert()
        .success();

    assert!(fs::read(&wal).unwrap().starts_with(&prefix));
    let events = FileEventLog::open_read_only(&wal)
        .unwrap()
        .read_all_valid()
        .unwrap();
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.event_id.as_u64(), index as u64 + 1);
    }
    assert!(events[genesis_events.len()..]
        .iter()
        .any(|event| event.tick == Tick(1)
            && matches!(event.data, EventData::TickProcessed { tick: Tick(1), .. })));
}
