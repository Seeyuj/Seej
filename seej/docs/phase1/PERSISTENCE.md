# Phase 1 — Persistence and crash recovery (Snapshot + WAL)

This document describes the persistence contract **as implemented**.

## Files on disk

Given `--data-dir <BASE>` and a `world_id`, files are stored under:

```text
<BASE>/
  worlds/
    <world_id>/
      meta.json
      snapshot.json
      events           (WAL file; despite the name, it is a file path)
```

The `server_d` binary typically creates worlds under ids like `world_<seed>` (e.g. `world_42`).

## Snapshot

### Snapshot contents

- `snapshot.json` contains the full serialized `World` state.
- `meta.json` contains `WorldMeta`, including the crash-recovery cursor:
  - `snapshot_tick: Tick`
  - `last_event_id: EventId`

### Atomic write strategy (best-effort durability)

`FilesystemStore::save_snapshot` writes the snapshot using:

- write to `*.tmp`
- `fsync` the temp file
- atomic rename to the final path
- `fsync` the parent directory (where supported; best-effort on Windows)

This minimizes corrupted snapshots on crash/power loss.

## WAL (Write-Ahead Log)

### Role

The WAL is an **append-only** log of persisted simulation events. It is used for:

- crash recovery (replay after the snapshot cursor),
- operator inspection (`sy_cli events`),
- future compaction/rotation (not implemented in Phase 1).

### Binary record format (as implemented)

Each record is:

```text
MAGIC   : u32  (little-endian)  0x57414C31  // "WAL1"
VERSION : u16  (little-endian)  1
LENGTH  : u32  (little-endian)  payload byte length
EVENT_ID: u64  (little-endian)  monotonic per WAL
TICK    : u64  (little-endian)  simulated tick
PAYLOAD : [u8; LENGTH]          JSON bytes of sy_api::events::EventData
CRC32   : u32  (little-endian)  CRC32 over (MAGIC..PAYLOAD), excluding CRC field
```

### Crash safety behavior

- When reading, if a record is incomplete or the CRC does not match:
  - recovery **stops** at the first invalid record,
  - the implementation may **truncate** the file tail (removing the partial record).
- This makes “torn writes” detectable and avoids replaying corrupted data.

### Event IDs

`FileEventLog` assigns `event_id` on append, starting at 1 and incrementing monotonically.
The core should treat `event_id` as the durable cursor.

## Crash recovery algorithm

Crash recovery is performed during `LoadWorld`:

1. Load `snapshot.json` into an in-memory `World`.
2. Read WAL events from the log.
3. Filter the replay set using the snapshot cursor:
   - replay events where `event.event_id > meta.last_event_id`
4. Apply each event using the deterministic event applier (core):
   - `sy_core::replay::apply_event(&mut world, &event)`

This makes recovery robust even if:
- the snapshot is taken while the WAL already contains events for the same tick,
- multiple events share the same tick,
- the process crashes mid-record append.

## Important note: `truncate_after` reassigns IDs

The Phase 1 `IEventLog::truncate_after(event_id)` implementation is a simple rewrite:
- it reloads events up to `event_id`,
- deletes the WAL file,
- rewrites the kept events by appending them again.

Because append assigns fresh `event_id`s, the rewritten WAL will have **new** `event_id` values starting at 1.
This is acceptable for an operator/manual maintenance tool in Phase 1, but it is not a stable “compaction” mechanism yet.

