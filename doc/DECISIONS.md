# Decision Log

## Navigation

- [`README.md`](../README.md)
- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`DECISIONS.md`](DECISIONS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`CONTRIBUTING.md`](../CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)

Log of architectural and conceptual decisions

## Role of This Document

This document records the **structural decisions** made for the project.

Its objectives are to:

- explain major technical, conceptual, and organizational choices;
- avoid perpetual re-discussion of already settled decisions;
- provide a clear reference for maintainers and contributors;
- guarantee the project's coherence in the long term.

This document is **neither a roadmap**, nor a list of planned features.  
It describes **what is decided**, **why**, and **what it implies**.

Any contribution is evaluated against the decisions recorded here.

---

## Decision Governance

- Decisions are made in the **long-term** interest of the project.
- A decision may evolve, but **never implicitly**.
- Any challenge must go through a structured discussion.
- Systemic coherence takes precedence over opportunistic innovation.
- No Pull Request can alone modify a foundational decision.

---

## D-001 — The Project is a Platform, Not a Game

**Status**: Accepted  

### Decision

The project is an **open-source platform for simulating persistent sandbox worlds**, and **not**:

- a video game,
- a graphics engine,
- a narrative RPG,
- a gameplay framework,
- a technology showcase.

### Justification

The project's value lies in:

- the stability of the simulation core;
- the real persistence of the world;
- systemic coherence;
- maintainability over several years.

A "game"-oriented positioning imposes compromises incompatible with these objectives.

### Consequences

- The core provides no ready-made gameplay.
- Player experience is not a core objective.
- Clients are implementations, never architectural pillars.

---

## D-002 — Simulation Before Narration

**Status**: Accepted, scope amended by D-012

### Decision

**Systemic simulation** takes priority over any form of narration.

### Justification

Credible persistent worlds produce their own stories through:

- time;
- resources;
- entities;
- conflicts;
- interactions.

Imposed narration weakens the system's coherence and credibility.

### Consequences

- No scenario, quest, or narrative progression in the core.
- Any story is emergent.
- Systems always precede the narrative.

---

## D-003 — Autonomous World Not Centered on the Player

**Status**: Accepted, scope amended by D-012

### Decision

The world must be able to **exist, evolve, and persist without any player**.

### Justification

A credible world does not need human presence to function.

### Consequences

- The server runs without a connected client.
- The kernel does not embed a player category or human privilege.
- A world extension may explicitly define different rules for humans, players,
  NPCs, or any other application-defined role.

---

## D-004 — Authoritative Server and Real Persistence

**Status**: Accepted, distributed authority amended by D-012

### Decision

The server-side kernel is the sole authority on world state. D-012 replaces
the implied single process with an explicitly authorized node group.

### Justification

Coherence, security, and persistence require a single source of truth.

### Consequences

- No critical logic on the client side.
- Explicit persistence to disk.
- Traceable, inspectable, and replayable states.
- Solo mode = local server.
- Multiplayer mode = identical remote server.

---

## D-005 — Strict Decoupling Between Simulation and Rendering

**Status**: Accepted  

### Decision

The simulation core is **totally independent** of any rendering technology or client.

### Justification

Rendering is an interchangeable implementation.  
Simulation constitutes the project's durable foundation.

Linking the core to a graphics engine would compromise portability and longevity.

### Consequences

- No graphics engine on the server side.
- No rendering code in the core.
- The simulated world can be consumed by:
  - a real-time 3D client,
  - a 2D client,
  - a web client,
  - a headless client (CLI, tools, bots, visualization),
  - or any other consumer conforming to the APIs.
- The client never owns the world logic.

---

## D-006 — Reference Rendering Client and Official Graphics Standard

**Status**: Superseded by D-012

### Decision

The project provides a **reference rendering client**, based on **Unreal Engine**, serving as the **official graphics standard**, **without exclusivity**.

### Justification

A reference client is necessary to:

- demonstrate the platform's visual viability;
- define a common standard for assets and pipeline;
- guarantee minimal visual coherence.

However, no engine or client must become a structural dependency.

### Consequences

- Unreal Engine is a **reference implementation**, not a constraint.
- Other clients can exist freely:
  - Godot,
  - web clients,
  - specialized clients (administration, analysis, visualization),
  - future engines or technologies.
- All clients are **consumers of the simulated world**, never decision-makers.
- The graphics standard:
  - imposes no rules on simulation;
  - introduces no server-side dependencies;
  - can evolve independently of the core.

### Scope note

This decision targets the roadmap's client phase (Phase 4+).  
It is not part of the current minimal server scope (Phase 1).

This historical direction no longer commits Seej to an official rendering
client or graphics standard. A renderer, including an Unreal client, may be a
community project consuming the public contracts.

---

## D-007 — Pragmatic, Deterministic, and Explainable AI

**Status**: Accepted for kernel inputs, scope amended by D-012

### Decision

The kernel accepts only validated, ordered, persisted inputs. It does not ship
or prescribe an agent model.

### Justification

A persistent world must be:

- debuggable;
- reproducible;
- understandable.

Opaque or magical AI is incompatible with these requirements.

### Consequences

- Agents and generative systems are community extensions.
- External nondeterministic results become explicit validated inputs before
  they can affect canonical state.
- Kernel decisions remain replayable from retained state, inputs, and active
  contracts.

---

## D-008 — Minimal Core, Modular Extensions

**Status**: Accepted, extended by D-012

### Decision

The core remains **minimal, strict, and stable**.  
Any non-essential feature is implemented as an **optional module**.

### Justification

A core that is too rich becomes unstable, rigid, and costly to maintain.

### Consequences

- Documented and versioned public APIs before extension work is opened.
- Fundamental providers and application extensions are replaceable when they
  satisfy the kernel contracts.
- No module bypasses the core.

### Scope note

In Phase 1, `sy_api` is the active internal command/event/persistence contract.
The first stable contract must support trusted, operator-approved providers
loaded at startup. Hot replacement and isolated execution remain later work.

---

## D-009 — Stability and Maintainability Before Speed

**Status**: Accepted  

### Decision

Stability, readability, and maintainability take precedence over development speed.

### Justification

The project aims for **years of life**, not a quick demo.

### Consequences

- Refactorings accepted.
- Rushed features refused.
- Documentation considered a priority.

---

## D-010 — Rust for the Core

**Status**: Accepted  

### Decision

The **simulation core** is developed in **Rust**.

### Justification

Rust offers an optimal balance for a persistent platform project:

- **Memory safety**: strong compile-time guarantees for safe Rust, essential
  for long-term stability;
- **Performance**: native performance without compromising safety;
- **Concurrency**: a safe and powerful concurrency model for multi-entity simulations;
- **Maintainability**: a strong type system and mature ecosystem that support multi-year maintenance;
- **Interoperability**: ability to expose C-compatible APIs for clients in other languages;
- **Reliability**: safe Rust excludes broad classes of undefined behavior;
  `unsafe` code and FFI remain explicit review boundaries.

### Consequences

- The simulation core is written in Rust.
- Clients can be developed in any language compatible with the exposed APIs.
- Optional modules can be written in Rust or other languages depending on their nature.
- Safe Rust provides memory-safety guarantees without a tracing garbage
  collector; `unsafe` and FFI code require their own invariants and tests.
- The Rust ecosystem (crates) can be used for non-critical features.

---

## D-011 — Contract-First Kernel, Usage-Pulled Hardening

**Status**: Superseded by D-012

### Decision

Seej is positioned as a **world kernel**: the kernel owns the invariants
(time, causality, identity, conservation), and everything else — observation
tools, agents, netcode, renderers — is userland consuming a **frozen,
versioned contract**. Phase 1 closure is split into two gates:

- **Gate 1-K — Kernel Contract**: the non-negotiable bar. Frozen contract v0,
  causal-closure and determinism invariants, single-writer safety, an
  observation surface, an anchor world, and at least one external consumer.
- **Gate 1-D — Durability Under Load**: production hardening where every item
  carries an explicit usage trigger and is not worked on before its trigger
  exists.

Guarantees are tiered:

1. **Causal closure** — every state mutation enters through the single
   journaled aperture (commands in, events out). Non-negotiable.
2. **Deterministic re-execution** — the same binary re-applies the same
   journal to the same result; crash recovery *is* a replay. Non-negotiable.
3. **Long-horizon replay** — bit-perfect across binary versions, platforms,
   and years. An opt-in product guarantee, pulled by demand (e.g.
   reproducible agent research), not a default obligation.

The operational form of the invariant is: **we do not break replay.**

### Justification

- The kernel's causal model is only falsifiable through replay: "same genesis
  + same inputs ⇒ same world" is the one empirical test of unbroken
  causality. Replay is the measurement instrument of determinism, not a
  feature preference.
- A kernel becomes a platform the day someone else can build on it without
  reading its source. That requires a frozen contract and a consumable
  surface more than it requires exhaustive operational armor.
- Hardening ahead of usage optimizes blindly: real consumers re-prioritize
  durability work better than any checklist. Deferred items remain tracked
  with stable IDs and explicit triggers — deferral is scheduling, not
  deletion.
- Historical evidence (documented in the exit checklist discussion): neutral
  world kernels without consumers stagnate; ecosystems form around an
  observable anchor and a stable ABI.

### Consequences

- `seej/docs/CONTRACT.md` is the kernel contract; freezing it is gap K-01.
  After freeze, any change that alters how a persisted journal re-executes is
  an explicit `contract_version` / `simulation_contract` bump, never a silent
  edit.
- `seej/docs/phase1/EXIT_CHECKLIST.md` is restructured into Gate 1-K and
  Gate 1-D; all stable gap IDs are preserved.
- Consumer bricks land in order: observation (in Gate 1-K), then agents, then
  human netcode. Netcode inherits the kernel's tick model, not the reverse.
- The anchor world and observation surface are Gate 1-K deliverables, not
  demo optimizations; D-001 (platform, not a game) is unchanged — the anchor
  world is proof of autonomous systemic life, not default content.
- Floating-point canonical decisions remain scheduled for removal (P0-03):
  portable recovery re-applies the journal through a different binary, which
  makes float determinism a contract concern, not a Tier 3 luxury.

---

## D-012 — Distributed Extensible World Kernel

**Status**: Accepted, operational semantics clarified by D-013

### Decision

Seej is a **logical distributed world kernel**. One world runs on one or more
explicitly authorized nodes. The kernel coordinates identity, simulated time,
ordered command admission, validation, atomic in-world transitions, durable
history, and inter-node replication through frozen, versioned contracts.

`sy_core` remains a pure deterministic transition engine inside the kernel.
Each node runtime assembles it with replaceable providers. The project ships a
minimal working distribution of fundamental providers, while world models,
agents, economies, spatial systems, multiresolution engines, interfaces,
renderers, and ingame netcode are community capabilities.

The network between kernel nodes is foundational. Ingame networking for
players, prediction, interpolation, and client synchronization is a later
plugin concern.

### Distributed authority

- There is no mandatory global coordinator or registry.
- Each world has an explicit authority group. Discovery never grants
  membership or write authority.
- The first reference provider targets three authorized nodes, quorum two, and
  a Raft-style consensus model for crash faults and network partitions. It is
  not a Byzantine or permissionless consensus promise.
- A world has one confirmed history. A replica does not advance canonical
  ticks independently. Loss of quorum suspends mutation rather than creating a
  divergent branch.
- A temporary coordinator is fenced by an authority term. Membership uses a
  separate confirmed configuration version; changes and rejoining-node catch-up
  are explicit transitions.
- A local WAL append is not distributed confirmation. Success and public
  canonical effects require the configured durable group confirmation.
- A failing local store stops or quarantines unsafe activity on that node.
  Other replicas may continue only while the world still has quorum.

### Identity, time, and compatibility

- `world_id`, node identity, seed, and genesis hash are distinct. Replication
  preserves `world_id`; an independent clone receives a new identity.
- Ticks and command ordering are canonical per world. There is no global
  simulation clock across worlds or nodes.
- A compatible authority group runs one declared execution profile:
  simulation contract, command and event schemas, provider contracts, and
  required plugin versions. Incompatible nodes are refused.
- Stable kernel-owned data maintains a migration path from every stable
  version, possibly through ordered intermediate steps. Activation is
  coordinated at a world transition boundary.
- Plugins own their data migrations. An incompatible or missing required
  plugin blocks migration or re-simulation.
- The operator retains historical plugin artifacts. Seej records exact
  references and cryptographic fingerprints and refuses re-simulation when
  required components are unavailable.

### Extensibility and world freedom

- Even storage, tick scheduling, validation, replication, and membership are
  provider capabilities behind mandatory contracts. The bundled providers are
  replaceable, but their required guarantees are not.
- The first stable contract loads only trusted providers explicitly approved
  by the operator at startup. Hot loading and isolated execution arrive later.
  Later application plugins are isolated by default with explicit permissions;
  native execution is an operator-approved exception.
- A data domain has one declared logical owner. Other extensions change it
  through public commands.
- A transaction may update multiple owners atomically inside one world:
  validation and durable effects all succeed, or none do. No automatic
  cross-world or external-system atomicity is promised.
- Opaque plugin payloads are allowed. Their envelope identifies the owner,
  format, and version. Seej preserves integrity; interpretation can require
  the plugin or a dedicated tool.
- Removing or replacing a plugin with active data or obligations requires an
  explicit transfer, migration, or resolution. A controlled pause between
  transitions is allowed without stopping the server process.
- The kernel does not require space, zones, creatures, resources, economics,
  agents, or human privileges. A world declares its application laws and
  validators; the kernel enforces those declarations.

### Commands, observation, and retention

- Commands carry a durable identifier, payload fingerprint, target tick, and
  canonical per-world order. A late command is refused rather than silently
  retargeted.
- Retrying the same identifier and payload returns its durable status without
  applying effects twice. Reusing the identifier with different content is a
  conflict.
- A response timeout does not prove refusal. The producer queries a durable
  status that may be pending, succeeded, refused, or unavailable while the
  authority group cannot answer safely.
- A declared status-retention window may expire. After it does, the identifier
  remains in an expired namespace and is refused forever rather than executed
  as new.
- Observation reads a confirmed prefix and reports freshness. It never changes
  simulation rules, fidelity, priority, or history. Readers may be limited or
  refused under load.
- An expired observation cursor produces an explicit expiration response and
  a coherent state with a new cursor. Deleted history is not reconstructed.
- History retention may be bounded. The current state, active obligations,
  command-identity safety, and recovery points remain intact; full archival is
  optional.

### Simulation policies

- A world can declare a real-time cadence and explicit degradation or catch-up
  policy. That policy cannot bypass durability, quorum, or observation
  neutrality.
- A world can declare protected established facts and permitted
  approximations. A community multiresolution engine may produce a different
  future from a fully detailed simulation while preserving those facts and
  laws.
- External nondeterminism, including model output and wall-clock observations,
  affects canonical state only through validated persisted inputs.

### First stable contract

The first stable contract requires:

- generic extension-owned data and trusted startup providers;
- fail-closed behavior for common storage failures;
- durable command identity, status, ordering, and retention semantics;
- causal linkage between resolutions, events, and before/after state;
- three-node distributed evidence with quorum, partition, fencing, catch-up,
  and compatible migration scenarios;
- a read-only confirmed observation surface;
- tested cold backup/restore and a baseline recovery procedure;
- a real third-party extension that declares data and uses public commands
  without internal kernel dependencies.

The current single-process prototype is useful evidence but does not satisfy
these requirements yet. D-012 reopens the foundation roadmap and replaces
D-011's rule that all protocol work waits until a later phase.

### Supersession map

| Earlier decision | Effect of D-012 |
| --- | --- |
| D-001 | Preserved: Seej remains infrastructure, not a game. |
| D-002 | Narrowed: the kernel prescribes no narration; extensions remain free. |
| D-003 | Narrowed: the kernel has no built-in player privilege; worlds may declare one. |
| D-004 | Extended from one authoritative server to one authoritative node group per world. |
| D-005 | Preserved: rendering remains outside the kernel. |
| D-006 | Superseded: no official Unreal client or graphics standard is promised. |
| D-007 | Narrowed to validated and replayable kernel inputs; Seej ships no official agents. |
| D-008 | Extended to fundamental replaceable providers and explicit lifecycle rules. |
| D-009 | Preserved. |
| D-010 | Preserved, with precise safe-Rust and `unsafe` boundaries. |
| D-011 | Superseded: contract-first remains; distributed networking and baseline durability move into the stable gate. |

---

## D-013 — Operational Failure Semantics

**Status**: Accepted

### Context

A consistency review of the D-012 documentation found that the product
direction was aligned but several failure boundaries still allowed incompatible
implementations. In particular, admission and application had no complete crash
state machine, compacted command IDs had no durable non-reuse mechanism,
cross-owner atomicity did not define candidate validation, membership changes
did not guarantee quorum intersection, and migration rollback was ambiguous
after activation.

### Decision

The first stable contract uses these operational semantics:

- A command has a kernel-issued identity namespace. A confirmed Admission
  creates durable `Pending` state without application effects. A confirmed
  Resolution atomically records `Succeeded` or `Refused`, events, and the state
  transition. Recovery resumes pending work and never reapplies a resolution.
- Status retention is measured in canonical ticks. Pending commands never
  expire. Confirmed closure of kernel-issued generations creates durable
  non-reusable expired namespaces without retaining every terminal result.
- A cross-owner transaction binds to a confirmed base cursor. Ordered owner
  proposals build a private candidate state and cross-domain validators inspect
  the complete result. A stale candidate is discarded and recomputed in full.
  Component failure is not fabricated as a world-law refusal.
- Active obligations leave an owner only through confirmed transfer to an
  active compatible successor or confirmed resolution. Archival alone is valid
  for terminal obligations, or for an entire world that no longer advances.
- The reference consensus provider separates authority terms from membership
  configuration versions. New voters first catch up as learners; configuration
  changes preserve overlapping old/new quorums. A confirmed-history conflict is
  quarantined, while an explicitly unconfirmed local suffix may be reconciled.
- A current-confirmed read requires a quorum authority barrier in the active
  term and configuration, plus local application through its cursor. A
  resynchronization state and cursor describe one retained confirmed cut.
- Migration preparation and activation are separate. Preparation may abort
  under the old profile. Once new profile and migrated state are confirmed
  together, recovery continues from the new state; return requires another
  explicit migration.
- Minimum causal linkage, cold backup/restore evidence, and a baseline recovery
  procedure are first-stable requirements. Rich diagnosis, online backup, and
  load hardening remain usage-triggered work.
- Hot loading and isolated plugin execution are later Seej kernel capabilities.
  Agents, economies, spatial models, rendering, multiresolution, and ingame
  netcode remain optional community capabilities, not official delivery
  promises.

These statements are target contracts. D-013 closes no implementation gap.

### Decision coverage

| Decision | Normative home | Release evidence |
| --- | --- | --- |
| D-001 — platform, not game | Kernel boundary and roadmap ecosystem section | K-01 contract review |
| D-002 — simulation before narration | World laws and plugin contract | K-06 extension fixture |
| D-003 — autonomous world | Headless kernel and observer-neutral world model | K-03 deterministic fixtures |
| D-004 — authority and persistence | Node-network confirmed history | K-07 distributed proof |
| D-005 — rendering separation | Kernel/plugin boundary | K-01 and dependency checks |
| D-006 — old official renderer direction | Superseded by D-012 optional ecosystem | No first-stable delivery gap |
| D-007 — deterministic external inputs | Deterministic transition and validation contracts | P0-03, P0-15 |
| D-008 — minimal extensible core | Provider/plugin contract | K-04, K-05, K-06 |
| D-009 — stability before speed | Exit-checklist evidence policy | Every closed gap names evidence |
| D-010 — Rust core | Dependency rules and pure-engine boundary | P0-01 purity gate |
| D-011 — contract-first kernel | Superseded scheduling retained in D-012 | K-01 and Gate 1-K |
| D-012 — distributed extensible kernel | Root, node, plugin, and world contracts | K-01 through K-10 |
| D-013 — operational failure semantics | Command, transaction, network, migration, and retention clauses | K-02, K-06 through K-10, P1-20, P2-02, P2-03 |

### Audit-correction traceability

| Finding corrected | Binding contract | Required evidence |
| --- | --- | --- |
| Admission and resolution crash gap | `CONTRACT.md` command lifecycle; `DATA_FLOW.md` | K-08 crash-boundary fixtures |
| Rollback after confirmed migration | `NODE_NETWORK.md` compatibility/migration | K-09 pre/post-activation fixtures |
| Incomplete cross-owner validation | `PLUGIN_CONTRACT.md` transactions | K-06 candidate/invariant fixtures |
| Expired command-ID reuse | `CONTRACT.md` retention; `PERSISTENCE.md` compaction | K-08 and K-10 generation fixtures |
| Unsafe membership transition ambiguity | `NODE_NETWORK.md` reference membership transition | K-07 joint-configuration fixtures |
| Unproved current-read freshness | `NODE_NETWORK.md` reads; `OBSERVATION_SLICE.md` | K-02 and K-07 barrier fixtures |
| Archiving active obligations | `PLUGIN_CONTRACT.md` lifecycle; `WORLD_SPEC.md` obligations | K-06 removal fixture |
| First-stable requirements placed in Gate 1-D | `EXIT_CHECKLIST.md` Gate 1-K | P1-20, P2-02, P2-03 |
| Official application capabilities implied | `ROADMAP.md` optional ecosystem; Phase 1 scope | Documentation status review |
| Lost evidence traceability | `EXIT_CHECKLIST.md` implemented evidence table | Named paths and reproduction commands |

The [Phase 1 exit checklist](../seej/docs/phase1/EXIT_CHECKLIST.md) remains the
authority for implementation status. The [root kernel contract](../seej/docs/CONTRACT.md),
[node-network contract](../seej/docs/NODE_NETWORK.md), and
[plugin contract](../seej/docs/PLUGIN_CONTRACT.md) define the referenced target
behavior.

### Consequences

- Implementations must model intermediate durable states rather than infer them
  from RPC success or local storage.
- Provider replaceability is tested against observable failure semantics,
  including membership, reads, recovery, and compaction.
- No checklist item is complete because D-013 exists; executable evidence is
  still required before contract freeze.

---

## Amendment Rules

- Any major decision must be added to this document.
- An existing decision can only be modified with:
  - explicit justification;
  - impact analysis;
  - maintainer validation.
- Foundational decisions can only be canceled collectively.

---

End of document.
