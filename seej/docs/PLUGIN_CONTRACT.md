# Seej Public Plugin Contract

```text
contract_version: 0.3.0-draft
status:           DRAFT — NOT FROZEN (gap K-01)
```

This document defines the **single public boundary targeted for freeze in v1**
(currently `DRAFT — NOT FROZEN` until `K-01`): the contract between the Seej
world kernel and application plugins. It must remain usable by third-party
authors without reading `sy_core` or internal storage/consensus code.

---

## 1. One Target Frozen Public Seam

Following [D-015](../../doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam):

- **Target frozen boundary in v1 (`K-01`):** the **application plugin
  contract** defined here (together with public command and observation schemas
  in [CONTRACT.md](CONTRACT.md)).
- **Target internal seams in v1 (not frozen publicly):** `StateStore`,
  `AuthorityLog`, and `Executor`. These are designed as internal Rust traits to
  be verified with in-memory and fault-injecting doubles (`K-05`), evolving
  freely without breaking plugins.
- **Trust & loading model:** v1 loads only operator-approved trusted native
  plugins at process startup. Hot loading, hot replacement, and isolated
  execution (e.g. WASM with deterministic fuel metering and NaN
  canonicalization) are Gate 1-D capabilities designed to preserve this same
  logical contract.

---

## 2. Manifest and Compatibility

Every plugin declares an immutable manifest specifying:

- stable component and publisher identifiers;
- semantic version and cryptographic artifact fingerprint;
- target kernel/plugin contract version;
- owned domains, table declarations (`TableKind`, format versions), public
  command types, and declared readable views;
- dependencies, permissions, and resource limits;
- deterministic execution profile requirements (including entropy algorithm);
- data migration steps from prior stable versions;
- lifecycle obligation types that block silent removal.

Startup activation verifies every manifest, fingerprint, permission,
dependency, and data version before allowing world mutations. Missing,
mismatched, or incompatible required plugins fail closed and block activation,
migration, or re-simulation.

---

## 3. Domain Data and Ownership

Every mutable application domain has **one declared logical owner**. Other
plugins may read permitted views and mutate another owner's data **only** by
invoking public commands owned by that domain inside a kernel transaction.

### Logical Data Primitives (`K-06`)

The kernel imposes no mandatory space, entity, zone, item, creature, economy,
or player model. Domains store canonical state exclusively through kernel
containers — **zero hidden canonical state** may live in plugin heap memory:

1. **`OrderedTable` (v1 baseline):**
   `(DomainId, TableId, PartitionKey: Bytes, RecordKey: Bytes) -> OpaqueBytes`
   sorted lexicographically.
   - Opaque single-blob domains are supported trivially (1 table, 1 partition,
     1 key).
   - Fine-grained domains use entity/account keys; dense simulations group
     records into contiguous `PartitionKey` buckets for cache locality and
     future multi-core execution.
   - Canonical secondary indexes are ordinary plugin-owned `OrderedTable`s
     updated in the same atomic `WriteSet`. Observer-only indexes stay outside
     canonical state.
   - `TableKind` is extensible to allow additive `ColumnarTable` (fixed scalar
     columns in bounded row chunks) in Gate 1-D without breaking v1 schemas.
2. **`ObligationRegistry` (v1 baseline):**
   Kernel-visible obligation envelopes `(obligation_id, owner_domain,
   counterparty_domain, status, due_tick, opaque_payload)` preserved even when
   the payload is opaque.

---

## 4. Pure Batch Execution and Transactions

To preserve determinism, avoid per-entity call overhead, and remain compatible
with future WASM sandboxing:

- **Pure, flat-batch entrypoints:** plugins are never invoked via per-entity
  callbacks (`on_tick(entity)`) or direct `&mut World` access. They receive an
  immutable `StateView` plus a batch of commands or a partition work unit and
  return deterministic outputs:
  ```text
  (StateView, InputBatch, EntropyScope, Context)
      -> Decision + WriteSet + EventBatch + OutboxIntents
  ```
- **No I/O or OS threads:** plugins never perform filesystem/network I/O, read
  wall clocks, use ambient randomness, or spawn OS threads.
- **Cross-owner transactions:**
  1. Bind to one confirmed `base_cursor` and order owner sub-commands
     canonically.
  2. Each owner validates against the candidate overlay produced by earlier
     sub-commands and proposes writes only to its own domain.
  3. Cross-domain validators inspect the complete candidate (preferring
     delta-oriented checks over full-domain scans so future parallel schedulers
     avoid unnecessary serialization).
  4. All owner writes and events confirm atomically, or none apply. Stale base
     cursors discard the candidate and recompute in full.
- **Optional access hints:** commands may declare optional partition/domain
  access hints (`AccessHint`) for routing and future parallel scheduling without
  requiring rigid static key lists for dynamic workloads.
- **Failure semantics:** only deterministic contract evaluation produces
  `Refused`. Plugin panics, timeouts, or missing artifacts leave the command
  `Pending` (recoverable via fix or authorized `CancelCommand`) and never
  fabricate a business refusal or partial effect.

---

## 5. Entropy and Read Semantics

- **Derived `EntropyScope`:** plugins receive a context-bound `EntropyScope`
  providing integer-only deterministic draws (`fork(stable_key)`, `next_u64`,
  bounded ranges, integer ratios — no `f32`/`f64` decisions). Forking by stable
  semantic key isolates plugins from one another and keeps draws invariant
  across 1 or $N$ cores.
- **Explicit $T-1$ vs. sequential read semantics (`D-015`):**
  - Command/transaction handlers observe exact sequential state in canonical
    `(target_tick, admission_seq)` order.
  - Autonomous tick passes that read an immutable $T-1$ snapshot and emit
    intents reduced in canonical order (BSP-style) **must declare $T-1$
    snapshot semantics explicitly** even on the v1 sequential executor. This
    guarantees `root(1 core) == root(N cores)` when parallel execution is
    enabled later.
- **Cause-derived IDs & canonical events:** plugins must not rely on shared
  mutable global counters across independent partitions; generated identifiers
  and events derive deterministic ordering from their canonical cause.

---

## 6. Obligations and Lifecycle

- **Migrations:** plugins own migrations for their owned tables and must supply
  deterministic conversion steps across versions. Missing steps block world
  migration (`K-09`).
- **Removal / replacement:** a plugin with owned data or active obligations
  cannot be detached merely by removing its binary. Removal requires explicit
  data migration plus either confirmed transfer of every active obligation to
  an activated compatible successor or confirmed resolution under world laws.
  Archival may detach code only when obligations are terminal or when the
  entire world is archived and cannot advance.
- **Historical artifacts:** operators archive plugin binaries needed for
  historical re-simulation; the kernel records exact fingerprints and refuses
  re-simulation when referenced artifacts are unavailable.

---

## 7. Optional Community Extensions

Spatial engines, economies, agents/AI, renderers, ingame netcode, and
multiresolution systems are optional community plugins, not kernel
deliverables.

### Multiresolution / Causal Resolution (Optional Pattern)

A community multiresolution plugin may represent inactive regions or groups at
coarser granularity and materialize detail later **only** through admitted
canonical commands under a world-declared approximation policy:

- Established past facts, protected identities, laws, and active obligations
  remain exact and cannot be approximated away.
- Future outcomes may diverge from a hypothetical fully detailed run only where
  the declared policy explicitly permits it; determinism means equal inputs and
  policy produce equal transitions.
- **Observer neutrality:** camera movement, subscriptions, or observer queries
  never trigger canonical materialization or change simulation fidelity.
  Fidelity changes enter only as validated canonical commands.

---

## 8. v1 Conformance Evidence

Before `K-01` freezes this contract, the test suite must prove:

1. **Real external write extension (`K-04`):** maintained outside `sy_core`,
   importing only public contracts, declaring non-spatial `OrderedTable` data,
   commands, and validators, surviving crash recovery.
2. **Transaction & obligation fixtures (`K-06`):**
   - with a non-negative balance law, two ordered withdrawals of 7 from a
     balance of 10 cannot both succeed;
   - payment + item transfer across two owners confirm atomically or not at all;
   - a stale candidate is discarded and recomputed in full;
   - a participant panic/failure produces no partial effect and no fabricated
     `Refused`;
   - an active future-dated obligation blocks owner removal until confirmed
     transfer or resolution;
   - a declared $T-1$ autonomous pass produces identical roots regardless of
     partition evaluation order.
