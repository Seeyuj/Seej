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

**Status**: Accepted  

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

**Status**: Accepted  

### Decision

The world must be able to **exist, evolve, and persist without any player**.

### Justification

A credible world does not need human presence to function.

### Consequences

- The server runs without a connected client.
- The player has no special status.
- Players and NPCs are subject to the same systemic rules.

---

## D-004 — Authoritative Server and Real Persistence

**Status**: Accepted  

### Decision

The server is the sole authority on the world state.

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

**Status**: Accepted  

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

> Unreal Engine is not the project.  
> It is an official client among others, replaceable.

---

## D-007 — Pragmatic, Deterministic, and Explainable AI

**Status**: Accepted  

### Decision

Entities are **deterministic agents**, explainable and observable.

### Justification

A persistent world must be:

- debuggable;
- reproducible;
- understandable.

Opaque or magical AI is incompatible with these requirements.

### Consequences

- No conscious or fantasized autonomous AI.
- Generative AI allowed only on the periphery.
- Decisions must be traceable and justifiable.

---

## D-008 — Minimal Core, Modular Extensions

**Status**: Accepted  

### Decision

The core remains **minimal, strict, and stable**.  
Any non-essential feature is implemented as an **optional module**.

### Justification

A core that is too rich becomes unstable, rigid, and costly to maintain.

### Consequences

- Documented and versioned public APIs before extension work is opened.
- Modules that can be activated, deactivated, or replaced.
- No module bypasses the core.

### Scope note

In Phase 1, `sy_api` is the active internal command/event/persistence contract.
The stabilized public extension surface and module-loading contract are Phase 2
work.

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

- **Memory safety**: compile-time guarantees, essential for long-term stability;
- **Performance**: native performance without compromising safety;
- **Concurrency**: a safe and powerful concurrency model for multi-entity simulations;
- **Maintainability**: a strong type system and mature ecosystem that support multi-year maintenance;
- **Interoperability**: ability to expose C-compatible APIs for clients in other languages;
- **Reliability**: absence of undefined behavior, crucial for persistence and reproducibility.

### Consequences

- The simulation core is written in Rust.
- Clients can be developed in any language compatible with the exposed APIs.
- Optional modules can be written in Rust or other languages depending on their nature.
- Rust compilation guarantees memory safety with no runtime overhead.
- The Rust ecosystem (crates) can be used for non-critical features.

---

## D-011 — Contract-First Kernel, Usage-Pulled Hardening

**Status**: Accepted

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

## Amendment Rules

- Any major decision must be added to this document.
- An existing decision can only be modified with:
  - explicit justification;
  - impact analysis;
  - maintainer validation.
- Foundational decisions can only be canceled collectively.

---

End of document.
