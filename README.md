# Seej

> **Language:** **English (authoritative)** | [Français](doc/fr/README.md)

> Deterministic, persistent world kernel with a minimal core and one target v1 plugin boundary.

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

- **Small core, one target public seam (Unix / Linux):** keep the kernel small
  and cohesive; freeze only the **public plugin and external contract** in v1
  (like Linux's syscall ABI) while keeping internal infrastructure seams
  (`StateStore`, `AuthorityLog`, `Executor`) clean, tested, and evolvable.
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

- **Implemented:** present in `seej/server/` and backed by committed tests.
- **Target:** required before the v1 contract freeze (tracked in
  [doc/ROADMAP.md](doc/ROADMAP.md)).
- **Later:** usage-pulled hardening or optional ecosystem work.

**Today (Phase 0 complete — clean-slate foundation):** per
[D-015](doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam),
the legacy spatial prototype has been retired (preserved at immutable tag
`archive/prototype-v1-final`). `seej/server/` holds an empty 4-crate Rust
workspace skeleton (`sy_types`, `sy_api`, `sy_core`, `sy_infra`) with strict
lints, pure-crate purity checks, full dependency-DAG checks, pinned Rust
`1.95.0`, and open-source PR CI — ready to begin **Phase 1.1** in
[doc/ROADMAP.md](doc/ROADMAP.md). No runtime daemon, storage backend, consensus
engine, or plugin host is implemented yet.

## Quick Start (Run Full Validation Suite)

Requires Rust `1.95.0` (pinned in `rust-toolchain.toml`) and Python 3:

```bash
python3 -B seej/server/scripts/check.py
```

This single command runs the purity gate (18 self-tests + pure-crate scan),
dependency-DAG check (normal + dev + build), Markdown link/parity checks,
`cargo fmt`, `cargo clippy`, `cargo test`, strict Rustdoc (`-D warnings`), and
`cargo deny` (when installed).

## Documentation Map

Every document has one authoritative role — no duplicated specifications:

| Document | Role |
| --- | --- |
| [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) | System model, state narrow waist, seams, dependency rules, and repo layout |
| [doc/DECISIONS.md](doc/DECISIONS.md) | Architectural decision log (`D-001` through `D-015`) and archive traceability |
| [doc/ROADMAP.md](doc/ROADMAP.md) | Ordered coding plan (`Phase 0`–`Phase 4`) and authoritative release checklist |
| [seej/docs/CONTRACT.md](seej/docs/CONTRACT.md) | Draft (`0.4.0-draft`) kernel, command, authority, recovery, migration, and observation contract |
| [seej/docs/PLUGIN_CONTRACT.md](seej/docs/PLUGIN_CONTRACT.md) | Draft (`0.3.0-draft`) target v1 public plugin boundary, domain ownership, and transactions |
| [seej/docs/BENCHMARKS.md](seej/docs/BENCHMARKS.md) | Empirical lab measurements grounding D-015 storage, hashing, and proof-mode choices |
| [doc/SECURITY.md](doc/SECURITY.md) | Threat model, trust boundaries, operational posture, and vulnerability reporting |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution rules, validation checks, and governance |
| [doc/CODE_OF_CONDUCT.md](doc/CODE_OF_CONDUCT.md) | Community standards |

## License

Seej is released under the [MIT License](LICENSE).
