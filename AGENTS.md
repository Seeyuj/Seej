# AGENTS.md — Seej

Seej is open-source infrastructure for deterministic, persistent sandbox
worlds. [D-012](doc/DECISIONS.md#d-012--distributed-extensible-world-kernel)
established the distributed extensible kernel direction,
[D-013](doc/DECISIONS.md#d-013--operational-failure-semantics) and
[D-014](doc/DECISIONS.md#d-014--first-stable-boundary-semantics) fixed its
operational failure semantics, and
[D-015](doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)
anchors the minimal core, single frozen plugin seam, and incremental execution
path on real workload measurements.

Resolve tradeoffs in this order: architectural durability, correctness,
determinism, persistence and recovery, maintainability, testability,
observability, performance, then implementation speed.

## Non-negotiable invariants

- **Single-world CP authority:** one world has one confirmed history owned by
  its authorized node group (1 node or 3 nodes, quorum 2; shared-nothing nodes).
  Node discovery never grants authority, and a minority partition cannot write.
  Durable authority terms fence coordinators; confirmed configuration versions
  govern membership. Peer-to-peer decentralization operates between autonomous
  worlds, not via lockstep multi-group world sharding.
- **Headless operation:** a world evolves without UI, rendering, clients,
  agents, or ingame netcode. One-node and three-node deployments obey the same
  public contract.
- **Determinism:** equal genesis or checkpoint, ordered admitted inputs, tick
  schedule, and execution profile produce identical transitions. `sy_core`
  cannot depend on wall-clock time, network timing, filesystem order, thread
  scheduling, ambient randomness, or nondeterministic iteration. Entropy is
  derived from canonical coordinates (`EntropyScope`), never a global mutable
  RNG or floating-point decisions.
- **Single mutation path and durability:** all canonical state mutations — live
  and during recovery — pass through one deterministic `apply(write_set)` path
  over partitioned ordered tables. Never report canonical success or publish
  effects before required durable confirmation. Fail closed on persistence
  errors; pause mutations on quorum loss while allowing explicitly stale
  read-only inspection. `CancelCommand` has a reserved control budget so full
  application quotas can never deadlock recovery from a `Pending` command.
- **One frozen public seam, no over-engineering:** v1 freezes only the public
  plugin/command/observation contract (`CONTRACT.md` + `PLUGIN_CONTRACT.md`).
  Internal infrastructure seams (`StateStore`, `AuthorityLog`, `Executor`)
  remain internal, testable traits — never freeze an internal seam without a
  real second provider and consumer, and never add complex optimizations
  (`Block-STM`, `ColumnarTable`, custom crypto hashes, WASM) ahead of measured
  need.
- **Compatibility:** replay-affecting changes require explicit contract or
  profile versions. Stable kernel data maintains documented migration paths;
  plugins own their migrations. `MigrationActivate` commits a mandatory
  boundary checkpoint so ordinary recovery never spans two execution profiles.
- **Architecture:** `sy_core` depends only on `sy_types` and `sy_api` (zero
  external crates) and must pass the purity gate. Real I/O stays outside the
  pure engine.

## Current status

`seej/server/` currently holds a local, single-process prototype (`server_d`,
`sy_cli`) with a fail-closed v1 WAL, format v3/v4 snapshots, and prototype
spatial `Entity`/`Zone` fixtures. Treat it as a replaceable baseline: when
building the generic D-015 core, starting clean is explicitly preferred over
carrying legacy spatial types or JSON snapshots forward.

Never present target features (generic domains, consensus, network observation,
migrations, WASM isolation, hot loading) as implemented.

## Context on demand

| Subject | Authority |
| --- | --- |
| Architectural decisions (`D-001`–`D-015`) | [doc/DECISIONS.md](doc/DECISIONS.md) |
| Architecture, data model, seams, purity | [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) |
| Public kernel, authority, and command contract | [seej/docs/CONTRACT.md](seej/docs/CONTRACT.md) |
| Frozen v1 public plugin boundary | [seej/docs/PLUGIN_CONTRACT.md](seej/docs/PLUGIN_CONTRACT.md) |
| Milestones and release checklist | [doc/ROADMAP.md](doc/ROADMAP.md) |
| Current prototype behavior and benchmarks | [seej/docs/PROTOTYPE.md](seej/docs/PROTOTYPE.md) |
| Threat model and security posture | [doc/SECURITY.md](doc/SECURITY.md) |

## Verification

Carry implementation changes through affected checks before handing back. For
documentation changes, verify links, anchors, commands, terminology, and the
distinction between implemented prototype behavior and targets.

Run Rust commands from `seej/server/` with the pinned toolchain
(`rust-toolchain.toml`). Select checks from `.github/workflows/rust.yml`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```
