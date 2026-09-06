# Dependency Rules

This document distinguishes the dependency rules enforced in the current Rust
workspace from the target provider boundaries. Target architecture does not
silently change today's Cargo graph.

## Current enforced matrix

| Crate | May depend on |
| --- | --- |
| `sy_types` | no Seej crate |
| `sy_config` | no Seej crate |
| `sy_api` | `sy_types` |
| `sy_core` | `sy_types`, `sy_api` |
| `sy_infra` | `sy_types`, `sy_config`, `sy_api`, `sy_core` |
| `sy_tools` | `sy_types`, `sy_config`, `sy_api`, `sy_core` |
| `sy_testkit` | `sy_types`, `sy_api`, `sy_core` |
| `sy_loader` | placeholder; no active runtime responsibility |
| `server_d` and `sy_cli` | compose allowed lower-level crates |

`sy_protocol` and `mods/*` are currently outside the active workspace. This
is a fact about the prototype, not a permanent rule that all networking or
plugins must remain deferred.

## Pure-engine boundary

`sy_core` must not depend on:

- infrastructure, storage, protocol, loaders, or binaries;
- wall-clock or process time;
- filesystem, sockets, environment variables, OS randomness, or process APIs;
- nondeterministic hash collections or iteration;
- concrete provider implementations.

The purity script and CI job enforce the current subset of these rules.
Transition inputs arrive through explicit `sy_api` types and traits.

## Target dependency direction

```text
stable primitives
      ^
public contracts and ports
      ^
pure transition engine
      ^
infrastructure/provider adapters
      ^
node runtime composition
```

An application plugin depends on public contract packages, not on `sy_core`,
`sy_infra`, another plugin's internals, or a node runtime. Cross-plugin
coordination uses owned commands and kernel transactions.

A fundamental provider implements one public capability contract. It can use
lower-level primitives and approved external libraries but cannot call back
into application internals or redefine public semantics.

The future inter-node protocol belongs outside `sy_core` and may be used by
node infrastructure/runtime components. Ingame netcode remains an application
plugin and cannot become a dependency of kernel replication.

## Provider boundaries

The target public capability set includes at least:

- state storage and recovery;
- command admission/status storage;
- tick scheduling;
- deterministic validation;
- transaction coordination;
- membership and authority epochs;
- consensus/replication;
- observation and retained-cursor access;
- component loading and compatibility checks.

This list specifies responsibilities, not future crate names. A Rust
reorganization must be proposed and validated when implementation begins.

## Forbidden shortcuts

- No provider-specific storage layout in pure domain types.
- No direct plugin mutation of another owner's data.
- No client or observer dependency in canonical transition code.
- No use of network arrival order as deterministic command order.
- No public contract defined only by serde output of internal Rust enums.
- No plugin granted authority because it is locally installed or discoverable.
- No mixed compatible/incompatible provider set advancing one world.

## Enforcement

Current checks:

- `python3 scripts/check_sy_core_purity.py --self-test`;
- `python3 scripts/check_sy_core_purity.py`;
- `cargo metadata --no-deps`;
- workspace build, test, clippy, rustdoc, and supply-chain jobs described by CI.

Target checks before contract freeze:

- dependency tests for public contract packages;
- a third-party extension built without internal dependencies;
- at least one replacement fundamental provider;
- protocol and ingame-netcode separation;
- three-node compatibility and authority tests.

The [exit checklist](phase1/EXIT_CHECKLIST.md) is authoritative for whether
these target checks exist and pass.
