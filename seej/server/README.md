# Server — Rust Workspace

This workspace contains the implemented local server prototype and the crates
that will evolve into a distributed node runtime. Target contracts are
documented separately and are not yet runnable features.

## Build and test

Use the pinned toolchain from this directory:

```bash
cargo build --workspace
cargo test --workspace
```

Run the implemented local binaries:

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

See [the binary guide](../docs/phase1/BINARIES.md) for all verified commands.

## Active dependencies

- `serde`, `serde_json`: current persistence DTO serialization and JSON
  inspection;
- `tracing`, `tracing-subscriber`: diagnostics;
- `clap`: `server_d` and `sy_cli` parsing;
- `ctrlc`: graceful local shutdown;
- `crc32fast`, `byteorder`: current WAL encoding and integrity;
- `xxhash-rust`: deterministic diagnostics and tests, not authentication;
- `windows-sys`: Windows filesystem replacement behavior;
- `assert_cmd`, `predicates`, `tempfile`, `proptest`: tests.

No active async runtime, consensus system, transport security, or wire-protocol
dependency currently implements the D-012 distributed target. Future library
selection is an implementation decision constrained by the contracts, not a
promise made by this file.

## Crates today

| Crate | Level | Current purpose |
| --- | --- | --- |
| `sy_types` | NIV 0 | stable primitive types |
| `sy_config` | NIV 0 | configuration |
| `sy_api` | NIV 1 | current commands, events, errors, and ports |
| `sy_core` | NIV 2 | pure local transition and replay logic |
| `sy_infra` | NIV 3 | filesystem, WAL, runtime, RNG, clock, and hashing adapters |
| `sy_tools` | NIV 3 | operator-tool placeholder |
| `sy_testkit` | NIV 3 | mocks and scenarios |
| `sy_loader` | NIV 4 | inert placeholder; `server_d` wires directly |
| `sy_protocol` | outside workspace | placeholder, not an implemented protocol |
| `mods/*` | outside workspace | placeholders, not active plugins |

The target inter-node protocol is foundational kernel infrastructure and will
move into the active architecture when implemented. Ingame netcode remains an
application plugin. Current exclusion of `sy_protocol` and `mods/*` is a
repository fact, not a rule that they must stay deferred forever.

## `sy_core` boundary

The pure engine cannot use:

- filesystem or network I/O;
- wall-clock or process time;
- environment access;
- OS or ambient randomness;
- nondeterministic collection order;
- a concrete storage, consensus, transport, plugin, or observer.

Use injected, explicit deterministic inputs. The enforced dependency details
and future provider direction live in
[DEPENDENCY_RULES.md](../docs/DEPENDENCY_RULES.md).

## Target runtime boundary

The logical world kernel includes responsibilities outside `sy_core`:
provider composition, command admission, durable status, authority membership,
replication, confirmed reads, and coordinated migrations.

The first stable runtime may link trusted operator-approved providers
statically or select them at startup. Dynamic hot loading and isolated
application plugins are later milestones.
