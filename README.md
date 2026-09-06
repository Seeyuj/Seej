# Seej

> Distributed, deterministic world kernel for persistent sandbox worlds.

Seej is open-source infrastructure for worlds that keep evolving without a
player, rendering engine, or permanently central coordinator. A logical world
can run on one node or an explicitly authorized group of nodes. The kernel
coordinates time, identity, ordered commands, validation, durable state, and
replication through versioned contracts.

Seej follows a kernel-and-ecosystem model. It ships a minimal working
distribution and replaceable fundamental providers. World models, agents,
economies, spatial systems, multiresolution engines, renderers, and ingame
netcode are optional extensions. The network between Seej nodes is part of the
kernel infrastructure; player-facing netcode is not.

- GitHub: [https://github.com/Seeyuj/Seej](https://github.com/Seeyuj/Seej)
- Follow: [https://x.com/Seeyuj](https://x.com/Seeyuj)

## Contract Status

The distributed architecture is the target established by
[decision D-012](doc/DECISIONS.md#d-012--distributed-extensible-world-kernel),
with operational failure semantics fixed by
[D-013](doc/DECISIONS.md#d-013--operational-failure-semantics).
It is **not implemented or frozen yet**. The public
[kernel contract](seej/docs/CONTRACT.md),
[node-network contract](seej/docs/NODE_NETWORK.md), and
[plugin contract](seej/docs/PLUGIN_CONTRACT.md) remain drafts until every
mandatory item in the
[Phase 1 exit checklist](seej/docs/phase1/EXIT_CHECKLIST.md) has evidence.

Documentation uses three status labels:

- **Implemented**: present in this repository and covered by named evidence.
- **Target**: required before the first stable contract and tracked by a gap ID.
- **Later**: intentionally outside the first stable contract.

## What Works Today

The current prototype is a single-process, single-writer headless server. It
can create a spatial demonstration world, run deterministic ticks, persist a
snapshot and local WAL, recover after interruption, and expose read-only CLI
inspection.

Implemented today:

- deterministic tick execution with an explicit seed;
- the `server_d` headless runtime;
- local snapshot and WAL persistence;
- recovery by loading a snapshot and replaying later WAL events;
- the current zone/entity model and demonstration degradation rules;
- `sy_cli` status, event, entity, zone, and JSON inspection;
- determinism, recovery, crash, and WAL fuzz evidence described by the Phase 1
  documents.

The prototype does not yet provide a distributed authority group, consensus,
generic plugin-owned data, a stable public extension ABI, hot loading, or
plugin isolation. The current zone/entity types and systemic rules are
prototype behavior, not the future universal world model.

## Target Architecture

```text
community capabilities                     operators and observers
agents · economy · space · UI · netcode     CLI · monitoring · administration
             |                                          |
             +---------- versioned contracts -----------+
                                |
                 logical distributed world kernel
        identity · time · commands · validation · transactions
                    durable history · replication
                                |
             +------------------+------------------+
             |                  |                  |
          node A             node B             node C
       pure engine +      pure engine +      pure engine +
        providers          providers          providers
```

`sy_core` is the deterministic transition engine inside this architecture. It
does not perform filesystem, clock, or network I/O. A node runtime wires it to
approved providers for persistence, scheduling, validation, membership,
replication, and other capabilities. Single-node operation is a deployment of
the same contracts, not a separate product.

The first distributed reference target uses three authorized nodes and a
quorum of two under a crash/partition fault model. This reference is allowed to
use a Raft-style provider, but the contract does not require every deployment
to use the same implementation.

Read the detailed model in:

- [Project architecture](doc/ARCHITECTURE.md)
- [Technical architecture](seej/docs/ARCHITECTURE.md)
- [Kernel contract](seej/docs/CONTRACT.md)
- [World specification](seej/docs/simulation/WORLD_SPEC.md)
- [Roadmap](doc/ROADMAP.md)

## Quick Start

Prerequisite: the pinned Rust toolchain described by `seej/server/rust-toolchain.toml`.

```bash
git clone https://github.com/Seeyuj/Seej.git
cd Seej/seej/server

cargo build --workspace
cargo test --workspace

cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

These commands run the current local prototype. No distributed command-line
interface is claimed until one is implemented and documented in
[BINARIES.md](seej/docs/phase1/BINARIES.md).

## Project Boundaries

Seej is infrastructure rather than a game, graphics engine, narrative
framework, content pack, or metaverse product. A community extension can build
any of those experiences while the kernel remains headless and neutral about
their application rules.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), the
[decision log](doc/DECISIONS.md), and the
[exit checklist](seej/docs/phase1/EXIT_CHECKLIST.md) before changing a public
contract. Contributions must state whether they implement an existing target,
change a contract, or describe current behavior.

## License

Seej is released under the [MIT License](LICENSE).
