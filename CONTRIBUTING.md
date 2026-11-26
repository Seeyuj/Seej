# Contribution Guide

Seej is open-source infrastructure for deterministic, persistent worlds. Every
contribution is evaluated against the decision log, public contracts, release
milestones, and actual repository behavior.

Start with:

- [README.md](README.md) — project overview and quick start;
- [doc/DECISIONS.md](doc/DECISIONS.md) — accepted decisions (`D-001`–`D-015`);
- [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) — system boundaries and dependency rules;
- [seej/docs/CONTRACT.md](seej/docs/CONTRACT.md) and
  [seej/docs/PLUGIN_CONTRACT.md](seej/docs/PLUGIN_CONTRACT.md) — draft v1 contracts;
- [doc/ROADMAP.md](doc/ROADMAP.md) — milestones and authoritative release checklist;
- [doc/SECURITY.md](doc/SECURITY.md) and
  [doc/CODE_OF_CONDUCT.md](doc/CODE_OF_CONDUCT.md).

Report security vulnerabilities privately per [doc/SECURITY.md](doc/SECURITY.md#vulnerability-reporting),
never in public issues or pull requests.

## Engineering Discipline

1. **Measure before designing complexity.** Do not add speculative abstractions,
   parallel schedulers, extra storage backends, or frozen internal interfaces
   without an executable benchmark or concrete consumer proving the need.
2. **Preserve the single mutation path.** Canonical state changes enter through
   admitted commands and apply through one deterministic write-set path.
3. **Never confuse design with implementation.** Do not check off a roadmap gap
   because prose exists; close items only with committed executable evidence.
4. **Keep the core minimal.** Application ontologies (space, entities, economy,
   AI, narration, rendering, ingame netcode) belong in plugins or external
   consumers, not in `sy_core`.

## Changes That Will Be Rejected

A contribution is incompatible when it:

- puts graphics, player netcode, or application ontologies inside `sy_core`;
- breaks `sy_core`'s zero-external-dependency rule or purity gate;
- lets clients, observers, discovered peers, or plugins mutate state outside
  the canonical command/write-set path;
- reports canonical success before required durable confirmation, or treats a
  local WAL append as distributed confirmation;
- uses wall-clock time, packet arrival order, thread scheduling, or ambient
  randomness as implicit transition inputs;
- silently drops opaque data, active obligations, or unknown required variants;
- silently repairs ambiguous storage corruption or performs implicit migrations;
- freezes an internal infrastructure seam prematurely without a second real
  provider and consumer;
- presents target behavior as implemented.

## Validation

Run commands from `seej/server/` using the pinned toolchain:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```

For documentation-only changes, verify all local links, anchors, commands, and
implemented-vs-target labels.

## Workflow & Governance

- Use focused branches (`fix/...`, `feat/...`, `docs/...`, `refactor/...`) and
  atomic commits.
- Significant architectural or contract changes require a recorded decision in
  [doc/DECISIONS.md](doc/DECISIONS.md) explaining the problem, evidence, and
  supersession impact.
- Maintainers protect long-term coherence and may decline technically sound PRs
  that conflict with accepted decisions or add premature complexity.
- All participation follows [doc/CODE_OF_CONDUCT.md](doc/CODE_OF_CONDUCT.md).
