# Contribution Guide

Seej is a distributed, extensible world kernel for deterministic persistent
worlds. Contributions are evaluated against the public contracts, decision
history, current release gate, and actual repository behavior.

Start with:

- [project overview](README.md);
- [decision D-012](doc/DECISIONS.md#d-012--distributed-extensible-world-kernel);
- [architecture](doc/ARCHITECTURE.md);
- [kernel contract](seej/docs/CONTRACT.md);
- [exit checklist](seej/docs/phase1/EXIT_CHECKLIST.md);
- [security policy](doc/SECURITY.md);
- [code of conduct](doc/CODE_OF_CONDUCT.md).

Report security vulnerabilities privately as described by the security policy,
not through a public issue or pull request.

## Project principles

### One authoritative history per world

Authority belongs to the explicitly authorized kernel node group. Clients,
observers, and application plugins cannot bypass admission, validation,
ownership, durable confirmation, or authority epochs.

A local deployment uses a one-node group. A multi-node deployment follows the
same public semantics. Kernel networking is foundational; ingame netcode is an
application plugin.

### Pure deterministic transitions

For equal confirmed state, ordered command, tick, and active contracts, the
pure engine produces the same decision and events. Core transitions cannot
depend on wall clock, packet timing, filesystem ordering, ambient randomness,
thread scheduling, or observer activity.

External services and generative systems are allowed as extensions. Their
results become untrusted, bounded, validated, persisted commands before
affecting canonical state.

### Explicit durability and compatibility

Never report a canonical success before the configured durable confirmation.
Recovery, re-simulation, migration, clone, backup, and repair are separate
operations with explicit behavior.

Stable kernel-owned data needs a migration path. Plugin authors own their data
migrations. Replay-affecting contract changes are versioned and cannot be
hidden inside an internal refactor.

### Kernel and ecosystem

The minimal distribution supplies fundamental providers, but storage,
scheduling, validation, membership, and replication remain replaceable behind
contracts. Agents, economies, space, multiresolution, rendering, interfaces,
and ingame netcode are optional community capabilities.

The kernel does not impose an ontology, narration style, economic law,
mortality rule, or human privilege. A world extension can define any of those
when it respects deterministic admission, validation, persistence, ownership,
and compatibility.

### Observation neutrality

Read-only activity never changes canonical priority, fidelity, tick rate, or
outcomes. Limit readers under load before changing simulation semantics.

## Current priority

The repository has a working local spatial prototype. The current priority is
the first stable distributed kernel gate:

- generic owner domains and commands;
- trusted startup providers and provider conformance;
- durable command identity, ordering, status, and retention;
- common storage-failure safety;
- one- and three-node authority evidence;
- confirmed observation;
- migrations and compatibility fixtures;
- one real third-party data-owning extension.

The current zone/entity/degradation implementation is evidence to preserve
during extraction, not a universal model to expand.

Hot loading, isolated plugin execution, sharding, official agents, official
rendering, and ingame netcode are later or community work.

## Useful contribution types

High-value contributions include:

- fixes with a precise failure scenario and regression test;
- deterministic transition, replay, and migration fixtures;
- injected storage and distributed-failure tests;
- public schema and compatibility tooling;
- provider conformance harnesses;
- read-only diagnostics and confirmed observation;
- bounded input, resource, and persistence limits;
- backup, recovery, and incident documentation;
- security improvements to membership, permissions, and artifact validation;
- performance work based on measurements and preserved semantics;
- example extensions using only public contracts.

Application plugins are welcome as ecosystem projects or contract fixtures.
They are not merged into the kernel merely because they are useful.

## Changes that will be rejected

A contribution is incompatible when it:

- places graphics, player sessions, or application rules inside `sy_core`;
- lets a client, observer, discovered peer, or plugin mutate state directly;
- treats local append as distributed confirmation;
- creates two writable canonical histories during a partition;
- uses network arrival order or wall time as an implicit transition input;
- lets a plugin edit another owner's data without a public command;
- silently drops opaque data, unknown required variants, or active obligations;
- silently retargets a late command or reuses an expired command ID;
- performs an implicit migration or continues with incompatible components;
- presents a target design as implemented behavior;
- adds default world content or one application ontology to the kernel;
- couples kernel operation to a particular renderer or closed client stack.

Narrative, player-centered, generative, spatial, or game-oriented behavior is
not rejected as community application work. It is rejected only when proposed
as a mandatory kernel policy or when it bypasses the kernel contracts.

## Architecture rules

The current dependency rules live in
[seej/docs/DEPENDENCY_RULES.md](seej/docs/DEPENDENCY_RULES.md).

`sy_core` depends only on `sy_types` and `sy_api`. It performs no real I/O.
Node networking and persistence belong in infrastructure/runtime layers.
Application plugins depend on public contracts rather than kernel internals.
Ingame netcode cannot become the node-replication protocol.

Persisted DTOs and public schemas do not inherit stability accidentally from
serde over internal Rust enums. Define explicit versions, limits, unknown
handling, migrations, and golden fixtures.

Before the first stable contract, trusted providers may be statically linked or
selected at startup. Do not introduce a partial hot-loader or sandbox and call
the lifecycle complete.

## Validation

Run Rust commands from `seej/server/` with the pinned toolchain. Select checks
proportionate to the change; CI remains the source of the complete matrix.

Common commands:

```bash
cargo check --workspace --all-targets
cargo fmt --all --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

For `sy_core` changes:

```bash
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```

A meaningful behavior change needs a test at the boundary where it could
regress. A documentation-only change needs link, anchor, command, terminology,
status, and diff validation rather than an unrelated full test run.

Report pre-existing failures separately from regressions.

## Workflow

For a significant contract or architecture change, open an issue or discussion
before implementation unless maintainers have already authorized the work.
Each branch and pull request should have one coherent objective.

Suggested branch names:

- `fix/short-description`;
- `feature/short-description`;
- `docs/short-description`;
- `refactor/short-description`.

Commits should be atomic and explain the outcome. A pull request includes:

- the concrete problem and resulting behavior;
- affected contracts, decisions, migrations, and compatibility;
- failure behavior and security implications;
- exact validation commands and results;
- remaining limitations;
- evidence or fixture links for any closed checklist gap.

Do not mark a gap closed because prose exists. Update its checkbox only with
reproducible committed evidence.

## Governance

Maintainers preserve the long-term architecture and may reject technically
correct work that changes the product promise without an accepted decision.
Major changes are added to the decision log with justification, impact, and
supersession.

Technical disagreement is welcome when it remains factual and respectful.
Participation follows the [code of conduct](doc/CODE_OF_CONDUCT.md).
