# Phase 1 — Determinism

```text
status: CURRENT LOCAL EVIDENCE + OPEN GENERIC CONTRACT
```

## Contract

For one declared execution profile:

```text
same genesis or retained checkpoint
+ same ordered admitted commands
+ same tick schedule
+ same deterministic provider and plugin contracts
= same transition decisions, events, and semantic state
```

Network delivery timing, filesystem order, thread scheduling, observers, and
wall clock do not become transition inputs unless converted into explicit
validated persisted commands.

This guarantee is scoped to advertised execution profiles. Cross-platform or
cross-version bit identity is not claimed without matching fixtures and CI
evidence.

## Implemented local evidence

The current prototype provides:

- `Tick(u64)` and tick-derived `SimTime`;
- an explicit required `--seed` at creation;
- seeded and checkpointed RNG state;
- deterministic `BTreeMap` traversal for current world collections;
- a pure runner that applies a fixed schedule and compares checkpoints;
- local recovery-parity and repeated-run tests;
- a `sy_core` purity gate rejecting known clock, filesystem, network,
  environment, OS-randomness, and nondeterministic-collection APIs.

Run from `seej/server/`:

```bash
cargo test -p sy_core
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
```

All workspace tests use:

```bash
cargo test --workspace
```

## Known limitations

The current `compute_canonical_hash` hashes the complete persisted `World`,
including storage recovery cursors. Tests comparing continuous and recovered
runs normalize those cursors by saving both worlds. It therefore mixes
semantic state and storage layout concerns.

The prototype also contains float-valued properties and float-based
degradation decisions. They remain known blockers for the stable deterministic
contract.

The current runner proves local engine behavior from a fixed schedule. It does
not prove durable command admission, distributed confirmation, plugin
compatibility, or migrations.

## Stable target

Before freeze:

- `P0-02`: checkpoint cadence is based on simulated ticks;
- `P0-03`: canonical decisions use deterministic numeric representations;
- `P0-04`: canonical counters fail closed;
- `P0-05`: history records its complete simulation contract;
- `P0-09`: concurrent producers receive confirmed per-world ordering;
- `P0-15`: a replay oracle recomputes retained transitions;
- `P1-04`: semantic hashes are separate from storage and authority hashes;
- `P1-05/P1-06`: golden versions and stable persisted variants exist;
- `K-07`: all compatible replicas reach the same semantic state;
- `K-09`: coordinated migrations produce a single declared result.

A deterministic plugin receives only explicit inputs and capabilities. An LLM,
external service, real-time clock, or sensor can influence the world only
through a validated persisted command result.

## Randomness

A randomness provider declares its algorithm and contract version. Its seed and
state, or the chosen result required for replay, are canonical. Replacing the
algorithm changes the simulation contract and requires migration or a new
world transition boundary.

## Observation and approximation

Observer activity is never a deterministic input. A world can explicitly
command a new approximation policy, but an observer cannot cause
materialization by looking at a region.

A multiresolution plugin may lead to a future different from a hypothetical
fully detailed run when the active world contract permits it. Determinism means
the declared approximation produces the same transition for the same explicit
inputs; it does not mean all allowed simulation policies share one future.

