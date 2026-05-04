# Phase 1 — Determinism (contract + validation)

This document describes the determinism rules and the verification utilities **as implemented**.

## Determinism contract

Given:

- the same **genesis** (seed + world config + schema assumptions),
- the same **input stream** (commands/events injected at the same tick, in the same order),
- the same **scheduling policy** (stable processing order),

the simulation must produce:

- the same sequence of outputs/events,
- and the same world state at every checkpoint.

## Rules enforced by design

### No OS time in the core

All core time is simulated:
- `Tick(u64)` is the fundamental step counter.
- `SimTime` is derived from ticks.

The core must not access `std::time::SystemTime`.

### Seed is required at world creation

The `server_d` binary requires `--seed <u64>` when creating a world, so genesis is never “accidentally random”.

### Stable iteration order

World collections that matter for reproducibility use deterministic ordering:
- `BTreeMap` is used for collections like entities/zones in the world state (stable iteration order).

## Canonical hashing

Phase 1 includes a canonical state hashing utility to validate determinism by checkpoints:

- canonical encoding must:
  - traverse collections in a stable order,
  - write fields in a stable order,
  - avoid non-deterministic sources (e.g. hash map iteration).
- a fast non-cryptographic hasher (`xxhash64`) is used for tests and diagnostics.

## Determinism runner (pure runner)

Phase 1 includes a runner that:
- creates a world from a seed,
- applies a fixed schedule of commands,
- advances ticks for a fixed number of steps,
- collects state hashes every N ticks (checkpoints),
- and compares two runs for byte-for-byte identical checkpoint sequences.

This is implemented under `sy_core` (so it can run without real I/O).

## How to run the determinism tests

From `seeyuj/server/`:

```bash
cargo test -p sy_core
```

If you want all workspace tests:

```bash
cargo test --workspace
```

## What the tests guarantee (Phase 1)

- Two runs with the same seed and the same scheduled inputs produce identical checkpoint hashes.
- Different seeds should diverge (different hashes).
- Canonical hashing is stable across repeated calls in the same process.

