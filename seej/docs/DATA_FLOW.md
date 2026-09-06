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
  | confirmed command envelope + authority epoch
  v
pure transition engine
  |
  | deterministic validation and events
  v
authority group durably confirms complete transition
  |
  +--> durable command status
  +--> apply confirmed state
  +--> publish confirmed events/cursor/freshness
```

Internal implementations can pipeline or combine steps. Observable behavior
must preserve these boundaries:

- receipt is not admission;
- local append is not distributed confirmation;
- computation is not canonical application;
- a response timeout is not proof of refusal;
- observers see only a confirmed prefix.

## Command retry flow

1. Look up the command ID in the durable admission namespace.
2. If the fingerprint matches, return the known status or pending result.
3. If the fingerprint differs, return `Conflict`.
4. If the status window elapsed, return `Expired`; never execute it as new.
5. If the authority group cannot answer safely, return `Unavailable` without
   fabricating a terminal status.

A producer may submit a new command with a new ID and future target tick after
an explicit late refusal.

## Cross-owner transaction

```text
command
  -> resolve participating owners
  -> validate all parts against one immutable pre-state
  -> produce one complete transition
  -> confirm command + events + status atomically
  -> expose all effects together
```

Failure, refusal, or missing compatibility for any participant yields no owner
effects. External systems and other worlds are coordinated asynchronously.

## Tick flow

A canonical tick is a confirmed input for one world. Commands scheduled for
the tick are ordered by their admitted sequence, then the active rules run in
a deterministic order. The resulting tick and state are confirmed once for the
authority group.

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
