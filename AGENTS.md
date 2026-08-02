# AGENTS.md — Seej Agent Operating Contract

This file defines how AI coding agents must work on Seej.

It is not a style guide.
It is not a suggestion.
It is the operating contract for agents modifying this repository.

## Project Identity

Seej is an open-source infrastructure project for deterministic, persistent sandbox worlds.

Seej is not:

- a video game;
- a graphics engine;
- a narrative RPG framework;
- a content project;
- a metaverse marketing project;
- a gameplay prototype.

Seej is a long-term, server-authoritative simulation platform.

Its value is:

- deterministic simulation;
- explicit persistence;
- reproducible state transitions;
- authoritative server execution;
- modular extensibility;
- clean architecture;
- observability;
- recoverability;
- long-term maintainability.

The platform comes before content.
Simulation comes before narration.
Persistence comes before convenience.
Architecture comes before feature velocity.

## Agent Mission

Your role is to strengthen Seej as durable infrastructure for autonomous persistent worlds.

Before every change, ask:

> Does this make Seej more reliable, deterministic, persistent, maintainable, or architecturally coherent?

If the answer is no, the change is probably out of scope.

Do not optimize for impressive demos.
Do not optimize for short-term gameplay.
Do not optimize for visual appeal.
Do not optimize for speculative future features.

Optimize for the foundation.

## Current Direction (Decision D-011)

Seej is a **world kernel**: the kernel owns time, causality, identity, and
conservation; everything else (observers, agents, netcode, renderers) is
userland consuming a frozen, versioned contract.

- The kernel contract lives in `seej/docs/CONTRACT.md`. Its change policy is:
  **we do not break replay.** Any change that alters how a persisted journal
  re-executes is an explicit contract bump, never a silent edit.
- Phase 1 closes through two gates in `seej/docs/phase1/EXIT_CHECKLIST.md`:
  Gate 1-K (kernel contract — work on this) and Gate 1-D (durability under
  load — every item has a usage trigger; do not work on an item before its
  trigger exists).
- The observation surface and the anchor world (gaps K-02, K-03) are Gate 1-K
  deliverables. They are contract evidence, not "impressive demos": a world
  observably evolving through a read-only stream is the proof of the
  autonomous-persistence claim.
- Consumer bricks land in order: observation → agents → human netcode. Do not
  pull netcode or protocol work forward.

## Priority Order

When tradeoffs appear, use this order:

1. Architectural durability
2. Correctness
3. Determinism
4. Persistence and recovery
5. Maintainability
6. Testability
7. Observability
8. Performance
9. Implementation speed

Never sacrifice determinism, persistence, or architectural boundaries for speed.

## Non-Negotiable Principles

### 1. Server Authority

The server is the only source of truth for world state.

Clients may:

- observe state;
- send intentions;
- request information;
- render results.

Clients must never:

- decide persistent world state;
- bypass validation;
- run critical simulation logic;
- own authority;
- be required for world evolution.

Solo mode means local server.
Multiplayer means remote server.
Same architecture. Same rules.

### 2. Headless Core

The simulation server must run without:

- UI;
- rendering engine;
- graphical dependency;
- connected client;
- asset pipeline;
- Unreal;
- Godot;
- Web client;
- editor tooling.

Rendering is a consumer of the world, not the owner of the world.

### 3. Determinism

Given the same genesis, same input stream, same tick schedule, and same ordering, the simulation must produce the same state transitions.

Determinism is a functional requirement.

Core logic must not depend on:

- wall-clock time;
- local machine time;
- non-injected randomness;
- filesystem ordering;
- thread scheduling;
- nondeterministic collection iteration;
- network timing;
- client timing.

Every state transition must be reproducible from:

```text
world state + input + tick
```

Core code must keep transition inputs explicit and ordered. If a change cannot
be replayed deterministically from those inputs, move it out of the core or
redesign it before implementation.

### 4. Persistence and Recovery

- Every accepted mutation is journaled (WAL) **before** in-memory state is accepted.
- The world must survive `kill -9`: recovery is a replay (snapshot + WAL after
  the cursor), never a reconstruction from memory.
- No canonical state lives only in memory, logs, or transient caches.
- Any change that alters how a persisted journal re-executes is an explicit
  contract bump (`seej/docs/CONTRACT.md`), never a silent edit.

### 5. Architectural Boundaries

- The NIV 0→4 layering is mandatory; allowed edges live in
  `seej/docs/DEPENDENCY_RULES.md`.
- `sy_core` depends only on `sy_types` and `sy_api`; the purity gate
  (`seej/server/scripts/check_sy_core_purity.py`, CI job `sy_core purity`)
  must stay green.
- Phase 2+ crates (`sy_protocol`, `mods/*`) stay outside the active workspace.
- Real I/O lives in `sy_infra`; in Phase 1, `server_d` wires the runtime.
