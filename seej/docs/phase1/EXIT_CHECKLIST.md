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

| Done | Evidence | Scope |
| --- | --- | --- |
| [x] | `P0-01` pure-core gate | The script and CI reject forbidden APIs in `sy_core`. |
| [x] | `P1-01` repository CI definitions | Check, build, fmt, test, clippy, rustdoc, dependency, fuzz-build, and purity jobs are versioned. |
| [x] | `P1-02` pinned Rust toolchain | Repository and CI select the same Rust release. |
| [x] | Determinism/recovery tests | Current local prototype compares repeated and recovered state hashes. |
| [x] | Forced-process interruption test | Current local runtime can be killed and recovered from snapshot + WAL. |
| [x] | Burn-in test | Ignored test exercises 100k local ticks and recovery parity. |
| [x] | WAL fuzz targets | Decoder and round-trip targets build; scheduled smoke commands are documented. |

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
  runs an empty world with bundled providers. A conformance test replaces at
  least one fundamental provider without weakening its capability contract.
  First-stable providers are trusted and operator-approved at startup.
- [ ] **K-06: Generic data ownership and transactions.** Support versioned
  structured or opaque owner domains, public owner commands, declared
  validators and obligations, and atomic transitions spanning at least two
  owners inside one world.
- [ ] **K-07: Distributed kernel proof.** Run the same contract on one node and
  on three authorized nodes with quorum two. Prove admission, confirmation,
  coordinator loss, minority partition, stale-epoch fencing, catch-up,
  membership change, compatibility refusal, and local-storage failure
  containment.
- [ ] **K-08: Durable command lifecycle.** Persist command ID, payload
  fingerprint, target tick, canonical order, and status. Prove retry without
  duplicate effects, ID/payload conflict, late refusal, uncertain timeout, and
  non-reusable expired IDs.
- [ ] **K-09: Compatibility and migrations.** Define an execution profile and a
  migration mechanism for kernel-owned stable data. Prove compatible
  coordinated activation, refusal of mixed profiles, migration failure
  behavior, and missing/incompatible plugin blocking.
- [ ] **K-10: Retention and recovery points.** Implement a declared bounded
  retention policy preserving current state, obligations, recovery, stable
  command-ID expiration, semantic continuity, and observer resynchronization.
  Full archival remains optional.

### Existing gaps incorporated into the stable gate

- [ ] **P0-02: Bind checkpoint cadence to simulated ticks.** Canonical
  checkpoint selection cannot depend on elapsed wall-clock time or node-local
  scheduling.
- [ ] **P0-03: Deterministic numeric decisions.** Canonical decisions cannot
  depend on unconstrained `f32` or `f64`; use explicit deterministic
  representations and fixtures.
- [ ] **P0-04: Fail-closed canonical counters.** Tick, event, entity/domain,
  command sequence, and authority epoch cannot wrap or saturate ambiguously.
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
  epochs and provider-specific fencing.
- [ ] **P0-11: Bind durable records.** Bind confirmed records to world identity,
  genesis, execution profile, authority epoch, and causal cursor.
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
- [ ] **P1-11: Define concurrent read behavior.** Inspection never mutates
  durable files and never presents a partial local tail as confirmed state.
- [ ] **P1-12: Add adversarial recovery and replication fixtures.** Cover
  duplicate, missing, reordered, wrong-world, wrong-contract, stale-authority,
  incompatible-plugin, and wrong-checkpoint inputs.
- [ ] **P1-13: Bound and validate external commands.** Authenticate the source
  when required and reject malformed data before pure execution.
- [ ] **P1-14: Enforce size and cardinality limits.** Bound commands, opaque
  payloads, events, transactions, snapshots, replication messages, and replay
  batches.
- [ ] **P1-15: Persist the limits profile.** Recovery and peers know which
  limits accepted prior data.
- [ ] **P1-16: Remove application rules from the kernel.** Resource and
  creature degradation execute as example/test extensions through the public
  provider boundary before freeze.
- [ ] **P1-17: Persist fencing state.** Stale local writers and stale network
  coordinators cannot append or publish after an authority change.
- [ ] **P1-18: Retire automatic WAL rewriting.** `truncate_after` remains only
  an explicit operator recovery mechanism until K-10 compaction replaces it.
- [ ] **P1-19: Implement retention-aware compaction.** Preserve stable cursors,
  checkpoints, obligations, causal linkage, and expired command identities.
- [ ] **P2-01: Give `sy_loader` one explicit role.** It remains inert or
  becomes startup provider composition with boundary tests; no hidden logic.
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

- [ ] **P1-20: Add causality hashes.** Link commands, events, contracts, and
  before/after semantic hashes.
- [ ] **P1-21: Add a deterministic flight recorder.** Persist compact execution
  evidence adequate for diagnosis.
- [ ] **P1-23: Build a divergence bisector.** Find the first different
  transition between two retained histories.

### Trigger: operators beyond the original maintainers

- [ ] **P1-22: Add a read-only doctor command.** Validate identity, profiles,
  checkpoints, history, ownership, obligations, and authority without repair.
- [ ] **P2-02: Prove backup and restore.** Document and test at least cold
  backup; do not claim hot backup without a coherent protocol.
- [ ] **P2-03: Publish an operator recovery runbook.** Cover corruption,
  storage failure, failed migration, lost quorum, stale nodes, and missing
  components.
- [ ] **P2-04: Emit structured recovery diagnostics.** Include identifiers,
  cursors, profiles, authority epoch, repair mode, and semantic hashes.
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
| Coordinator stopped | A new coordinator continues after fencing the old epoch. |
| One-node minority | Read-only confirmed state with freshness; mutation refused or unavailable. |
| Delayed old coordinator returns | Cannot confirm or publish stale work. |
| Replica storage fails | It stops unsafe work; remaining quorum may continue. |
| Quorum storage fails | World mutations stop with no fabricated refusal for uncertain commands. |
| Node rejoins | Validates profile and catches up before authority/read-current claims. |
| Membership changes | Change itself is confirmed and old membership is fenced. |
| Profile mismatch | Node is denied authority and current-read claims. |
| Migration | All participants activate one compatible profile at a confirmed boundary. |

## Platform durability note

The contract defines observable durability, not one universal filesystem
recipe. Every supported storage provider documents and tests its guarantees on
its advertised operating systems. Where directory sync, rename, locking, or
error behavior differs, the provider either supplies equivalent evidence or
declares the profile unsupported.
