# Phase 1 Exit Checklist

Phase 1 closes through **two gates** with different bars:

- **Gate 1-K — Kernel Contract.** The non-negotiable bar. It proves the kernel
  is a causally closed, deterministic world with a contract that a third party
  can consume without reading the Rust source. Its operational invariant is:
  **we do not break replay.**
- **Gate 1-D — Durability Under Load.** Production-grade hardening. Every item
  here has an explicit **trigger**: the real-world condition that pulls it into
  scope. Items are not worked on before their trigger exists. They are tracked,
  not scheduled.

This split is recorded as decision `D-011` in `../../../doc/DECISIONS.md`.
The contract itself is drafted in [`../CONTRACT.md`](../CONTRACT.md); freezing
it is gap `K-01` below.

## Guarantee Tiers

The checklist distinguishes three levels of guarantee. They have different
costs and different owners:

1. **Tier 1 — Causal closure** (non-negotiable, Gate 1-K). Every state
   mutation enters through the single journaled aperture (commands in, events
   out). No effect without a journaled cause.
2. **Tier 2 — Deterministic re-execution** (non-negotiable, Gate 1-K). The
   same binary re-applies the same journal to the same result. Crash recovery
   *is* a replay; breaking this breaks survival of `kill -9`.
3. **Tier 3 — Long-horizon replay** (opt-in, Gate 1-D). Bit-perfect replay
   across binary versions, platforms, and years. This is a product guarantee
   pulled by demand (e.g. reproducible agent research), not a metaphysical
   requirement. Items serving only Tier 3 live in Gate 1-D.

## Tracking Convention

Use the checkbox as the source of truth for status:

- `[x] Code-covered`: implementation exists and the row names a test, command, or
  durable artifact that proves the behavior.
- `[x] Gate-covered`: a CI, scheduled, manual, or inspection gate exists, but the
  command still must be run before claiming current-checkout success.
- `[ ] Open`: required work remains incomplete or lacks enough code/test
  evidence to close.
- `Trigger:` (Gate 1-D only): the condition that pulls the item into active
  scope. Before the trigger exists, the item is intentionally not worked on.

Do not mark an item complete because it appears in a design document. A gap can
be checked only when the code, persistence artifacts, and tests or operator
commands prove the invariant. When a gap is closed, keep its stable ID, change
its checkbox to `[x]`, and add the evidence command or artifact in the same
change.

## Implemented Evidence (minimal recovery slice)

The table tracks implemented evidence and the command or artifact that proves
it; it is not a claim that every command was run in the current checkout.

| Done | Criterion | Coverage state | Evidence | Reproduce |
| --- | --- | --- | --- | --- |
| [x] | Headless world creation with mandatory seed | Code-covered | `server_d create` requires `--seed` and creates `world_<seed>` | `cargo run --bin server_d -- create --name MyWorld --seed 42` |
| [x] | Deterministic tick loop | Code-covered | canonical hash determinism tests in `sy_core` | `cargo test -p sy_core determinism::tests` |
| [x] | RNG is injected and recoverable | Code-covered | `restore_seeded_from_fresh_instance` and recovery parity tests | `cargo test -p sy_infra rng::tests::restore_seeded_from_fresh_instance` |
| [x] | Snapshot + WAL recovery cursor | Code-covered | snapshot cursor parity, interrupted save recovery, plus stale/missing WAL rejection | `cargo test -p sy_infra --test recovery_determinism` |
| [x] | WAL corruption handling | Code-covered | empty, corrupt-first, truncated-tail, CRC, magic, and partial-batch tests | `cargo test -p sy_infra store::wal::tests` |
| [x] | Replay rejects incoherent events | Code-covered | strict replay tests in `sy_core::replay` | `cargo test -p sy_core replay::tests` |
| [x] | Clean restart parity | Code-covered | continuous run hash equals save/load/run hash | `cargo test -p sy_infra --test recovery_determinism clean_restart_matches_continuous_run_hash` |
| [x] | Crash replay parity | Code-covered | continuous run hash equals snapshot + WAL replay hash | `cargo test -p sy_infra --test recovery_determinism crash_replay_matches_continuous_run_hash` |
| [x] | Forced process kill recovery | Code-covered; ignored gate | force-kills `server_d`, reloads, compares to continuous hash | `cargo test -p server_d --test forced_kill_recovery -- --ignored` |
| [x] | Long burn-in | Code-covered; ignored gate | 100k ticks twice, checkpoint hash parity, WAL growth, RSS bound where supported | `cargo test -p sy_infra --test recovery_determinism burn_in_100k_ticks_keeps_deterministic_hashes_and_bounded_growth -- --ignored` |
| [x] | No graphics/client/Phase 2 drift | Gate-covered | Phase 2 crates/modules remain outside active workspace | `cargo metadata --no-deps` |
| [x] | Standard CI gate | Gate-covered | check/build, fmt, test, clippy, rustdoc, supply-chain and boundary checks | `cargo check --workspace --all-targets && cargo fmt --all --check && cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings` |
| [x] | Ignored Phase 1 gate | Gate-covered | nightly/manual workflow job `Phase 1 ignored tests` | `cargo test --workspace --all-targets -- --ignored` |
| [x] | WAL fuzz build | Gate-covered | fuzz targets compile on push/PR | `cargo +nightly fuzz build --dev --target x86_64-unknown-linux-gnu` from `crates/sy_infra/fuzz` |
| [x] | WAL fuzz smoke | Gate-covered | decoder and round-trip fuzz targets run for bounded time | `cargo +nightly fuzz run --target x86_64-unknown-linux-gnu decode_record -- -max_total_time=180` and `wal_round_trip` from `crates/sy_infra/fuzz` |

Architecture references for future hardening:

- [`../simulation/WORLD_SPEC.md`](../simulation/WORLD_SPEC.md) formalizes the
  target world contract for genesis, world identity, simulation contract,
  ontology, module contracts, limits, persistence compatibility, and replay.
- [`../simulation/CAUSAL_RESOLUTION.md`](../simulation/CAUSAL_RESOLUTION.md)
  is referenced here only for the foundations it depends on. It must not be
  implemented as a runtime shortcut before those foundations exist.

These references do not mark any gap as completed. They define the contracts
future implementation work must satisfy.

---

## Gate 1-K — Kernel Contract (close before anything else)

Every unchecked item below blocks Gate 1-K. Preserve the stable ID when
editing so humans, commits, issues, and agents can refer to the same gap
without relying on fragile prose matching.

### Kernel deliverables (new in the kernel direction)

- [ ] **K-01: Freeze the kernel contract v0.** Promote
  [`../CONTRACT.md`](../CONTRACT.md) from draft to frozen v0: a short,
  versioned document that binds world identity, genesis, the time model, the
  command aperture, the event vocabulary, persistence formats, recovery
  semantics, and determinism rules — extracted from the implementation, not
  aspirational. A third party must be able to build a consumer against it
  without reading the Rust source. Required evidence: `CONTRACT.md` carries a
  `contract_version`, a change policy ("we do not break replay"), and every
  statement is either implemented or explicitly marked as a numbered gap.

- [ ] **K-02: Ship the observation surface (first userland brick).** A
  read-only, streaming view of a world's event log consumable by external
  tools, per [`OBSERVATION_SLICE.md`](OBSERVATION_SLICE.md). Required
  evidence: `sy_cli tail --follow --json` (or equivalent) emits JSON Lines
  matching the contract event shapes; reading never mutates durable files;
  concurrent-read policy of P1-11 is implemented for this path.

- [ ] **K-03: Provide an anchor world worth observing.** A documented genesis
  (seeded population plus the Phase 1 systemic rules) whose event stream shows
  visible systemic activity, with a reproducibility check: two runs from the
  same genesis produce identical event streams. Required evidence: documented
  scenario in `OBSERVATION_SLICE.md`; a test or scripted run proving
  stream-level determinism.

- [ ] **K-04: First external consumer.** At least one consumer of the
  observation surface or contract that is not maintained by the kernel author
  (a visualizer, a bot, an analysis script). This is the exit criterion of a
  kernel: someone else builds on it. Required evidence: a linked repository,
  script, or contribution consuming the contract surface.

### Contract integrity (formerly P0 items — the ABI)

- [ ] **P0-03: Forbid floating-point values in canonical deterministic decisions.** Canonical transitions must not depend on `f32` or `f64` arithmetic. Random chances, thresholds, rates, and rule decisions must use integers, fixed-point, or explicit rational representation. Persistent state/events must either reject floats or encode them using a deterministic fixed representation. This restriction applies to canonical persistent state transitions and deterministic simulation decisions; it is not a blanket ban on floats everywhere in the repository. Note: this looks like a Tier 3 concern but is not — restoring a world on a different machine after hardware failure re-applies the journal through a different binary. Cheap insurance now, expensive once persisted state accumulates floats. Subsumes the persistence side of P1-08.

- [ ] **P0-05: Persist a simulation contract, not only a storage format.** `WorldMeta::format_version` gates snapshot shape, but it does not identify the rules that produced the state. Add a persistent `ruleset_version` or `simulation_contract` that covers systemic rules, RNG algorithm, command/event schema assumptions, and replay semantics. Include it in snapshot/meta/WAL compatibility checks and canonical hashing.

- [ ] **P0-06: Formalize genesis.** The current world identity is derived from the seed (`world_<seed>`), and CLI population adds resources/creatures with hard-coded positions after `CreateWorld`. Introduce a persisted, hashable, versioned `GenesisSpec` that contains seed, initial topology, initial entities, schema assumptions, and initial rule contract. The seed must be a genesis parameter, not the complete world identity.

- [ ] **P0-07: Decouple `world_id` from seed.** Two distinct worlds must be able to share the same seed with different genesis specs. `world_id` should be explicit or derived from a stable genesis hash; persistence and recovery should reject mismatches between `world_id`, `genesis_hash`, and stored state.

- [ ] **P0-08: Persist external commands as canonical intentions.** The current WAL stores resulting `SimEvent`s, which is sufficient for crash recovery but incomplete for audit, command deduplication, causality, and full re-simulation from intentions. Add a durable `CommandEnvelope` containing at minimum `command_id`, `world_id`, target `tick`, durable `command_seq`, command payload, and optional source/correlation metadata. This is the journaled aperture of the closed system: all causation enters here.

- [ ] **P0-09: Define durable input ordering within a tick.** The pure determinism runner sorts scheduled inputs by tick, but runtime persistence does not yet define a canonical command order for concurrent producers. Add a monotonic `command_seq` or `ingress_seq` per world and make state transitions depend on `(tick, command_seq)`, not arrival timing.

- [ ] **P0-10: Enforce single-writer ownership per world.** `FileEventLog` reconstructs `next_event_id` locally and appends to one WAL path; two runtime processes can race and corrupt event ordering or duplicate IDs. Add an exclusive world lock or lease before opening the persistent runtime for writes. Recovery/inspection may remain read-only. Placed in Gate 1-K because a corrupted world in front of the first external consumer is fatal.

- [ ] **P0-11: Bind WAL records to world identity and simulation contract.** WAL records currently identify format, event id, and tick, but not the world/genesis/ruleset they belong to. Add a WAL header or manifest that includes `world_id`, `genesis_hash`, and `simulation_contract`, and reject replay if those values disagree with the snapshot.

- [ ] **P0-12: Add a pure world-integrity validator (minimal scope for 1-K).** Snapshot decode and replay should be followed by `validate_world_integrity(world)` that checks metadata/tick parity, RNG checkpoint consistency, `next_entity_id`, entity IDs, zone IDs, zone membership indexes, missing references, and invalid states. The validator must live outside infrastructure I/O and be callable from tests, snapshot load, replay, and save paths. Gate 1-K requires the minimal invariant set on load/replay; the exhaustive recoverability matrix belongs to Gate 1-D.

- [ ] **P0-15: Add a replay oracle for Phase 1.** Provide one authoritative test/tool path that rebuilds world state from `GenesisSpec + CommandEnvelope[] + simulation_contract`, compares it with snapshot/WAL recovery, and fails on any divergence. This catches the highest-risk bug class: code that mutates state correctly during live execution but cannot be reproduced from persisted intent. Depends on P0-06 and P0-08. This is the empirical proof of causal closure — the test that Tier 1 and Tier 2 actually hold.

- [ ] **P1-07: Separate persisted DTOs from Rust domain types.** Current persistence relies on serde over runtime/API structs such as `SimEvent` and `EventData`. Renaming a Rust enum variant or reshaping a field can silently become a persistence-format change. Add explicit storage DTOs for snapshots, WAL records, command logs, and manifests, with versioned conversion into domain types. In Gate 1-K because the contract cannot be "whatever serde does to internal enums": the frozen contract (K-01) must describe a format the code cannot change by accident.

- [ ] **P1-09: Add snapshot integrity metadata (minimal scope for 1-K).** The WAL has CRC validation; `snapshot.json` and `meta.json` do not yet have an explicit manifest/checksum binding them together. Gate 1-K requires the minimal binding checksum so torn, swapped, or stale files are detectable beyond serde validation; a full world manifest belongs with P0-11.

- [ ] **P1-11: Define concurrent read semantics for `sy_cli`.** `sy_cli` must have a clear policy when reading files while `server_d` writes. Either `sy_cli` requires the writer to be stopped for coherent inspection, or `sy_cli` tolerates an in-progress WAL tail by stopping at the first incomplete record without reporting it as suspicious corruption. Required evidence: documented policy; tests for reading during partial WAL append; read-only inspection never mutates durable files. In Gate 1-K because it is the direct prerequisite of the observation surface (K-02).

### Cheap hygiene (do in passing; none of these blocks the contract)

- [x] **P0-01: Enforce `sy_core` purity with automated gates.** Gate-covered by `scripts/check_sy_core_purity.py` and the CI job `sy_core purity`, which runs the gate self-test and scans `crates/sy_core/src/**`. The gate fails on forbidden wall-clock time, environment access, OS randomness, filesystem I/O, networking, and nondeterministic hash collections including `std::time::SystemTime`, `std::time::Instant`, `std::env::*`, `rand::rngs::OsRng`, and `std::collections::HashMap`/`HashSet`. Reproduce from `seej/server/` with `python3 scripts/check_sy_core_purity.py --self-test && python3 scripts/check_sy_core_purity.py`.

- [ ] **P0-02: Bind checkpoint cadence to simulated ticks, not wall-clock time.** Snapshot/checkpoint cadence must be deterministic and expressed in simulation ticks, not elapsed seconds. Required evidence: `server_d --save-interval` is documented as tick-based; tests prove two identical scheduled runs produce identical snapshot cursors; no wall-clock timer decides canonical checkpoint state.

- [ ] **P0-04: Make canonical counter overflow fail closed.** Canonical counters must not saturate silently or wrap ambiguously. This includes `Tick`, `EventId`, `EntityId`, `command_seq`, `ingress_seq`, and `writer_epoch`. If a canonical counter reaches its max value, the runtime must refuse the next mutation with an explicit error; no `next()` method may silently return the same value forever; recovery must reject ambiguous persisted cursor states. Honest scale note: a `u64` tick counter does not overflow in realistic lifetimes; this is hygiene, not a blocker.

- [x] **P1-01: Version all Phase 1 gates in the repository.** Gate-covered: `.github/workflows/rust.yml` commits the purity, check/build, fmt, test, clippy, rustdoc, supply-chain, boundary, fuzz-build, and scheduled ignored/burn-in jobs; `scripts/check_sy_core_purity.py` is committed; checklist rows reference reproducible commands. The commands must still be run before claiming current-checkout success.

- [x] **P1-02: Pin and verify the Rust toolchain.** Gate-covered: `seej/server/rust-toolchain.toml` pins `channel = "1.95.0"` with `rustfmt` and `clippy`; CI installs the same version via `dtolnay/rust-toolchain@1.95.0` and runs the workspace tests (including determinism tests) under it.

- [ ] **P1-18: Remove automated reliance on `truncate_after`.** It rewrites the WAL and reassigns event IDs, which is incompatible with durable audit trails and stable causal references. Keep it only as a manual operator escape hatch until real compaction has an explicit metadata contract. The trait method is already deprecated; finish by documenting the operator-only policy alongside P0-13.

- [ ] **P2-01: Define the Phase 1 contract of `sy_loader`.** `sy_loader` exists as an architectural placeholder. Its Phase 1 status must be explicit to prevent hidden runtime logic or unreviewed wiring. Preferred policy: keep `sy_loader` intentionally inert during Phase 1. Required evidence: document whether `sy_loader` is intentionally inert in Phase 1; if inert, CI proves no runtime depends on it; if active, tests prove its wiring boundaries.

- [ ] **P2-06: Keep a boundary decision log for Phase 1 rules.** If minimal degradation/cleanup rules remain in `sy_core`, record why they are core invariants and which contract version owns them. If they are future module behavior, track the extraction plan before more rules accumulate. The contract draft (`../CONTRACT.md`) records the v0 position; close this by confirming it in the decision log.

---

## Gate 1-D — Durability Under Load (usage-pulled)

These gaps are real, documented, and intentionally **not scheduled**. Each
group lists its trigger: the condition that pulls it into active scope. Do not
work on an item before its trigger exists; do not delete an item because its
trigger is far away.

### Trigger: a world runs under real, sustained load

- [ ] **P0-13: Define a fail-closed corruption and repair policy.** `FileEventLog::new` can repair by truncating an invalid tail, which is correct for torn writes but dangerous if the bytes indicate unexpected corruption. Distinguish partial-tail recovery from suspicious corruption, add explicit repair/quarantine modes, and document when recovery must refuse to continue rather than mutate durable evidence. Interim 1-K policy: fail-stop and refuse on anything beyond a torn tail.

- [ ] **P0-14: Define and test durable I/O failure policy.** Crashpoint injection covers process interruption, but Phase 1 must also define fail-closed behavior when persistence operations return errors without a crash: ENOSPC, EROFS, EIO, fsync failure, failed rename, and failed WAL append. The policy must prove no accepted in-memory mutation without durable WAL commit, no future ticks after critical persistence failure until recovery policy is explicit, no fresh `meta.json` accepted against a missing, partial, or older snapshot, and no silent repair of suspicious durable evidence. Required evidence: injected `IWorldStore` / `IEventLog` failures at precise write/fsync/rename/append boundaries; tests proving rollback or refusal behavior; tests proving no incoherent snapshot/meta/WAL tuple is produced.

- [ ] **P0-16: Add crashpoint injection around persistence boundaries.** Tests should force interruption before/after snapshot temp write, after snapshot rename, before/after `meta.json`, before/after WAL append, and during mixed snapshot/WAL recovery. The existing forced-kill and interrupted-save tests already cover the highest-risk paths; the exhaustive matrix is pulled by real load.

- [ ] **P1-10: Define durable creation semantics for world directories and WAL files.** Snapshot and metadata writes are already described, but initial directory and WAL creation need explicit durability semantics. Required evidence: parent directory fsync where supported; OS-specific behavior documented; crashpoint tests for world directory creation, WAL creation, and first append.

- [ ] **P1-12: Add adversarial replay tests.** Recovery should reject or safely handle duplicated events, missing event IDs, reordered records, wrong `world_id`, wrong `genesis_hash`, wrong `simulation_contract`, wrong command sequence, stale manifests, and valid-looking WAL records attached to the wrong snapshot. Several of these become nearly free once P0-11 lands; add those with P0-11.

- [ ] **P1-17: Persist a single-writer fencing token.** A file lock is useful, but a durable fencing token or writer epoch makes stale writers detectable after process death, VM resume, or lock implementation differences. Every append/checkpoint should prove it still owns the active writer epoch before mutating durable state. The P0-10 lock suffices until real multi-process operation exists.

- [ ] **P1-19: Design real WAL compaction/checkpointing.** A long-lived world needs a way to cut or rotate WAL history without losing the causal chain. Compaction must preserve stable cursors, manifest metadata, genesis/ruleset bindings, and enough audit material to explain how a snapshot was produced.

### Trigger: external input producers exist (agents brick, netcode brick)

- [ ] **P1-13: Strengthen command validation.** `validate_spawn_entity` is intentionally minimal today. Before accepting remote or multi-producer inputs, bound names/properties/positions and reject malformed commands before they reach the core.

- [ ] **P1-14: Add explicit size and cardinality limits.** Bound command payload size, WAL record size, snapshot size, events per command/tick, entities per world, properties per entity, string lengths, and replay batch sizes. Without hard limits, local CLI or future network ingress can create denial-of-service states that are technically valid but operationally unrecoverable.

- [ ] **P1-15: Persist a limits manifest.** Hard limits should not live only as constants in the current binary. Store the active limits profile with the world so recovery can distinguish a valid old world from a new binary that changed limits, and so operators can explain why an input was accepted or rejected.

### Trigger: Tier 3 guarantee is claimed (cross-version / cross-platform replay)

- [ ] **P1-03: Prove canonical hash parity across supported platforms.** The same genesis, simulation contract, command schedule, and canonical world state must produce the same semantic hash across supported CI platforms. Required evidence: a golden fixture with expected canonical hash; CI matrix runs at least Linux and Windows; macOS is optional unless declared supported. Pull this forward the day a second machine runs a shared world — the CI matrix itself is cheap.

- [ ] **P1-04: Separate simulation-state hashes from storage-layout hashes.** Canonical state hashing should prove semantic equality, while storage hashing should prove byte/layout continuity. Mixing the two makes it hard to know whether a failure is a real world divergence or only a persistence encoding change.

- [ ] **P1-05: Add golden compatibility fixtures.** Store small snapshot/WAL/command-log fixtures and assert explicit accept/reject behavior across format/ruleset changes. This prevents accidental semantic drift hidden behind passing current-version tests.

- [ ] **P1-06: Version event and payload variants explicitly.** A global `format_version` is not enough. Event and payload variants need stable explicit tags, and unknown variants must be rejected with typed errors. Required evidence: an unknown event variant fixture is rejected clearly; an unknown property/value variant fixture is rejected clearly; old known variants remain accepted through golden compatibility fixtures.

- [ ] **P1-08: Forbid or strictly constrain floating-point persistence.** `PropertyValue::Float(f64)` is risky for deterministic replay because NaN handling, serialization, comparison, and cross-platform behavior are subtle. The canonical-decision side is handled by P0-03 in Gate 1-K; this item tracks the remaining persistence-encoding policy for property values.

### Trigger: the first deterministic-divergence bug, or reproducible-research demand

- [ ] **P1-20: Add causality hashes to persisted transitions.** A durable command record should be linkable to the events it produced and, eventually, to a `state_after_hash`. This gives operators and tests a cheap way to prove `command -> events -> state` continuity and to bisect deterministic divergence.

- [ ] **P1-21: Add a deterministic flight recorder.** On every accepted command, persist enough compact metadata to explain execution without trusting runtime logs: `command_id`, `tick`, `command_seq`, ruleset/simulation contract, `state_before_hash`, produced event IDs, `state_after_hash`, and validation result. This turns future bug reports into replayable artifacts instead of anecdotes.

- [ ] **P1-23: Build a divergence bisector.** Given two runs or two persisted histories, the tool should binary-search command/tick ranges using state hashes and report the first divergent transition. This directly reduces time-to-debug for determinism regressions. Build it the day the first divergence appears; it will justify itself.

### Trigger: operators other than the author run worlds

- [ ] **P1-22: Add `sy_cli doctor` as a read-only world audit command.** It should validate manifest/snapshot/WAL compatibility, replay cursors, world identity, genesis hash, ruleset contract, command/event continuity, checksums, and state integrity without mutating files. Operators need a safe diagnostic path before any repair path exists. A minimal check can ride on the P0-12 validator.

- [ ] **P2-02: Write a coherent backup and restore procedure.** Operators must know how to copy and restore a world without breaking snapshot/WAL/meta consistency. Phase 1 documents cold backup only unless a coherent backup command exists: state whether the daemon must be stopped before backup, state that hot backup is forbidden or deferred to a future `sy_cli backup`, and warn that raw `cp -r` while a writer is active is not guaranteed safe. Required evidence: documented cold-backup procedure; restore procedure tested on a copied world directory.

- [ ] **P2-03: Write an operator recovery runbook.** Document what to do for corrupt snapshots, WAL cursor behind snapshot, stale metadata, suspicious WAL repair, orphaned world locks, failed migrations, incompatible simulation contracts, and partial compaction. Durable infrastructure needs explicit incident procedures, not only code paths.

- [ ] **P2-04: Emit structured recovery diagnostics.** Startup/recovery logs should include requested world id, snapshot cursor, durable WAL cursor, genesis hash, simulation contract, number of replayed records/events, repair mode, and pre/post recovery state hashes where available.

- [ ] **P2-05: Keep a failure-mode matrix.** Track anticipated Phase 1 bugs and the exact test/tool that catches each one: live mutation without replay equivalent, insufficient event payload, incoherent snapshot/meta/WAL tuple, hidden WAL truncation, duplicate writer, serde persistence drift, seed/genesis confusion, cursor-only hash checks, unbounded WAL growth, and unsafe repair without operator evidence.

### Trigger: systemic rules grow beyond the Phase 1 minimal set

- [ ] **P1-16: Keep systemic rules either versioned or outside core.** Minimal resource/creature degradation rules are acceptable only if covered by the simulation contract. If they become gameplay/module behavior, move them out of `sy_core` behind public APIs without violating deterministic replay. The v0 position (rules stay in core, owned by the simulation contract) is recorded in `../CONTRACT.md`; extraction is pulled by rule growth.

## Windows Durability Note

Phase 1 uses temp-file + fsync + atomic replacement for snapshot and metadata writes. On Unix, the parent directory is fsynced after rename. On Windows, `FilesystemStore` uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` for `save_snapshot` and `save_meta`. The WAL remains append-only and relies on file `sync_all` plus record CRC/truncation recovery.
