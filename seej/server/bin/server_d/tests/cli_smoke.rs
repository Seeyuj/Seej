//! CLI smoke tests for `server_d`.
//!
//! These tests drive the binary like an operator would, asserting on
//! exit codes, stderr/stdout, and the on-disk state after the command.

use assert_cmd::Command;
use predicates::prelude::*;
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
