# AGENTS.md — Seej

Seej is open-source infrastructure for deterministic, persistent sandbox
worlds. Decision D-012 defines it as a distributed, extensible world kernel.
The kernel runs on one or more authorized nodes and coordinates replaceable
providers for its fundamental capabilities. Ingame netcode, agents, rendering,
economies, spatial models, and multiresolution engines remain userland.

Resolve tradeoffs in this order: architectural durability, correctness,
determinism, persistence and recovery, maintainability, testability,
observability, performance, then implementation speed.

## Non-negotiable invariants

- **Distributed authority:** authority belongs to the authorized group for a
  world. A confirmed history is the source of truth. Node discovery does not
  grant authority, and a minority partition cannot create another canonical
  history.
- **Headless operation:** a world evolves without UI, rendering, assets,
  connected clients, agents, or ingame netcode. A one-node deployment uses the
  same kernel contracts as a multi-node deployment.
- **Determinism:** the same genesis or retained checkpoint, ordered admitted
  inputs, tick schedule, and active contracts produce the same transitions.
  `sy_core` cannot depend on wall-clock time, network timing, filesystem
  ordering, thread scheduling, ambient randomness, or nondeterministic
  iteration.
- **Durability:** never report success or publish canonical effects before the
  required durable confirmation. A local WAL append is not a distributed
  commit. On critical persistence failure, fail closed; on loss of quorum,
  pause mutations while permitting explicitly stale, read-only inspection.
- **Extensibility:** world data has declared owners and is changed through
  public commands. Cross-owner changes inside one world are atomic. The kernel
  enforces declared validators without imposing application laws, spatial
  types, creature/resource categories, or human privileges.
- **Compatibility:** replay-affecting changes require explicit contract
  versions. Stable kernel-owned data always has a documented migration path.
  Plugins own their data migrations and incompatible components block
  activation rather than being silently skipped.
- **Architecture:** preserve the documented dependency direction. `sy_core`
  depends only on `sy_types` and `sy_api` and must pass the purity gate. Real
  I/O belongs outside the pure transition engine.

## Current status

The repository currently implements a local, single-writer Phase 1 prototype.
Do not describe target networking, generic extension data, hot loading, or
isolation as implemented. The current zone/entity model and degradation rules
are prototype fixtures that must move behind extension boundaries before the
first stable contract.

Gate 1-K in `seej/docs/phase1/EXIT_CHECKLIST.md` is the mandatory bar for the
first stable contract. It includes the distributed proof, fail-closed common
storage faults, a real external data-owning extension, and contract migration
evidence. Gate 1-D contains later hardening whose triggers are genuinely tied
to scale or operational use.

Hot loading and isolated execution are later capabilities. The first stable
contract may load only operator-approved trusted providers at startup.

## Context on demand

Read the documents relevant to the affected boundary:

| Subject | Authority |
| --- | --- |
| Direction and supersession | [Decision D-012](doc/DECISIONS.md#d-012--distributed-extensible-world-kernel) |
| Public guarantees and statuses | [Kernel contract](seej/docs/CONTRACT.md) |
| Inter-node authority | [Node-network contract](seej/docs/NODE_NETWORK.md) |
| Providers and world extensions | [Plugin contract](seej/docs/PLUGIN_CONTRACT.md) |
| Crate and purity boundaries | [Dependency rules](seej/docs/DEPENDENCY_RULES.md) |
| Stable-release evidence | [Exit checklist](seej/docs/phase1/EXIT_CHECKLIST.md) |

## Completion and verification

Carry implementation requests through affected checks and fix regressions
before handing back. Preserve user changes and keep commits scoped. For
documentation, verify links, anchors, terminology, status labels, commands,
and the distinction between implemented behavior and targets.

Run Rust commands from `seej/server` with its pinned toolchain. Select affected
checks from `.github/workflows/rust.yml`; start with targeted crate tests and
broaden only when boundaries are crossed. For `sy_core` changes, run
`python3 scripts/check_sy_core_purity.py`; add `--self-test` when changing the
gate itself.
