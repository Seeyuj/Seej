# SeeYuj

> Deterministic, headless simulation kernel for persistent sandbox worlds.

SeeYuj is open-source simulation infrastructure for worlds that evolve without players, clients, or rendering engines.
It focuses on deterministic server execution, explicit persistence, and replayable state transitions.
Clients are rendering-agnostic consumers: they observe and interact through APIs, but the server remains authoritative.
In Phase 1, this means: create a world, run ticks, persist state, kill the process, restart it, replay events, and verify identical state.

- GitHub: [https://github.com/Seeyuj/Seeyuj](https://github.com/Seeyuj/Seeyuj)
- Follow: [https://x.com/SeeYuj](https://x.com/SeeYuj)

## What Works Today (Phase 1)

Phase 1 core capabilities are implemented, but the phase is not closed yet and is under active hardening.

Implemented and testable today:

- Deterministic tick loop with mandatory world seed
- Headless authoritative daemon (`server_d`)
- Snapshot + WAL persistence (`snapshot.json`, `meta.json`, `events`)
- Crash recovery by replaying WAL events after snapshot cursor
- Minimal autonomous simulation (zones, entities, systemic degradation rules)
- Inspection CLI (`sy_cli`) for status, events, entities, zones, and JSON dumps
- Determinism validation utilities and tests in `sy_core`

Implemented but being hardened:

- Longer burn-in scenarios under operational conditions
- Expanded reproducibility and failure-mode coverage
- Explicit CI gating for full Phase 1 closure criteria

Reference docs:

- [Phase 1 Scope](seeyuj/docs/phase1/README.md)
- [Determinism Contract](seeyuj/docs/phase1/DETERMINISM.md)
- [Persistence and Recovery](seeyuj/docs/phase1/PERSISTENCE.md)
- [Binary Usage](seeyuj/docs/phase1/BINARIES.md)

## Architecture

SeeYuj follows strict dependency layering (NIV 0 to NIV 4):

- **NIV 0**: stable primitives (`sy_types`, `sy_config`)
- **NIV 1**: protocol/API definitions (`sy_protocol`, `sy_api`)
- **NIV 2**: pure simulation core (`sy_core`)
- **NIV 2b**: optional simulation modules (`mods/*`)
- **NIV 3**: infrastructure, tooling, testkit (`sy_infra`, `sy_tools`, `sy_testkit`)
- **NIV 4**: runtime wiring (`sy_loader`)

Core principles:

- **Headless first**: no graphics dependency on the server
- **Determinism first**: same genesis + same input stream = same state transitions
- **Event-sourced persistence**: mutations are persisted and replayable
- **Strict decoupling**: simulation logic is independent from rendering technologies

Documentation structure:

- Repository-level docs in [`doc/`](doc/) define project governance and architecture
- Implementation-level docs in [`seeyuj/docs/phase1/`](seeyuj/docs/phase1/) define current Phase 1 scope and behavior

See [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md), [seeyuj/docs/ARCHITECTURE.md](seeyuj/docs/ARCHITECTURE.md), and [seeyuj/docs/phase1/README.md](seeyuj/docs/phase1/README.md).

## Quick Start

Prerequisite: Rust toolchain installed.

```bash
git clone https://github.com/Seeyuj/Seeyuj.git
cd Seeyuj/seeyuj/server

cargo build --workspace
cargo test --workspace

cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

## What SeeYuj Is Not

SeeYuj is not:

- a game engine
- a graphics engine
- a narrative RPG framework
- a metaverse platform claim
- a default-content game project

It is a persistent world kernel: server simulation infrastructure that others can build worlds on top of.

## Current Status

- **Phase 0** (conceptual foundations): complete
- **Phase 1** (minimal headless core): core capabilities implemented, phase not closed
- **Phase 2+** (`sy_protocol`, optional modules, advanced client/network concerns): intentionally deferred

Roadmap: [doc/ROADMAP.md](doc/ROADMAP.md)

## Contributing

If you want to help build robust simulation infrastructure:

1. Read [CONTRIBUTING.md](CONTRIBUTING.md)
2. Check open [Issues](https://github.com/Seeyuj/Seeyuj/issues)
3. Open a PR aligned with architecture and decision documents

Key references:

- [doc/DECISIONS.md](doc/DECISIONS.md)
- [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md)
- [seeyuj/docs/phase1/README.md](seeyuj/docs/phase1/README.md)
