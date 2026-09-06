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
  v
pure transition engine
  |
  | deterministic validation and events
  v
authority group confirms one Resolution record
  |
  +--> Succeeded or Refused + complete events/transition
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
reply, the durable resolution is returned without reapplying effects.

## Command retry flow

1. Validate the kernel-issued generation and look up the full command identity
   in the durable admission namespace.
2. If its generation is at or below the durable closed-generation floor,
   return `Expired`; never execute it as new.
3. If an admitted entry exists and its fingerprint matches, return its durable
   state or pending result before evaluating whether the target tick is late.
4. If an admitted entry exists and its fingerprint differs, return `Conflict`.
5. If no entry exists in an open valid generation, continue normal admission;
   an identifier in an open generation that was never used is `NotFound` only
   for a status query.
6. If the authority group cannot answer safely, return `Unavailable` without
   fabricating a terminal status.

A producer may submit a new command with a new ID and future target tick after
an explicit late refusal.

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
include freshness.

When retention removes the cursor:

1. return explicit cursor expiration;
2. provide or identify a coherent current state;
3. issue a new cursor for subsequent events;
4. never claim that the removed interval remains available.

Reader backpressure can slow or reject the read path, but it cannot change the
canonical simulation.

## Migration flow

A migration proposal identifies the old and new execution profiles and all
kernel/plugin steps. The authority group pauses mutation at a confirmed
boundary, verifies artifacts and conversions, confirms the new profile and
state together, then resumes. Failure leaves the prior compatible state
authoritative.

No current binary implements this distributed flow. Its evidence is tracked by
the exit checklist.
