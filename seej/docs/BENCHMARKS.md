# Empirical Benchmarks (D-015 Lab Evidence)

```text
status: EMPIRICAL REFERENCE — MEASURED IN THROWAWAY LAB HARNESSES BEFORE CORE REWRITE
```

To ground [D-015](../../doc/DECISIONS.md#d-015--measured-kernel-core-and-frozen-plugin-seam)
in measurements before writing the clean-slate v1 kernel, two standalone
throwaway single-threaded benchmark campaigns (`seej_lab_workloads`, outside
workspace/CI) evaluated three synthetic workloads. Both raw harnesses, reports,
and JSON summaries are preserved permanently at Git archive tags:

- **Campaign 1:** tag `archive/lab-campaign-1` (`1c98282`, `seej/server/lab/workloads/RESULTS.md`) — 6 runs (`BTreeMap` baseline × `delta`/`root`), **6/6 clean recoveries**.
- **Campaign 2:** tag `archive/lab-campaign-2` (`5856396`, `seej/server/lab/workloads/RESULTS-layout.md`) — 24 runs (4 state/hash variants, plus reverse-order B/`delta` rerun), **24/24 clean recoveries**.

---

## Test Environment and Workloads

- **Hardware / toolchain:** AMD Ryzen 9 5900X + Crucial P2 1 TB NVMe, Windows 11,
  `rustc 1.95.0 --release`, single-threaded execution.
- **State size:** **1,000,000 keys × 128 B** (1,000 partitions × 1,000 records;
  144,000,068-byte checkpoint payload).
- **Target cadence:** **10 Hz / 100 ms tick budget** (300 ticks, 10 warm-up,
  290 measured ticks, full checkpoint at tick 100).
- **Recovery check:** every run's clean recovery (tick-100 checkpoint + 200
  replayed ticks, OS page cache not purged — effective cache temperature
  unknown — and **no fault injection**) reproduced the exact live semantic state
  root (`6/6` in Campaign 1; `24/24` in Campaign 2).

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

| Load & Variant | `delta` p99 (ms) | `root` p99 (ms) | `delta` bytes/tick | `root` bytes/tick | Replay (ms/tick) & ratio (/100 ms) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A** — `btree/acc` | 10.01–10.83 | 8.82–9.20 | 417 KB (4.17 MB/s) | 129 KB (1.29 MB/s) | 2.78 (`delta`, 0.028) |
| **A** — `partitioned/acc-cached` | **8.62** | **7.55** | 417 KB | 129 KB | **2.06** (`delta`, 0.021) |
| **A** — `partitioned/rehash` | 162.58 | 166.81 | 417 KB | 129 KB | 106.30 `delta` (1.06) / 109.19 `root` (1.09) |
| **C** — `btree/acc` | 11.77–12.36 | 10.72–11.69 | 561 KB (5.61 MB/s) | 129 KB | 3.35 (`delta`, 0.034) |
| **C** — `partitioned/acc-cached` | 21.74* (p50 **8.38**) | 29.42* (p50 **7.59**) | 561 KB | 129 KB | **2.75** `delta` (0.028) / **3.74** `root` (0.037) |
| **B** — `btree/acc` | 161.13–444.59 | 139.69–147.07 | **14.4 MB (144 MB/s)** | **89 B** | 113.12–129.44 (ratio **1.13–1.29**) |
| **B** — `partitioned/acc` | 129.64–348.07 | **95.13** | 14.4 MB | 89 B | 69.85–82.34 (ratio **0.70–0.82**) |
| **B** — `partitioned/acc-cached` | 279.18–383.31 | **78.95** (p50 **66.13**) | 14.4 MB | 89 B | **54.62** `delta` (**0.55**) / **61.79** `root` (**0.62**) |
| **B** — `partitioned/rehash` | 113.41–482.78 | 98.48 | 14.4 MB | 89 B | 73.65–77.71 (ratio **0.74–0.78**) |

*\*In C (`partitioned/acc-cached`), p99 rose to 21.74 ms (`delta`) and 29.42 ms (`root`) while medians fell (`8.38` / `7.59 ms`). As noted in `RESULTS-layout.md`, this was measured once and is not explained by these measurements.*

---

## Key Empirical Takeaways Driving D-015

1. **Command and mixed loads (A and C) pass in `delta` mode with 4.6×–11.6×
   headroom** on a single core (`p99 = 8.62–21.74 ms`, median `6.64–9.95 ms`),
   validating `delta` as the safe v1 default while startup plugins remain
   unisolated native Rust.
2. **Dense autonomous Load B fails `delta` across all 9 runs** (1 in Campaign 1
   + 8 in Campaign 2): full-tick p99 ranges from **113.41 to 482.78 ms**
   (driven by variable 14.4 MB/tick `sync_data` commit tails up to
   **283.10–400.12 ms** p99, max **689.37 ms**, and projecting 144 MB/s before
   replication), confirming that proof mode (`delta` vs. future `root`) must
   live in the versioned execution profile rather than being hardcoded.
3. **Contiguous per-partition state layout dominates dense CPU cost, but the
   `< 0.5` replay-ratio target on Load B was NOT met:** layout cuts Load B
   `root` p99 from **147.07 ms → 78.95 ms** on one thread, yet every tested
   variant missed the `< 0.5` replay-ratio target (best: **0.55** in `delta` /
   **0.62** in `root`). Conversely, full per-partition rehashing breaks
   scattered command workloads (**115–169 ms** median on A/C) when
   per-resolution roots are maintained — proving roots must update incrementally
   per modified record.
4. **Synchronous full checkpoints (144 MB) stall tick 100 for 237–435 ms**
   across Campaign 2 (one **1,485 ms** outlier; Campaign 1: **247–326 ms**),
   confirming that off-path async CoW checkpoints are required before
   large-state production scale (Phase 4.1 / Gate 1-D).
5. **Limits of these measurements:** single thread, in-RAM state store,
   experimental additive BLAKE3 accumulator (not a production cryptographic
   scheme), clean replay with OS page cache not purged (effective cache
   temperature unknown) and no fault injection, no network/quorum overhead, and
   no embedded storage engine (`redb`/`Fjall`) benchmarked yet — real storage
   engine benchmarks belong at the start of **Phase 1.4**
   ([doc/ROADMAP.md](../../doc/ROADMAP.md)).
