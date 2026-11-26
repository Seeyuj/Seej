# Empirical Benchmarks (D-015 Lab Evidence)

```text
status: EMPIRICAL REFERENCE — MEASURED IN THROWAWAY LAB HARNESSES BEFORE CORE REWRITE
```

To ground [D-015](../../doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)
in measurements before writing the clean-slate v1 kernel, two standalone
throwaway single-threaded benchmark campaigns (`seej_lab_workloads` on branches
`lab/wal-fail-closed-load-b` `1c98282` and `lab/state-layout-b` `5856396`,
outside workspace/CI) evaluated three synthetic workloads.

---

## Test Environment and Workloads

- **Hardware / toolchain:** AMD Ryzen 9 5900X + Crucial P2 1 TB NVMe, Windows 11,
  `rustc 1.95.0 --release`, single-threaded execution.
- **State size:** **1,000,000 keys × 128 B** (1,000 partitions × 1,000 records;
  ~144 MB checkpoint payload).
- **Target cadence:** **10 Hz / 100 ms tick budget**.
- **Recovery check:** **24/24 recoveries** (checkpoint at tick 100 + 200 ticks
  of replay) reproduced the exact live semantic state root.

Workloads tested:

- **Load A (commands):** 1,000 commands/tick (10,000 cmds/s), 0 autonomous
  writes.
- **Load B (dense autonomous):** 100,000 autonomous writes/tick (1M writes/s),
  0 commands.
- **Load C (mixed + hotspots):** 1,000 commands + 1,000 autonomous writes/tick,
  80% hitting 32 hot keys.

Storage & hashing variants tested:

- `btree/acc`: `BTreeMap` + overlay + incremental additive BLAKE3 accumulator.
- `partitioned/acc`: contiguous per-partition storage with in-place undo log +
  incremental accumulator.
- `partitioned/acc-cached`: contiguous per-partition storage + cached
  per-record digests (1 BLAKE3 hash per write instead of 2).
- `partitioned/rehash`: contiguous per-partition storage + full per-partition
  rehash on touched partitions.

---

## Measured Results

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

---

## Key Empirical Takeaways Driving D-015

1. **Command and mixed loads (A and C) pass with ~10× headroom in `delta` mode**
   on a single core, validating `delta` as the safe v1 default while startup
   plugins remain unisolated native Rust.
2. **Dense autonomous Load B fails `delta` on disk tail latency** (14.4 MB/tick
   synchronous writes yield p99 up to 283–400 ms and 144 MB/s before
   replication), confirming that proof mode (`delta` vs. future `root`) must
   live in the versioned execution profile rather than being hardcoded.
3. **Contiguous per-partition state layout dominates dense CPU cost:** it cuts
   Load B `root` p99 from **147 ms → 79 ms** on one thread. Conversely, full
   per-partition rehashing destroys scattered command workloads (**115–169 ms**
   on A/C) when per-resolution roots are maintained — proving roots must update
   incrementally per modified record.
4. **Synchronous full checkpoints (144 MB) stall every workload for 237–435 ms**
   (one 1,485 ms outlier), confirming that off-path async CoW checkpoints are
   required before large-state production scale (Phase 4.1 / Gate 1-D).
5. **Limits of these measurements:** single thread, in-RAM state store,
   experimental additive BLAKE3 accumulator (not a production cryptographic
   scheme), warm OS page cache during recovery, no network/quorum overhead, and
   no embedded storage engine (`redb`/`Fjall`) benchmarked yet — real storage
   engine benchmarks belong at the start of **Phase 1.4**
   ([doc/ROADMAP.md](../../doc/ROADMAP.md)).
