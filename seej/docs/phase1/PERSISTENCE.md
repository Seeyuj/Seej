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
- `snapshot.json` is the authoritative crash-recovery unit. Its embedded
  `WorldMeta` contains the recovery cursor:
  - `snapshot_tick: Tick`
  - `last_event_id: EventId`
- `meta.json` is an inspectable mirror of the snapshot metadata. Snapshot
  recovery accepts a missing or stale `meta.json` only when the snapshot embeds a
  strictly newer coherent cursor for the same world id, seed, creation tick, and
  format version. Other metadata disagreement fails explicitly.

### Atomic write strategy

`FilesystemStore::save_snapshot` and `FilesystemStore::save_meta` write using:

- write to `*.tmp`
- `fsync` the temp file
- atomic replacement to the final path
- `fsync` the parent directory on Unix

This minimizes corrupted snapshots on crash/power loss.

On Windows, `FilesystemStore` uses `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` for the replacement step.

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
PAYLOAD : [u8; LENGTH]          JSON bytes of current WAL batch payload:
                                {"type":"Batch","data":{"events":[...]}}
CRC32   : u32  (little-endian)  CRC32 over (MAGIC..PAYLOAD), excluding CRC field
```

`append_batch` writes the whole batch as one WAL record. Recovery therefore either sees the complete batch or discards/truncates the partial record; it must not replay a valid prefix of a partially written batch as a half-tick.

For batch records, the header `EVENT_ID` and `TICK` are the `event_id` and `tick` of the last `SimEvent` in the batch. Recovery rejects a batch whose header does not match the last event.

### Crash safety behavior

- When reading, if a record is incomplete or the CRC does not match:
  - recovery **stops** at the first invalid record,
  - the implementation may **truncate** the file tail (removing the partial record).
- This makes “torn writes” detectable and avoids replaying corrupted data.

### Event IDs

`FileEventLog` assigns `event_id` on append, starting at 1 and incrementing monotonically.
Infrastructure treats `event_id` as the durable cursor.

New WAL entries are limited to reconstructable state-transition events. Legacy
`WorldLoaded` events remain replay-compatible no-ops. Legacy `WorldSaved`
events remain no-ops only when their event tick and payload tick match the
current recovered world tick; incoherent `WorldSaved` records are rejected during
replay. The Phase 1 architecture lock no longer writes lifecycle-only events to
the WAL.

`WorldCreated` is retained as the genesis state-transition event. During replay
it is idempotent and leaves an already coherent recovered world unchanged; the
full world state is reconstructed from `snapshot.json`, then advanced by WAL
events after the snapshot cursor.

### Snapshot format compatibility

`WorldMeta::format_version` is an exact compatibility gate in Phase 1. Loading a snapshot whose format version differs from the current format is refused with an explicit storage error. No implicit v2-to-v3 migration is performed because old snapshots do not contain the per-tick RNG checkpoint contract required for deterministic recovery.

## Crash recovery algorithm

Crash recovery is performed by the infrastructure runtime and shared CLI loader:

1. Load `snapshot.json` into an in-memory `World`.
2. Load `meta.json`, when present, and verify it either exactly matches the
   snapshot metadata or is an older mirror left by a crash between
   `save_snapshot` and `save_meta`.
3. Read all valid WAL events and verify the durable tail:
   - the durable last event matches `event_log.last_event_id()`
   - the WAL prefix is contiguous from `event_id == 1`
   - the durable last event covers the snapshot cursor
   - `durable_last_event_id >= snapshot.last_event_id`
   - otherwise recovery fails with `CorruptedState`
4. Filter the replay set using the snapshot cursor:
   - replay events where `event.event_id > snapshot.last_event_id`
5. Verify the replay range is contiguous through the durable WAL tail.
6. Apply each event using the deterministic event applier (core):
   - `sy_core::replay::apply_event(&mut world, &event)`

`WorldSaved` is intentionally replay-safe and must remain a no-op for old WALs
only when the record is coherent with the recovered world tick.
`TickProcessed` replay is also strict: the outer event tick must match the
payload tick, payload `sim_time` must match `SimTime::from_ticks(tick)`, and
`rng_state_after` must be present. Legacy WAL records that deserialize without
an RNG checkpoint are refused in Phase 1 recovery unless an explicit migration
has already rewritten them.
The snapshot cursor points to the last state event included in the snapshot.
If the WAL is absent, truncated, or corrupted before that cursor, Phase 1
refuses recovery rather than moving the cursor backward. Real compaction needs a
separate, explicit metadata contract.

Create refuses to reuse incomplete durable storage. If a world directory, orphan
WAL, `meta.json`, or `snapshot.json` exists without a coherent snapshot/meta
pair, `CreateWorld` fails explicitly instead of treating the id as free.

This makes recovery robust even if:
- the snapshot is taken while the WAL already contains events for the same tick,
- multiple events share the same tick,
- the process crashes mid-record append.

## Core boundary

`sy_core` does not own snapshot encoding, WAL append, filesystem paths, metadata
mirroring, or recovery orchestration. It only applies deterministic simulation
commands and deterministic replay events. The persistent runtime in `sy_infra`
commits events to the WAL before accepting the corresponding in-memory state and
rolls the core simulation back if the WAL append fails.

## Important note: `truncate_after` reassigns IDs

The Phase 1 `IEventLog::truncate_after(event_id)` implementation is a simple rewrite:
- it reloads events up to `event_id`,
- deletes the WAL file,
- rewrites the kept events by appending them again.

Because append assigns fresh `event_id`s, the rewritten WAL will have **new** `event_id` values starting at 1.
This is acceptable for an operator/manual maintenance tool in Phase 1, but it is not a stable “compaction” mechanism yet.
The trait method is deprecated to prevent accidental automated use.

