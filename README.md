# Seej

> Deterministic, persistent world kernel with a minimal core and a frozen plugin boundary.

Seej is open-source infrastructure for persistent sandbox worlds that evolve
without a player, renderer, or global central coordinator. A logical world runs
on one authorized node or a small authority group (three nodes, quorum two)
under one confirmed history. World models, spatial systems, economies, agents,
multiresolution policies, renderers, and ingame netcode stay outside the kernel
as plugins and consumers.

- Repository: [https://github.com/Seeyuj/Seej](https://github.com/Seeyuj/Seej)
- Updates: [https://x.com/Seeyuj](https://x.com/Seeyuj)

## Design Philosophy

Seej combines three disciplined systems principles — reconciled with distributed
consistency and determinism rather than slogans:

- **Small core, one frozen seam (Unix / Linux):** keep the kernel small and
  cohesive; freeze the **public plugin and external contract** (like Linux's
  syscall ABI) while keeping internal infrastructure seams (`StateStore`,
  `AuthorityLog`, `Executor`) clean, tested, and evolvable.
- **Composable capabilities over a single mutation substrate (`dsh` / Cordis):**
  application features are plugins, not core patches, composed over one
  unprivileged mutation primitive — ordered, logged write-sets applied through
  a single deterministic path.
- **Autonomous worlds, shared-nothing nodes:** no global registry or SPOF.
  Each world maintains strong consistency (CP) inside its own shared-nothing
  authority group; decentralization and peer-to-peer cooperation live between
  autonomous worlds via asynchronous envelopes.

## Status

Documentation uses three labels:

- **Implemented:** present in `seej/server/` and backed by named tests.
- **Target:** required before the v1 contract freeze (tracked in
  [doc/ROADMAP.md](doc/ROADMAP.md)).
- **Later:** usage-pulled hardening or optional ecosystem work.

**Today:** the repository holds a **local, single-process prototype**
(`server_d` and `sy_cli`) that demonstrates deterministic ticks, format v3/v4
snapshots, a fail-closed v1 WAL, recovery parity, and a `sy_core` purity gate.
Its built-in `Entity`/`Zone` structures and degradation rules are prototype
fixtures scheduled for replacement by the generic core defined in
[D-015](doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam).
It does **not** yet implement generic plugin domains, distributed consensus,
network observation, or migrations.

## Quick Start (Current Local Prototype)

Requires the pinned Rust toolchain in `seej/server/rust-toolchain.toml`:

```bash
cd seej/server

cargo test --workspace --all-targets
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py

cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

For full CLI flags, storage format details, and known prototype limits (such as
`P1-11` on `sy_cli`), see [seej/docs/PROTOTYPE.md](seej/docs/PROTOTYPE.md).

## Documentation Map

Every document has one authoritative role — no duplicated specifications:

| Document | Role |
| --- | --- |
| [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) | System model, state narrow waist, seams, dependency rules, and repo layout |
| [doc/DECISIONS.md](doc/DECISIONS.md) | Architectural decision log (`D-001` through `D-015`) and supersession trail |
| [doc/ROADMAP.md](doc/ROADMAP.md) | Incremental milestones, implemented evidence, and authoritative release checklist |
| [seej/docs/CONTRACT.md](seej/docs/CONTRACT.md) | Draft kernel, command, authority, recovery, migration, and observation contract |
| [seej/docs/PLUGIN_CONTRACT.md](seej/docs/PLUGIN_CONTRACT.md) | Draft v1 public plugin boundary, domain ownership, transactions, and extensions |
| [seej/docs/PROTOTYPE.md](seej/docs/PROTOTYPE.md) | Current local prototype commands, WAL/snapshot formats, limits, and lab benchmarks |
| [doc/SECURITY.md](doc/SECURITY.md) | Threat model, trust boundaries, operational posture, and vulnerability reporting |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution rules, validation checks, and governance |
| [doc/CODE_OF_CONDUCT.md](doc/CODE_OF_CONDUCT.md) | Community standards |

## License

Seej is released under the [MIT License](LICENSE).
