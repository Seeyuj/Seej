# Phase 1 — Persistence and Recovery

```text
status: CURRENT LOCAL PROVIDER + TARGET DISTRIBUTED GUARANTEES
```

This document describes the implemented FileStore format and separates it from
the persistence guarantees required by the target kernel. Other providers may
use different physical formats after they pass the same public conformance
contract.

## Current files

With `--data-dir <BASE>` and `world_id`, the prototype stores:

```text
<BASE>/
  worlds/
    <world_id>/
      meta.json
      snapshot.json
      events
```

`events` is a WAL file despite its name. `server_d` currently creates IDs
such as `world_42`; this seed-derived identity is scheduled for replacement.

## Current snapshot behavior

`snapshot.json` contains the serialized `World` and its recovery cursor:
`snapshot_tick` and `last_event_id`. `meta.json` mirrors inspectable
metadata.

Recovery accepts a missing or older `meta.json` only when the snapshot embeds
a strictly newer coherent cursor for the same current-format world, seed,
creation tick, and version. Other disagreement fails explicitly.

Snapshot and metadata writes use a temporary file, file synchronization,
atomic replacement, and parent-directory synchronization on Unix. Windows uses
`MoveFileExW` with replacement and write-through flags. Durable directory,
WAL creation, and first-append behavior remain open under `P1-10`.

## Current WAL format

Each record is:

```text
MAGIC    u32 LE   0x57414C31 ("WAL1"; bytes 31 4C 41 57)
VERSION  u16 LE   1
LENGTH   u32 LE   payload byte length
EVENT_ID u64 LE   last event ID in the batch
TICK     u64 LE   last event tick in the batch
PAYLOAD  bytes    JSON {"type":"Batch","data":{"events":[...]}}
CRC32    u32 LE   CRC32 over MAGIC through PAYLOAD
```

`append_batch` writes a complete event batch as one record. Recovery validates
the record header against the final event and never replays a valid-looking
prefix of a partial batch.

FileEventLog assigns monotonically increasing event IDs starting at 1. Legacy
`WorldLoaded` is a replay no-op. Legacy `WorldSaved` is a no-op only when its
outer and payload ticks match recovered state. `WorldCreated` is idempotent on
an already coherent snapshot. `TickProcessed` requires matching outer/payload
ticks, derived simulation time, and an RNG checkpoint.

This is an event-result WAL. It is not yet the durable command and confirmed
history required by K-08 and K-07.

## Current recovery algorithm

1. Decode `snapshot.json`.
2. Validate `meta.json` as equal or an allowed older mirror.
3. Read WAL records through the first invalid record.
4. Verify durable event IDs are contiguous from 1 and cover the snapshot
   cursor.
5. Select events after `snapshot.last_event_id`.
6. Verify the replay range is contiguous.
7. Apply each event through `sy_core::replay::apply_event`.
8. Refuse incoherent ticks, RNG state, cursors, formats, or missing prefix.

The current format version is 3 and requires an exact match. No implicit
migration from an older format exists.

Creation refuses an existing directory, orphan WAL, metadata, or snapshot that
does not form a coherent current world.

## Current torn-tail and corruption behavior

The reader stops at an incomplete record or CRC mismatch and can truncate the
invalid tail. This protects ordinary torn appends but does not yet distinguish
every expected partial write from suspicious corruption.

Before the first stable contract, `P0-13` requires explicit classification:
safe torn-tail recovery, refusal, quarantine, and operator repair must be
separate modes that preserve evidence.

## Current write failure behavior

The local persistent runtime rolls back the in-memory transition when WAL
append fails before acceptance. This is useful evidence, not complete coverage.

Gap `P0-14` requires injected tests for:

- full and read-only media;
- generic I/O failure;
- failed WAL append or synchronization;
- failed snapshot/metadata synchronization;
- failed replacement or rename.

The required outcome is no durable success, observer publication, later tick,
or accepted memory-only mutation after a critical failure. Recovery cannot
accept a new metadata file against an absent, partial, or older snapshot.

## Target distributed persistence

The target distinguishes:

1. received proposal;
2. local durable staging;
3. confirmed admission with canonical sequence and `Pending` state;
4. deterministic resolution computation from the confirmed predecessor;
5. confirmed resolution containing decision, events, and complete transition;
6. one-time state application and public event/status publication.

A local WAL append does not prove distributed confirmation. The authority
provider defines the quorum rule under
[NODE_NETWORK.md](../NODE_NETWORK.md). Storage providers expose sufficient
durability and fencing behavior for that rule.

Confirmed admission and resolution records bind world identity, genesis,
execution profile, authority term, configuration version, full command identity
and fingerprint, order, decision, events, and state/checkpoint cursor. Recovery
resumes a confirmed pending admission and never applies an already confirmed
resolution twice.

## Retention and compaction

The current `truncate_after(event_id)` rewrites the WAL and reassigns IDs. It
is deprecated and suitable only as an explicit manual recovery mechanism. It is
not stable compaction.

K-10/P1-19 replace it with retention-aware checkpointing that preserves:

- stable external cursors or explicit expiration;
- command-ID non-reuse;
- active obligations;
- confirmed causal linkage;
- a coherent recovery point;
- migration and compatibility metadata.

Terminal command results may be summarized only after their tick-based status
windows expire and their kernel-issued generation closes through a confirmed
operation. A durable per-producer closed-generation floor then rejects the
entire old namespace. Pending admissions, open generations, and their required
execution profile are never removed.

Full archival is optional. Genesis replay is unavailable when required history
or artifacts were deliberately removed.

## Backup

Only cold backup is currently supportable as an operator procedure: stop the
writer, copy the complete world directory, restore it elsewhere, and run
ordinary recovery. A raw copy while the writer is active is not guaranteed
coherent. Hot backup remains unclaimed until a tested protocol exists.

## Boundaries

`sy_core` owns deterministic application and integrity logic, not file paths,
JSON, WAL encoding, network replication, snapshots, or backup. FileStore is the
current `sy_infra` reference provider. Future providers can change bytes and
layout while preserving the root contract.
