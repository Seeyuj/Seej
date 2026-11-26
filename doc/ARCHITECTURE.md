# Architecture

```text
status: TARGET ARCHITECTURE (D-015) — CURRENT PROTOTYPE IS DESCRIBED IN seej/docs/PROTOTYPE.md
```

This document is the single architectural blueprint for Seej. It consolidates
system boundaries, the logical state model, execution flow, internal vs. public
seams, dependency rules, and repository layout. Normative guarantees live in
[CONTRACT.md](../seej/docs/CONTRACT.md) and
[PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md); historical decisions
and measurement rationale live in [DECISIONS.md](DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam).

## Philosophy and Boundaries

Seej separates **mechanisms** (inside the kernel) from **policies and
application models** (in plugins and consumers):

- **World kernel:** owns world identity, simulated time (`Tick`), ordered
  command admission and durable status, single-path write-set application,
  state-root verification, obligation enforcement, persistence, and single-world
  authority replication.
- **Pure transition engine (`sy_core`):** deterministic, I/O-free state
  transition and integrity logic. Zero external crate dependencies.
- **Node runtime (`sy_infra`, `server_d`):** wires the pure engine to internal
  storage, authority/replication, scheduling, and observation surfaces.
- **Application plugins:** own domain schemas, commands, validators,
  migrations, and optional policies (space, economy, AI, multiresolution,
  ingame netcode).
- **Observers:** read-only consumers of confirmed state and events; their
  activity never alters canonical transitions.

```text
community plugins & consumers               operators & observers
space · economy · AI · multires · netcode   CLI · monitoring · streams
                 |                                      |
                 +---- ONE FROZEN PUBLIC BOUNDARY ------+
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

Live execution and crash recovery share **one** canonical mutation path:

```text
immutable StateView (at base_cursor)
  -> plugin batch evaluation -> CandidateOverlay / WriteSet + Events + Decision
  -> cross-domain validation on complete candidate
  -> durable confirmation (group-committed per tick)
  -> StateStore::apply(WriteSet)  [same function in live execution and recovery]
```

- **No `world.clone()`:** failed commands, stale base cursors, or plugin
  failures discard only the small candidate overlay / undo record in $O(k)$.
- **No dual mutation code:** unlike the legacy prototype (where `sim.rs` and
  `replay.rs` independently re-implemented every state change), `apply(WriteSet)`
  is the sole state mutator.

## Execution, Determinism, and Parallelism

### Derived entropy (`EntropyScope`)

Global mutable RNG state (`&mut IRng`) breaks plugin isolation and blocks
parallelism. The target core derives deterministic entropy from canonical
coordinates:

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

v1 ships a **batched sequential executor** (1 group commit per tick). This
delivers two major advantages proven in production engines (TigerBeetle,
FoundationDB):

1. It already handles 10k commands/sec on 1M keys in ~6–12 ms/tick on one core
   (see [PROTOTYPE.md](../seej/docs/PROTOTYPE.md#benchmark-measurements-workloads-harness)).
2. It enables **single-threaded Deterministic Simulation Testing (DST)** —
   running a simulated 3-node cluster, network partitions, message reordering,
   and injected disk failures reproducibly from a single seed.

Multi-core execution (`Block-STM` for dynamic transactions, parallel `BSP` over
partitions for dense ticks) plugs behind the internal `Executor` seam in
Gate 1-D with an absolute oracle: `root(1 core) == root(N cores)`.

## Persistence, Durability, and Replication

### Single-world CP authority and inter-world federation

- **Inside one world:** one authorized node group (1 node local, or 3 shared-nothing
  nodes with quorum 2 using a Raft-style consensus log) produces one confirmed
  history. A local append is never distributed confirmation; loss of quorum
  pauses writes while allowing explicitly stale read-only inspection.
- **Across worlds:** decentralization, local autonomy, and peer-to-peer
  cooperation happen **between independent worlds** through asynchronous
  envelopes and durable obligations — never by lockstep-coupling $N$ independent
  authority groups inside one tick (whose joint availability collapses as $p^N$).
- **Partitions inside one world** belong to the same authority group and tick
  clock; they exist for memory locality, cache efficiency, incremental
  checkpointing, and intra-node multi-core scaling.

### Group-committed ticks, roots, and proof modes

A confirmed tick commits a compact batch of resolutions anchored by
`pre_state_root`, `post_state_root`, and `events_root`:

- **Roots are frozen in the contract; proof mode and hash algorithm live in the
  execution profile.**
  - **Default v1 proof mode (`delta`):** `Resolution` carries the explicit
    canonical `WriteSet`, so ordinary recovery materializes state mechanically
    without invoking native plugins (preserving D-014 safety while plugins are
    unisolated).
  - **Future compact proof mode (`root` / verified tick SMR):** opt-in profile
    for dense autonomous workloads (e.g. $10^5$ writes/tick, where benchmarks
    show `delta` produces 14.4 MB/tick and saturates disk tail latency) once
    determinism is enforced by isolation or verified profiles.
- **Incremental per-record state hash:** benchmarks showed full per-partition
  rehashing breaks scattered command workloads (115–169 ms/tick vs. 6–8 ms).
  The hash algorithm itself stays a versioned profile parameter (never frozen
  permanently to one cryptographic primitive).
- **Checkpoints:** `MigrationActivate` commits a mandatory boundary checkpoint
  so ordinary recovery never spans two execution profiles. Moving full
  checkpoint I/O off the tick critical path (async CoW snapshot) is required
  before large-state production scale (benchmarks showed 240–435 ms stalls for
  a synchronous 144 MB checkpoint).

### Three distinct cursors

1. `confirmed_cursor` — highest canonical history record confirmed by quorum.
2. `checkpoint_cursor` — confirmed position atomically bound to a checkpoint's
   state, execution profile, and semantic root.
3. `applied_cursor` — highest confirmed position materialized into a local
   state instance (cannot serve current confirmed reads while lagging the
   quorum read barrier).

## Internal Seams vs. Frozen Public Boundary

Following Linux's rule (*stable user/syscall ABI, no frozen internal driver
API*) and Cordis's 3-role rule (*a seam requires definition, provider, and
consumer*):

- **Frozen public boundary in v1 (1 boundary):**
  - The **Plugin, Command, and Observation Contract**
    ([CONTRACT.md](../seej/docs/CONTRACT.md) and
    [PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md)).
- **Internal engineering seams (3 seams — tested, not frozen publicly in v1):**
  1. `StateStore` — MVCC snapshot cuts (`pin_cut`), atomic `apply(WriteSet)`,
     chunk streaming, incremental root maintenance (tested with in-memory +
     fault-injecting doubles for `P0-14`).
  2. `AuthorityLog` — WAL group-commit, 1-node / 3-node Raft replication,
     authority terms, joint membership, fencing, and read barriers (tested with
     DST network/storage doubles).
  3. `Executor` — batched sequential execution in v1; parallel `Block-STM` /
     `BSP` and Wasmtime sandboxing later without changing plugin semantics.

## Dependency Rules and Purity

### Target dependency direction

```text
stable primitives (sy_types)
        ^
public contracts & internal traits (sy_api)
        ^
pure deterministic engine (sy_core — zero external crates)
        ^
infrastructure adapters (sy_infra)
        ^
node runtime binaries (server_d, sy_cli)
```

Application plugins depend **only** on public contract packages — never on
`sy_core`, `sy_infra`, another plugin's internals, or a node runtime.

### Current enforced workspace matrix (`seej/server/`)

| Crate | Allowed workspace dependencies |
| --- | --- |
| `sy_types` | none |
| `sy_config` | none |
| `sy_api` | `sy_types` |
| `sy_core` | `sy_types`, `sy_api` (strictly enforced by CI — zero external deps) |
| `sy_infra` | `sy_types`, `sy_config`, `sy_api`, `sy_core` |
| `sy_testkit` | `sy_types`, `sy_api`, `sy_core` |
| `server_d`, `sy_cli` | compose lower crates |

### Pure-engine rules (`sy_core`)

`sy_core` must never access filesystem, network, wall-clock/process time,
environment variables, OS randomness, threads/locks, or randomized hash
collections (`HashMap`/`HashSet`). Enforced by
`seej/server/scripts/check_sy_core_purity.py` and the CI dependency check in
`.github/workflows/rust.yml`.

### Forbidden shortcuts

- No provider-specific storage layout in pure domain types.
- No direct plugin mutation of another owner's domain.
- No observer dependency in canonical transitions.
- No public contract defined accidentally by `serde` over internal Rust enums.
- No implicit migration or mixed-profile execution within one world.

## Repository Layout

```text
.
├── README.md                  # project overview & quick start
├── AGENTS.md                  # operating invariants & verification commands
├── CONTRIBUTING.md            # contribution discipline & governance
├── .github/workflows/rust.yml # CI gates (purity, fmt, clippy, test, doc, audit, fuzz)
├── doc/
│   ├── ARCHITECTURE.md        # this blueprint (single architecture reference)
│   ├── DECISIONS.md           # architectural decision log (D-001 through D-015)
│   ├── ROADMAP.md             # incremental milestones & authoritative exit checklist
│   ├── SECURITY.md            # threat model & vulnerability reporting
│   └── CODE_OF_CONDUCT.md     # community standards
└── seej/
    ├── docs/
    │   ├── CONTRACT.md        # draft v1 kernel, authority, command & observation contract
    │   ├── PLUGIN_CONTRACT.md # draft v1 frozen public plugin boundary
    │   └── PROTOTYPE.md       # current local prototype reference & lab benchmarks
    └── server/                # active Rust workspace (current local prototype)
        ├── Cargo.toml
        ├── deny.toml          # license & advisory policy (MIT/Apache/BSD; no copyleft/MPL)
        ├── rust-toolchain.toml
        ├── scripts/check_sy_core_purity.py
        ├── crates/
        │   ├── sy_types/
        │   ├── sy_config/
        │   ├── sy_api/
        │   ├── sy_core/
        │   ├── sy_infra/      # includes isolated WAL fuzz harness in fuzz/
        │   └── sy_testkit/
        └── bin/
            ├── server_d/
            └── sy_cli/
```
