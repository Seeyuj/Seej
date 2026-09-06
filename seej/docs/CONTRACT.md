# Seej Kernel Contract

```text
contract_version: 0.2.0-draft
status:           DRAFT — NOT FROZEN
```

## Status and interpretation

This is the root public contract between the Seej world kernel and providers,
plugins, producers, observers, and operators. A third party must eventually be
able to use it without reading internal Rust code.

Statements use these labels:

- **Implemented:** behavior in the current repository with evidence named in
  the Phase 1 checklist.
- **Target (gap ID):** mandatory before the first stable contract.
- **Later:** intentionally outside that contract.

A section without an Implemented label describes the target. Freezing this
contract is gap `K-01`. Editing this draft does not freeze it.

After the first stable version, a change that alters admitted command meaning,
state transition, confirmed-history interpretation, stable identifiers, or
migration results requires an explicit contract version. Silent replay changes
are forbidden.

Specialized normative documents:

- [NODE_NETWORK.md](NODE_NETWORK.md) for inter-node authority;
- [PLUGIN_CONTRACT.md](PLUGIN_CONTRACT.md) for providers and plugins;
- [simulation/WORLD_SPEC.md](simulation/WORLD_SPEC.md) for world semantics.

## 1. Kernel boundary

Seej is a logical distributed kernel for persistent worlds. It owns:

- stable world identity and declared genesis;
- canonical simulated time;
- command admission, durable identity, status, and per-world ordering;
- orchestration of declared validators and atomic in-world transitions;
- confirmed history, persistence, recovery, and retention;
- node authority, compatibility, and coordinated migration;
- versioned public provider, plugin, and observation contracts.

Seej supplies minimal providers that allow an empty world to run. Application
laws and models remain extensions. There is no mandatory space, entity
category, agent, economy, renderer, or ingame netcode.

## 2. Identity and genesis

**Implemented:** the prototype requires a `u64` seed, derives
`world_<seed>`, and creates an origin zone. These are current local-format
facts, not the target identity contract.

**Target (P0-06, P0-07, K-06):**

- `world_id` is a stable identity independent of seed, node, path, and
  authority membership;
- a versioned, hashable `GenesisSpec` records seed or randomness policy,
  initial extension data, schema/contracts, and initial authority profile;
- `genesis_hash` proves genesis contents but is not the world identity;
- a replica retains `world_id`; an independent clone obtains a new one;
- loading refuses world, genesis, contract, or authority metadata mismatch.

## 3. Deterministic transition

The canonical transition is derived from:

```text
confirmed state + ordered admitted command + tick + active contracts
```

The pure engine does not read wall clock, network timing, filesystem ordering,
thread scheduling, ambient randomness, clients, or observers. Randomness is
explicitly seeded and its state or results are persisted as required by the
active contract. External nondeterminism enters as a validated persisted input.

Canonical counters fail closed on overflow. Canonical numeric rules avoid
unconstrained floating-point decisions. Supported execution profiles state the
platform and numeric guarantees they have actually proved; Seej does not claim
universal bit identity on untested platforms.

**Implemented:** the local transition engine has deterministic scheduling,
seeded RNG state, canonical entity ordering, recovery-parity tests, and a
purity gate. Current float-based degradation is a known defect.

**Target (P0-03, P0-04, P0-05, P0-15):** explicit simulation contract, safe
counters, deterministic numeric policy, and command-to-state replay oracle.

## 4. World data and ownership

World state is a collection of versioned domains. A domain declares:

- stable owner capability;
- format and schema or opaque payload version;
- commands and queries;
- validators and limits;
- dependencies and active obligations;
- migration steps.

Opaque data is allowed. The kernel preserves its envelope and bytes but may
require the owner plugin or a tool to interpret them.

Only the owner commands mutate a domain. A transaction can invoke several
owners inside the same world. It binds to one confirmed base cursor and orders
its owner sub-commands canonically. Starting from the immutable base state, the
kernel builds a private candidate state: each owner validates and proposes only
its own domain changes while observing the candidate produced by earlier
sub-commands. After all parts succeed, cross-domain validators inspect the
complete candidate state.

The resulting decision, events, and complete candidate transition are
confirmed atomically or no owner effects are applied. If the confirmed head no
longer matches the base cursor before confirmation, the proposal is discarded
and the entire transaction is recomputed from its actual canonical predecessor.
It is never partially rebased. Plugin timeout, crash, or unavailable compatible
code leaves the command pending or the authority unavailable; it is not
fabricated as a deterministic `Refused` decision.

The kernel does not define universal conservation. A world declares its laws,
including any creation, destruction, privilege, mortality, spatial, or
economic rules, and the kernel enforces the active validators.

**Implemented:** the prototype exposes fixed zone/entity structures and direct
commands. This is demonstration behavior scheduled for extraction.

**Target (K-05, K-06):** generic domains, ownership, trusted startup providers,
and atomic cross-owner transactions.

## 5. Time and commands

`Tick` is the only canonical simulation clock for a world. There is no global
network tick across worlds. A replica never independently advances canonical
time.

A target `CommandEnvelope` contains at least:

- a stable command identity composed of `world_id`, a kernel-issued
  `admission_generation`, a `producer_id`, and a producer-local sequence;
- `world_id` and target tick;
- payload plus an immutable fingerprint over the complete intention: world,
  target tick, owner contract and version, and canonical payload;
- owning command contract and version;
- canonical admission sequence once accepted;
- authorized source and correlation metadata when present.

Correlation and transport metadata that do not change the intention are not
part of the fingerprint. Every producer, including an internal scheduler, uses
a kernel-issued generation. A generation is distinct from a node authority
epoch and cannot be invented from network discovery.

Command handling has two durable logical records in the confirmed history:

1. **Admission** binds the envelope to a per-world admission sequence and
   creates durable `Pending` state. Admission alone has no application effect.
2. **Resolution** records `Succeeded` or `Refused` together with the complete
   event set and state transition. Applying this confirmed record once is the
   only way command effects become canonical.

An implementation may confirm admission and resolution in one storage or
consensus transaction, but the two logical states and their recovery behavior
remain observable. After recovery, an admitted command without a resolution is
still `Pending` and is deterministically resumed from its confirmed predecessor.
A confirmed resolution is never executed again.

New commands execute by target tick and then admission sequence. Closing a tick
is itself confirmed; after that boundary, a new command targeting that tick is
resolved as refused and is never silently moved. Deduplication precedes this
late check, so retrying an older command returns its existing result.

For one command namespace:

- same ID and same fingerprint returns the durable status without repeating
  effects;
- same ID and different fingerprint returns a conflict;
- an admitted command remains queryable at least through its declared status
  window, measured in canonical world ticks from its terminal resolution;
- a `Pending` command never expires and cannot be removed by compaction;
- generations close in increasing order through a confirmed operation only
  after every admitted command is terminal and its status window has elapsed;
- the durable closed-generation floor preserves non-reuse without retaining
  every individual result: any identifier in a closed generation is expired
  and cannot be admitted as new;
- a timeout or unavailable authority result is not a refusal.

Durable command states are `Pending`, `Succeeded`, and `Refused`. `Conflict`,
`Expired`, `NotFound`, and `Unavailable` are submission or query results and do
not replace a stored command state. `Unavailable` means the authority group
cannot answer safely; it is never a fabricated terminal result.

The persisted limits profile bounds open generations and pending commands. At
a bound, the authority group limits new admission with a typed non-terminal
response; it does not evict pending commands or manufacture a refusal.

**Implemented:** local commands and ticks exist, but no durable command journal,
deduplication contract, or network admission order exists.

**Target (P0-08, P0-09, K-08):** durable envelopes, canonical ordering, status
queries, fingerprint conflicts, late refusal, and non-reusable expired IDs.

## 6. Events and causal history

An admitted command eventually resolves with zero or more versioned events plus
a durable decision. Events identify the world, command, tick, resolution
position, owner, schema version, and stable event position. Unknown mandatory
variants are refused rather than silently skipped.

A confirmed causal chain binds command, resulting events, active execution
profile, and semantic state transition. Diagnostic logs are never canonical
history.

**Implemented:** `SimEvent` values have local event IDs and are stored in the
FileEventLog WAL. Some legacy events are replay no-ops.

**Target (P0-11, P1-06, P1-20):** explicit persisted DTOs and version tags,
world/contract binding, and causal linkage.

## 7. Distributed confirmation

A world authority group produces one confirmed history. Node discovery does
not grant membership. A local append, received proposal, or computed transition
is not a distributed commit.

Canonical success and observer publication occur only after the active
authority group provides the configured durable confirmation. Loss of quorum
pauses mutation. Authority epochs fence stale coordinators. Read-only access to
an older confirmed state may continue with explicit freshness.

**Implemented:** only the one-process, one-writer FileStore path.

**Target (K-07):** the behavior in [NODE_NETWORK.md](NODE_NETWORK.md), proven on
one and three nodes.

## 8. Persistence, recovery, replay, and migration

These operations are distinct:

- **Recovery:** load a compatible retained checkpoint and apply the confirmed
  retained history after its cursor.
- **Re-simulation:** recompute transitions from retained commands and exact
  execution components.
- **Migration:** convert durable data from one explicit stable version to
  another at a coordinated boundary.
- **Clone:** create a new world identity from copied or exported state.

Every accepted canonical mutation is durably confirmed before success. A
storage error such as full/read-only media, I/O failure, failed append, failed
rename, or failed synchronization prevents unsafe new acceptance. Suspicious
corruption is quarantined or refused; it is not silently repaired.

Kernel-owned data has a migration path from every stable version. Plugins own
their migrations. Re-simulation refuses to run when an exact referenced
component is missing. Operators retain plugin artifacts; Seej records their
references and cryptographic fingerprints.

**Implemented:** local snapshot + WAL recovery, CRC-protected WAL records, and
atomic snapshot replacement on the supported path. Format version 3 requires
an exact match; no migration currently exists.

**Target (P0-14, K-09):** injected common-I/O-failure evidence, explicit
migrations, compatible group activation, and historical-component checks.

The current `snapshot.json`, `meta.json`, and FileEventLog encoding are
reference-provider formats, not mandatory formats for every future provider.

## 9. Retention and observation

A world declares its retention policy. Compaction may remove old commands and
events only while preserving:

- a coherent recovery point and current confirmed state;
- active obligations;
- stable command-ID expiration safety;
- required migration and integrity metadata.

Compaction may replace terminal per-command results with a confirmed
closed-generation floor only after their status windows expire. It never
removes a pending admission or reopens a generation.

Full history archival is optional. Genesis re-simulation is possible only when
the full required history and artifacts were retained.

Observers read a confirmed prefix. Responses contain a stable cursor and
freshness. Reconnection may redeliver identified events. An expired cursor
returns an explicit expiration plus a coherent state and replacement cursor;
deleted history stays unavailable.

Observing is semantically neutral. Reader presence, speed, or identity cannot
change canonical fidelity, rules, ordering, or outcomes. The runtime may rate
limit or refuse observation under load.

**Implemented:** `sy_cli` provides read-only local inspection. It is not yet a
concurrent network observation surface.

**Target (K-02, K-10):** confirmed stream, retention, resynchronization, and
concurrent-read evidence.

## 10. Plugin lifecycle

The first stable release may activate only trusted, operator-approved providers
at startup. Activation validates compatibility, permissions, versions,
fingerprints, data migrations, and required capabilities.

Removing or replacing a plugin with owned data or active obligations requires
an explicit migration of its data and either transfer to an active successor or
resolution of every active obligation. Archival alone may detach code only when
all obligations are terminal, or when the entire world is explicitly archived
and cannot advance. The world may pause at a confirmed boundary while the
server remains available for inspection.

Hot loading and isolated application-plugin execution are **Later**. Their
eventual implementation must preserve this contract.

## 11. Semantic integrity

A semantic state hash covers canonical world meaning in a stable order. Storage
checksums cover physical artifacts. Network authentication verifies identities
and messages. These are separate mechanisms: the prototype's xxhash and CRC
checks do not authenticate nodes.

Integrity validation runs after decode, recovery, migration, and before a node
claims a current state.

**Target (P0-12, P1-04):** generic integrity validation and separation of
semantic, storage, and authority metadata.

## 12. Stable-contract acceptance

The first contract cannot freeze until the checklist proves at least:

- the generic kernel and provider boundary;
- fail-closed common storage errors;
- durable command lifecycle and retention;
- one- and three-node authority behavior;
- coherent confirmed observation;
- kernel-owned migrations;
- a third-party data-owning extension using public commands without internal
  dependencies.

The authoritative status and evidence live in
[phase1/EXIT_CHECKLIST.md](phase1/EXIT_CHECKLIST.md).
