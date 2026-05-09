# Phase 1 Exit Checklist

This checklist is the binary closure gate for Phase 1. A criterion is closed only when the listed command or artifact passes.

| Criterion | Status | Evidence | Reproduce |
| --- | --- | --- | --- |
| Headless world creation with mandatory seed | Pass | `server_d create` requires `--seed` and creates `world_<seed>` | `cargo run --bin server_d -- create --name MyWorld --seed 42` |
| Deterministic tick loop | Pass | canonical hash determinism tests in `sy_core` | `cargo test -p sy_core determinism::tests` |
| RNG is injected and recoverable | Pass | `restore_seeded_from_fresh_instance` and recovery parity tests | `cargo test -p sy_infra rng::tests::restore_seeded_from_fresh_instance` |
| Snapshot + WAL recovery cursor | Pass | snapshot cursor parity, interrupted save recovery, plus stale/missing WAL rejection | `cargo test -p sy_infra --test recovery_determinism` |
| WAL corruption handling | Pass | empty, corrupt-first, truncated-tail, CRC, magic, and partial-batch tests | `cargo test -p sy_infra store::wal::tests` |
| Replay rejects incoherent events | Pass | strict replay tests in `sy_core::replay` | `cargo test -p sy_core replay::tests` |
| Clean restart parity | Pass | continuous run hash equals save/load/run hash | `cargo test -p sy_infra --test recovery_determinism clean_restart_matches_continuous_run_hash` |
| Crash replay parity | Pass | continuous run hash equals snapshot + WAL replay hash | `cargo test -p sy_infra --test recovery_determinism crash_replay_matches_continuous_run_hash` |
| Forced process kill recovery | Pass when ignored job passes | force-kills `server_d`, reloads, compares to continuous hash | `cargo test -p server_d --test forced_kill_recovery -- --ignored` |
| Long burn-in | Pass when ignored job passes | 100k ticks twice, checkpoint hash parity, WAL growth, RSS bound where supported | `cargo test -p sy_infra --test recovery_determinism burn_in_100k_ticks_keeps_deterministic_hashes_and_bounded_growth -- --ignored` |
| No graphics/client/Phase 2 drift | Pass | Phase 2 crates/modules remain outside active workspace | `cargo metadata --no-deps` |
| Standard CI gate | Pass | fmt, test, clippy | `cargo fmt --all --check && cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings` |
| Ignored Phase 1 gate | Pass when scheduled/manual CI passes | nightly/manual workflow job `Phase 1 ignored tests` | `cargo test --workspace --all-targets -- --ignored` |

## Windows Durability Note

Phase 1 uses temp-file + fsync + atomic replacement for snapshot and metadata writes. On Unix, the parent directory is fsynced after rename. On Windows, `FilesystemStore` uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` for `save_snapshot` and `save_meta`. The WAL remains append-only and relies on file `sync_all` plus record CRC/truncation recovery.
