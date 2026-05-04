# Architecture

## Navigation

- [`README.md`](../README.md)
- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`DECISIONS.md`](DECISIONS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)

## Document Purpose

This document describes the fundamental architecture of the persistent world kernel.

It defines:
- the responsibilities of each system layer;
- non-negotiable architectural boundaries;
- technical constraints imposed on contributions;
- structural choices ensuring coherence, persistence, and long-term maintainability.

Implementation references for current scope are documented in:
- [`../seej/docs/phase1/README.md`](../seej/docs/phase1/README.md)
- [`../seej/docs/phase1/DETERMINISM.md`](../seej/docs/phase1/DETERMINISM.md)
- [`../seej/docs/phase1/PERSISTENCE.md`](../seej/docs/phase1/PERSISTENCE.md)
- [`../seej/docs/phase1/BINARIES.md`](../seej/docs/phase1/BINARIES.md)

This document is authoritative.  
Any proposal or contribution incompatible with this architecture is rejected.

---

## Overview

The platform is designed as a **simulation infrastructure**, not as a game.

It is based on a strict separation between:

1. the server simulation core  
2. optional modules and extensions  
3. clients (rendering, UI, interaction)  
4. operational and observability tools  

The server is always the sole authority.  
The world exists and evolves independently of any client or player presence.

---

## Fundamental Principle: Authoritative Server

### Absolute Rule

The server is the single source of truth for the world state.

This implies:

- no simulation decisions on the client side;
- no critical logic executed outside the server;
- no implicit or recomputed persistence;
- no server dependency on a graphics engine.

Solo mode = local server  
Multiplayer mode = remote server  

Same architecture.  
Same rules.  
No exceptions.

---

## Layered Architecture

### 1. Simulation Core

The core is intentionally:

- minimal;
- deterministic;
- stable;
- decoupled;
- maintainable over several years.

It contains **only what is strictly necessary** for the persistent simulation of an autonomous world.

#### Core Responsibilities

- simulated time management;
- space representation (regions, chunks, topology);
- persistent entities (agents, objects, structures);
- rule systems (economy, needs, production, conflicts, flows);
- systemic events;
- explicit persistence to disk;
- recovery after shutdown or crash;
- public, documented, and versioned APIs.

#### What the Core Never Does

- graphical rendering;
- user interface;
- "fun"-oriented gameplay;
- narration, quests, or scripted scenarios;
- player-specific or privileged logic;
- game balance;
- central generative AI.

If a feature is not essential to the **autonomous and persistent simulation of the world**, it has no place in the core.

---

### 2. Modules and Extensions

Non-essential features are implemented as **optional modules**.

#### Module Properties

A module:

- uses only the core's public APIs;
- never bypasses the core;
- can be enabled or disabled without compromising the world;
- is versioned independently;
- can be replaced by another implementation.

Possible module examples:

- alternative economic systems;
- social or political rules;
- advanced behavioral AI;
- analysis or replay tools;
- external integrations.

A module must **never**:

- modify the core implicitly;
- introduce a graphics dependency on the server side;
- break the core's determinism.

---

### 3. Clients (World Consumers)

Clients are **consumers of the world state**, never decision-makers.

#### Client Role

- display the world state;
- enable user interaction;
- transmit intentions to the server;
- provide visual or textual rendering.

#### Strict Decoupling

The core:

- knows no graphics engine;
- imports no client dependencies;
- can run entirely in headless mode.

An official client based on Unreal Engine may exist as a reference implementation (Phase 4+), but:

- it has no privilege;
- it is interchangeable;
- it dictates no simulation rules.

Other clients can coexist:
- Godot
- Web
- CLI
- specialized visualization tools

---

### 4. Operational Tools

Tools are not part of the core, but are essential to the project's viability.

They may include:

- server administration;
- world state inspection;
- system visualization;
- metrics and profiling;
- persistence validation;
- deterministic replay.

They respect the same constraints:

- no authority over the world;
- no hidden critical logic;
- interaction via controlled APIs.

---

## Simulation and Determinism

### Mandatory Determinism

Given equal inputs, the core must produce:

- the same decisions;
- the same state transitions;
- the same results.

Determinism is a functional requirement, not an optimization.

Any source of non-determinism must be:

- explicitly isolated;
- documented;
- optional;
- never central.

---

## Persistence

### Real, Explicit, and Traceable Persistence

Persistence:

- is explicitly written to disk;
- survives restarts and crashes;
- does not depend on implicit states;
- can be inspected or replayed.

The world never disappears when the server stops.

Any logic relying on:

- volatile memory;
- implicit recomputations;
- unsaved temporary states;

is forbidden in the core.

---

## Scalability and Spatial Partitioning

The world is simulated by:

- regions;
- chunks;
- simulation detail levels.

The core must be able to:

- partially simulate the world;
- load and unload zones;
- adapt computational cost;
- function without connected clients.

Scalability is a **structural property**, not a late optimization.

---

## Non-Negotiable Technical Constraints

- mandatory headless server;
- no server graphics dependencies;
- stable and versioned public APIs;
- strict core / modules separation;
- readability and maintainability prioritized;
- rejection of opaque or "magical" abstractions;
- rejection of unjustified closed dependencies.

---

## Long-Term Philosophy

This architecture is designed to:

- last for years;
- support very different worlds;
- survive technological evolution;
- enable clear governance;
- avoid complexity explosion.

The platform always takes precedence over content.  
Coherence always takes precedence over speed.  
The world always takes precedence over the player.

---

## Final Rule

Any proposal is evaluated according to a single question:

**Is this feature necessary for the operation of an autonomous, persistent, and coherent world?**

If the answer is no, it does not belong in this architecture.
