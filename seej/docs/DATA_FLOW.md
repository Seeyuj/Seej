# Data Flow

```text
status: CURRENT LOCAL FLOW + TARGET DISTRIBUTED FLOW
```

## Current local prototype

```text
server_d CLI command
        |
        v
local validation -> sy_core transition -> SimEvent batch
        |                                  |
        +---------------- append WAL <-----+
                                           |
                                accept in-memory state
                                           |
                                  optional snapshot
```

Today `server_d` is the mutating ingress. `sy_cli` is a read-only inspector
of local durable data. The local WAL stores resulting events rather than a
complete durable external-command lifecycle.

The FileStore flow is described exactly in
[phase1/PERSISTENCE.md](phase1/PERSISTENCE.md). It must not be generalized into
a distributed success guarantee.

## Target command flow

```text
producer
  |
  | command ID, payload fingerprint, target tick
  v
receiving node
  |
  | authenticate, bound, deduplicate, check compatibility
  v
authority group proposes and confirms canonical order
  |
  | confirmed Admission: envelope + sequence + Pending
  | authority term + configuration version
  v
pure transition engine
  |
  | deterministic validation and events
  v
authority group confirms one Resolution record
  |
  +--> Succeeded, Refused, or Cancelled + complete events/transition
  +--> apply a successful confirmed transition once
  +--> publish confirmed events/cursor/freshness
```

Internal implementations can pipeline or combine steps. Observable behavior
must preserve these boundaries:

- receipt is not admission;
- local append is not distributed confirmation;
- computation is not canonical application;
- a confirmed admission is durable `Pending`, not a confirmed application
  effect;
- resolution is the single commit point for the decision, events, and state
  transition;
- a response timeout is not proof of refusal;
- observers see only a confirmed prefix.

If a node fails after admission and before resolution, the next coordinator
finds `Pending` in confirmed history and recomputes from the same confirmed
predecessor and execution profile. If it fails after resolution and before the
reply, the durable resolution is returned without re-evaluating the command.
A fresh state instance may materialize that recorded resolution when recovering
from an older checkpoint; it does not create a second canonical effect.

Recovery tracks three positions: the authority group's `confirmed_cursor`, the
retained checkpoint's `checkpoint_cursor`, and each state instance's
`applied_cursor`. Starting at the checkpoint cursor, the node consumes retained
confirmed records in order. Each record's predecessor cursor and semantic hash
must match the materialized state, and state plus `applied_cursor` advance
together. Gaps or mismatches fail closed. Recovery delivers no external effect
directly; identified events may be redelivered for consumer deduplication.

An authorized `CancelCommand` follows the kernel control path. It identifies
the requester, reason, target command, and its own stable identity. It can win
only while the target is `Pending`; whichever terminal resolution is confirmed
first wins the race, and retries return that result. Cancellation has no
application effects and cannot erase obligations or already confirmed state.

## Command retry flow

1. Authenticate the producer and check its durable per-producer
   closed-generation floor before consulting generation records that compaction
   may have removed. At or below the floor, return `Expired`.
2. Verify the retained generation state and its confirmed issuance to this
   producer, then look up the full command identity.
3. If an admitted command is terminal and the confirmed tick has reached its
   recorded `expiration_tick`, return `Expired` before fingerprint comparison.
4. If an unexpired admitted entry exists and its fingerprint matches, return
   its durable state before evaluating whether the target tick is late.
5. If an unexpired admitted entry exists and its fingerprint differs, return
   `Conflict`.
6. If no entry exists in an `Open` valid generation, continue normal admission;
   a status query for that unused ID returns `NotFound`.
7. If no entry exists in a `Sealed` generation, a submission returns
   `GenerationSealed`; after generation closure, step 1 returns `Expired`.
8. If the authority group cannot answer safely, return `Unavailable` without
   fabricating a terminal status.

A producer may submit a new command with a new ID and future target tick after
an explicit late refusal.

## Generation and retention flow

1. Confirmed `OpenGeneration` creates the next monotonic generation for a
   producer and atomically seals its prior open generation.
2. A finite count, byte, or canonical-tick-age limit can also confirm
   `SealGeneration`. Commands ordered before the seal may be admitted; commands
   ordered after it are not.
3. A terminal resolution records its exact `expiration_tick`. Until that tick,
   its detailed status remains queryable.
4. At that tick, the status becomes `Expired` and compaction may retain only a
   tombstone proving that the ID cannot be reused.
5. Once every command in a sealed generation is terminal and expired, confirmed
   `CloseGeneration` advances the per-producer floor and allows its individual
   records and tombstones to be removed.

Finite per-producer and per-world bounds on non-closed generations, pending
commands, retained results, tombstones, and their bytes return `LimitExceeded`
before creating a generation or Admission when capacity is unavailable. A
later retry may succeed after closure frees capacity. No limit handling drops
pending work or extends status promises.

## Cross-owner transaction

```text
command
  -> resolve participating owners
  -> bind the transaction to a confirmed base cursor
  -> order owner sub-commands canonically
  -> apply each owner proposal to a private candidate state
  -> validate cross-domain invariants on the complete candidate
  -> confirm one Resolution: decision + events + state atomically
  -> expose all effects together
```

A deterministic refusal by any participant yields no owner effects. If the
confirmed head changed before confirmation, discard the candidate and
recompute the whole transaction from its canonical predecessor. Missing
compatibility, plugin crash, or timeout leaves the command pending or the
authority unavailable instead of manufacturing a refusal. External systems and
other worlds are coordinated asynchronously.

## Tick flow

A canonical tick is a confirmed input for one world. Commands scheduled for
the tick are ordered by their admitted sequence. A confirmed tick-closing
boundary prevents further admission for that tick, then the active rules run
in a deterministic order. Each command resolution and the resulting tick state
occupy an unambiguous confirmed order for the authority group.

Wall-clock cadence is orchestration input, never an implicit transition input.
A replica or observer cannot advance the tick independently.

## Read and resynchronization flow

An observer starts from a coherent confirmed state and cursor, receives
identified events after that cursor, and deduplicates on reconnect. Responses
include freshness. A current-confirmed state requires a quorum-backed read
barrier in the active term and configuration, and the serving node must have
applied through that barrier. Without it, the response is stale or unavailable.

When retention removes the cursor:

1. return explicit cursor expiration;
2. provide or identify a coherent confirmed state and cursor from the same cut;
3. pin that cut through stream handoff, or restart with another explicit
   expiration and replacement cut;
4. never claim that the removed interval remains available.

Reader backpressure can slow or reject the read path, but it cannot change the
canonical simulation.

## Migration flow

1. A confirmed `MigrationPrepare` identifies the old and target execution
   profiles, affected capabilities, conversion steps, required artifacts, and
   active membership configuration. It pauses new affected application
   admissions but does not yet select the conversion state.
2. Commands already admitted under the old profile reach `Succeeded`,
   `Refused`, or an explicitly authorized `Cancelled` decision. Application
   failure alone never manufactures a terminal result.
3. A confirmed `MigrationBarrier` fixes the final old-profile source cursor and
   semantic state after every affected command is terminal. Affected ticks and
   application transitions cannot pass that barrier.
4. The quorum required by the unchanged active configuration verifies target
   artifacts and derives the same migrated candidate and semantic hash from the
   barrier.
5. A confirmed `MigrationActivate` binds the barrier, target profile, and
   migrated state atomically, then progression resumes.

Membership changes and migration are serialized; a configuration change makes
an unfinished migration abort and restart under the new confirmed
configuration. Elections and authority terms may still advance. A failure
before activation can produce a confirmed abort under the old profile. A
failure after activation recovers the new state. An uncertain reply is resolved
from confirmed history; returning to the old profile is a new explicit
migration.

No current binary implements this distributed flow. Its evidence is tracked by
the exit checklist.
