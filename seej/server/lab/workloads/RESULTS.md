# Workload harness — first measurements

Throwaway prototype (`seej_lab_workloads`), not part of the Seej workspace or
CI, and not intended to be merged into `main`. It measures one implementation
on one machine: a single-threaded `BTreeMap` state in RAM, an additive BLAKE3
accumulator, a local experimental journal (`write_all` + `sync_data` once per
tick) and one full checkpoint. Nothing here is a Seej contract or a decision
record.

## Validity

All six recoveries passed: for each load, the `delta` and `root` runs reached
the same final root, each recovery rebuilt that root from checkpoint + journal
(`root_matches_run: true`, `replayed_ticks: 200`), and every end-of-run oracle
(incremental world/partition accumulators and count against a full recompute,
counter sum `keys × 1_000_000 + ticks × auto_writes`) held. The comparisons
below are therefore valid for this machine and these synthetic rules.

|Load|Final root (both modes and both recoveries)|
|---|---|
|A|`7ff722d296e57470b612e40f7e8ac907d4e7cc28a0330fcbcda124b9a08e41cd`|
|B|`e62fabe91e178a2f91d2180556f140bf99b19d707ab9caa2299cda3a56caee71`|
|C|`b1a728c2fa5b5c0a0ae7a8402d270fdcc18793ffd49dca2fbe5b1db151f74c1a`|

## Environment (recorded at measurement time, 2026-11-26)

- CPU: AMD Ryzen 9 5900X (12 cores; the harness uses one thread); RAM 63.9 GB.
- OS: Windows 11 Pro 10.0.26300.
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`, `--release`, `Cargo.lock` committed.
- Volume: `F:` NTFS on a Crucial P2 1 TB NVMe SSD (CT1000P2SSD8); 93,426,274,304
  bytes free before the campaign.
- No other build or benchmark ran during the campaign; the six cases ran in
  series on the same volume. Total campaign wall time ≈ 161 s.

## Commands and parameters

From `seej/server/lab/workloads`, after `cargo test --release --locked --lib
--test invariants` (18 passed) and `cargo build --release --locked --bin
workloads`:

```bash
root=$(mktemp -d F:/tmp/seej-workloads-XXXXXX)
for load in b a c; do
  for mode in delta root; do
    d="$root/$load-$mode"
    "$exe" run --dir "$d" --load "$load" --journal "$mode"
    "$exe" recover --dir "$d"   # new process; OS cache not purged
  done
done
```

Common: `seed 42`, `keys 1_000_000` (1,000 partitions × 1,000 records,
128-byte values), `ticks 300`, `warmup_ticks 10` (290 measured ticks),
`checkpoint_at 100`.

|Load|`auto_writes`|`commands`|`hot_keys`|`hot_percent`|Resolutions / tick|
|---|---:|---:|---:|---:|---:|
|A|0|1,000|0|0|1,001|
|B|100,000|0|0|0|1|
|C|1,000|1,000|32|80|1,001|

Per-case `config.json`, `summary.json` and `recover_summary.json` are in
`results/<load>-<mode>/`. The journals and checkpoints themselves
(5,554,960,608 logical bytes) were not kept.

## Tick time against the 100 ms budget

`total` is a monotonic wall-clock time per tick, from command generation to
the end of apply, including the checkpoint pause on tick 100. Percentiles use
rank `ceil(q × n)` over the 290 measured ticks: p99 is the 288th sorted value,
i.e. the third slowest tick. `budget_met` is the p99 verdict only; it never
means that every tick held 100 ms.

|Case|p50 ms|p99 ms|max ms|`budget_met` (p99 ≤ 100)|≥ 1 tick > 100 ms|Tick 100 total ms|`checkpoint_write_ms`|
|---|---:|---:|---:|---|---|---:|---:|
|A / delta|7.62|10.01|254.62|true|yes (checkpoint tick)|254.62|246.81|
|A / root|7.28|8.82|262.74|true|yes (checkpoint tick)|262.74|254.95|
|B / delta|138.15|444.59|540.70|**false**|yes — median tick exceeds 100 ms|444.59|309.60|
|B / root|118.19|139.69|438.13|**false**|yes — median tick exceeds 100 ms|438.13|326.01|
|C / delta|8.74|11.77|287.94|true|yes (checkpoint tick)|287.94|278.81|
|C / root|8.72|10.72|280.74|true|yes (checkpoint tick)|280.74|271.28|

- A and C hold the p99 budget with an order of magnitude to spare. Their
  slowest tick is the checkpoint tick (its total equals the maximum); the
  288th sorted tick is at most 11.77 ms, and the 289th is not reported. The
  checkpoint pause (≈ 250–330 ms for 144,000,068 bytes, write + `sync_all` +
  rename) escapes p99 but is a real stall in a single-threaded tick loop.
- B misses the budget in both modes, already at the median. In B/delta the
  checkpoint tick happens to be the p99 rank (444.59 ms); the slowest tick
  (540.70 ms) is a different tick, in a run whose commit maximum is 419.33 ms.

## Attribution (B, per-phase distributions; centiles are not additive)

|Phase (ms)|B / delta p50|B / delta p99|B / root p50|B / root p99|
|---|---:|---:|---:|---:|
|`compute` (rule + 100,000 logical reads through the view)|37.37|48.80|37.36|45.47|
|`hash` (accumulator deltas for 100,000 writes)|36.90|39.94|36.55|39.34|
|`encode`|3.08|4.90|≈ 0|≈ 0|
|`merge` (overlay of 100,000 distinct keys)|13.27|16.37|12.85|16.77|
|`commit` (`write_all` + `sync_data`)|16.60|283.10|2.89|3.27|
|`apply` (materialize overlay into the `BTreeMap`)|28.68|36.67|28.92|37.50|

- In B/root the disk is not the bottleneck (commit p99 3.27 ms for 89 bytes);
  the time goes to in-memory work: rule evaluation and view lookups, the hash
  of each write, overlay merge, and apply. `compute`, `merge` and `apply`
  include `BTreeMap` traversal and insertion over 100,000 distinct keys; that
  cost is state access, not hashing or disk, and the phases do not measure
  cache misses. Whether a different state structure would change B is a
  separate measurement question; this branch tests only `BTreeMap` + full
  checkpoint.
- In B/delta the 14.4 MB synchronous write per tick adds a heavy, variable
  tail: commit p50 16.60 ms, p99 283.10 ms, max 419.33 ms.
- `hash` is the cost of this accumulator implementation (two BLAKE3 digests of
  a 144-byte record per write plus 256-bit add/sub), not a bound for
  Merkle/LtHash/EcMH or any other scheme.
- Full-state checks outside the tick loop: full scan 16.0–21.2 ms, full
  recompute of the accumulators over 1,000,000 records 209.3–218.4 ms,
  genesis build 482–522 ms.

A and C: `compute` p99 2.62–3.84 ms, `hash` p99 0.91–1.36 ms, `commit` p99
3.30–5.69 ms (max 18.83 ms in A/delta).

## Journal volume

|Case|Bytes / tick (on disk)|Projection at 10 Hz (MB/s, decimal)|Journal total (bytes)|Coalesced overlay size (bytes / tick, theoretical)|
|---|---:|---:|---:|---:|
|A / delta|417,089|4.17089|125,126,700|287,718|
|A / root|129,089|1.29089|38,726,700|287,718|
|B / delta|14,400,089|144.00089|4,320,026,700|14,400,000|
|B / root|89|0.00089|26,700|14,400,000|
|C / delta|561,089|5.61089|168,326,700|321,020|
|C / root|129,089|1.29089|38,726,700|321,020|

- Every journal total equals 300 × the per-tick size and the actual file size.
  B/delta per tick = `100,000 × 144 + 81 + 8` as specified.
- The 10 Hz figures are projections from bytes per tick, not measured transfer
  rates; no network, quorum or compression was tested.
- The coalesced column is the overlay size × 144 bytes, a theoretical size of
  final values per tick. It is not a second journal that was measured, and
  coalescing would drop the per-resolution intermediate roots that the
  current journal lets recovery verify. It is not a measured saving.

## Recovery from the tick-100 checkpoint

Fresh process, OS cache not purged, effective cache temperature unknown. The
journal suffix after tick 100 (200 ticks) is replayed after loading and
verifying the checkpoint. These are not cold-SSD throughput figures after a
reboot and not a production RTO.

|Case|Checkpoint load ms|Checkpoint verify ms|Replay ms|Replay ms / tick|Replay / 100 ms budget|Journal bytes read|Total ms|
|---|---:|---:|---:|---:|---:|---:|---:|
|A / delta|248.54|203.28|555.32|2.78|0.028|83,417,800|1,007.1|
|A / root|245.84|206.11|781.97|3.91|0.039|25,817,800|1,233.9|
|B / delta|262.98|206.99|22,624.14|113.12|1.131|2,880,017,800|23,094.1|
|B / root|247.77|204.84|23,870.14|119.35|1.194|17,800|24,322.7|
|C / delta|242.12|206.01|669.03|3.35|0.033|112,217,800|1,117.2|
|C / root|274.60|215.61|869.90|4.35|0.043|25,817,800|1,360.1|

The checkpoint read is 144,000,068 bytes in every case, on top of the journal
bytes read. In B both modes replay slower than real time at 10 Hz (ratio
> 1): a node replaying B could not catch up with a live group producing at
that rate on this implementation.

## What these numbers support

Comparison of `delta` vs `root` journaling under identical rules, for this
machine and this implementation (input to trade-off 1, not a decision on the
distributed protocol):

- **A (1,000 commands)**: `delta` journals 3.2× more bytes (417 KB vs
  129 KB/tick) for a slightly higher p99 (10.01 vs 8.82 ms) and a faster replay
  (2.78 vs 3.91 ms/tick). Both are far within budget.
- **B (100,000 autonomous writes)**: `delta` costs 14.4 MB/tick and a 283 ms
  commit p99 while `root` journals 89 bytes; replay costs about the same
  (113 vs 119 ms/tick) because the materialization and hashing work dominates
  either way. Neither mode meets the budget; the in-memory work alone
  (B/root p50 118 ms) exceeds it.
- **C (mixed, hot keys)**: `delta` journals 4.3× more than `root` (561 KB vs
  129 KB/tick) for a similar p99 (11.77 vs 10.72 ms) and a slightly faster
  replay (3.35 vs 4.35 ms/tick).

Not established here: Byzantine protection, quorum behavior, network
throughput, real command-queue stability, other hashing schemes (the additive
digest sum is an experimental fingerprint of logical state, not a production
cryptographic scheme, and proves neither order nor completeness of the
journal), other storage engines (redb, Fjall) or incremental checkpoints,
power-loss durability of the checkpoint rename on every OS, and trade-offs
3, 5 and 6 or `CancelCommand` liveness. No D-015 is proposed from these
measurements.
