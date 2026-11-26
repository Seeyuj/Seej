# Architecture

```text
status: TARGET ARCHITECTURE (D-015) — WORKSPACE IS AN EMPTY PHASE 0 SKELETON (READY FOR PHASE 1.1)
```

This document is the single architectural blueprint for Seej. It consolidates
system boundaries, the logical state model, execution flow, internal vs. public
seams, dependency rules, and repository layout. Normative draft contracts live
in [CONTRACT.md](../seej/docs/CONTRACT.md) and
[PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md); historical decisions
and measurement rationale live in
[DECISIONS.md](DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)
and [BENCHMARKS.md](../seej/docs/BENCHMARKS.md); authoritative phase ordering
lives solely in [ROADMAP.md](ROADMAP.md).

## Philosophy and Boundaries

Seej separates **mechanisms** (inside the kernel) from **policies and
application models** (in plugins and consumers):

- **World kernel:** owns world identity, simulated time (`Tick`), ordered
  command admission and durable status, single-path write-set application,
  state-root verification, obligation enforcement, persistence, and single-world
  authority replication.
- **Pure transition engine (`sy_core`):** deterministic, I/O-free state
  transition and integrity logic. Zero external crate dependencies.
- **Node runtime (`sy_infra`, future `server_d` / `sy_cli`):** wires the pure
  engine to internal storage, authority/replication, scheduling, and
  observation surfaces.
- **Application plugins:** own domain schemas, commands, validators,
  migrations, and optional policies (space, economy, AI, multiresolution,
  ingame netcode).
- **Observers:** read-only consumers of confirmed state and events; their
  activity never alters canonical transitions.

```text
community plugins & consumers               operators & observers
space · economy · AI · multires · netcode   CLI · monitoring · streams
                 |                                      |
                 +--- ONE TARGET v1 PUBLIC BOUNDARY ----+
                 |         (DRAFT — NOT FROZEN)         |
                                    |
                     logical world kernel (per world)
           identity · tick · commands · write-set · roots · obligations
                                    |
               +--------------------+--------------------+
               |                    |                    |
           StateStore         AuthorityLog           Executor
        (internal seam)     (internal seam)      (internal seam)
```

## Logical State and Mutation Model

### Narrow waist: partitioned ordered tables + obligations

The kernel defines no `Entity`, `Zone`, spatial coordinates, ECS archetypes, or
automatic secondary indexes. Every world state is a collection of versioned
owner domains exposing two logical primitives:

1. **`OrderedTable`:** lexicographically ordered key-value space
   `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes`.
   - A simple opaque-blob plugin uses one table, one partition, one key.
   - A transactional domain (accounts, inventories, governance) uses fine keys.
   - A dense simulation partitions records into contiguous `PartitionKey`
     buckets for cache locality and future multi-core execution.
   - Canonical secondary indexes are ordinary plugin-owned `OrderedTable`s
     updated atomically in the same write-set; observer-only indexes stay
     outside canonical state.
   - The schema reserves an extensible `TableKind` tag so additive columnar
     tables (`ColumnarTable`) can be added later without breaking v1.
2. **`ObligationRegistry`:** kernel-visible lifecycle envelopes
   `(obligation_id, owner_domain, counterparty_domain, status, due_tick, opaque_payload)`
   that prevent silently detaching a plugin with unresolved active obligations.

### Single mutation path (eliminates full-world cloning and replay drift)

Live execution and crash recovery are designed to share **one** canonical
mutation path:

```text
immutable StateView (at base_cursor)
  -> plugin batch evaluation -> CandidateOverlay / WriteSet + Events + Decision
  -> cross-domain validation on complete candidate
  -> durable confirmation (group-committed per tick)
  -> apply(WriteSet)  [same canonical path in live execution and recovery]
```

- **No `world.clone()`:** failed commands, stale base cursors, or plugin
  failures discard only the small candidate overlay / undo record in $O(k)$.
- **No dual mutation code:** `apply(WriteSet)` is the sole canonical state
  mutator for both live execution and crash recovery.

## Execution, Determinism, and Parallelism

### Derived entropy (`EntropyScope`)

Global mutable RNG state breaks plugin isolation and blocks parallelism. The
target core derives deterministic entropy from canonical coordinates:

```text
EntropyScope(world_seed, entropy_epoch, tick, admission_seq | phase, domain_id)
  .fork(stable_key) -> pure integer draws (no floats)
```

The underlying counter-based PRF/PRNG algorithm is declared in the execution
profile (not hardcoded forever in the root contract). Canonical counters and
IDs are cause-derived or checked (fail closed on overflow); events collect into
local buffers and receive canonical IDs in deterministic order.

### Explicit read semantics ($T-1$ vs. sequential)

- **Commands & cross-owner transactions** observe the exact sequential state
  produced by prior commands in canonical `(target_tick, admission_seq)` order.
- **Autonomous tick passes** that read an immutable $T-1$ snapshot and reduce
  intents deterministically (BSP-style) **must declare that read semantics
  explicitly** even when executed sequentially in v1. Otherwise switching from
  1 core to $N$ cores later would change observable outputs.

### Sequential v1 + Deterministic Simulation Testing (DST)

Target v1 ships a **batched sequential executor** (1 group commit per tick; not
yet implemented in the Phase 0 skeleton). This delivers two major advantages:

1. Throwaway lab benchmarks show a single-threaded sequential path can handle
   10k commands/sec on 1M keys in **8.62–21.74 ms p99** (median **6.64–9.95 ms**,
   see [BENCHMARKS.md](../seej/docs/BENCHMARKS.md)).
2. It enables **single-threaded Deterministic Simulation Testing (DST)** —
   running a simulated 3-node cluster, network partitions, message reordering,
   and injected disk failures reproducibly from a single seed.

Multi-core execution (`Block-STM` for dynamic transactions, parallel `BSP` over
partitions for dense ticks) plugs behind the internal `Executor` seam in
Phase 4.1 (Gate 1-D) with an absolute oracle: `root(1 core) == root(N cores)`.

## Persistence, Durability, and Replication

### Single-world CP authority and inter-world federation

- **Inside one world:** one authorized node group (1 node local, or 3
  shared-nothing nodes with quorum 2 using a Raft-style consensus log) produces
  one confirmed history. A local append is never distributed confirmation; loss
  of quorum pauses writes while allowing explicitly stale read-only inspection.
- **Across worlds:** decentralization, local autonomy, and peer-to-peer
  cooperation happen **between independent worlds** through asynchronous
  envelopes and durable obligations — never by lockstep-coupling $N$
  independent authority groups inside one tick (whose joint availability
  collapses as $p^N$).
- **Partitions inside one world** belong to the same authority group and tick
  clock; they exist for memory locality, cache efficiency, incremental
  checkpointing, and intra-node multi-core scaling.

### Group-committed ticks, roots, and proof modes

A confirmed tick commits a compact batch of resolutions anchored by
`pre_state_root`, `post_state_root`, and `events_root`:

- **Roots are frozen in the target v1 contract; proof mode and hash algorithm
  live in the execution profile.**
  - **Default v1 proof mode (`delta`):** `Resolution` carries the explicit
    canonical `WriteSet`, so ordinary recovery materializes state mechanically
    without invoking native plugins (preserving D-014 safety while plugins are
    unisolated).
  - **Future compact proof mode (`root` / verified tick SMR):** opt-in profile
    for dense autonomous workloads (e.g. $10^5$ writes/tick, where lab
    benchmarks show `delta` produces 14.4 MB/tick and fails the 100 ms budget
    on disk tail latency) once determinism is enforced by isolation or verified
    profiles.
- **Incremental per-record state hash:** lab benchmarks showed full
  per-partition rehashing breaks scattered command workloads (115–169 ms median
  vs. 6–10 ms). The hash algorithm itself stays a versioned profile parameter
  (never frozen permanently to one cryptographic primitive).
- **Checkpoints:** `MigrationActivate` commits a mandatory boundary checkpoint
  so ordinary recovery never spans two execution profiles. Moving full
  checkpoint I/O off the tick critical path (async CoW snapshot) is required
  before large-state production scale (lab benchmarks measured **237–435 ms**
  stalls, with one **1,485 ms** outlier, for a synchronous 144 MB checkpoint).

### Three distinct cursors

1. `confirmed_cursor` — highest canonical history record confirmed by quorum.
2. `checkpoint_cursor` — confirmed position atomically bound to a checkpoint's
   state, execution profile, and semantic root.
3. `applied_cursor` — highest confirmed position materialized into a local
   state instance (cannot serve current confirmed reads while lagging the
   quorum read barrier).

## Internal Seams vs. Target Public Boundary

Following Linux's rule (*stable user/syscall ABI, no frozen internal driver
API*) and Cordis's 3-role rule (*a seam requires definition, provider, and
consumer*):

- **Target public boundary to freeze at `K-01` (currently DRAFT — NOT FROZEN):**
  - The **Plugin, Command, and Observation Contract**
    ([CONTRACT.md](../seej/docs/CONTRACT.md) and
    [PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md)).
- **Target internal engineering seams (not frozen publicly in v1):**
  1. `StateStore` — MVCC snapshot cuts (`pin_cut`), atomic `apply(WriteSet)`,
     chunk streaming, incremental root maintenance (target: verified with
     in-memory and fault-injecting doubles for `P0-14`).
  2. `AuthorityLog` — WAL group-commit, 1-node / 3-node Raft replication,
     authority terms, joint membership, fencing, and read barriers (target:
     verified with DST network/storage doubles for `K-05` / `K-07`).
  3. `Executor` — batched sequential execution in v1; parallel `Block-STM` /
     `BSP` and Wasmtime sandboxing later without changing plugin semantics.

## Dependency Rules and Purity

### Target dependency direction

```text
stable primitives (sy_types — zero dependencies)
        ^
draft public contracts (sy_api — depends strictly on sy_types)
        ^
pure deterministic engine (sy_core — depends strictly on sy_types, sy_api)
        ^
infrastructure adapters (sy_infra)
        ^
future node binaries (server_d, sy_cli)
```

Application plugins depend **only** on public contract packages — never on
`sy_core`, `sy_infra`, another plugin's internals, or a node runtime.

### Enforced workspace matrix (`seej/server/`)

| Crate | Allowed dependencies (normal, dev, and build) |
| --- | --- |
| `sy_types` | none (enforced by `scripts/check_dag.py` and CI) |
| `sy_api` | `sy_types` (enforced by `scripts/check_dag.py` and CI) |
| `sy_core` | `sy_types`, `sy_api` (zero external deps, enforced by `scripts/check_dag.py` and CI) |
| `sy_infra` | `sy_types`, `sy_api`, `sy_core` (+ external crates added only when their phase starts) |

### Pure-stack rules (`sy_types`, `sy_api`, `sy_core`)

The three pure crates must never access filesystem, network, wall-clock/process
time, environment variables, OS randomness, OS threads, randomized hash
collections (`HashMap`/`HashSet`), or unconstrained floating-point types
(`f32`/`f64`). Enforced by `seej/server/scripts/check_sy_core_purity.py`
(18 self-test cases), `#![deny(clippy::float_arithmetic)]`,
`#![forbid(unsafe_code)]`, and `seej/server/scripts/check_dag.py`. Every crate
enforces `#![deny(missing_docs)]`.

### Forbidden shortcuts

- No provider-specific storage layout in pure domain types.
- No direct plugin mutation of another owner's domain.
- No observer dependency in canonical transitions.
- No public contract defined accidentally by `serde` over internal Rust enums.
- No implicit migration or mixed-profile execution within one world.

## Repository Layout

```text
.
├── README.md                  # project overview & language switcher (EN default)
├── AGENTS.md                  # single root agent guide (map, checks, rules)
├── CONTRIBUTING.md            # contribution discipline & governance
├── rust-toolchain.toml        # pinned Rust 1.95.0 at repository root
├── .omp/RULES.md              # permanent hard prohibitions
├── .github/workflows/rust.yml # open-source PR CI (docs, purity, DAG, rust, supply-chain, build)
├── doc/
│   ├── ARCHITECTURE.md        # this blueprint (single architecture reference)
│   ├── DECISIONS.md           # architectural decision log (D-001 through D-015)
│   ├── ROADMAP.md             # single authoritative coding plan (Phase 0–4) & exit checklist
│   ├── SECURITY.md            # threat model & vulnerability reporting
│   ├── CODE_OF_CONDUCT.md     # community standards
│   └── fr/                    # non-normative French documentation mirror
└── seej/
    ├── docs/
    │   ├── CONTRACT.md        # draft v1 kernel, authority, command & observation contract
    │   ├── PLUGIN_CONTRACT.md # draft v1 target public plugin boundary
    │   └── BENCHMARKS.md      # empirical lab measurements grounding D-015
    └── server/                # clean-slate Rust workspace (empty Phase 0 skeleton)
        ├── Cargo.toml
        ├── deny.toml          # advisories, licenses, bans, sources policy
        ├── rust-toolchain.toml
        ├── scripts/           # check.py, check_sy_core_purity.py, check_dag.py, check_docs.py
        └── crates/
            ├── sy_types/
            ├── sy_api/
            ├── sy_core/
            └── sy_infra/
```
