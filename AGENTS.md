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
