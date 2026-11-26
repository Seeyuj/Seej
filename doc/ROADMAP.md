# Roadmap and Release Checklist

This document is the single authority for project sequencing, implementation
status, and release evidence. Design contracts in `seej/docs/` describe targets
and never close a checklist item.

## Status Convention

- `[x]` — reproducible evidence is committed in this repository.
- `[ ]` — open target, regardless of how detailed its specification is.
- Stable gap IDs (`K-*`, `P0-*`, `P1-*`, `P2-*`) are preserved across
  reorganizations; retired items are marked explicitly with their decision ID.
- **Rule of execution ([D-015](DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)):**
  build and verify Milestone 1 before starting Milestone 2, and Milestone 2
  before Milestone 3. Starting the new generic kernel cleanly (replacing the
  legacy spatial prototype in `seej/server/`) is explicitly preferred over
  dragging prototype types forward.

---

## Implemented Prototype Evidence

Evidence present in the current local spatial prototype (`seej/server/`).
These checks prove local prototype mechanics only; they do **not** close
generic-core or distributed Gate 1-K items.

| Done | Criterion | Evidence | Reproduce from `seej/server/` |
| --- | --- | --- | --- |
| [x] | `P0-01` pure-core gate | [`scripts/check_sy_core_purity.py`](../seej/server/scripts/check_sy_core_purity.py) (16 self-test cases, including char literals) + CI dep check | `python3 scripts/check_sy_core_purity.py --self-test` && `python3 scripts/check_sy_core_purity.py` |
| [x] | `P1-01` repository CI | [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --all-targets`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |
| [x] | `P1-02` pinned toolchain | [`rust-toolchain.toml`](../seej/server/rust-toolchain.toml) | `rustc --version` |
| [x] | Deterministic prototype loop | `sy_core/src/determinism.rs`, `sim.rs` (`TickN` rollback, SplitMix64 v4 genesis) | `cargo test -p sy_core` |
| [x] | Recoverable RNG (v3/v4) | `sy_infra/src/rng/mod.rs`, `recovery_determinism.rs` | `cargo test -p sy_infra rng::` |
| [x] | Fail-closed v1 WAL open | `sy_infra/src/store/wal.rs` (1–25 B short header repair only; refuse ≥ 26 B / bad version / CRC / I/O error) + `server_d` CLI tests | `cargo test -p sy_infra store::wal`; `cargo test -p server_d --test cli_smoke` |
| [x] | Snapshot + WAL recovery parity | `sy_infra/tests/recovery_determinism.rs` (v3/v4, clean restart, crash replay, proptest) | `cargo test -p sy_infra --test recovery_determinism` |
| [x] | Forced kill & 100k burn-in | `bin/server_d/tests/forced_kill_recovery.rs`, `recovery_determinism.rs` (ignored CI nightly) | `cargo test --workspace --all-targets -- --ignored` |
| [x] | WAL fuzz build/targets | [`crates/sy_infra/fuzz/README.md`](../seej/server/crates/sy_infra/fuzz/README.md) | `cargo +nightly fuzz build` in `crates/sy_infra/fuzz/` |
| [x] | `P2-01` inert `sy_loader` retired | Removed dead placeholders (`sy_loader`, `sy_tools`, `sy_protocol`, `mods/`, `examples/`) per D-015 | `cargo metadata --no-deps` |

---

## Gate 1-K — First Stable Kernel & Plugin Contract (3 Incremental Milestones)

Every unchecked item below blocks freezing the v1 contract (`K-01`). Work is
sequenced into three executable milestones so each layer is tested and measured
before stacking the next.

### Milestone 1 — Minimal Generic Core & Crash-Safe Local Store

Outcome: an empty or non-spatial world runs locally on the generic narrow waist
(`OrderedTable` + `ObligationRegistry` + single `apply(WriteSet)` +
`EntropyScope`) with a fail-closed durable store and injected storage faults.

- [ ] **K-06: Generic data ownership and transactions.** Partitioned
  `OrderedTable`, `ObligationRegistry`, public owner commands, delta-oriented
  validators, atomic cross-owner transactions via immutable `StateView` +
  `CandidateOverlay`, stale-base full recomputation, and active-obligation
  enforcement on owner removal.
- [ ] **K-03: Deterministic contract fixtures.** Fixed expected fixtures for an
  empty world and a non-spatial extension (no position, zone, creature, or
  resource), plus explicit $T-1$ read-semantics fixture. No probabilistic
  assertions.
- [ ] **P0-02: Tick-bound checkpoints.** Checkpoint cadence depends only on
  canonical ticks, never wall-clock time.
- [ ] **P0-03: Deterministic numeric decisions.** Eliminate `f32`/`f64` from
  canonical decisions; pure integer `EntropyScope` API.
- [ ] **P0-04: Fail-closed canonical counters.** Checked arithmetic on tick,
  event, sequence, incarnation, expiration, term, and configuration counters.
- [ ] **P0-05: Persist the execution profile.** History and checkpoints bind
  complete replay-affecting contract, proof mode, hash algorithm, entropy
  algorithm, and limits.
- [ ] **P0-06: Formalize `GenesisSpec`.** Versioned, hashable genesis recording
  initial domains, contracts, randomness policy, and initial authority profile.
- [ ] **P0-07: Decouple `world_id` from seed.** Reject world/genesis/history
  mismatch; replicas keep `world_id`, clones allocate a new one.
- [ ] **P0-12: Generic world integrity validator.** Pure validator invoked after
  decode, recovery, migration, and before authority claims.
- [ ] **P0-13: Separate torn-tail recovery from corruption.** Explicit modes
  for safe torn-tail, refusal, quarantine, and operator repair preserving
  forensic evidence.
- [ ] **P0-14: Common I/O failure injection.** Inject full/read-only media,
  append, sync, rename, and read errors via `StateStore`/`AuthorityLog` fault
  doubles; prove fail-closed with zero accepted undurable state.
- [ ] **P0-15: Replay / re-simulation oracle.** Recompute retained transitions
  from checkpoint or genesis + commands + exact profile and compare semantic
  roots.
- [ ] **P1-04: Separate semantic, storage, and authority digests.** Incremental
  per-record semantic root independent of physical storage layout.
- [ ] **P1-05: Golden compatibility fixtures.** Explicit accept, migrate, and
  refuse vectors across stable versions.
- [ ] **P1-06: Version persisted variants.** Stable tags and typed refusal for
  unknown mandatory variants.
- [ ] **P1-07: Separate persisted DTOs from internal Rust types.**
- [ ] **P1-09: Bind checkpoint integrity.** Detect torn, swapped, stale, or
  mismatched checkpoint/manifest/log/authority artifacts.
- [ ] **P1-10: Durable world creation.** Test directory, manifest, initial
  confirmation, and first checkpoint boundaries.
- [ ] **P1-11: Safe non-mutating local inspection.** Opening an inspector never
  creates directories or mutates files, never exposes unconfirmed local tails,
  and rejects export paths resolving inside or aliasing canonical storage.
- [ ] **P1-13: Bound and validate external commands before pure execution.**
- [ ] **P1-14: Enforce size and cardinality limits.** Bound payloads, events,
  transactions, checkpoints, messages, producer slots, non-closed generations,
  pending commands, tombstones, and reserved control-plane budgets.
- [ ] **P1-15: Persist the limits profile.**
- [ ] **P1-16: Extract prototype application rules.** Resource/creature rules
  execute only as an example/test plugin outside `sy_core`.
- [ ] **P1-18: Retire automatic WAL rewriting (`truncate_after`).**
- [ ] **P1-20: Minimum causal chain and distinct cursors.** Bind `pre_root`,
  `post_root`, `events_root`, profile hash, and distinct `confirmed_cursor`,
  `checkpoint_cursor`, and `applied_cursor`.
- [ ] **P2-06: Application-rule boundary enforced in tests/CI.** (Policy
  recorded in D-012/D-015; verify zero application ontology in the new core.)

### Milestone 2 — Single-World Distributed Authority & DST

Outcome: one-node and three-node (quorum 2) deployments obey the exact same
contract, proven under a single-threaded Deterministic Simulation Testing (DST)
harness plus real multi-process tests.

- [ ] **K-05: Internal seam testability & fault-injection conformance (refined
  by D-015).** Clean internal seams (`StateStore`, `AuthorityLog`, `Executor`)
  verified against in-memory and fault-injecting doubles covering normal and
  failure semantics (replaces D-014's nine frozen public provider contracts).
- [ ] **K-07: Single-world distributed authority proof.** Prove one-node and
  three-node (quorum 2) operation: group-committed `TickCommit`, coordinator
  crash and replacement, minority read-only partition, stale-term/config
  fencing, learner catch-up, joint membership transition (including interruption
  between joint and final configs), full restart from durable term/vote/config,
  quorum read barriers, profile-mismatch refusal, and local-storage failure
  containment.
- [ ] **K-08: Durable command lifecycle & reserved control path.** Full command
  identity, producer slot/incarnation, `intention_fingerprint`, target tick,
  positive status window, durable `Pending` admission, terminal resolution
  (`Succeeded`/`Refused`/`Cancelled`), `CancelCommand` on its **reserved control
  budget** (never starved by full application quotas), explicit `CloseTick(T)`
  blocking while a `Pending` command targets `T`, cancel-vs-execution race,
  exact checked expiry, confirmed `Open`/`Sealed`/`Closed` generations, slot
  retirement/reassignment, and suffix materialization without command
  re-execution.
- [ ] **P0-08: Persist durable command envelopes.**
- [ ] **P0-09: Canonicalize concurrent input order per world.**
- [ ] **P0-10: Enforce exclusive local writer lock & distributed authority fencing.**
- [ ] **P0-11: Bind confirmed records to world, genesis, profile, term, config, and cursor.**
- [ ] **P1-12: Adversarial recovery and replication fixtures** (duplicate,
  missing, reordered, wrong-world, wrong-profile, stale-term, wrong-checkpoint).
- [ ] **P1-17: Persist fencing state across restarts.**

### Milestone 3 — Observation, Migrations, Backup & External Extension → Freeze v1

Outcome: external observers and a real third-party write plugin consume the
public contract, kernel data migrates safely with mandatory boundary
checkpoints, and operators have tested backup/recovery procedures.

- [ ] **K-02: Confirmed observation surface.** Read-only state and event access
  from a confirmed prefix with stable event IDs, cursor, freshness
  (current-confirmed via quorum read barrier vs. explicitly stale), rate limits,
  reconnect deduplication, cursor expiration, and pinned coherent
  resynchronization. Strictly neutral to canonical simulation.
- [ ] **K-04: Real external write extension.** Independently maintained plugin
  declaring non-spatial data, owned commands, and validators, participating in
  a cross-owner transaction, surviving recovery, and importing only the frozen
  public contract (zero dependency on `sy_core` internals).
- [ ] **K-09: Compatibility and 5-phase migrations.** `MigrationPrepare`
  (`WorldPaused`), complete drain/cancellation of old-profile `Pending`
  commands, `MigrationBarrier`, deterministic candidate derivation on quorum,
  `MigrationActivate` with **mandatory boundary checkpoint**, pre-activation
  abort, post-activation recovery, fixed membership during migration, and
  incompatible/missing plugin refusal.
- [ ] **K-10 & P1-19: Retention-aware compaction.** Bounded retention preserving
  recovery points, obligations, tombstones until generation closure,
  per-incarnation closed-generation floors, per-slot incarnation high-water
  marks, and observer resynchronization under continuous producer churn.
- [ ] **P2-02: Tested cold backup and restore procedure.**
- [ ] **P2-03: Baseline operator recovery runbook** (corruption, storage error,
  failed migration, lost quorum, stale node, missing plugin).
- [ ] **K-01: Freeze the v1 public contract.** Freeze `CONTRACT.md`,
  `PLUGIN_CONTRACT.md`, and public schemas only after Milestones 1–3 pass and
  every documented command, link, anchor, and failure-matrix test is verified.

---

## Gate 1-D — Usage-Pulled Hardening and Scale (Post-v1)

Deferred until triggered by real workloads or measurements; none blocks v1
freeze, and all are additive over the frozen v1 plugin contract:

### Trigger: dense massive workloads (e.g. Load B — $10^5$ writes/tick)
- [ ] **Async off-path CoW checkpoints** (eliminate the 240–435 ms synchronous
  144 MB checkpoint stall measured in the lab harness).
- [ ] **Compact verified-tick proof mode (`root`)** in execution profiles once
  plugin determinism is enforced by isolation or verified profiles.
- [ ] **Multi-core parallel `Executor`** (`Block-STM` + optional `BoundedDelta`
  for K-06 transactions; parallel `BSP` over partitions for declared $T-1$
  passes), verified against the single-thread reference oracle.
- [ ] **High-write storage backends & `TableKind::Columnar`** (e.g. LSM /
  chunked columnar tables) behind the internal `StateStore` seam.

### Trigger: untrusted / dynamic plugins (Evolution 2)
- [ ] **Isolated plugin execution (WASM)** with deterministic fuel metering,
  NaN canonicalization, and flat-batch memory-buffer transport.
- [ ] **Hot plugin installation, pause, replacement, and obligation transfer.**

### Trigger: multi-world P2P scale
- [ ] **Inter-world federation protocol:** asynchronous authenticated envelopes
  and cross-world obligations between independent CP worlds.

### Trigger: multi-platform / external operators / diagnostics
- [ ] **P1-03:** Cross-platform semantic root parity fixtures (Linux, Windows, macOS).
- [ ] **P1-08:** Portable numeric persistence policy if any non-canonical float
  payload remains.
- [ ] **P1-21:** Deterministic flight recorder.
- [ ] **P1-22:** Read-only `doctor` verification command.
- [ ] **P1-23:** Divergence bisector across retained histories.
- [ ] **P2-04:** Structured recovery diagnostics.
- [ ] **P2-05:** Extended chaos/failure matrix beyond baseline scenarios.
- [ ] **P0-16:** Expanded crashpoint/chaos matrix across all boundaries.

---

## Optional Community Ecosystem

Never Seej kernel release obligations; communities build these independently
over the frozen v1 plugin and observation contracts:

- Spatial models (2D/3D grids, octrees, graphs) and non-spatial world models;
- Economies, ecologies, physics, narratives, agents, and LLM integrations;
- Multiresolution / causal-resolution aggregation engines;
- 2D/3D/web/Unreal/Godot renderers and administration dashboards;
- Ingame player netcode (prediction, interpolation, client sessions).
