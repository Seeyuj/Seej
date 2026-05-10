# Architecture Overview

This document describes the high-level architecture of the Seej platform.

## Layer Structure

The codebase is organized into dependency layers (NIV 0 to NIV 4):

- **NIV 0**: Stable primitives (`sy_types`, `sy_config`)
- **NIV 1**: active API definitions (`sy_api`) and Phase 2+ protocol placeholder (`sy_protocol`)
- **NIV 2**: Pure simulation core (`sy_core`)
- **NIV 2b**: Extension modules (`mods/`, Phase 2+, outside the active workspace)
- **NIV 3**: Infrastructure implementations (`sy_infra`, `sy_tools`, `sy_testkit`)
- **NIV 4**: Wiring and dependency injection (`sy_loader`; Phase 1 placeholder)

## Key Principles

1. **Determinism**: The simulation core must be fully deterministic
2. **Dependency Inversion**: Core depends on abstractions (ports), not implementations
3. **Snapshot + WAL Recovery**: committed state changes are persisted as events, while snapshots bound recovery cost
4. **Testability**: Every component can be tested in isolation with mocks

## Foundational Target Contracts

The following documents define future architecture contracts needed to close
known Phase 1 hardening gaps. They are not implemented feature claims and should
not be treated as completed work:

- [World Specification Contract](simulation/WORLD_SPEC.md): defines the target
  `WorldSpec` contract for genesis, world identity, simulation rules, ontology,
  module contracts, limits, persistence compatibility, and replay semantics.
- [Causal Resolution Policy](simulation/CAUSAL_RESOLUTION.md): defines the
  target policy for deterministic multi-resolution simulation so persistent
  worlds can preserve causal reality without simulating everything at full
  detail all the time.

## Persistence Ports

Persistence ports (`IEventLog`, `IWorldStore`) live in `sy_api`. They are abstract contracts shared by infrastructure implementers and runtime consumers without coupling `sy_core` to filesystem I/O, WAL storage, or other infrastructure. `sy_core` remains pure simulation logic; `sy_infra` owns concrete persistence, recovery, and storage behavior.

`sy_protocol` and `mods/*` are preserved for Phase 2+ work and are outside the active Phase 1 workspace build.

## Diagram

```
┌─────────────────────────────────────────────────────────┐
│              sy_loader (NIV 4, Phase 1 placeholder)     │
├─────────────────────────────────────────────────────────┤
│  sy_infra (NIV 3)  │  sy_tools (NIV 3)  │ sy_testkit   │
├─────────────────────────────────────────────────────────┤
│              sy_core (NIV 2)  │  mods (NIV 2b)         │
├─────────────────────────────────────────────────────────┤
│         sy_protocol (NIV 1)  │  sy_api (NIV 1)         │
├─────────────────────────────────────────────────────────┤
│           sy_types (NIV 0)   │  sy_config (NIV 0)      │
└─────────────────────────────────────────────────────────┘
```
