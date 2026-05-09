# Phase 1 Exit Checklist

This checklist is the closure gate for Phase 1. The table tracks implemented
evidence and the command or artifact that proves it; it is not a claim that
every command was run in the current checkout.

The implemented evidence does not mean the engineering contract is complete for
durable infrastructure. The open hardening gaps after the table must be resolved
before treating Phase 1 as a reliable long-term persistence and replay
foundation.

| Criterion | Repository status | Evidence | Reproduce |
| --- | --- | --- | --- |
| Headless world creation with mandatory seed | Implemented | `server_d create` requires `--seed` and creates `world_<seed>` | `cargo run --bin server_d -- create --name MyWorld --seed 42` |
| Deterministic tick loop | Implemented | canonical hash determinism tests in `sy_core` | `cargo test -p sy_core determinism::tests` |
| RNG is injected and recoverable | Implemented | `restore_seeded_from_fresh_instance` and recovery parity tests | `cargo test -p sy_infra rng::tests::restore_seeded_from_fresh_instance` |
| Snapshot + WAL recovery cursor | Implemented | snapshot cursor parity, interrupted save recovery, plus stale/missing WAL rejection | `cargo test -p sy_infra --test recovery_determinism` |
| WAL corruption handling | Implemented | empty, corrupt-first, truncated-tail, CRC, magic, and partial-batch tests | `cargo test -p sy_infra store::wal::tests` |
| Replay rejects incoherent events | Implemented | strict replay tests in `sy_core::replay` | `cargo test -p sy_core replay::tests` |
| Clean restart parity | Implemented | continuous run hash equals save/load/run hash | `cargo test -p sy_infra --test recovery_determinism clean_restart_matches_continuous_run_hash` |
| Crash replay parity | Implemented | continuous run hash equals snapshot + WAL replay hash | `cargo test -p sy_infra --test recovery_determinism crash_replay_matches_continuous_run_hash` |
| Forced process kill recovery | Implemented; ignored gate | force-kills `server_d`, reloads, compares to continuous hash | `cargo test -p server_d --test forced_kill_recovery -- --ignored` |
| Long burn-in | Implemented; ignored gate | 100k ticks twice, checkpoint hash parity, WAL growth, RSS bound where supported | `cargo test -p sy_infra --test recovery_determinism burn_in_100k_ticks_keeps_deterministic_hashes_and_bounded_growth -- --ignored` |
| No graphics/client/Phase 2 drift | Tracked in CI | Phase 2 crates/modules remain outside active workspace | `cargo metadata --no-deps` |
| Standard CI gate | Tracked in CI | check/build, fmt, test, clippy, rustdoc, supply-chain and boundary checks | `cargo check --workspace --all-targets && cargo fmt --all --check && cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings` |
| Ignored Phase 1 gate | Scheduled/manual CI | nightly/manual workflow job `Phase 1 ignored tests` | `cargo test --workspace --all-targets -- --ignored` |
| WAL fuzz build | Tracked in CI | fuzz targets compile on push/PR | `cargo +nightly fuzz build --dev --target x86_64-unknown-linux-gnu` from `crates/sy_infra/fuzz` |
| WAL fuzz smoke | Scheduled/manual CI | decoder and round-trip fuzz targets run for bounded time | `cargo +nightly fuzz run --target x86_64-unknown-linux-gnu decode_record -- -max_total_time=180` and `wal_round_trip` from `crates/sy_infra/fuzz` |

## Open Engineering Gaps

These are architecture-level gaps discovered after the implemented Phase 1
checks. They should be treated as Phase 1 hardening work, not Phase 2 gameplay or
network scope.

### P0 - must close before durable Phase 1 sign-off

- Persist a simulation contract, not only a storage format. `WorldMeta::format_version` gates snapshot shape, but it does not identify the rules that produced the state. Add a persistent `ruleset_version` or `simulation_contract` that covers systemic rules, RNG algorithm, command/event schema assumptions, and replay semantics. Include it in snapshot/meta/WAL compatibility checks and canonical hashing.

- Formalize genesis. The current world identity is derived from the seed (`world_<seed>`), and CLI population adds resources/creatures with hard-coded positions after `CreateWorld`. Introduce a persisted, hashable, versioned `GenesisSpec` that contains seed, initial topology, initial entities, schema assumptions, and initial rule contract. The seed must be a genesis parameter, not the complete world identity.

- Decouple `world_id` from seed. Two distinct worlds must be able to share the same seed with different genesis specs. `world_id` should be explicit or derived from a stable genesis hash; persistence and recovery should reject mismatches between `world_id`, `genesis_hash`, and stored state.

- Persist external commands as canonical intentions. The current WAL stores resulting `SimEvent`s, which is sufficient for crash recovery but incomplete for audit, command deduplication, causality, and full re-simulation from intentions. Add a durable `CommandEnvelope` containing at minimum `command_id`, `world_id`, target `tick`, durable `command_seq`, command payload, and optional source/correlation metadata.

- Define durable input ordering within a tick. The pure determinism runner sorts scheduled inputs by tick, but runtime persistence does not yet define a canonical command order for concurrent producers. Add a monotonic `command_seq` or `ingress_seq` per world and make state transitions depend on `(tick, command_seq)`, not arrival timing.

- Enforce single-writer ownership per world. `FileEventLog` reconstructs `next_event_id` locally and appends to one WAL path; two runtime processes can race and corrupt event ordering or duplicate IDs. Add an exclusive world lock or lease before opening the persistent runtime for writes. Recovery/inspection may remain read-only.

- Bind WAL records to world identity and simulation contract. WAL records currently identify format, event id, and tick, but not the world/genesis/ruleset they belong to. Add a WAL header or manifest that includes `world_id`, `genesis_hash`, and `simulation_contract`, and reject replay if those values disagree with the snapshot.

- Separate persisted DTOs from Rust domain types. Current persistence relies on serde over runtime/API structs such as `SimEvent` and `EventData`. Renaming a Rust enum variant or reshaping a field can silently become a persistence-format change. Add explicit storage DTOs for snapshots, WAL records, command logs, and manifests, with versioned conversion into domain types.

- Add a pure world-integrity validator. Snapshot decode and replay should be followed by `validate_world_integrity(world)` that checks metadata/tick parity, RNG checkpoint consistency, `next_entity_id`, entity IDs, zone IDs, zone membership indexes, missing references, invalid states, and all other recoverability invariants. The validator must live outside infrastructure I/O and be callable from tests, snapshot load, replay, and save paths.

- Define a fail-closed corruption and repair policy. `FileEventLog::new` can repair by truncating an invalid tail, which is correct for torn writes but dangerous if the bytes indicate unexpected corruption. Distinguish partial-tail recovery from suspicious corruption, add explicit repair/quarantine modes, and document when recovery must refuse to continue rather than mutate durable evidence.

- Add a replay oracle for Phase 1. Provide one authoritative test/tool path that rebuilds world state from `GenesisSpec + CommandEnvelope[] + simulation_contract`, compares it with snapshot/WAL recovery, and fails on any divergence. This catches the highest-risk bug class: code that mutates state correctly during live execution but cannot be reproduced from persisted intent.

- Add crashpoint injection around persistence boundaries. Tests should force interruption before/after snapshot temp write, after snapshot rename, before/after `meta.json`, before/after WAL append, and during mixed snapshot/WAL recovery. This is more valuable than generic crash tests because it targets the exact states that create unrecoverable or incoherent worlds.

- Persist a single-writer fencing token. A file lock is useful, but a durable fencing token or writer epoch makes stale writers detectable after process death, VM resume, or lock implementation differences. Every append/checkpoint should prove it still owns the active writer epoch before mutating durable state.

- Separate simulation-state hashes from storage-layout hashes. Canonical state hashing should prove semantic equality, while storage hashing should prove byte/layout continuity. Mixing the two makes it hard to know whether a failure is a real world divergence or only a persistence encoding change.

### P1 - should close before network or multi-producer work

- Add snapshot integrity metadata. The WAL has CRC validation; `snapshot.json` and `meta.json` do not yet have an explicit manifest/checksum binding them together. Add a world manifest or snapshot checksum so torn, swapped, or stale files are detectable beyond serde validation.

- Add golden compatibility fixtures. Store small snapshot/WAL/command-log fixtures and assert explicit accept/reject behavior across format/ruleset changes. This prevents accidental semantic drift hidden behind passing current-version tests.

- Add causality hashes to persisted transitions. A durable command record should be linkable to the events it produced and, eventually, to a `state_after_hash`. This gives operators and tests a cheap way to prove `command -> events -> state` continuity and to bisect deterministic divergence.

- Add a deterministic flight recorder. On every accepted command, persist enough compact metadata to explain execution without trusting runtime logs: `command_id`, `tick`, `command_seq`, ruleset/simulation contract, `state_before_hash`, produced event IDs, `state_after_hash`, and validation result. This turns future bug reports into replayable artifacts instead of anecdotes.

- Build a divergence bisector. Given two runs or two persisted histories, the tool should binary-search command/tick ranges using state hashes and report the first divergent transition. This directly reduces time-to-debug for determinism regressions.

- Add `sy_cli doctor` as a read-only world audit command. It should validate manifest/snapshot/WAL compatibility, replay cursors, world identity, genesis hash, ruleset contract, command/event continuity, checksums, and state integrity without mutating files. Operators need a safe diagnostic path before any repair path exists.

- Strengthen command validation. `validate_spawn_entity` is intentionally minimal today. Before accepting remote or multi-producer inputs, bound names/properties/positions and reject malformed commands before they reach the core.

- Forbid or strictly constrain floating-point persistence. `PropertyValue::Float(f64)` is risky for deterministic replay because NaN handling, serialization, comparison, and cross-platform behavior are subtle. Either disallow floats in persistent state/events for Phase 1 or replace them with explicit fixed-point/integer representations.

- Add explicit size and cardinality limits. Bound command payload size, WAL record size, snapshot size, events per command/tick, entities per world, properties per entity, string lengths, and replay batch sizes. Without hard limits, local CLI or future network ingress can create denial-of-service states that are technically valid but operationally unrecoverable.

- Persist a limits manifest. Hard limits should not live only as constants in the current binary. Store the active limits profile with the world so recovery can distinguish a valid old world from a new binary that changed limits, and so operators can explain why an input was accepted or rejected.

- Remove automated reliance on `truncate_after`. It rewrites the WAL and reassigns event IDs, which is incompatible with durable audit trails and stable causal references. Keep it only as a manual operator escape hatch until real compaction has an explicit metadata contract.

- Design real WAL compaction/checkpointing. A long-lived world needs a way to cut or rotate WAL history without losing the causal chain. Compaction must preserve stable cursors, manifest metadata, genesis/ruleset bindings, and enough audit material to explain how a snapshot was produced.

- Add adversarial replay tests. Recovery should reject or safely handle duplicated events, missing event IDs, reordered records, wrong `world_id`, wrong `genesis_hash`, wrong `simulation_contract`, wrong command sequence, stale manifests, and valid-looking WAL records attached to the wrong snapshot.

- Keep systemic rules either versioned or outside core. Minimal resource/creature degradation rules are acceptable only if covered by the simulation contract. If they become gameplay/module behavior, move them out of `sy_core` behind public APIs without violating deterministic replay.

### P2 - operator and observability hardening

- Write an operator recovery runbook. Document what to do for corrupt snapshots, WAL cursor behind snapshot, stale metadata, suspicious WAL repair, orphaned world locks, failed migrations, incompatible simulation contracts, and partial compaction. Durable infrastructure needs explicit incident procedures, not only code paths.

- Emit structured recovery diagnostics. Startup/recovery logs should include requested world id, snapshot cursor, durable WAL cursor, genesis hash, simulation contract, number of replayed records/events, repair mode, and pre/post recovery state hashes where available.

- Keep a failure-mode matrix. Track anticipated Phase 1 bugs and the exact test/tool that catches each one: live mutation without replay equivalent, insufficient event payload, incoherent snapshot/meta/WAL tuple, hidden WAL truncation, duplicate writer, serde persistence drift, seed/genesis confusion, cursor-only hash checks, unbounded WAL growth, and unsafe repair without operator evidence.

- Keep a boundary decision log for Phase 1 rules. If minimal degradation/cleanup rules remain in `sy_core`, record why they are core invariants and which contract version owns them. If they are future module behavior, track the extraction plan before more rules accumulate.

## Windows Durability Note

Phase 1 uses temp-file + fsync + atomic replacement for snapshot and metadata writes. On Unix, the parent directory is fsynced after rename. On Windows, `FilesystemStore` uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` for `save_snapshot` and `save_meta`. The WAL remains append-only and relies on file `sync_all` plus record CRC/truncation recovery.
