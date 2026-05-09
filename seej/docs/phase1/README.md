# Phase 1 — Implemented Scope and Remaining Hardening (Authoritative)

This folder documents **exactly what is implemented** for Phase 1 in this
repository, what is explicitly out of scope, and what still blocks durable Phase
1 sign-off.

## Goals (Phase 1)

- **Headless simulation** (no UI, no client, no networking).
- **Determinism**: same genesis + same input stream + same scheduling policy ⇒ same outputs/state.
- **Persistence**: snapshot + WAL (write-ahead log).
- **Crash recovery**: load snapshot then replay WAL events after the snapshot cursor.
- **Strict layering**:
  - `sy_core` must stay pure (no I/O, no system time, no OS RNG).
  - real I/O lives in `sy_infra`.

## Non-goals (Phase 1)

- Any network protocol / wire format (`sy_protocol` is Phase 2+ and is not part of the Phase 1 workspace build).
- Gameplay / modules such as economics/physics (`mods/*` are Phase 2+ and are not part of the Phase 1 workspace build).
- Client, rendering, player logic, multiplayer.

## What exists and is working (Phase 1)

### Binaries

- `server_d` — headless daemon:
  - create a world (seed is **required**),
  - run ticks,
  - auto-save,
  - graceful shutdown (save),
  - load with crash recovery (WAL replay).
- `sy_cli` — admin inspection:
  - status, dump snapshot JSON, list events, inspect entities/zones.

See `BINARIES.md` for exact commands.

### Core simulation (`sy_core`)

- **Command → events → apply** pipeline.
- A deterministic **event replay** function (`apply_event`) used by crash recovery.
- Determinism validation utilities and tests (canonical state hashing + “run twice, compare hashes”).
- **Simulated time**: `Tick` + `SimTime` stored in world state and advanced by the simulation.
- **Minimal world model**:
  - Zones with stable IDs (`ZoneId`) and persistent storage.
  - Entities with stable identity, state, and lifecycle (no player entities).
- **Minimal systemic rules** (Phase 1 scope only):
  - Resource degradation (low-probability decay per tick).
  - Creature health degradation (low-probability decay per tick).
  - Periodic cleanup of dead entities.

See `DETERMINISM.md`.

### Persistence (`sy_infra`)

- Snapshot store (`FilesystemStore`) that writes:
  - `meta.json`
  - `snapshot.json`
  with an atomic write strategy (tmp + fsync + rename, best-effort durability).
- WAL (`FileEventLog`) with:
  - monotonic `EventId`,
  - binary records with CRC32,
  - recovery that stops on the first invalid/partial record (optionally truncates tail).

See `PERSISTENCE.md` for the exact record layout and recovery algorithm.

## Exit checklist

Phase 1 closure is tracked in `EXIT_CHECKLIST.md`. The default CI gate covers
fast checks; scheduled/manual gates cover ignored tests, forced-kill recovery,
long burn-in, coverage, and WAL fuzz smoke.

The same checklist also tracks open engineering gaps that are not closed by the
current tests: simulation-contract versioning, formal genesis, stable persisted
DTOs, canonical command ordering, command journaling, world integrity
validation, single-writer ownership, WAL/world contract binding, corruption
policy, replay oracle tooling, crashpoint injection, deterministic flight
recording, read-only world diagnostics, divergence bisection, compaction, and
operator recovery procedures. Those gaps must be closed before treating Phase 1
as durable infrastructure rather than an implemented minimal recovery slice.

### Observability (minimal)

- Structured logs via `tracing`.
- CLI inspection for world status, events, entities, and zones.

## Workspace scope for Phase 1 (minimal build)

The Phase 1 workspace members are defined in `seej/server/Cargo.toml`.
Phase 2+ crates/modules are kept in the repository to preserve the architecture, but are **commented out** from the workspace members and (when applicable) from dependencies.

Active workspace members:

- `sy_types`, `sy_config`, `sy_api`, `sy_core`, `sy_infra`, `sy_loader`, `sy_tools`, `sy_testkit`
- `server_d`, `sy_cli`

Present but deferred:

- `sy_protocol`
- `mods/mod_economics`
- `mods/mod_physics`

## Determinism invariants (Phase 1)

- **Seed is mandatory** when creating a world (`server_d create --seed <u64>`).
- Core time is **simulated** (`Tick`, `SimTime`); core code must not read OS time.
- Collections that influence canonical hashing / iteration are ordered (`BTreeMap` in the world state).
- Persistence cursors use `EventId` (not “last saved time”) to avoid ambiguity at a snapshot boundary.

