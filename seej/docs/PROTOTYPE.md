# Current Local Prototype Reference

```text
status: IMPLEMENTED LOCAL PROTOTYPE — REPLACEABLE BASELINE (NOT THE v1 CONTRACT)
```

This document describes what actually runs in `seej/server/` today: CLI
binaries, on-disk formats, determinism/purity checks, known limitations, and
empirical measurements from the throwaway workload lab.

Per [D-015](../../doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam),
this prototype's fixed `Entity`/`Zone` types, degradation rules, JSON snapshots,
and single-writer runtime are **reference fixtures**, not the target v1 kernel
API. Rebuilding the generic v1 core cleanly from scratch is explicitly
preferred over stretching these types.

---

## Scope and Replacement Policy

Implemented today in `seej/server/`:

- headless single-process `server_d` daemon and `sy_cli` inspection CLI;
- explicit `--seed` world creation with SplitMix64-mixed initial RNG (format v4)
  and backward-compatible continuation of format v3 worlds;
- deterministic tick execution over `BTreeMap` collections;
- fail-closed v1 WAL (CRC32-protected batch records) + atomic JSON snapshots;
- strict event replay and `TickN` batch rollback;
- `sy_core` static purity gate (`scripts/check_sy_core_purity.py`), workspace
  dependency check, recovery parity `proptest`, forced-kill test, 100k burn-in,
  and isolated WAL fuzz targets.

Not implemented in this prototype: generic plugin domains, `OrderedTable` /
`ObligationRegistry`, single `apply(WriteSet)` path, distributed consensus,
network observation, durable command envelopes, or schema migrations.

---

## Binaries and Commands

All commands run from `seej/server/` with the pinned toolchain
(`rust-toolchain.toml`):

```bash
cargo build --workspace
cargo test --workspace --all-targets
```

### `server_d`

```bash
# Create a seeded demonstration world (seed required; creates format v4)
cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- create --name "MyWorld" --seed 42 --resources 10 --creatures 5

# List local worlds
cargo run --bin server_d -- list

# Run simulation (--ticks 0 runs until Ctrl+C graceful save; --save-interval in ticks)
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100

# Override storage directory (default: ./data)
cargo run --bin server_d -- --data-dir ./my_data list
```

### `sy_cli`

```bash
cargo run --bin sy_cli -- status world_42
cargo run --bin sy_cli -- events world_42 --count 50
cargo run --bin sy_cli -- events world_42 --from-tick 1000 --count 50
cargo run --bin sy_cli -- dump world_42 --pretty
cargo run --bin sy_cli -- entities world_42 --kind creature
cargo run --bin sy_cli -- entity world_42 1
cargo run --bin sy_cli -- zones world_42
```

---

## Storage and WAL Format

With `--data-dir <BASE>` and `<world_id>` (currently seed-derived `world_<seed>`):

```text
<BASE>/worlds/<world_id>/
  meta.json      # mirrored WorldMeta
  snapshot.json  # serialized WorldSnapshotDto + recovery cursor
  events         # binary v1 WAL file (JSON batch payloads + CRC32)
```

### Snapshot Formats (v3 / v4)

- **Format v4 (current default for new worlds):** initializes `rng_state` with
  SplitMix64's finalizer over the seed (fixing the v3 defect where every raw
  seed `< 2^27` produced `0` on the first PCG32 `next_u32()` draw).
- **Format v3 (legacy read/continue):** loaded and continued without migration;
  saving preserves the world's existing format version (`3` or `4`). Other
  versions are refused.
- Snapshot and `meta.json` writes use temp file + `sync_all` + atomic rename
  (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH` on
  Windows; `rename` + parent dir sync on Unix).

### WAL v1 Binary Record Format

```text
MAGIC    u32 LE   0x57414C31 ("WAL1")
VERSION  u16 LE   1
LENGTH   u32 LE   payload length (1..=16 MiB)
EVENT_ID u64 LE   last event ID in batch (nonzero)
TICK     u64 LE   last event tick in batch
PAYLOAD  bytes    JSON {"type":"Batch","data":{"events":[...]}}
CRC32    u32 LE   CRC32 over MAGIC through PAYLOAD
```

### Fail-Closed WAL Recovery Behavior

At a validated record boundary during normal open (`FileEventLog::new`):

- **Exact EOF** (or missing file): succeeds unchanged.
- **Terminal 1–25-byte fragment** compatible with a v1 header prefix (matching
  magic/version bytes, extendable length in `1..=16 MiB`, nonzero `event_id` if
  complete): truncates only that fragment and calls `sync_all`.
- **Everything else fails closed before any write:** complete 26-byte header
  with missing payload/CRC, unsupported WAL version, bad magic, CRC mismatch,
  unparseable/incoherent batch, or OS read/seek/metadata error.
- **Read-only inspection (`FileEventLog::open_read_only`):** never truncates
  the file and propagates any stored or newly observed scan error on read.

Recovery loads `snapshot.json`, validates `meta.json` (allowing missing or
strictly older mirror for the same world/seed/format/created_tick), validates
contiguous WAL IDs from `1` covering `snapshot.last_event_id`, and replays
events after the snapshot cursor through `sy_core::replay::apply_event`.

---

## Determinism and Purity Gate

Run from `seej/server/`:

```bash
python3 scripts/check_sy_core_purity.py --self-test
python3 scripts/check_sy_core_purity.py
cargo test -p sy_core
```

The purity script masks comments, strings, raw strings, and Rust character
literals (including `'"'` and `b'"'`) before checking `crates/sy_core/src` for
wall-clock time, environment access, filesystem/I/O, networking, OS randomness,
and randomized hash collections. CI additionally verifies that `sy_core`
depends strictly on `sy_types` and `sy_api` with zero external crates.

---

## Known Prototype Limits

These are deliberate or tracked prototype limitations (not production claims):

- **Full-world cloning:** `PersistentSimulation::process_command`,
  `Simulation::process_command`, and `replay::apply_event` clone the entire
  `World` for rollback; `save_world` clones and JSON-encodes the full world.
- **Dual mutation paths:** `sim.rs` mutates `World` directly while `replay.rs`
  re-implements every event application separately.
- **Float decisions & saturating counters:** degradation uses `rng.chance(f32)`
  (`P0-03`), `Tick::next` / `EventId::next` saturate (`P0-04`), and
  `compute_canonical_hash` hashes storage cursors alongside world state using
  non-cryptographic xxHash64 (`P1-04`).
- **No exclusive writer lock (`P0-10`) & static `.tmp` filenames:** running two
  `server_d` processes on the same world directory is unsafe.
- **No WAL compaction:** the WAL grows without bound (`truncate_after` is
  deprecated and must not be used automatically, `P1-18`/`K-10`).
- **Inspection caveats (`P1-11`):** `sy_cli` opens the WAL read-only, but
  `FilesystemStore::new` may create `<data-dir>/worlds`, `dump --output` does
  not block paths aliasing canonical storage, and concurrent reads while
  `server_d` writes are unsupported.
- **No I/O fault-injection suite (`P0-14`):** `FileEventLog` does not poison
  its `BufWriter` after a partial write failure (`server_d` exits on first
  error, but library callers must not reuse a failed log instance).
- **CI test OS coverage:** unit/integration tests currently run on
  `ubuntu-latest`; Windows and macOS CI jobs run `cargo build --release` only.

---

## Benchmark Measurements (Workloads Harness)

To ground [D-015](../../doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)
in measurements before writing the new core, a standalone throwaway harness
(`seej_lab_workloads` on branches `lab/wal-fail-closed-load-b` `1c98282` and
`lab/state-layout-b` `5856396`, outside workspace/CI) benchmarked three
synthetic workloads at **1,000,000 keys × 128 B** (1,000 partitions × 1,000
records), **10 Hz / 100 ms tick budget**, single-threaded on an AMD Ryzen 9
5900X + Crucial P2 1 TB NVMe (Windows 11, `rustc 1.95.0 --release`), with
24/24 root-verified checkpoint + 200-tick recoveries:

- **Load A (commands):** 1,000 commands/tick (10,000 cmds/s), 0 autonomous
  writes.
- **Load B (dense autonomous):** 100,000 autonomous writes/tick (1M writes/s),
  0 commands.
- **Load C (mixed + hotspots):** 1,000 commands + 1,000 autonomous writes/tick,
  80% hitting 32 hot keys.

### Summary of Measured Results

| Load & Variant | `delta` p99 (ms) | `root` p99 (ms) | `delta` bytes/tick | `root` bytes/tick | Best replay (ms/tick) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A** — `btree/acc` | 10.0–10.8 | 8.8–9.2 | 417 KB (4.2 MB/s) | 129 KB (1.3 MB/s) | 2.78 (`delta`) |
| **A** — `partitioned/acc-cached` | **8.6** | **7.6** | 417 KB | 129 KB | **2.06** (`delta`) |
| **A** — `partitioned/rehash` | 162.6 | 166.8 | 417 KB | 129 KB | — (broken by per-cmd rehash) |
| **C** — `btree/acc` | 11.8–12.4 | 10.7–11.7 | 561 KB (5.6 MB/s) | 129 KB | 3.35 (`delta`) |
| **C** — `partitioned/acc-cached` | 21.7* (p50 **8.4**) | 29.4* (p50 **7.6**) | 561 KB | 129 KB | **~2.5–3.7** |
| **B** — `btree/acc` | 161–445 | 140–147 | **14.4 MB (144 MB/s)** | **89 B** | 113–129 (ratio > 1.1) |
| **B** — `partitioned/acc` | 130–348 | **95.1** | 14.4 MB | 89 B | 70–82 |
| **B** — `partitioned/acc-cached` | 279–383 | **79.0** (p50 **66.1**) | 14.4 MB | 89 B | **54.6** (`delta`) / **61.8** (`root`) |
| **B** — `partitioned/rehash` | 113–483 | 98.5 | 14.4 MB | 89 B | 73.7–77.7 |

*\*In C (`partitioned/acc-cached`), all CPU phases stayed ≤ 7.6 ms max; the p99 jump came from two 50–60 ms NVMe `sync_data` jitter spikes shifted into rank 288/290 by the tick-100 checkpoint.*

### Key Empirical Takeaways

1. **A and C pass with ~10× headroom in `delta` mode** on one thread, validating
   `delta` as the safe v1 default while plugins are unisolated native Rust.
2. **Dense autonomous Load B fails `delta` on disk tail latency** (14.4 MB/tick
   sync p99 up to 283–400 ms; 144 MB/s exceeds 1 Gbps before replication),
   confirming the need to keep proof mode profile-configurable.
3. **Contiguous per-partition storage cuts Load B `root` p99 from 147 → 79 ms**
   on one core; full per-partition rehashing breaks scattered command workloads
   (A/C 115–169 ms) when per-resolution roots are maintained.
4. **Synchronous full checkpoints (144 MB) stall every run for 237–435 ms**
   (one 1,485 ms outlier), confirming that checkpoints must move off the tick
   path in Gate 1-D.
5. **Limits of these measurements:** single thread, in-RAM storage, additive
   experimental BLAKE3 accumulator (not a production cryptographic scheme),
   warm OS cache during recovery, no network/quorum, and no embedded DB
   (`redb`/`Fjall`) tested yet.
