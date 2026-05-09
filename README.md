# Seej

> Deterministic, headless simulation kernel for persistent sandbox worlds.

Seej is open-source simulation infrastructure for worlds that evolve without players, clients, or rendering engines.
It focuses on deterministic server execution, explicit persistence, and replayable state transitions.
Clients are rendering-agnostic consumers: they observe and interact through APIs, but the server remains authoritative.
In Phase 1, this means: create a world, run ticks, persist state, kill the process, restart it, replay events, and verify identical state.

- GitHub: [https://github.com/Seej/Seej](https://github.com/Seej/Seej)
- Follow: [https://x.com/Seeyuj](https://x.com/Seej)

## What Works Today (Phase 1)

The current repository implements the minimal Phase 1 server slice: a headless,
deterministic world can be created, ticked, persisted, recovered, and inspected.
This is not yet a durable Phase 1 sign-off; remaining hardening work is tracked
in the exit checklist.

Implemented and testable today:

- Deterministic tick loop with mandatory world seed
- Headless authoritative daemon (`server_d`)
- Snapshot + WAL persistence (`snapshot.json`, `meta.json`, `events`)
- Crash recovery by replaying WAL events after snapshot cursor
- Minimal autonomous simulation (zones, entities, systemic degradation rules)
- Inspection CLI (`sy_cli`) for status, events, entities, zones, and JSON dumps
- Determinism validation utilities and tests in `sy_core`

Closure and hardening evidence tracked separately:

- Fast CI gates for check/build, format, clippy, tests, rustdoc,
  supply-chain checks, dependency-boundary checks, and WAL fuzz-target build
- Scheduled/manual gates for ignored tests, forced-kill recovery, long burn-in,
  coverage, and WAL fuzz smoke
- Open hardening gaps for simulation contracts, formal genesis, command
  journaling, integrity validation, single-writer ownership, compatibility
  fixtures, compaction, and operator recovery procedures

Reference docs:

- [Phase 1 Scope](seej/docs/phase1/README.md)
- [Phase 1 Exit Checklist and Engineering Gaps](seej/docs/phase1/EXIT_CHECKLIST.md)
- [Determinism Contract](seej/docs/phase1/DETERMINISM.md)
- [Persistence and Recovery](seej/docs/phase1/PERSISTENCE.md)
- [Binary Usage](seej/docs/phase1/BINARIES.md)

## Architecture

Seej follows strict dependency layering (NIV 0 to NIV 4):

- **NIV 0**: stable primitives (`sy_types`, `sy_config`)
- **NIV 1**: API definitions (`sy_api`) and Phase 2+ protocol placeholder (`sy_protocol`)
- **NIV 2**: pure simulation core (`sy_core`)
- **NIV 2b**: optional simulation modules (`mods/*`, Phase 2+, outside active workspace)
- **NIV 3**: infrastructure, tooling, testkit (`sy_infra`, `sy_tools`, `sy_testkit`)
- **NIV 4**: runtime wiring target (`sy_loader`; Phase 1 wires directly in `server_d`)

Core principles:

- **Headless first**: no graphics dependency on the server
- **Determinism first**: same genesis + same input stream = same state transitions
- **Snapshot + WAL persistence**: snapshots are recovered, then WAL events after the snapshot cursor are replayed
- **Strict decoupling**: simulation logic is independent from rendering technologies

Documentation structure:

- Repository-level docs in [`doc/`](doc/) define project governance and architecture
- Implementation-level docs in [`seej/docs/phase1/`](seej/docs/phase1/) define current Phase 1 scope and behavior

See [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md), [seej/docs/ARCHITECTURE.md](seej/docs/ARCHITECTURE.md), and [seej/docs/phase1/README.md](seej/docs/phase1/README.md).

## Quick Start

Prerequisite: Rust toolchain installed.

```bash
git clone https://github.com/Seej/Seej.git
cd Seej/seej/server

cargo build --workspace
cargo test --workspace

cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

## What Seej Is Not

Seej is not:

- a game engine
- a graphics engine
- a narrative RPG framework
- a metaverse platform claim
- a default-content game project

It is a persistent world kernel: server simulation infrastructure that others can build worlds on top of.

## Current Status

- **Phase 0** (conceptual foundations): complete
- **Phase 1** (minimal headless server slice): implemented; durable sign-off blocked by hardening gaps in the exit checklist
- **Phase 2+** (`sy_protocol`, optional modules, advanced client/network concerns): intentionally deferred

Roadmap: [doc/ROADMAP.md](doc/ROADMAP.md)

## Contributing

If you want to help build robust simulation infrastructure:

1. Read [CONTRIBUTING.md](CONTRIBUTING.md)
2. Check open [Issues](https://github.com/Seej/Seej/issues)
3. Open a PR aligned with architecture and decision documents

Key references:

- [doc/DECISIONS.md](doc/DECISIONS.md)
- [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md)
- [seej/docs/phase1/README.md](seej/docs/phase1/README.md)
