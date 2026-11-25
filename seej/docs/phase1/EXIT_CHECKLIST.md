# Phase 1 Exit Checklist

Phase 1 closes the first stable distributed kernel contract. This checklist is
the authority for status; architecture and contract documents describe targets
but do not prove them.

## Status convention

- `[x]` means the named evidence exists in the repository.
- `[ ]` means open, even when a design document describes the result.
- Existing gap IDs are permanent references. Moving a gap between gates does
  not rename it.
- A new gap requires objective evidence and links to the affected contract.

## Implemented evidence

These rows identify evidence already present for the local prototype. A checked
row means the named code or gate exists; it does not claim that the command was
run for this checkout or close a distributed target.

| Done | Criterion | Evidence | Reproduce from `seej/server/` |
| --- | --- | --- | --- |
| [x] | `P0-01` pure-core gate | [`scripts/check_sy_core_purity.py`](../../server/scripts/check_sy_core_purity.py) and CI reject forbidden `sy_core` dependencies/APIs. | `python3 scripts/check_sy_core_purity.py --self-test` then `python3 scripts/check_sy_core_purity.py` |
| [x] | `P1-01` repository CI definitions | [`.github/workflows/rust.yml`](../../../.github/workflows/rust.yml) versions check, build, fmt, test, clippy, rustdoc, dependency, fuzz-build, and purity jobs. | Inspect the workflow; run the standard commands documented in [`DETERMINISM.md`](DETERMINISM.md). |
| [x] | `P1-02` pinned Rust toolchain | [`rust-toolchain.toml`](../../server/rust-toolchain.toml) selects the repository toolchain. | `rustc --version` from this directory uses the pinned toolchain. |
| [x] | Deterministic tick loop | Canonical hash tests in [`sy_core/src/determinism.rs`](../../server/crates/sy_core/src/determinism.rs). | `cargo test -p sy_core determinism::tests` |
| [x] | Recoverable injected RNG | `restore_seeded_from_fresh_instance` in [`sy_infra/src/rng/mod.rs`](../../server/crates/sy_infra/src/rng/mod.rs). | `cargo test -p sy_infra rng::tests::restore_seeded_from_fresh_instance` |
| [x] | Snapshot + WAL recovery | Recovery parity and cursor cases in [`recovery_determinism.rs`](../../server/crates/sy_infra/tests/recovery_determinism.rs). | `cargo test -p sy_infra --test recovery_determinism` |
| [x] | WAL corruption cases | Header, CRC, partial-batch, and torn-tail tests in [`store/wal.rs`](../../server/crates/sy_infra/src/store/wal.rs). | `cargo test -p sy_infra store::wal::tests` |
| [x] | Strict event replay | Incoherent-event rejection tests in [`sy_core/src/replay.rs`](../../server/crates/sy_core/src/replay.rs). | `cargo test -p sy_core replay::tests` |
| [x] | Clean restart parity | `clean_restart_matches_continuous_run_hash`. | `cargo test -p sy_infra --test recovery_determinism clean_restart_matches_continuous_run_hash` |
| [x] | Crash replay parity | `crash_replay_matches_continuous_run_hash`. | `cargo test -p sy_infra --test recovery_determinism crash_replay_matches_continuous_run_hash` |
| [x] | Forced process kill | Ignored test in [`forced_kill_recovery.rs`](../../server/bin/server_d/tests/forced_kill_recovery.rs). | `cargo test -p server_d --test forced_kill_recovery -- --ignored` |
| [x] | Long burn-in | Ignored 100k-tick test in `recovery_determinism.rs`. | `cargo test -p sy_infra --test recovery_determinism burn_in_100k_ticks_keeps_deterministic_hashes_and_bounded_growth -- --ignored` |
| [x] | Workspace boundary | Deferred protocol and mod crates remain outside active workspace membership. | `cargo metadata --no-deps` |
| [x] | Standard CI gate | Repository workflow contains the standard workspace checks. | `cargo check --workspace --all-targets`; `cargo fmt --all --check`; `cargo test --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings` |
| [x] | Ignored Phase 1 gate | Workflow job `Phase 1 ignored tests` exists. | `cargo test --workspace --all-targets -- --ignored` |
| [x] | WAL fuzz build/smoke | Decoder and round-trip targets and scheduled smoke commands are documented in [`fuzz/README.md`](../../server/crates/sy_infra/fuzz/README.md). | Run the documented `cargo +nightly fuzz` commands from `crates/sy_infra/fuzz/`. |

These rows prove only their stated local behavior. They do not close a
distributed or generic-contract gap.

## Gate 1-K — first stable kernel contract

Every item in this section blocks the first stable contract.

### Kernel deliverables

- [ ] **K-01: Freeze the public contract.** Freeze the root, node-network,
  plugin, world, schema, error, and compatibility contracts only after all
  other Gate 1-K items pass. A third party can implement against them without
  reading internal Rust.
- [ ] **K-02: Confirmed observation surface.** Provide read-only state and event
  access from a confirmed prefix, with stable event IDs, cursor, freshness,
  rate limits, reconnect deduplication, cursor-expiration response, and
  coherent state resynchronization. Observer activity has no canonical effect.
- [ ] **K-03: Deterministic contract fixtures.** Provide fixed expected fixtures
  for an empty world and extension-defined data. At least one fixture has no
  position, zone, creature, or resource. Avoid probabilistic assertions.
- [ ] **K-04: Real external extension.** A separately maintained integration
  declares its own data and commands, uses only public contracts, survives
  recovery, and does not depend on `sy_core` internals. An observer alone does
  not satisfy this gap.
- [ ] **K-05: Replaceable fundamental providers.** The minimal distribution
  runs an empty world with bundled providers. Public-interface conformance
  fixtures substitute every fundamental capability boundary listed in the
  plugin contract and exercise its success and failure semantics. Test-only
  alternatives qualify; a second production implementation is not required.
  First-stable providers are trusted and operator-approved at startup.
- [ ] **K-06: Generic data ownership and transactions.** Support versioned
  structured or opaque owner domains, public owner commands, declared
  validators and obligations, and atomic transitions spanning at least two
  owners inside one world. Prove ordered candidate-state validation,
  cross-domain invariant checks, stale-base recomputation, and transfer or
  resolution of active obligations before owner removal.
- [ ] **K-07: Distributed kernel proof.** Run the same contract on one node and
  on three authorized nodes with quorum two. Prove admission, confirmation,
  coordinator loss, minority partition, stale-term/configuration fencing,
  catch-up, joint membership change, full restart, current-read barriers,
  compatibility refusal, and local-storage failure containment.
- [ ] **K-08: Durable command lifecycle.** Persist full command identity,
  producer slot/incarnation, intention fingerprint, target tick, positive
  status window, admission order, `Pending` admission, and terminal resolution.
  Prove recovery before admission, between admission and resolution, and after
  resolution before reply; also prove retry without duplicate effects,
  ID/payload conflict, tick closure, uncertain timeout, authorized cancellation
  using the same bounded identity rules, a cancellation/execution race with one
  durable winner, checked exact status expiry, confirmed generation
  opening/sealing/closure, producer retirement and slot reassignment, continuous
  producer activity and churn, non-reusable expired IDs, and suffix
  materialization without command re-execution.
- [ ] **K-09: Compatibility and migrations.** Define an execution profile and a
  migration mechanism for kernel-owned stable data. Prove compatible
  preparation, complete old-profile application-command drain or cancellation,
  a final source-state barrier that freezes every world tick and application
  mutation, activation commit, pre-activation abort, post-activation recovery,
  typed `WorldPaused` responses without admission, refusal of mixed profiles,
  stable membership during migration, and missing/incompatible plugin blocking.
- [ ] **K-10: Retention and recovery points.** Implement a declared bounded
  retention policy preserving current state, obligations, recovery, stable
  command-ID expiration, semantic continuity, and observer resynchronization.
  Prove terminal-result tombstones, per-incarnation closed-generation floors,
  per-slot incarnation high-water marks, and finite growth under continuous
  producer activity and producer churn. Full archival remains optional.

### Existing gaps incorporated into the stable gate

- [ ] **P0-02: Bind checkpoint cadence to simulated ticks.** Canonical
  checkpoint selection cannot depend on elapsed wall-clock time or node-local
  scheduling.
- [ ] **P0-03: Deterministic numeric decisions.** Canonical decisions cannot
  depend on unconstrained `f32` or `f64`; use explicit deterministic
  representations and fixtures.
- [ ] **P0-04: Fail-closed canonical counters.** Tick, event, entity/domain,
  command/generation sequence, producer incarnation, expiration arithmetic,
  authority term, and configuration version cannot wrap or saturate
  ambiguously.
- [ ] **P0-05: Persist the simulation contract.** State and history identify
  the complete replay-affecting contract, not only a physical format.
- [ ] **P0-06: Formalize genesis.** Persist and hash a versioned `GenesisSpec`
  containing initial domains, contracts, randomness policy, and authority
  profile.
- [ ] **P0-07: Decouple world identity from seed.** Reject mismatched world,
  genesis, and durable history.
- [ ] **P0-08: Persist command intentions.** Add the durable command envelope
  required by K-08.
- [ ] **P0-09: Canonicalize concurrent input order.** Transitions depend on
  confirmed per-world order, not arrival or thread timing.
- [ ] **P0-10: Enforce safe writer/authority ownership.** The current local
  store requires an exclusive writer; the distributed path requires authority
  terms, confirmed membership configurations, and provider-specific fencing.
- [ ] **P0-11: Bind durable records.** Bind confirmed records to world identity,
  genesis, execution profile, authority term, configuration version, and causal
  cursor.
- [ ] **P0-12: Validate world integrity.** Provide a pure generic validator
  callable after decode, recovery, migration, and before authority claims.
- [ ] **P0-13: Separate torn-tail recovery from corruption.** Fail or quarantine
  suspicious evidence; never silently repair it as an ordinary partial tail.
- [ ] **P0-14: Test common I/O failures before stability.** Inject full disk,
  read-only storage, I/O, append, sync, and rename failures. Prove no success,
  publication, or in-memory acceptance without required durability.
- [ ] **P0-15: Provide a replay oracle.** Recompute retained transitions from a
  checkpoint or genesis plus commands and exact contracts, then compare
  semantic state. State clearly when retention prevents genesis replay.
- [ ] **P1-04: Separate semantic and physical hashes.** World-state equality,
  storage integrity, and authenticated authority messages use distinct
  domains and metadata.
- [ ] **P1-05: Golden compatibility fixtures.** Preserve explicit accept,
  migrate, and refuse results across stable formats and contracts.
- [ ] **P1-06: Version persisted variants.** Commands, events, domain
  envelopes, manifests, and statuses use stable tags and typed unknown handling.
- [ ] **P1-07: Separate persisted DTOs from Rust internals.** Renaming an
  internal enum cannot silently rewrite the public persistence contract.
- [ ] **P1-09: Bind checkpoint integrity.** Detect torn, swapped, stale, or
  mismatched snapshot, manifest, confirmed-log, and authority data.
- [ ] **P1-10: Define durable world creation.** Test directory/store creation,
  manifest creation, initial confirmation, and first checkpoint boundaries.
- [ ] **P1-11: Define concurrent read behavior.** Opening an inspector never
  creates directories or mutates durable files and never presents a partial
  local tail as confirmed state.
  An explicit export must reject any resolved destination inside the canonical
  data directory or aliasing an existing store file, including normalized,
  symlink/junction, and hard-link aliases supported by the platform.
- [ ] **P1-12: Add adversarial recovery and replication fixtures.** Cover
  duplicate, missing, reordered, wrong-world, wrong-contract, stale-authority,
  incompatible-plugin, and wrong-checkpoint inputs.
- [ ] **P1-13: Bound and validate external commands.** Authenticate the source
  when required and reject malformed data before pure execution.
- [ ] **P1-14: Enforce size and cardinality limits.** Bound commands, opaque
  payloads, events, transactions, snapshots, replication messages, and replay
  batches. Also bound producer slots, non-closed generations, pending commands,
  retained terminal results, tombstones, and their bytes per producer and per
  world.
- [ ] **P1-15: Persist the limits profile.** Recovery and peers know which
  limits accepted prior data, sealed a generation, and fixed result expiry.
- [ ] **P1-16: Remove application rules from the kernel.** Resource and
  creature degradation execute as example/test extensions through the public
  provider boundary before freeze.
- [ ] **P1-17: Persist fencing state.** Stale local writers and stale network
  coordinators cannot append or publish after an authority change.
- [ ] **P1-18: Retire automatic WAL rewriting.** `truncate_after` remains only
  an explicit operator recovery mechanism until K-10 compaction replaces it.
- [ ] **P1-19: Implement retention-aware compaction.** Preserve stable cursors,
  checkpoints, obligations, causal linkage, per-command tombstones until
  generation closure, expired command identities through the durable
  per-incarnation generation floor, and retired incarnations through a bounded
  per-slot high-water mark.
- [ ] **P1-20: Add the minimum causal chain.** Link admission, resolution,
  events, active contracts, predecessor/result cursors, and before/after
  semantic hashes. Persist distinct confirmed, checkpoint, and applied cursor
  roles so the first stable contract can prove which confirmed operation
  produced each materialized state.
- [ ] **P2-01: Give `sy_loader` one explicit role.** It remains inert or
  becomes startup provider composition with boundary tests; no hidden logic.
- [ ] **P2-02: Prove baseline backup and restore.** Document and test at least a
  cold backup of the complete authoritative state and ordinary recovery after
  restore. Do not claim hot backup without a coherent protocol.
- [ ] **P2-03: Publish the baseline operator recovery runbook.** Cover the
  first-stable failure model: corruption, storage failure, failed migration,
  lost quorum, stale nodes, and missing components.
- [ ] **P2-06: Record the application-rule boundary decision.** The decision
  log and contracts identify the extracted prototype rules as fixtures and
  prevent new application logic from accumulating in the kernel.

### Stable documentation and operational proof

- [ ] The node-network, provider/plugin, persistence, security, backup, restore,
  and migration procedures agree on failure behavior.
- [ ] Every runnable command in documentation exists and has a successful smoke
  result.
- [ ] A failure matrix maps each first-stable promise to at least one test,
  conformance fixture, or read-only diagnostic.
- [ ] Documentation labels current, target, and later behavior consistently.
- [ ] All local links and anchors resolve.

### Evidence required for K-05

| Capability boundary | Required proof through an alternative public implementation |
| --- | --- |
| State storage and recovery | Durable ordering, coherent checkpoint recovery, integrity refusal, and injected write/sync/replace failures. |
| Command admission and status | Producer incarnation, identity, ordering, retry, cancellation, generation lifecycle, exact expiry, non-reuse, and bounds. |
| Tick scheduling | Equal canonical ticks from equal explicit scheduling input; no ambient-time dependency. |
| Deterministic validation | Equal typed results and enforcement for equal state, command, tick, and contracts. |
| Transaction coordination | Atomic multi-owner result, complete-candidate validation, and stale-base recomputation. |
| Membership and authority terms | Learner admission, joint configuration, persisted term/vote, and fencing. |
| Consensus and replication | One- and three-node order, quorum loss, coordinator replacement, catch-up, and restart. |
| Observation and retained cursors | Current/stale reads, permission checks, neutrality, cursor expiry, and coherent handoff. |
| Component loading and compatibility | Manifest, fingerprint, permission, dependency, version, migration, and refusal checks. |

Each row runs against the bundled implementation and at least one test-only or
external alternative that imports only the public capability contract.

## Gate 1-D — hardening pulled by real use

These gaps remain recorded and become scheduled when their trigger exists.
They do not weaken Gate 1-K.

### Trigger: a second supported execution platform

- [ ] **P1-03: Prove semantic hash parity across declared platforms.** Use fixed
  fixtures for every advertised compatibility profile.
- [ ] **P1-08: Complete portable numeric persistence policy.** Define NaN,
  infinities, rounding, and encoding if any persisted non-canonical float
  remains.

### Trigger: first deterministic divergence or research demand

- [ ] **P1-21: Add a deterministic flight recorder.** Persist compact execution
  evidence adequate for diagnosis.
- [ ] **P1-23: Build a divergence bisector.** Find the first different
  transition between two retained histories.

### Trigger: operators beyond the original maintainers

- [ ] **P1-22: Add a read-only doctor command.** Validate identity, profiles,
  checkpoints, history, ownership, obligations, and authority without repair.
- [ ] **P2-04: Emit structured recovery diagnostics.** Include identifiers,
  cursors, profiles, authority term, configuration version, repair mode, and
  semantic hashes.
- [ ] **P2-05: Maintain the failure-mode matrix beyond stable scenarios.**

### Trigger: sustained scale beyond baseline limits

- [ ] **P0-16: Expand crashpoint and chaos matrices.** Cover every provider
  boundary and multi-node interleaving beyond the common stable cases.
- [ ] Define performance targets from measurements.
- [ ] Propose a separate contract before world sharding or distributed
  execution of one transition.

## Evidence required for K-07

The reference distributed test matrix must include:

| Scenario | Required result |
| --- | --- |
| One node | Same public semantics under a one-member authority profile. |
| Three healthy nodes | One confirmed order and equal semantic state. |
| Coordinator stopped | A new coordinator continues after fencing the old term. |
| One-node minority | Read-only confirmed state with freshness; mutation refused or unavailable. |
| Delayed old coordinator returns | Cannot confirm or publish stale work. |
| Replica storage fails | It stops unsafe work; remaining quorum may continue. |
| Quorum storage fails | World mutations stop with no fabricated refusal for uncertain commands. |
| Existing voter restarts | Recovers durable term, vote, configuration, and log before participating; applies the read barrier before current-read claims. |
| New node joins | Acts as a non-voting learner until compatible and caught up. |
| Membership changes | Joint configuration preserves old/new quorum intersection, including interruption before final configuration. |
| Full group restarts | Recovers one term/configuration/history without local configuration overriding confirmed membership. |
| Profile mismatch | Node is denied authority and current-read claims. |
| Migration preparation fails | A confirmed abort preserves the old profile before activation. |
| Command remains blocked during migration | An authorized confirmed cancellation resolves only the still-pending command and permits the final migration barrier. |
| Cancellation races normal execution | Confirmed order yields exactly one terminal decision and retries return it. |
| Command finishes after migration preparation | Its effects precede and are included in the confirmed source-state barrier. |
| New application command arrives after preparation | After prior-ID lookup it receives `WorldPaused` with migration identity/phase and creates no Admission; retry after completion follows normal compatibility and late-tick rules. |
| Application command or tick arrives after the source barrier | It cannot confirm until activation or abort; the migrated candidate cannot overwrite post-barrier world state. |
| Migration activation reply is lost | Confirmed history decides whether old or new profile is authoritative. |
| Node fails after migration activation | Recovery finishes on the confirmed new profile; no implicit rollback occurs. |
| Configured voter is absent during activation | Its configured role and quorum weight remain; participation resumes only after compatibility and catch-up, or changes through explicit joint reconfiguration. |

## Evidence required for K-08 and K-10

| Scenario | Required result |
| --- | --- |
| Terminal result at tick `R`, window `100` | Detailed status is returned before `R + 100`; at `R + 100`, the result is `Expired`. |
| Different fingerprint at or after expiry | Returns `Expired`, never `Conflict` and never a new admission. |
| Result expires while generation remains sealed | Detailed result may compact to a non-reuse tombstone; retry remains `Expired`. |
| Admission races `SealGeneration` | Confirmed order either admits the command before the seal or returns `GenerationSealed` after it. |
| Producer submits continuously | Finite generation bounds force rotation or `LimitExceeded`; retained terminal data remains bounded. |
| Producer identities churn continuously | Finite slots force retirement/reassignment or `LimitExceeded`; per-slot incarnation high-water marks keep metadata bounded and old IDs expired. |
| Closed generation records are compacted | The current incarnation's generation floor rejects every old ID before any removed generation record is consulted. |
| Sealed generation contains a pending command | The command and required execution profile remain retained; limits prevent unlimited additional admission. |
| Checkpoint is at cursor 100 and history is confirmed through 110 | Recovery materializes records 101–110 into a new state instance without re-evaluating their resolved commands. |
| Materialization sees a cursor gap or wrong predecessor hash | Recovery fails closed and does not advance the applied cursor. |
| A recovered event or effect intention is delivered again | It keeps its stable identity; recovery itself invokes no external service and the asynchronous consumer deduplicates. |

## Platform durability note

The contract defines observable durability, not one universal filesystem
recipe. Every supported storage provider documents and tests its guarantees on
its advertised operating systems. Where directory sync, rename, locking, or
error behavior differs, the provider either supplies equivalent evidence or
declares the profile unsupported.
