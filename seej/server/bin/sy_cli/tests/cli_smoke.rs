//! CLI smoke tests for `sy_cli` (read-only inspection tool).

use assert_cmd::Command;
use predicates::prelude::*;
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
