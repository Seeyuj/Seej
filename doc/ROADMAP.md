# Roadmap — Seej

## Status model

Decision D-012 reopened the project foundation. The earlier local Phase 1 slice
remains implemented evidence, but it is not the first stable kernel contract.

Roadmap labels mean:

- **Implemented evidence:** exists in the repository; does not imply release
  readiness.
- **Active target:** required for the current release gate.
- **Later:** sequenced after the first stable contract.
- **Optional ecosystem:** may be built by the community and is not a Seej
  release obligation.

The detailed proof state is maintained in
[the Phase 1 exit checklist](../seej/docs/phase1/EXIT_CHECKLIST.md).

## Foundation 0 — Local deterministic prototype

**Status: implemented evidence**

The repository already demonstrates:

- a headless Rust transition engine;
- explicit seed and deterministic tick execution;
- local snapshot and WAL persistence;
- recovery after forced process interruption;
- a single-writer runtime and inspection CLI;
- tests, purity checks, fuzz targets, and dependency gates.

This foundation intentionally uses a spatial entity/zone model and built-in
resource/creature degradation rules. Those choices prove the local mechanics;
they do not define the future universal world contract.

Completion of this historical slice is not revoked. Its architectural
assumptions are superseded where D-012 says so.

## Foundation 1 — Generic kernel contracts

**Status: active target**

Outcome: an empty or non-spatial world can run through public, versioned
capability contracts without depending on internal Rust types.

Required work:

- separate `world_id`, seed, genesis hash, and node identity;
- formalize generic owner domains and structured or opaque payload envelopes;
- define durable command IDs, fingerprints, ordering, statuses, late refusal,
  and expiration;
- define deterministic validation and atomic transactions across owners;
- move spatial entities and degradation rules into example/test extensions;
- provide replaceable fundamental providers loaded at startup;
- preserve `sy_core` as the pure transition engine;
- separate semantic state hashes from storage and authority metadata;
- define retained checkpoints, cursor expiry, and recovery semantics.

Evidence includes one replacement provider and one real external extension that
owns data and commands without internal dependencies.

## Foundation 2 — Distributed kernel

**Status: active target; required before first stable contract**

Outcome: the kernel itself operates as an authorized network of nodes while
preserving one history per world.

Reference proof:

- same contracts in one-node and three-node deployments;
- three authorized nodes, quorum two;
- explicit discovery, admission, and membership changes;
- Raft-style reference consensus for crash faults and network partitions;
- coordinator replacement and stale-authority fencing;
- no mutation without quorum;
- node catch-up after interruption;
- confirmed reads with freshness;
- local storage failure containment;
- compatibility checks for kernel, contracts, providers, and plugins.

The first distributed implementation replicates the complete world. Sharding,
Byzantine consensus, permissionless validators, automatic divergent-history
merge, and synchronous cross-world transactions are separate future designs.

The inter-node protocol is foundational work in this stage. Ingame netcode is
not.

## Foundation 3 — Durability and migration gate

**Status: active target; required before first stable contract**

Outcome: the documented guarantees survive common operational failures and
future kernel releases.

Required work:

- fail closed on full or read-only storage, I/O errors, failed append,
  synchronization, and rename;
- distinguish torn-tail recovery from suspicious corruption;
- preserve coherent snapshot, manifest, confirmed log, and authority metadata;
- implement bounded retention without command-ID reuse or broken recovery;
- maintain migrations for kernel-owned data from every stable version;
- coordinate migration activation across the authority group;
- require plugin-owned migration paths and exact historical component
  fingerprints;
- publish conformance fixtures for accepted and refused versions;
- document cold backup, restore, diagnosis, and recovery.

## First stable contract

The first stable release occurs only when Gates 1-K and their mandatory
evidence are complete. It includes:

- minimal runnable distribution;
- generic owner data and public commands;
- trusted operator-approved providers loaded at startup;
- distributed operation on the reference three-node profile;
- common storage-failure safety;
- command status and retention;
- read-only confirmed observation and resynchronization;
- stable kernel data migrations;
- a genuine third-party extension.

The stable promise does not include hot loading, plugin isolation, agents,
multiresolution, rendering, or ingame netcode.

## Evolution 2 — Safe plugin lifecycle

**Status: later**

- isolated application-plugin runtime with explicit permissions;
- native execution only by operator approval;
- hot installation, activation, pause, state transfer, replacement, and
  removal;
- obligation-aware lifecycle transactions;
- resource metering and failure containment;
- component distribution and artifact-management tooling.

The process may stay running during a controlled world pause. No component with
unresolved owned data or obligations is silently detached.

## Evolution 3 — Operational scale

**Status: later, pulled by real use**

- richer observability and divergence tooling;
- online backup when a coherent protocol exists;
- large-history compaction and archival services;
- load, chaos, and platform matrices derived from declared support profiles;
- performance work supported by measurements;
- optional world sharding under a new consistency contract;
- explicit asynchronous cross-world coordination.

Replication is never advertised as computation scaling unless measurements and
a dedicated execution contract prove it.

## Optional ecosystem

The community may develop these independently against stable contracts:

- spatial and non-spatial world models;
- agents, AI systems, economies, and narratives;
- multiresolution and causal-resolution engines;
- administration interfaces and observability tools;
- 2D, 3D, web, Unreal, Godot, or other renderers;
- ingame netcode, player sessions, prediction, and interpolation;
- import/export, research, or simulation-specific toolchains.

Seej does not reserve an official implementation or impose application policy
for these categories.

## Decision rules for roadmap changes

A roadmap change must state:

1. which contract or user need it serves;
2. whether it changes the first stable promise;
3. its failure behavior and compatibility impact;
4. the gap ID and objective evidence;
5. whether an earlier decision is superseded.

Documentation cannot close a gate. Only committed implementation and
reproducible evidence can.
