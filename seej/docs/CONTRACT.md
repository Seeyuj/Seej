# Seej Kernel & Authority Contract

```text
contract_version: 0.4.0-draft
status:           DRAFT — NOT FROZEN (gap K-01)
```

This is the single normative draft contract for the Seej world kernel: identity,
deterministic transitions, state mutation, commands, resolution roots,
single-world authority, recovery, migration, observation, and inter-world
federation. Plugin-facing contracts live in
[PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md). Current prototype behavior lives in
[PROTOTYPE.md](PROTOTYPE.md); implementation status lives in
[doc/ROADMAP.md](../../doc/ROADMAP.md).

Statements without an **Implemented** note describe the v1 target. After v1
freeze (`K-01`), any replay-affecting change requires an explicit contract or
execution-profile version bump.

---

## 1. Kernel Boundary and Identity

Seej is a logical world kernel owning:

- stable world identity and versioned genesis;
- canonical simulated time (`Tick`);
- command admission, durable identity, bounded status retention, and reserved
  control-plane cancellation;
- orchestration of plugin domains, atomic cross-owner transactions, and active
  obligations;
- single-path write-set materialization, semantic state roots, persistence,
  recovery, and coordinated migration;
- single-world authority confirmation, membership transitions, and confirmed
  observation.

Application ontologies (space, entities, zones, economies, agents, renderers,
ingame netcode) remain outside the kernel.

### Identity & Genesis (`P0-06`, `P0-07`)

- `world_id` is a stable identity independent of seed, `genesis_hash`, node
  identity, filesystem path, and authority membership.
- A versioned, hashable `GenesisSpec` records randomness policy, initial domain
  data, execution profile, limits profile, and initial authority configuration.
- `genesis_hash` proves genesis contents but is not the world identity.
- A replica preserves `world_id`; an independent clone allocates a new one.
- Loading refuses any world, genesis, profile, or authority metadata mismatch.

---

## 2. Deterministic Transition and Entropy

Canonical transitions are pure functions of:

```text
confirmed state + ordered admitted inputs + tick + active execution profile
```

The pure engine never reads wall-clock time, packet arrival timing, filesystem
ordering, thread scheduling, ambient randomness, or connected observers.
External nondeterminism (LLMs, sensors, clocks) enters only as bounded,
validated, persisted commands.

- **Checked counters (`P0-04`):** tick, event, command/generation sequence,
  producer incarnation, expiration, authority term, and configuration version
  counters fail closed on overflow — never wrap or saturate silently.
- **No unconstrained floats (`P0-03`):** canonical decisions use explicit
  integer or fixed-point representations.
- **Derived entropy (`EntropyScope`):** instead of a single global mutable RNG
  cursor, entropy is derived deterministically from canonical coordinates
  `(world_seed, entropy_epoch, tick, admission_seq | phase, domain_id)` and
  forked per stable key. The counter-based PRF/PRNG algorithm is declared in
  the execution profile.
- **Execution profile (`P0-05`):** pins kernel contract version, proof mode,
  state-hash algorithm, entropy algorithm, active plugin manifests and
  fingerprints, limits profile, and retention policy. Supported platforms state
  their verified parity profile rather than claiming untested universal bit
  identity.

---

## 3. World State and Single Mutation Path

World state separates **canonical state** (values affecting future transitions
or established history) from **operational state** (local paths, sockets,
caches, leases, diagnostics).

- **Logical state model (`K-06`):** versioned owner domains backed by
  partitioned ordered tables
  `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes` plus
  kernel-visible `ObligationRegistry` envelopes.
- **Single mutation path (`D-015`):** every canonical mutation — live execution
  and crash recovery alike — constructs a deterministic `WriteSet` against an
  immutable `StateView` and commits it through one canonical `apply(WriteSet)`
  operation.
- **Cross-owner atomicity (within one world):** a transaction binds to one
  confirmed base cursor and orders its owner sub-commands canonically. Each
  owner mutates only its own domain in a private candidate overlay while
  observing earlier sub-commands; cross-domain validators inspect the complete
  candidate. Effects confirm atomically or not at all. If the confirmed head
  moves before confirmation, the candidate is discarded and recomputed in full
  from the actual canonical predecessor.
- **No fabricated refusals:** plugin crash, timeout, missing artifact, or
  storage/quorum failure never produces a deterministic `Refused` decision; the
  command remains `Pending` or the request returns `Unavailable`.

---

## 4. Time, Commands, and Control Plane

`Tick` is the sole canonical clock for a world. There is no global clock across
worlds, and replicas never advance ticks independently. Real-time cadence and
overload/catch-up policies are world configuration inputs that cannot weaken
durability, quorum, or observation neutrality.

### Command Envelope (`K-08`, `P0-08`)

A `CommandEnvelope` binds:

- stable command identity: `(world_id, producer_slot, producer_incarnation,
  admission_generation, producer_seq)`;
- `target_tick` and positive finite `status_window_ticks` fixed at admission;
- `owner_contract` + version, canonical payload, and immutable
  `intention_fingerprint` covering world, target tick, status window, contract,
  payload, and validation-affecting authorization context (excluding transport
  correlation metadata);
- canonical `admission_seq` once confirmed.

### Two Durable States

1. **Admission** binds the envelope to `admission_seq` and creates durable
   `Pending` state with no application effects.
2. **Resolution** records a terminal decision — `Succeeded`, `Refused`, or
   `Cancelled` — together with canonical events, roots, and transition proof.
   Applying a confirmed `Succeeded` resolution is the only way application
   effects become canonical. Admission and resolution may commit in one
   consensus batch, provided recovery preserves both logical states.

Request/query results `Conflict`, `Expired`, `GenerationSealed`,
`LimitExceeded`, `NotFound`, `Unavailable`, and `WorldPaused` are transient or
lookup outcomes and never replace a stored terminal state.

### Tick Closure, `Pending` Commands, and Reserved Control Budget (`D-015`)

Commands execute by `(target_tick, admission_seq)`. Closing tick `T` via
confirmed `CloseTick(T)` rejects genuinely new commands targeting `T` as
`Refused` (never silently retargeted); deduplication precedes late checks.

- **Tick closure rule:** `CloseTick(T)` **cannot** close tick `T` while any
  admitted `Pending` command targeting `T` remains unresolved. Such a command
  either reaches a normal terminal resolution (`Succeeded`/`Refused`) or is
  resolved as `Cancelled` by an authorized control operation.
- **Authorized `CancelCommand`:** a confirmed kernel control operation carrying
  its own bounded identity, authorized requester, target command, and reason.
  It resolves only a still-`Pending` command, produces no application effects,
  cannot erase confirmed state or settle application obligations, and races
  normal execution strictly through confirmed order (first terminal record
  wins).
- **Reserved control-plane budget:** the limits profile allocates a dedicated,
  isolated capacity budget for kernel control operations (`CancelCommand` and
  migration control) separate from application quotas. Saturated application
  limits must never return `LimitExceeded` to authorized control operations,
  preventing liveness deadlocks when a `Pending` command blocks tick
  advancement.

### Deduplication, Exact Expiry, and Bounded Generations (`K-08`, `K-10`)

Lookup order for a submission or query:

1. Verify `producer_slot` incarnation. An older incarnation immediately returns
   `Expired` via the durable per-slot incarnation high-water mark.
2. Check the active incarnation's durable closed-generation floor. At or below
   the floor returns `Expired`.
3. Verify the generation was opened by confirmed `OpenGeneration` (which
   atomically seals the prior open generation). Look up the command ID.
4. For a terminal resolution at canonical tick `R`, checked arithmetic fixes
   `expiration_tick = R + status_window_ticks`. At `confirmed_tick >=
   expiration_tick`, return `Expired` (even if fingerprints differ); compaction
   may replace detailed results with a non-reuse tombstone until generation
   closure.
5. For an unexpired existing entry: matching fingerprint returns its durable
   status (preceding late-tick and migration-pause checks); differing
   fingerprint returns `Conflict`.
6. If migration admission pause is active, genuinely new application
   submissions return `WorldPaused` with migration identity/phase and create no
   Admission.
7. For an unused ID: `Open` generation proceeds to admission (or `NotFound` on
   status query); `Sealed` generation returns `GenerationSealed` on submission.

A `Pending` command never expires and cannot be compacted away. Finite
persisted limits bound producer slots, non-closed generations, command counts
and bytes, pending work, results, and tombstones — returning `LimitExceeded`
without mutating state or extending promised windows. Once every command in a
`Sealed` generation is terminal and expired, confirmed `CloseGeneration`
advances the closed-generation floor; retiring an incarnation requires all its
generations closed, after which reassigning the slot increments its monotonic
incarnation.

---

## 5. Resolution, Roots, and Proof Modes

A confirmed tick commits a group-committed batch (`TickCommit`) containing
ordered admissions, control operations, and resolutions. Every confirmed
resolution/tick binds:

- `pre_state_root` and `post_state_root` (semantic state roots independent of
  physical storage layout, `P1-04`, `P1-20`);
- `events_root` (committing the canonical event stream emitted for observers);
- `execution_profile_hash`, `authority_term`, `config_version`, and causal
  cursor.

### Frozen Roots vs. Profile-Selected Proof Mode & Hash (`D-015`)

- **Roots are frozen in the v1 contract.** Predecessor mismatch, cursor gap, or
  post-root mismatch fails closed immediately.
- **State-hash algorithm lives in the execution profile**, never frozen to a
  single permanent algorithm in the root contract. While per-resolution roots
  are verified, the hash updates incrementally per modified record (benchmarks
  show per-partition full rehash breaks scattered command loads).
- **Proof mode lives in the execution profile:**
  - **`delta` (v1 default):** confirmed records carry the canonical `WriteSet`
    and events; recovery and followers materialize state mechanically via
    `apply(WriteSet)` without invoking application plugin code.
  - **`root` (future opt-in profile):** confirmed records carry compact ordered
    inputs, verdicts, `post_state_root`, and `events_root` without bulk
    internal deltas, for dense autonomous workloads once plugin determinism is
    enforced by isolation or verified profiles.

Storage CRCs and semantic state roots are integrity checks, never peer
authentication.

---

## 6. Single-World Authority and Consensus

Each world runs on one authorized authority group (1 node local, or 3
shared-nothing nodes with quorum 2 under crash faults, storage failures, and
network partitions). Byzantine nodes and permissionless admission are outside
v1.

- **Two monotonic fences:** `authority_term` fences coordinators during
  elections; `config_version` governs membership changes. Every authority
  message binds both.
- **One confirmed history:** a local WAL append is not distributed commit.
  Canonical success, state exposure, and observer publication occur only after
  quorum confirmation. Loss of quorum suspends mutations; nodes may serve
  explicitly stale read-only state.
- **Reference membership transition (joint consensus):**
  1. New nodes join as non-voting learners, verify the execution profile, and
     catch up.
  2. Current configuration confirms a joint configuration requiring majorities
     of both old and new voter sets.
  3. Final configuration confirms under the joint rule before removed voters
     lose authority.
  - An offline or incompatible configured voter retains its voter role and
    quorum weight until explicitly removed/demoted via joint reconfiguration;
    it cannot vote or claim current reads until compatible and caught up.
  - Durable term, vote, membership, and confirmed log survive full restart;
    local config files never override confirmed membership.
- **Returning node:** may reconcile an explicitly unconfirmed local suffix; any
  conflict in an already confirmed prefix is quarantined and never overwritten.
- **Local storage failure:** affected node fails closed or quarantines before
  accepting undurable work; the group continues only while quorum survives.

---

## 7. Recovery, Migration, and Retention

### Three Distinct Cursors

- `confirmed_cursor` — highest canonical history record confirmed by quorum.
- `checkpoint_cursor` — confirmed position atomically bound to a checkpoint's
  state, execution profile, and semantic root.
- `applied_cursor` — highest confirmed position materialized into a local state
  instance (advances atomically with state; lagging instances cannot serve
  current reads).

### Distinct Operations

- **Recovery:** validate a compatible checkpoint at `checkpoint_cursor`, then
  apply retained confirmed records after it in order (checking predecessor and
  post roots; never re-deciding confirmed terminal commands or directly
  re-invoking external services — external consumers deduplicate redelivered
  identified events).
- **Re-simulation:** separate diagnostic/audit operation recomputing retained
  commands with exact archived component artifacts and comparing roots. Refuses
  to run if any referenced artifact is missing.
- **Migration:** explicit conversion between stable versions at a confirmed
  boundary.
- **Clone:** creates a new `world_id` from exported state.

### 5-Phase Coordinated Migration (`K-09`)

Membership changes and migration are serialized (configuration version stays
fixed from prepare through activate or abort; elections may still advance
`authority_term`).

1. **`MigrationPrepare`:** identifies old/target execution profiles, conversion
   steps, required artifacts, and fixed `config_version`; pauses all new
   application admissions (`WorldPaused`).
2. **Complete drain:** every old-profile `Pending` application command reaches
   `Succeeded`, `Refused`, or authorized `Cancelled` (via the reserved control
   path).
3. **`MigrationBarrier`:** fixes the exact old-profile state, root, and cursor
   after complete drain. No world tick or application mutation can pass this
   barrier; observation, elections, and migration control continue.
4. **Deterministic derivation:** nodes forming quorum verify target artifacts
   and derive the identical migrated candidate and root from the barrier.
5. **`MigrationActivate` (commit point):** atomically confirms barrier
   reference, target profile, migrated state, and a **mandatory boundary
   checkpoint** under the new profile so ordinary recovery never crosses two
   execution profiles.
   - Before activation: confirmed abort releases the pause under the old
     profile.
   - After activation: no automatic rollback; returning requires a new explicit
     migration. Uncertain replies resolve from confirmed history.

### Retention & Backup (`K-10`, `P1-19`, `P2-02`)

Compaction may prune history older than retained recovery points only while
preserving current state, active obligations, non-reuse tombstones /
closed-generation floors / incarnation high-water marks, and migration
metadata. Full archival is optional. Cold backup/restore and a baseline
operator recovery runbook are v1 requirements; hot backup is later work.

---

## 8. Confirmed Observation

Observers read a confirmed prefix (`K-02`), never an unconfirmed local tail.
Every response identifies `world_id`, `authority_term`, `config_version`,
`confirmed_cursor`, `applied_cursor`, execution profile, and freshness:

- **Current confirmed read:** backed by a quorum read barrier in the active
  term and configuration, with the serving node applied through that barrier.
  An isolated former coordinator can never claim current state.
- **Known confirmed but stale:** explicitly marked stale/unknown freshness.

Event streams carry stable monotonic event IDs (at-least-once delivery on
reconnect; consumers deduplicate). When compaction expires an observer's
cursor, the server returns explicit cursor expiration plus a coherent
replacement state and cursor from the same pinned cut (restarting explicitly if
the cut cannot remain pinned through handoff).

**Neutrality:** observer presence, count, filters, or backpressure never alter
canonical rules, tick rate, fidelity, or outcomes. Under overload, observation
is rate-limited or refused.

---

## 9. Inter-World Federation

Seej does **not** shard one world's synchronous tick across independent
authority groups in v1 (lockstep coupling collapses availability as $p^N$).
Instead, horizontal scale and peer-to-peer decentralization across worlds use
**asynchronous inter-world envelopes and durable obligations**:

- Each world advances autonomously under its own CP authority group and tick
  clock.
- Cross-world effects are explicit asynchronous protocols (outbox events /
  escrow obligations / validated incoming commands), never synchronous
  cross-world atomic transactions.
- A dedicated inter-world P2P transport/federation protocol is scheduled in
  Gate 1-D once single-world v1 is proven.
