# Seej — Agent Guide

Seej is open-source infrastructure for deterministic, persistent sandbox worlds with a minimal core and one target v1 plugin boundary (D-015), currently at an **empty Phase 0 clean-slate skeleton** (no application runtime yet). Stack: Rust `1.95.0` (pinned in `rust-toolchain.toml`, zero external Cargo deps yet), Python 3 stdlib scripts (`seej/server/scripts/`), `cargo-deny`. Authoritative docs, code comments, and commits are in **English** (non-normative French mirror in `doc/fr/`).

- Entry points: `seej/server/Cargo.toml` (workspace), `doc/ROADMAP.md` (single authoritative ordered coding plan). No executable product binary exists yet.
- Code, configuration, and executable schemas are the source of truth: report any discrepancy with documentation instead of reproducing it.

## Map

| Path | Role | Public API | Allowed dependencies (all kinds) | Tests |
| --- | --- | --- | --- | --- |
| `seej/server/crates/sy_types/` | Stable primitives, checked counters, profile DTOs | none yet (Phase 0 skeleton) | none | none yet |
| `seej/server/crates/sy_api/` | Draft v1 public plugin/command/observation boundary | none yet (Phase 0 skeleton) | `sy_types` | none yet |
| `seej/server/crates/sy_core/` | Pure deterministic transition engine | none yet (Phase 0 skeleton) | `sy_types`, `sy_api` | none yet |
| `seej/server/crates/sy_infra/` | Persistence, authority, execution, DST, observation adapters | none yet (Phase 0 skeleton) | `sy_types`, `sy_api`, `sy_core` | none yet |
| `seej/server/scripts/` | Unified validation (`check.py`), purity gate, DAG check, doc link/parity check | `check.py`, `check_sy_core_purity.py`, `check_dag.py`, `check_docs.py` | Python stdlib only | Built-in `--self-test` (18 cases) |

Outside source, do not explore: `seej/server/target/`, `graft/`, `assets/`.

## Progressive Reading

1. This file (`AGENTS.md`).
2. [doc/ROADMAP.md](doc/ROADMAP.md) for the single authoritative phase order and open gap IDs.
3. [seej/docs/CONTRACT.md](seej/docs/CONTRACT.md) and [seej/docs/PLUGIN_CONTRACT.md](seej/docs/PLUGIN_CONTRACT.md) whenever a public boundary or invariant changes; [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md), [doc/DECISIONS.md](doc/DECISIONS.md), and [seej/docs/BENCHMARKS.md](seej/docs/BENCHMARKS.md) for rationale.
4. The owning crate in the Map (`src/lib.rs` and target symbols), then its tests.

## Validation

| Goal | Command (PowerShell and POSIX shells from repository root) |
| --- | --- |
| Full validation (local & CI equivalent) | `python3 -B seej/server/scripts/check.py` |
| Targeted crate test | `cargo test --manifest-path seej/server/Cargo.toml --locked -p sy_core` |
| Documentation links, anchors & parity only | `python3 -B seej/server/scripts/check_docs.py` |

Commands run identically from repository root in Windows PowerShell and Linux/macOS shells (`rust-toolchain.toml` pins `1.95.0`). `.github/workflows/rust.yml` runs the same checks plus `cargo audit` and multi-OS release builds on every PR. `cargo-audit` is not installed locally (runs in CI only). No executable product runtime exists yet. A skipped or unexecuted check is not a pass: report it explicitly.

## Rules and Mechanisms

| Rule | Mechanism | Level |
| --- | --- | --- |
| `sy_types` = ∅, `sy_api` = `{sy_types}`, `sy_core` = `{sy_types, sy_api}` across normal, dev, and build deps | `seej/server/scripts/check_dag.py` (`check.py` + CI) | lint |
| Pure crates (`sy_types`, `sy_api`, `sy_core`) never use wall-clock time, env, fs/io, net, OS RNG, threads, `HashMap`/`HashSet`, or `f32`/`f64` | `seej/server/scripts/check_sy_core_purity.py` (18 self-tests) + `#![deny(clippy::float_arithmetic)]` | lint |
| Pure crates contain zero `unsafe` code | `#![forbid(unsafe_code)]` in each pure crate root | lint |
| Every public item is documented with zero Rustdoc warnings | `#![deny(missing_docs)]` + `cargo doc` (`RUSTDOCFLAGS=-D warnings`) | lint |
| Dependency policy (MIT/Apache/BSD licenses, advisories, bans, sources) | `seej/server/deny.toml` (`cargo deny check advisories licenses bans sources`) | lint |
| Valid Markdown links, Unicode anchors, single root `AGENTS.md`, EN/FR ID parity | `seej/server/scripts/check_docs.py` (`check.py` + CI) | test |
| Fail-closed checked canonical counters (`P0-04`), never wrap or saturate | none | prose |
| No application ontology (space, entities, zones, economy, UI, netcode) in `sy_core` | none | prose |
| Single mutation path `apply(WriteSet)` for live execution and recovery; never report success before durable quorum confirmation | none | prose |
| Target v1 freezes only the public plugin/command/observation contract; keep internal seams internal until a real second provider & consumer exist | none | prose |
| `CancelCommand` and migration control use an isolated reserved control budget; `MigrationActivate` commits a mandatory boundary checkpoint | none | prose |
| Never present target behavior as implemented | none | prose |

Any rule with `none` broken twice must be encoded: architecture, then types, then lint, then test. Remove a rule from this table once its error becomes impossible.

## Generated Files

| File | Generator |
| --- | --- |
| `seej/server/Cargo.lock` | `cargo generate-lockfile --manifest-path seej/server/Cargo.toml` (not run during bootstrap) |

## Working Method

- **Before coding:** read `git status`; locate the owning module in the Map; read its contracts and tests; verify that any copied example obeys the Rules and Mechanisms table.
- **During:** stay within requested scope; call other modules only through their public API; fix the root cause without guards, delays, or error-swallowing wrappers hiding impossible states; never leave an old implementation alongside a new one; write no comments narrating code or justifying workarounds; never suppress a lint without a written reason, expiry, and explicit human agreement.
- **Before finishing:** review the diff and remove unrelated edits; run validation; verify observable behavior on the real surface and keep evidence; update affected documentation.
- **After a user correction:** fix it; if the same error class has occurred before, propose a control that makes it impossible and update the Rules and Mechanisms table after agreement.
- **Without explicit agreement:** no push, merge, or destructive Git command; no new dependency; no out-of-scope refactor or deletion; no breaking change to a public contract. Never overwrite local uncommitted changes.
- **Git:** focused branches (`fix/...`, `feat/...`, `docs/...`, `refactor/...`), atomic commits; no commit, push, or merge without explicit user request.

## Documentation to Keep Updated

| Change | Document |
| --- | --- |
| Crate added, moved, or removed; public API changed | Map in this file, `doc/ARCHITECTURE.md` (+ `doc/fr/ARCHITECTURE.md`) |
| Validation or launch command changed | Validation in this file, `README.md` (+ `doc/fr/README.md`) |
| Roadmap item closed with committed tests | `doc/ROADMAP.md` (+ `doc/fr/ROADMAP.md`) |
| Architectural or contract decision | `doc/DECISIONS.md` (+ `doc/fr/DECISIONS.md`), `seej/docs/CONTRACT.md` / `PLUGIN_CONTRACT.md` (+ `doc/fr/`) |

Update only affected documents; point to code or schemas rather than copying them.

## End-of-Task Report

- Changes made and modules touched.
- Checks executed: exact command and result.
- Documentation updated, or why none was needed.
- Unverified items, risks, and decisions needed.
