# Seej Kernel Contract

```text
contract_version: 0.1.0-draft
status:           DRAFT — NOT FROZEN
```

## Status

This document is the **kernel contract**: the durable interface between the
Seej world kernel and everything that consumes it (observers, agents, future
netcode, tools). It is the artifact a third party should be able to build
against **without reading the Rust source**.

It is currently a **draft**. Freezing it as v0 is Gate 1-K gap `K-01` in
[`phase1/EXIT_CHECKLIST.md`](phase1/EXIT_CHECKLIST.md). Until frozen, every
statement below is either:

- **Binding today** — implemented and tested in the current repository; or
- **Target (gap ID)** — a named gap in the exit checklist; the contract states
  the destination, the checklist tracks the work.

Change policy after freeze: **we do not break replay.** A change that alters
how a persisted journal re-executes is a new `contract_version` and a new
`simulation_contract` value (P0-05), never a silent edit.

The long-form architecture behind this contract lives in
[`simulation/WORLD_SPEC.md`](simulation/WORLD_SPEC.md). This document is the
short, binding subset.

## 1. The Model: a Closed System with One Journaled Aperture

A Seej world is a causally closed system:

- **Nothing mutates world state except a command** entering through the single
  aperture (`SimCommand`, section 5). *Binding today* inside the process;
  *Target (P0-08, P0-09)* as a durable, ordered command journal.
- **Every accepted mutation is journaled** as events (section 6) before the
  in-memory state is accepted. *Binding today.*
- **All randomness is lawful**: a seeded, persisted RNG. Events look random
  from inside the world; from outside, every transition is reproducible.
  *Binding today.*
- **Recovery is replay**: restart = load snapshot + re-apply journal after the
  cursor. There is no other recovery mechanism. *Binding today.*

### Guarantee tiers

| Tier | Guarantee | Status |
| --- | --- | --- |
| 1 | Causal closure: no effect without a journaled cause | Binding (in-process); durable command journal is Target (P0-08) |
| 2 | Deterministic re-execution: same binary + same journal ⇒ same state | Binding; proven by recovery-parity and determinism tests |
| 3 | Long-horizon replay: bit-perfect across versions/platforms/years | Opt-in Target (Gate 1-D, Tier-3 trigger group) |

## 2. World Identity and Genesis

*Binding today:*

- A world is created from a mandatory `RngSeed` (`u64`). Creation without a
  seed is refused.
- `world_id` is the string `world_<seed>` (e.g. `world_42`).
- Creation auto-creates the origin zone (`ZoneId 0`, named "Origin").
- Creation refuses to reuse incomplete durable storage (orphan WAL or partial
  snapshot/meta under the same id).

*Target:*

- A persisted, hashable, versioned `GenesisSpec` — seed, initial topology,
  initial entities, schema assumptions, initial rule contract (P0-06).
- `world_id` decoupled from seed; identity bound to a genesis hash, with
  recovery rejecting `world_id`/`genesis_hash`/state mismatches (P0-07).
- The current CLI population flags (`--resources N --creatures M`) become part
  of the recorded genesis instead of post-creation commands (P0-06).

## 3. Time Model

*Binding today:*

- `Tick(u64)` is the only canonical clock. It starts at 0 and advances by
  exactly 1 per tick command.
- `SimTime { units: u64 }` is derived: 1 tick = 1 unit in Phase 1.
- The kernel never reads wall-clock time; the purity gate enforces this
  mechanically (section 8).
- One tick executes: advance counters → run systemic rules over active
  entities in ascending `EntityId` order → persist RNG state into world state
  → emit `TickProcessed`.

*Target:* canonical counters fail closed at their maximum instead of
saturating (P0-04).

## 4. Core Types (wire-visible)

*Binding today* (serde/JSON shapes as serialized by the current binary):

| Type | Shape | Notes |
| --- | --- | --- |
| `Tick` | `u64` | display form `T<n>` |
| `SimTime` | `{ "units": u64 }` | derived from ticks |
| `EntityId` | `u64` | `0` is INVALID; allocation starts at 1, monotonic |
| `ZoneId` | `u32` | `0` is the origin zone |
| `EventId` | `u64` | `0` means "not yet assigned"; assigned by the WAL from 1, monotonic per world |
| `RngSeed` | `u64` | mandatory at genesis |
| `Position` | `{ "x": i32, "y": i32, "z": i32 }` | zone-local |
| `WorldPos` | `{ "zone": ZoneId, "pos": Position }` | |
| `EntityKind` | `"Resource" \| "Creature" \| "Item" \| "Structure"` | non-exhaustive: consumers must tolerate unknown kinds |
| `EntityState` | `"Active" \| "Dormant" \| "Dead"` | |
| `EntityProperties` | `{ "name": string?, "amount": u32?, "health": u32? }` | |

*Target:* these shapes become explicit persisted DTOs, versioned independently
from the Rust domain types (P1-07); event/payload variants gain explicit
version tags (P1-06, Tier 3).

## 5. The Aperture: Commands

*Binding today* — the kernel accepts exactly these `SimCommand` variants:

| Command | Payload | Effect |
| --- | --- | --- |
| `CreateWorld` | `{ name: string, seed: RngSeed }` | creates world + origin zone; refused if a world is loaded |
| `Tick` | — | advances the world by one tick |
| `TickN` | `u32` | N sequential ticks (identical to N `Tick` commands) |
| `SpawnEntity` | `{ position: WorldPos, kind: EntityKind, properties: EntityProperties }` | refused if the zone does not exist |
| `DespawnEntity` | `EntityId` | refused if the entity does not exist |
| `CreateZone` | `{ zone_id: ZoneId, name: string? }` | refused if the zone exists |

Command semantics, *binding today*:

- Commands are validated before execution; validation failure produces a typed
  error and **no** state change.
- Execution is transactional: on any error the kernel rolls back world state,
  RNG state, clock, and pending events to the pre-command checkpoint.
- A successful command returns the list of events it produced; those events
  are the only record of what changed.
- There are no player commands, no persistence/admin commands, and no partial
  successes in Phase 1.

*Target:* every externally submitted command is persisted as a durable
`CommandEnvelope` (`command_id`, `world_id`, target `tick`, `command_seq`,
payload, source metadata) before execution (P0-08), with canonical
`(tick, command_seq)` ordering for concurrent producers (P0-09). The envelope
journal — not arrival timing — defines history.

## 6. Events (the only outputs)

*Binding today* — `SimEvent` is `{ event_id: EventId, tick: Tick, data: EventData }`,
with `EventData` one of:

| Variant | Payload | Emitted when |
| --- | --- | --- |
| `WorldCreated` | `{ world_id, name, seed }` | genesis (idempotent on replay) |
| `WorldLoaded` | `{ world_id, tick }` | legacy; replay no-op; no longer written |
| `WorldSaved` | `{ tick }` | legacy; replay no-op only if coherent with recovered tick; no longer written |
| `TickProcessed` | `{ tick, sim_time, entities_processed, rng_state_after? }` | each tick; `rng_state_after` is required by Phase 1 replay |
| `ZoneCreated` | `{ zone_id, name? }` | zone creation (incl. origin at genesis) |
| `ZoneLoaded` / `ZoneUnloaded` | `{ zone_id }` | reserved; not emitted by current rules |
| `EntitySpawned` | `{ entity_id, kind, position, properties }` | spawn |
| `EntityDespawned` | `{ entity_id, reason }` | despawn; `reason ∈ Command, Death, Depleted, Expired` |
| `EntityMoved` | `{ entity_id, from, to }` | reserved; not emitted by current rules |
| `EntityStateChanged` | `{ entity_id, old_state, new_state }` | lifecycle transitions |
| `EntityPropertyChanged` | `{ entity_id, property, old_value, new_value }` | generic property change; `PropertyValue ∈ None, Int(i64), UInt(u64), Float(f64), Bool, String` |
| `ResourceDepleted` | `{ entity_id, amount, remaining }` | systemic rule |
| `EntityDegraded` | `{ entity_id, old_health, new_health }` | systemic rule |

Consumer obligations, *binding today*:

- Treat the event stream as the source of truth for state changes; do not
  infer hidden transitions.
- Tolerate unknown future variants only by refusing clearly (typed error), not
  by skipping silently.
- `PropertyValue::Float` exists in the vocabulary but is scheduled for
  restriction (P0-03 / P1-08); consumers should not depend on float property
  semantics.

## 7. World State

*Binding today:*

- The persistent `World` is: `WorldMeta` + `current_tick` + `sim_time` +
  `rng_state: u64` + `next_entity_id: u64` + ordered maps of entities and
  zones (`BTreeMap` — deterministic iteration is a state invariant, not an
  implementation detail).
- `WorldMeta` is `{ world_id, name, seed, current_tick, sim_time,
  created_tick, snapshot_tick, last_event_id, format_version }`.
- `format_version` is currently `3` and is an exact-match compatibility gate:
  loading any other version is refused; no implicit migration exists.

*Target:* a persisted `simulation_contract` / `ruleset_version` alongside
`format_version`, included in compatibility checks and canonical hashing
(P0-05); a pure `validate_world_integrity` pass on load/replay (P0-12).

## 8. Determinism Rules (kernel physics)

*Binding today, enforced by the CI purity gate over `sy_core`:*

- No wall-clock time (`SystemTime`, `Instant`), no environment access, no OS
  randomness, no filesystem I/O, no networking, no `HashMap`/`HashSet` in the
  kernel.
- RNG and clock are injected ports (`IRng`, `ISimClock`); RNG state is
  persisted in world state and checkpointed per tick (`rng_state_after`).
- Same genesis + same command stream + same ordering ⇒ same events and same
  state at every checkpoint (tested by run-twice hash comparison).

*Known v0 defect, tracked, not hidden:* the Phase 1 systemic rules draw
chances via floating-point probabilities (`chance(0.01)`, `chance(0.005)`).
This is deterministic on a fixed binary but is scheduled to move to
integer/fixed-point decisions (P0-03). When it changes, the ruleset identity
changes with it (P0-05) — an explicit contract bump, not a silent edit.

### Systemic rules v0 (owned by this contract until P1-16 extraction)

Per tick, over active entities in ascending `EntityId` order:

- A `Resource` with `amount > 0` loses 1 unit with probability 1% → emits
  `ResourceDepleted`; at 0 it becomes `Dead` (`EntityStateChanged`).
- A `Creature` with `health > 0` loses 1 health with probability 0.5% → emits
  `EntityDegraded`; at 0 it becomes `Dead` (`EntityStateChanged`).
- Every 100th tick (`tick % 100 == 0`), `Dead` entities are removed →
  `EntityDespawned { reason: Death }`.

## 9. Persistence Formats

*Binding today* — a world lives under `worlds/<world_id>/` as:

- `snapshot.json` — the full serialized `World`; authoritative recovery unit;
  written via temp file + fsync + atomic replace (Windows:
  `MoveFileExW(..., REPLACE_EXISTING | WRITE_THROUGH)`).
- `meta.json` — inspectable mirror of the snapshot metadata; staleness rules
  in [`phase1/PERSISTENCE.md`](phase1/PERSISTENCE.md).
- `events` — the append-only WAL. Binary record layout (little-endian):

```text
MAGIC   : u32  0x57414C31 ("WAL1")
VERSION : u16  1
LENGTH  : u32  payload byte length (max 16 MiB)
EVENT_ID: u64  header cursor = last event of the batch
TICK    : u64  header tick   = last event of the batch
PAYLOAD : [u8] JSON {"type":"Batch","data":{"events":[SimEvent...]}}
CRC32   : u32  over MAGIC..PAYLOAD
```

- A batch is atomic: recovery replays a record fully or not at all; a torn
  tail is detected by length/CRC and never replayed as a half-tick.
- `event_id` is assigned by the WAL on append, from 1, monotonic; it is the
  durable recovery cursor.

Recovery algorithm (*binding today*): load snapshot → verify meta coherence →
verify WAL contiguity from `event_id == 1` through the durable tail and
coverage of the snapshot cursor → replay events with
`event_id > snapshot.last_event_id` through the pure applier
(`sy_core::replay::apply_event`) → refuse (`CorruptedState`) on any
incoherence. Details and edge policies: [`phase1/PERSISTENCE.md`](phase1/PERSISTENCE.md).

*Target:* WAL/manifest bound to `world_id` + `genesis_hash` +
`simulation_contract`, with replay refusing mismatches (P0-11); snapshot/meta
binding checksum (P1-09); durable command journal alongside the event WAL
(P0-08).

## 10. Canonical Hashing

*Binding today:*

- `compute_canonical_hash` (xxhash64) hashes the complete persistent `World`
  in stable field/collection order, including recovery cursors
  (`snapshot_tick`, `last_event_id`). Comparing a continuous run to a
  recovered run requires normalizing cursors first (save both sides, then
  hash).
- Purpose in v0: determinism validation and recovery-parity testing. It is a
  diagnostic hash, not yet a cross-platform golden value.

*Target:* semantic-state hashing separated from storage-layout hashing
(P1-04); cross-platform golden parity when Tier 3 is claimed (P1-03).

## 11. Observation Surface

*Binding today:* read-only inspection via `sy_cli` — `status`, `events`
(with `--from-tick`/`--count`), `dump --pretty`, `entities`, `entity`,
`zones`. Read-only opens never truncate or repair the WAL; corruption is
reported, not mutated.

*Target (K-02):* a streaming, follow-mode event surface emitting the section-6
shapes as JSON Lines, with an explicit concurrent-read policy (P1-11), per
[`phase1/OBSERVATION_SLICE.md`](phase1/OBSERVATION_SLICE.md). This is the
first userland brick; renderers, dashboards, bots, and analysis tools are
consumers of this surface and never gain write access through it.

## 12. Out of Contract

The following may change freely between releases without a contract bump:

- log formats and verbosity (`tracing` output);
- CLI flag names and help text (the *shapes* of emitted JSON are contract;
  the flags that produce them are not, until K-02 freezes the observation
  surface);
- internal crate layout, module boundaries, and dependency choices;
- performance characteristics, memory usage, and tick throughput;
- anything in `target/`, examples, and test scaffolding.

If a consumer depends on something in this list, that dependency is the
consumer's bug. If a consumer must depend on something in this list, that is a
contract change request.
