# Roadmap and Ordered Coding Plan

This document is the single authority for project sequencing, ordered coding
phases, implementation status, and release evidence. Design contracts in
`seej/docs/` describe targets and never close a checklist item.

## Status Convention and Discipline

- `[x]` — reproducible evidence is committed in this repository.
- `[ ]` — open target, regardless of how detailed its specification is.
- Stable gap IDs (`K-*`, `P0-*`, `P1-*`, `P2-*`) are preserved across
  reorganizations; retired items are marked explicitly with their decision ID.
- **Strict sequential discipline ([D-015](DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)):**
  complete and verify each phase in order (`Phase 1.1` → `1.2` → `1.3` → `1.4`
  → `1.5`, then `Phase 2`, then `Phase 3`). Do not add speculative abstractions,
  extra crates, or Phase 4 optimizations ahead of their phase.
- **In-code documentation:** every crate in `seej/server/crates/` carries
  `#![deny(missing_docs)]` and crate-level Rustdoc aligned with its exact phase
  below (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`).

---

## Phase 0 — Clean-Slate Foundation & Guardrails (Complete)

The legacy spatial prototype (`Entity`/`Zone`, JSON snapshots, dual
`sim.rs`/`replay.rs`, `server_d`/`sy_cli` fixtures) has been removed per D-015.
The repository now starts from a minimal 4-crate workspace skeleton
(`sy_types`, `sy_api`, `sy_core`, `sy_infra`) with active guardrails:

| Done | Criterion | Evidence | Reproduce from `seej/server/` |
| --- | --- | --- | --- |
| [x] | `P0-01` pure-core gate | [`scripts/check_sy_core_purity.py`](../seej/server/scripts/check_sy_core_purity.py) (17 self-tests incl. `std::thread` and char literals) + CI dep DAG check | `python3 scripts/check_sy_core_purity.py --self-test` && `python3 scripts/check_sy_core_purity.py` |
| [x] | `P1-01` repository CI | [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) (purity, fmt, clippy, test, doc, audit, deny, DAG) | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --all-targets`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |
| [x] | `P1-02` pinned toolchain | [`rust-toolchain.toml`](../seej/server/rust-toolchain.toml) (`1.95.0`) | `rustc --version` |
| [x] | `P2-01` inert placeholders retired | Removed dead placeholders (`sy_loader`, `sy_tools`, `sy_protocol`, `mods/`, `examples/`) and legacy prototype per D-015 | `cargo metadata --no-deps` |
| [x] | D-015 baseline & empirical benchmarks | [DECISIONS.md](DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam) and [BENCHMARKS.md](../seej/docs/BENCHMARKS.md) | Clean 4-crate workspace compiles & passes Rustdoc with `-D warnings` |

---

## Gate 1-K — First Stable Kernel & Plugin Contract (Phases 1–3)

Every unchecked item below blocks freezing the v1 contract (`K-01`).

### Phase 1 — Minimal Generic Core & Crash-Safe Local Store

Outcome: an empty or non-spatial world runs locally on the generic narrow waist
(`OrderedTable` + `ObligationRegistry` + single `apply(WriteSet)` +
`EntropyScope`) with a fail-closed durable store and injected storage faults.

#### Phase 1.1 — Stable Primitives, Checked Counters & Profile Types (`sy_types`)

- [ ] **P0-04: Fail-closed canonical counters.** Checked arithmetic on `Tick`,
  `EventId`, `AdmissionSeq`, `AuthorityTerm`, `ConfigVersion`, producer
  incarnation, generation, and expiration counters (never wrap or saturate).
- [ ] **P0-03: Deterministic numeric discipline.** Eliminate `f32`/`f64` from
  canonical decisions; pure integer/fixed-point primitives.
- [ ] **P0-07: Decouple `world_id` from seed.** Independent `WorldId` and
  `NodeId`; reject world/genesis/history mismatch; replicas keep `world_id`,
  clones allocate a new one.
- [ ] **P0-06: Formalize `GenesisSpec`.** Versioned, hashable genesis recording
  initial domains, contracts, randomness policy, and initial authority profile.
- [ ] **P0-05: Persist the execution profile.** Strongly typed
  `ExecutionProfile` binding contract versions, `ProofMode` (`delta` v1 default,
  future `root`), hash algorithm, entropy algorithm, active plugin fingerprints,
  and retention policy.
- [ ] **P1-15: Persist the limits profile.** Finite `LimitsProfile` including
  the isolated reserved control-plane budget for `CancelCommand` and migration
  control.
- [ ] **P1-06: Version persisted variants.** Stable tags and typed refusal for
  unknown mandatory variants.
- [ ] **P1-07: Separate persisted DTOs from internal Rust types.**

#### Phase 1.2 — Frozen Public Plugin Seam & Internal Traits (`sy_api`)

- [ ] **Public logical state & flat-batch plugin contract (`K-06` foundation):**
  partitioned `OrderedTable` `(DomainId, TableId, PartitionKey, RecordKey) ->
  OpaqueBytes`, extensible `TableKind`, `ObligationRegistry`, immutable
  `StateView`, `WriteSet`, `PluginManifest`, pure batch entrypoint
  `(StateView, InputBatch, EntropyScope, Context) -> Decision + WriteSet +
  EventBatch + OutboxIntents`, explicit `Sequential` vs. `SnapshotTMinus1` read
  semantics, optional `AccessHint`, and delta-oriented cross-domain validator
  traits.
- [ ] **Public command & resolution types:** `CommandEnvelope`, `Admission`,
  `Resolution` (`Succeeded`, `Refused`, `Cancelled`), `CancelCommand`,
  `CloseTick`, and lookup/transient outcomes (`Conflict`, `Expired`,
  `GenerationSealed`, `LimitExceeded`, `NotFound`, `Unavailable`,
  `WorldPaused`).
- [ ] **Internal infrastructure seams (`K-05` foundation):** clean, internal
  (non-frozen) `StateStore`, `AuthorityLog`, and `Executor` Rust traits.

#### Phase 1.3 — Pure Transition Engine & Single Mutation Path (`sy_core`)

- [ ] **K-06: Generic data ownership and transactions.** Single canonical
  `apply(WriteSet)` mutator (no `world.clone()`, no dual `sim`/`replay` logic),
  `CandidateOverlay`, atomic cross-owner transactions, stale-base full
  recomputation, zero fabricated `Refused` on plugin panic/failure, and
  active-obligation blocking on owner removal.
- [ ] **Derived `EntropyScope`:** counter-based integer PRF/PRNG derived from
  canonical coordinates with `.fork(stable_key)` and cause-derived IDs.
- [ ] **P0-12: Generic world integrity validator.** Pure validator invoked after
  decode, recovery, migration, and before authority claims.
- [ ] **P1-04: Separate semantic, storage, and authority digests.** Incremental
  per-record semantic root independent of physical storage layout.
- [ ] **P1-20: Minimum causal chain and distinct cursors.** Bind `pre_root`,
  `post_root`, `events_root`, profile hash, and distinct `confirmed_cursor`,
  `checkpoint_cursor`, and `applied_cursor`.
- [ ] **P1-13: Bound and validate external commands before pure execution.**
- [ ] **P1-14: Enforce size and cardinality limits.** Bound payloads, events,
  transactions, checkpoints, messages, producer slots, non-closed generations,
  pending commands, tombstones, and reserved control-plane budgets.
- [ ] **K-03: Deterministic contract fixtures.** Fixed expected fixtures for an
  empty world, a non-spatial extension (no position, zone, creature, or
  resource), cross-owner transactions, obligation enforcement, and an explicit
  $T-1$ autonomous pass proving partition-order independence.
- [ ] **P2-06: Application-rule boundary enforced in tests/CI.** Verify zero
  application ontology in `sy_core` and zero plugin imports of `sy_core`.

#### Phase 1.4 — Durable Local `StateStore`, Fail-Closed WAL & Checkpoints (`sy_infra`)

- [ ] **Partitioned local `StateStore`:** benchmark candidate storage engines
  (`redb`, `Fjall`, or custom paged partitioned store) against real workloads
  and implement atomic `apply(WriteSet)` + `pin_cut`.
- [ ] **P0-13: Separate torn-tail recovery from corruption.** Fail-closed
  binary WAL with explicit modes for safe short-header torn-tail, refusal,
  quarantine, and operator repair preserving forensic evidence, plus fresh
  `cargo-fuzz` decoder/round-trip targets restored to CI.
- [ ] **P1-18: Zero automatic WAL rewriting.** Never truncate past valid records
  or rewrite confirmed history silently.
- [ ] **P0-02: Tick-bound checkpoints.** Checkpoint cadence depends only on
  canonical ticks, never wall-clock time.
- [ ] **P1-09: Bind checkpoint integrity.** Detect torn, swapped, stale, or
  mismatched checkpoint/manifest/log/authority artifacts.
- [ ] **P1-10: Durable world creation.** Test directory, manifest, initial
  confirmation, and first checkpoint boundaries.
- [ ] **P0-14: Common I/O failure injection.** Inject full/read-only media,
  append, sync, rename, and read errors via `StateStore`/`AuthorityLog` fault
  doubles; prove fail-closed with zero accepted undurable state.

#### Phase 1.5 — Replay Oracle, Golden Vectors & Safe Local CLI (`sy_infra`, `server_d`, `sy_cli`)

- [ ] **P0-15: Replay / re-simulation oracle.** Recompute retained transitions
  from checkpoint or genesis + commands + exact profile and compare semantic
  roots.
- [ ] **P1-05: Golden compatibility fixtures.** Explicit accept, migrate, and
  refuse vectors across stable versions.
- [ ] **P1-11: Safe non-mutating local inspection.** Opening an inspector never
  creates directories or mutates files, never exposes unconfirmed local tails,
  and rejects export paths resolving inside or aliasing canonical storage.
- [ ] **P1-16: Example application plugin outside `sy_core`.** Optional
  reference/example domain plugin demonstrating public-contract usage without
  touching `sy_core` internals.

---

### Phase 2 — Single-World Distributed Authority & DST

Outcome: one-node and three-node (quorum 2) deployments obey the exact same
contract, proven under a single-threaded Deterministic Simulation Testing (DST)
harness plus real multi-process tests.

#### Phase 2.1 — Internal Seam Doubles & Single-Threaded DST

- [ ] **K-05: Internal seam testability & fault-injection conformance (refined
  by D-015).** Clean internal seams (`StateStore`, `AuthorityLog`, `Executor`)
  verified against in-memory and fault-injecting doubles covering normal and
  failure semantics (replaces D-014's nine frozen public provider contracts).

#### Phase 2.2 — Durable Command Lifecycle & Reserved Control Plane

- [ ] **P0-08: Persist durable command envelopes.**
- [ ] **P0-09: Canonicalize concurrent input order per world.**
- [ ] **K-08: Durable command lifecycle & reserved control path.** Full command
  identity, producer slot/incarnation, `intention_fingerprint`, target tick,
  positive status window, durable `Pending` admission, terminal resolution
  (`Succeeded`/`Refused`/`Cancelled`), `CancelCommand` on its **reserved control
  budget** (never starved by full application quotas), explicit `CloseTick(T)`
  blocking while a `Pending` command targets `T`, cancel-vs-execution race,
  exact checked expiry, confirmed `Open`/`Sealed`/`Closed` generations, slot
  retirement/reassignment, and suffix materialization without command
  re-execution.

#### Phase 2.3 — Single-World Authority, Fencing & Joint Membership

- [ ] **P0-10: Enforce exclusive local writer lock & distributed authority fencing.**
- [ ] **P0-11: Bind confirmed records to world, genesis, profile, term, config, and cursor.**
- [ ] **P1-17: Persist fencing state across restarts.**
- [ ] **K-07: Single-world distributed authority proof.** Prove one-node and
  three-node (quorum 2) operation: group-committed `TickCommit`, coordinator
  crash and replacement, minority read-only partition, stale-term/config
  fencing, learner catch-up, joint membership transition (including interruption
  between joint and final configs), full restart from durable term/vote/config,
  quorum read barriers, profile-mismatch refusal, and local-storage failure
  containment.

#### Phase 2.4 — Adversarial Replication & Multi-Process Proof

- [ ] **P1-12: Adversarial recovery and replication fixtures** (duplicate,
  missing, reordered, wrong-world, wrong-profile, stale-term, wrong-checkpoint)
  verified under both single-threaded DST and real multi-process crash tests.

---

### Phase 3 — Observation, Migrations, Compaction & External Extension → Freeze v1

Outcome: external observers and a real third-party write plugin consume the
public contract, kernel data migrates safely with mandatory boundary
checkpoints, and operators have tested backup/recovery procedures.

#### Phase 3.1 — Confirmed Observation Surface

- [ ] **K-02: Confirmed observation surface.** Read-only state and event access
  from a confirmed prefix with stable event IDs, cursor, freshness
  (current-confirmed via quorum read barrier vs. explicitly stale), rate limits,
  reconnect deduplication, cursor expiration, and pinned coherent
  resynchronization. Strictly neutral to canonical simulation.

#### Phase 3.2 — 5-Phase Coordinated Migrations

- [ ] **K-09: Compatibility and 5-phase migrations.** `MigrationPrepare`
  (`WorldPaused`), complete drain/cancellation of old-profile `Pending`
  commands, `MigrationBarrier`, deterministic candidate derivation on quorum,
  `MigrationActivate` with **mandatory boundary checkpoint**, pre-activation
  abort, post-activation recovery, fixed membership during migration, and
  incompatible/missing plugin refusal.

#### Phase 3.3 — Retention-Aware Compaction

- [ ] **K-10 & P1-19: Retention-aware compaction.** Bounded retention preserving
  recovery points, obligations, tombstones until generation closure,
  per-incarnation closed-generation floors, per-slot incarnation high-water
  marks, and observer resynchronization under continuous producer churn.

#### Phase 3.4 — External Write Extension & Operator Procedures

- [ ] **K-04: Real external write extension.** Independently maintained plugin
  declaring non-spatial data, owned commands, and validators, participating in
  a cross-owner transaction, surviving recovery, and importing only the frozen
  public contract (zero dependency on `sy_core` internals).
- [ ] **P2-02: Tested cold backup and restore procedure.**
- [ ] **P2-03: Baseline operator recovery runbook** (corruption, storage error,
  failed migration, lost quorum, stale node, missing plugin).

#### Phase 3.5 — Freeze the v1 Public Contract

- [ ] **K-01: Freeze the v1 public contract.** Freeze `CONTRACT.md`,
  `PLUGIN_CONTRACT.md`, and public schemas only after Phases 1–3 pass and every
  documented command, link, anchor, and failure-matrix test is verified.

---

## Phase 4 — Post-v1 Scale, Isolation & Federation (Gate 1-D, Usage-Pulled)

Deferred until triggered by real workloads or measurements; none blocks v1
freeze, and all are additive over the frozen v1 plugin contract:

### Phase 4.1 — Dense Workload Scale (e.g. Load B — $10^5$ writes/tick)
- [ ] **Async off-path CoW checkpoints** (eliminate the 240–435 ms synchronous
  144 MB checkpoint stall measured in [BENCHMARKS.md](../seej/docs/BENCHMARKS.md)).
- [ ] **Compact verified-tick proof mode (`root`)** in execution profiles once
  plugin determinism is enforced by isolation or verified profiles.
- [ ] **Multi-core parallel `Executor`** (`Block-STM` + optional `BoundedDelta`
  for K-06 transactions; parallel `BSP` over partitions for declared $T-1$
  passes), verified against the single-thread reference oracle.
- [ ] **High-write storage backends & `TableKind::Columnar`** (e.g. LSM /
  chunked columnar tables) behind the internal `StateStore` seam.

### Phase 4.2 — Untrusted / Dynamic Plugins (Evolution 2)
- [ ] **Isolated plugin execution (WASM)** with deterministic fuel metering,
  NaN canonicalization, and flat-batch memory-buffer transport.
- [ ] **Hot plugin installation, pause, replacement, and obligation transfer.**

### Phase 4.3 — Multi-World P2P Federation
- [ ] **Inter-world federation protocol:** asynchronous authenticated envelopes
  and cross-world obligations between independent CP worlds.

### Phase 4.4 — Cross-Platform Parity & Diagnostics
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
