# Decision Log

This document records the structural architectural decisions for Seej: what was
decided, why, what it implies, and what later decisions superseded. It is not a
roadmap (see [ROADMAP.md](ROADMAP.md)) or a contract (see
[CONTRACT.md](../seej/docs/CONTRACT.md) and
[PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md)).

## Decision Index

> **Traceability note:** Entries `D-001` through `D-014` below were condensed
> into concise summaries by `D-015` without changing their accepted intent.
> Their full verbatim pre-D-015 text is preserved at archive tag
> `archive/pre-d015-docs` (`9662a60`).

| ID | Decision | Status |
| --- | --- | --- |
| [D-001](#d-001--the-project-is-a-platform-not-a-game) | Platform, not a game | Accepted |
| [D-002](#d-002--narrative-policy-outside-the-kernel) | Narrative policy outside the kernel | Accepted (narrowed by D-012) |
| [D-003](#d-003--autonomous-world-not-centered-on-the-player) | Autonomous world without players | Accepted (narrowed by D-012) |
| [D-004](#d-004--authoritative-server-and-real-persistence) | Authoritative group and real persistence | Accepted (extended by D-012) |
| [D-005](#d-005--strict-decoupling-between-simulation-and-rendering) | Strict decoupling from rendering | Accepted |
| [D-006](#d-006--reference-rendering-client-and-official-graphics-standard) | Official Unreal reference client | Superseded by D-012 |
| [D-007](#d-007--pragmatic-deterministic-and-explainable-ai) | Validated deterministic external inputs | Accepted (narrowed by D-012) |
| [D-008](#d-008--minimal-core-modular-extensions) | Minimal core, modular extensions | Accepted (refined by D-012, D-015) |
| [D-009](#d-009--stability-and-maintainability-before-speed) | Stability and maintainability before speed | Accepted |
| [D-010](#d-010--rust-for-the-core) | Rust for the pure core | Accepted |
| [D-011](#d-011--contract-first-kernel-usage-pulled-hardening) | Contract-first kernel, usage-pulled hardening | Superseded by D-012 (incremental discipline restored by D-015) |
| [D-012](#d-012--distributed-extensible-world-kernel) | Distributed extensible world kernel | Accepted (amended by D-013, D-014, D-015) |
| [D-013](#d-013--operational-failure-semantics) | Operational failure semantics | Accepted (amended by D-014, D-015) |
| [D-014](#d-014--first-stable-boundary-semantics) | First-stable boundary semantics | Accepted (amended by D-015) |
| [D-015](#d-015--measured-kernel-core-and-frozen-plugin-seam) | Measured kernel core and frozen plugin seam | **Accepted (current baseline)** |

---

## D-001 — The Project is a Platform, Not a Game

**Status:** Accepted

Seej is open-source infrastructure for persistent deterministic sandbox worlds —
not a game, graphics engine, narrative RPG, or gameplay framework. Its value
lies in simulation-core stability, real persistence, systemic coherence, and
multi-year maintainability. Clients remain consumers, never architectural
pillars.

---

## D-002 — Narrative Policy Outside the Kernel

**Status:** Accepted, scope amended by D-012

The kernel defines deterministic world mechanics and prescribes no narrative
model. Extensions may implement authored, procedural, or emergent narration,
subject to the same admission, validation, ownership, and persistence rules as
all other application capabilities.

---

## D-003 — Autonomous World Not Centered on the Player

**Status:** Accepted, scope amended by D-012

A world evolves and persists without any connected player or human presence.
The kernel embeds no player category or human privilege; extensions may declare
application roles explicitly.

---

## D-004 — Authoritative Server and Real Persistence

**Status:** Accepted, extended to authority groups by D-012

One explicitly authorized node group is the sole canonical authority for a
world's history. One-node and multi-node deployments follow the same public
contract. Clients hold no canonical authority.

---

## D-005 — Strict Decoupling Between Simulation and Rendering

**Status:** Accepted

The simulation core is completely independent of rendering technologies and
client engines. No graphics library or client framework is a kernel or runtime
dependency.

---

## D-006 — Reference Rendering Client and Official Graphics Standard

**Status:** Superseded by D-012

Historical direction promising an official Unreal Engine reference client.
Superseded by D-012: renderers (Unreal, Godot, web, CLI) are optional community
consumers of the public contracts, not Seej kernel deliverables.

---

## D-007 — Pragmatic, Deterministic, and Explainable AI

**Status:** Accepted for kernel inputs, scope amended by D-012

The kernel accepts only bounded, validated, ordered, persisted inputs and ships
no built-in agent framework. Nondeterministic external systems (LLMs, sensors,
bots) influence canonical state only by submitting validated commands.

---

## D-008 — Minimal Core, Modular Extensions

**Status:** Accepted, extended by D-012 and refined by D-015

Keep the core minimal, strict, and stable. Application capabilities are
extensions composed over the kernel's public contract rather than accumulated
inside a monolith.

---

## D-009 — Stability and Maintainability Before Speed

**Status:** Accepted

Architectural durability, correctness, determinism, and maintainability take
precedence over premature optimization or rushed features. Performance choices
must preserve semantics and be grounded in measurements.

---

## D-010 — Rust for the Core

**Status:** Accepted

The simulation core is written in Rust for memory safety, predictable native
performance, strong typing, and explicit `unsafe`/FFI review boundaries.
Consumers and future isolated plugins are not restricted to Rust.

---

## D-011 — Contract-First Kernel, Usage-Pulled Hardening

**Status:** Superseded by D-012 (incremental milestone discipline restored by D-015)

Positioned Seej as a contract-first world kernel splitting the release into
Gate 1-K (frozen contract) and Gate 1-D (usage-pulled hardening). D-012
replaced its single-writer scope and built-in conservation assumption while
retaining the contract-first principle and Gate 1-K / Gate 1-D split.

---

## D-012 — Distributed Extensible World Kernel

**Status:** Accepted, operational semantics amended by D-013, D-014, and D-015

Defined Seej as a logical distributed world kernel without a global coordinator:

- Each world has an explicit authorized node group (reference profile: three
  shared-nothing nodes, quorum two, crash-fault Raft-style consensus; Byzantine
  consensus is out of v1 scope). Discovery never grants membership.
- `world_id`, `node_id`, seed, and `genesis_hash` are distinct. Ticks and
  command order are canonical per world with no global clock across worlds.
- World data belongs to declared owner domains without mandatory space,
  entities, zones, or built-in economic laws.
- Startup activation loads only operator-approved trusted components in v1; hot
  loading and isolated plugin execution are later work.
- Observation reads a confirmed prefix with freshness and never alters
  canonical simulation.
- Replicating a world improves availability and recoverability; it does not
  shard tick computation across machines.

---

## D-013 — Operational Failure Semantics

**Status:** Accepted, amended by D-014 and D-015

Closed major failure-boundary ambiguities from D-012:

- Separated confirmed **Admission** (`Pending`, no application effects) from
  confirmed **Resolution** (`Succeeded`, `Refused`, or `Cancelled`).
- Required checked canonical-tick expiry and non-reuse of expired command IDs.
- Defined cross-owner transactions against an immutable base cursor with
  private candidate validation and full recomputation on stale base; component
  crashes never fabricate deterministic `Refused` decisions.
- Required joint-configuration membership transitions preserving old/new quorum
  intersection and quorum-backed read barriers for current confirmed reads.
- Separated migration preparation, activation commit, and abort (no automatic
  post-activation rollback).
- Made causal hash linkage, cold backup/restore, and baseline operator recovery
  procedures mandatory before v1 freeze.

---

## D-014 — First-Stable Boundary Semantics

**Status:** Accepted, amended by D-015

Added concrete boundary mechanisms to D-013:

- Defined authorized confirmed `CancelCommand` racing normal execution through
  confirmed order as the administrative escape path for blocked `Pending`
  commands.
- Added post-drain `MigrationBarrier` freezing world ticks and application
  admissions (`WorldPaused`) across a fixed membership configuration before
  deterministic candidate derivation and `MigrationActivate`.
- Defined bounded per-world producer slots with monotonic incarnations,
  confirmed `Open` / `Sealed` / `Closed` generations, tombstones, closed
  generation floors, and finite persisted limits.
- Distinguished `confirmed_cursor`, `checkpoint_cursor`, and `applied_cursor`.
- Clarified the retired prototype's `sy_cli` read-only gaps
  (`FilesystemStore::new` directory creation and unguarded `dump --output`
  aliasing) under `P1-11`, retained as acceptance criteria for the clean-slate
  CLI.

---

## D-015 — Measured Kernel Core and Frozen Plugin Seam

**Status:** Accepted

### Context and empirical evidence

An audit of the repository history and codebase after D-014 exposed three
structural problems:

1. **Specification ahead of execution:** 29 consecutive commits edited docs and
   CI without advancing the Rust core; Gate 1-K had 0 closed items and zero
   benchmarks. Meanwhile the local prototype (`seej/server/`) cloned the entire
   `World` twice per command and once per replayed event, split mutation logic
   between `sim.rs` and `replay.rs`, and carried dead placeholder crates.
2. **Over-fragmented public seams and unmeasured assumptions:** requiring nine
   separately frozen public provider contracts (`PLUGIN_CONTRACT.md` §8 in
   D-014) created microkernel fragmentation across components that share one
   atomic commit boundary (log, consensus, membership, fencing), while
   specifying heavy persistence/hashing mechanisms before measuring them.
3. **A liveness gap in `CONTRACT.md`:** `CancelCommand` shared normal
   application admission quotas, so a world blocked on a `Pending` command
   with full quotas could reject `CancelCommand` with `LimitExceeded` while
   ticks could no longer advance to expire old records.

Two throwaway single-threaded benchmark campaigns preserved at archive tags
`archive/lab-campaign-1` (`1c98282`, 6/6 clean recoveries) and
`archive/lab-campaign-2` (`5856396`, 24/24 clean recoveries — OS page cache not
purged, effective cache temperature unknown, no fault injection), on Ryzen 9
5900X + Crucial P2 NVMe + Rust 1.95.0, 1,000,000 keys × 128 B, 10 Hz / 100 ms
tick budget, established five concrete facts (full tables in
[BENCHMARKS.md](../seej/docs/BENCHMARKS.md)):

- **Command/mixed loads (A: 1,000 cmds/tick; C: 1,000 cmds + 1,000 autonomous
  writes + 80% hot keys)** meet the 100 ms budget on one thread in `delta` mode
  across all non-rehash runs: **p99 = 8.62–21.74 ms** (`A`: `8.62–10.83 ms`,
  `C`: `11.77–21.74 ms`, median `6.64–9.95 ms`; **4.6×–11.6× headroom**),
  journal **4.17–5.61 MB/s**, clean replay **2.06–3.35 ms/tick**.
- **Dense autonomous load (B: 100,000 writes/tick)** fails the 100 ms budget in
  `delta` mode across **all 9 runs** (1 in Campaign 1 + 8 in Campaign 2):
  14.4 MB/tick synchronous writes produce full-tick **p99 = 113.41–482.78 ms**
  (commit `sync_data` p99 up to **283.10–400.12 ms**, max **689.37 ms**) and
  project 144 MB/s before replication.
- **State layout dominates dense load CPU cost, while replay ratio `< 0.5`
  remains unmet:** switching from `BTreeMap` + overlay to contiguous
  per-partition storage with in-place undo brings B in `root` mode from
  **147.07 ms → 78.95 ms p99** (66.13 ms median) on one thread, though **no
  variant reached the `< 0.5` replay-ratio target on B** (best: **0.55** in
  `delta` / **0.62** in `root` with `partitioned/acc-cached`).
- **Full per-partition rehashing breaks scattered command loads** when
  per-resolution roots are required (A/C jump from **6–10 ms → 115–169 ms**
  median; even B is slower at 51 ms vs. 29 ms for cached per-record digests).
- **Synchronous full checkpoints (144 MB)** stall tick 100 for **237–435 ms**
  across Campaign 2 (outlier **1,485 ms**; Campaign 1: **247–326 ms**), proving
  checkpoint I/O must move off the tick critical path at scale.

### Decision

1. **Logical state narrow waist (zero application ontology):**
   World state is modeled as versioned owner domains backed by partitioned
   ordered tables `(DomainId, TableId, PartitionKey, RecordKey) -> OpaqueBytes`
   plus `ObligationRegistry`. No ECS archetypes or kernel-managed secondary
   indexes are added to the core. An extensible `TableKind` tag reserves room
   for additive columnar tables later without breaking v1.
2. **One mutation path for live execution and recovery:**
   All canonical state mutations pass through a single deterministic
   `apply(WriteSet)` path (never two diverging implementations like the
   prototype's `sim.rs` and `replay.rs`, and never full-world cloning for
   rollback).
3. **Freeze roots in the contract; keep proof mode and hash algorithm in the
   execution profile:**
   `Resolution` binds `pre_state_root`, `post_state_root`, and `events_root`.
   - **Mode / proof:** explicit `WriteSet` materialization (`delta`) is the
     **v1 default** (keeping D-014 recovery safe while startup plugins are
     unisolated native Rust and handling A/C workloads with 4.6×–11.6×
     headroom); a compact verified-tick proof mode (`root`) remains an explicit
     future profile option for dense workloads once isolation/determinism
     requirements are met.
   - **Hash algorithm** is a versioned profile parameter, never frozen to one
     cryptographic primitive in the root contract. While per-resolution roots
     are active, the hash must update incrementally per modified record rather
     than rehashing entire partitions.
   - `MigrationActivate` commits a **mandatory boundary checkpoint** so
     ordinary recovery never spans two execution profiles.
4. **One frozen public seam (Linux / Cordis discipline):**
   v1 freezes **only the public plugin, command, and observation contract**
   ([CONTRACT.md](../seej/docs/CONTRACT.md) and
   [PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md)). Internal
   infrastructure seams — `StateStore`, `AuthorityLog`, and `Executor` — remain
   clean internal traits tested with in-memory and fault-injecting doubles
   (replacing D-014's nine separately frozen public provider contracts under
   `K-05`). No internal seam is promoted to a frozen public ABI without a
   second real production provider and consumer.
5. **Parallel-safe pure contracts, batched sequential v1 + DST:**
   - Replace global mutable `IRng` and floating-point decisions with
     coordinate-derived integer `EntropyScope` (algorithm selected in execution
     profile). Eliminate global mutable ID counters and concurrent event
     accumulation.
   - Plugin entrypoints are pure, stateless, and batch/partition-oriented.
   - Any autonomous tick pass that observes an immutable $T-1$ snapshot (BSP
     semantics) must declare that read semantics explicitly in v1 even when run
     sequentially, so future multi-core execution preserves exact bit parity.
   - v1 ships a **batched sequential executor** (1 group commit per tick) paired
     with **single-threaded Deterministic Simulation Testing (DST)** to prove
     distributed consensus, crash recovery, migrations, and storage faults.
     Multi-core execution (`Block-STM`, parallel `BSP`), async off-path CoW
     checkpoints, `ColumnarTable`, `BoundedDelta`, and WASM isolation remain
     additive Gate 1-D work.
6. **Single-world CP authority + inter-world asynchronous federation:**
   One world remains one CP authority group (1 node or 3 shared-nothing nodes,
   quorum 2). Local partitions inside a world share its authority group and
   clock. Peer-to-peer decentralization and local autonomy operate **between
   autonomous worlds** via asynchronous envelopes and durable obligations,
   avoiding lockstep multi-group availability collapse ($p^N$).
7. **Close the `CancelCommand` / `CloseTick` liveness gap:**
   - The kernel reserves a dedicated bounded **control-plane budget** for
     authorized `CancelCommand` and migration-control operations, isolated from
     application quotas so saturated application limits can never return
     `LimitExceeded` to administrative cancellation.
   - `CloseTick(T)` cannot close tick `T` while an admitted `Pending` command
     targeting `T` remains unresolved; it either resolves normally or is
     resolved as `Cancelled` via the reserved control path.
8. **Clean-slate core reset and ordered coding phases:**
   The legacy spatial prototype (`Entity`/`Zone`, JSON snapshots, dual
   `sim.rs`/`replay.rs`, `server_d`/`sy_cli` fixtures, and inert placeholder
   crates) is removed rather than incrementally mutated. The workspace resets
   to a minimal 4-crate skeleton (`sy_types`, `sy_api`, `sy_core`, `sy_infra`)
   preserving the purity gate, pinned Rust `1.95.0`, license/advisory checks,
   and CI dependency-DAG rules, with in-code Rustdoc aligned to the ordered
   coding phases (`Phase 0` through `Phase 4`) in [ROADMAP.md](ROADMAP.md).

### Consequences

- Documentation is consolidated into a single non-redundant set (with a
  non-normative French mirror in `doc/fr/` and empirical lab evidence preserved
  in [BENCHMARKS.md](../seej/docs/BENCHMARKS.md)); `CONTRACT.md` advances to
  `0.4.0-draft` and `PLUGIN_CONTRACT.md` to `0.3.0-draft` (both unfrozen).
- **Archived baselines:** full pre-D-015 decision text and detailed Gate 1-K
  acceptance criteria (`seej/docs/phase1/EXIT_CHECKLIST.md` — including `K-08`
  crash boundaries, finite generation-seal thresholds, per-stable-version
  migration paths, and bounded authenticated replay-protected inter-node
  messages) remain archived at archive tag `archive/pre-d015-docs` (`9662a60`)
  and serve as reference acceptance criteria until each phase re-specifies and
  closes them with executable tests. The last runnable local prototype (with
  its fail-closed WAL, RNG v4, and `TickN` rollback fixes) is archived at
  `archive/prototype-v1-final` (`5841a00`), and the two lab campaigns are
  archived at `archive/lab-campaign-1` (`1c98282`) and `archive/lab-campaign-2`
  (`5856396`).
- `K-05` is refocused from nine public provider replacements to internal-seam
  conformance and fault-injection doubles (`StateStore`, `AuthorityLog`,
  `Executor`); `P2-01` is retired with the removal of the inert `sy_loader`
  placeholder.
- Storage engine choice (`redb`, `Fjall`, or custom paged store) and production
  hash construction remain internal profile choices to be benchmarked against
  real storage workloads when **Phase 1.4** begins.
- D-015 closes no Gate 1-K implementation item by itself; executable evidence
  in [ROADMAP.md](ROADMAP.md) remains mandatory before v1 freeze.

---

## Amendment Rules

- Any structural decision must be added here with context, evidence, decision,
  consequences, and explicit supersession links.
- Existing decisions are never silently rewritten; amendments add a new entry
  and update the status table.
