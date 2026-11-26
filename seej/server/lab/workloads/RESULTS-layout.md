# Experiment 2 — state layout and hashing for load B

Question: with the same rules and journals as the first campaign
(`RESULTS.md`), does load B reach **p99 tick ≤ 100 ms** and a **replay ratio
< 0.5** (replay time per tick / 100 ms) when the state is stored per
partition, contiguously, and either caches record digests or rehashes whole
partitions? Same throwaway harness, same machine, single thread. Nothing here
is a Seej contract; no D-015 is proposed.

## Answer

- **B / root journal: yes for p99, no for replay.** All three partitioned
  variants meet p99 ≤ 100 ms (95.13, 78.95, 98.48 ms vs 147.07 ms for the
  `BTreeMap` baseline). None reaches a replay ratio < 0.5; the best is 0.62.
- **B / delta journal: no.** None of the 8 B/delta runs (4 variants × 2
  orders) meets p99 ≤ 100 ms (113–483 ms). The p99 is driven by the variable
  latency of the 14.4 MB `sync_data` per tick on this drive, not by the state
  layout. Best replay ratio 0.55 (still > 0.5).
- **Checkpoint pause unchanged in kind:** the tick-100 full checkpoint
  (144 MB) still takes 237–435 ms across all runs (one 1,485 ms outlier), so
  every run has at least one tick > 100 ms.
- **Per-partition rehash with per-resolution roots breaks A and C** (p50
  115–169 ms vs 6–10 ms), because each scattered command rehashes 1–2 whole
  144 KB partitions.

## Validity

All 24 runs recovered to the root of their live run (`root_matches_run:
true`, 200 ticks replayed). Every variant's `acc_root`, the first campaign's
additive root recomputed in full at the end of the run and of the recovery,
equals the first campaign's final root for its load (A `7ff722d2…`, B
`e62fabe9…`, C `b1a728c2…`). The rehash variants also report their own
partition-hash root (`final_root`: A `b0d70205…`, B `247cb5d1…`, C
`30988287…`), with every partition hash recomputed in full at the end of the
run and checked. Journal bytes are unchanged: the `btree/acc` journals of the
reduced test runs are pinned to BLAKE3 digests of the first campaign's
binary (commit `1c98282`).

## Variants

|Layout / hash|State storage|State hash per write|
|---|---|---|
|`btree/acc` (baseline)|`BTreeMap` + per-tick overlay, applied after commit|2 BLAKE3 (new and old record)|
|`partitioned/acc`|Partitions sorted by 8-byte key prefix, records sorted by 8-byte suffix, both found by binary search (no slot derived from the numeric key); 144-byte records contiguous per partition; written in place with an undo log, published after commit|2 BLAKE3|
|`partitioned/acc-cached`|Same, plus a stored 32-byte digest per record|1 BLAKE3 (old digest read from the cache)|
|`partitioned/rehash`|Same layout|Rehash of each partition touched by a resolution as one contiguous slice; root = BLAKE3 over the 1,000 partition hashes + count|

The additive variants share one root; the rehash variant keeps the same
per-resolution root chain in the journal, so its cost includes a rehash per
resolution.

## Environment and commands

Same machine as `RESULTS.md` (Ryzen 9 5900X, Windows 11 Pro 10.0.26300,
`rustc 1.95.0`, `F:` NTFS on Crucial P2 1 TB NVMe), 2026-11-26, 91.1 GB free
at start, nothing else building. Presets unchanged (`seed 42`, 1,000,000 keys,
300 ticks, 10 warm-up, checkpoint at 100). Before measuring:
`cargo test --release --locked --lib --test invariants` (11 + 12 passed),
`cargo build --release --locked --bin workloads`.

```bash
# for each case, in this order:
#   B delta: btree/acc, partitioned/acc, partitioned/acc-cached, partitioned/rehash
#   B root : same four
#   A, C   : delta then root × btree/acc, partitioned/acc-cached, partitioned/rehash
workloads run --dir "$d" --load "$load" --journal "$mode" --layout "$layout" --hash "$hash"
workloads recover --dir "$d"     # new process, OS cache not purged
# then copy the three JSON files to results-layout/<load>-<mode>-<layout>-<hash>/ and delete $d
```

Main campaign 750 s. Then a second B/delta series in **reverse order**
(rehash, acc-cached, acc, btree) to check whether the commit tail follows the
variant or the position in the sequence: `results-layout-rerun/`.

## Load B — tick time (ms; p99 = 288th of 290 measured ticks)

|Variant|root p50|root p99|root max|delta p50 (1st / 2nd series)|delta p99 (1st / 2nd)|delta max (1st / 2nd)|
|---|---:|---:|---:|---:|---:|---:|
|btree/acc|128.49|147.07|493.13|141.42 / 157.54|161.13 / 207.32|480.16 / 551.49|
|partitioned/acc|83.97|**95.13**|371.89|98.98 / 106.43|129.64 / 348.07|381.48 / 397.82|
|partitioned/acc-cached|66.13|**78.95**|362.66|97.61 / 86.95|383.31 / 279.18|535.40 / 437.47|
|partitioned/rehash|87.00|**98.48**|403.61|208.87 / 101.35|482.78 / 113.41|1,709.92 / 420.32|

In root mode the slowest tick is the checkpoint tick in every variant
(checkpoint write 289–368 ms).

### Attribution, B / root (p50 per phase, ms; not additive across centiles)

|Variant|`compute`|`hash`|`merge`|`commit`|`apply`|
|---|---:|---:|---:|---:|---:|
|btree/acc|40.78|36.40|14.81|3.20|33.25|
|partitioned/acc|22.38|46.64|11.96|3.01|≈ 0|
|partitioned/acc-cached|21.74|29.42|11.91|3.00|≈ 0|
|partitioned/rehash|21.69|51.20|10.67|2.98|≈ 0|

- Storing state per partition removes almost all of `apply` (publishing is
  dropping the undo log) and halves `compute` (lookups by binary search over
  contiguous partitions instead of `BTreeMap` + overlay). That accounts for
  most of the gain: 128 → 84 ms p50 with the hash scheme unchanged.
- Caching the old digest lowers `hash` from 46.6 to 29.4 ms (84 → 66 ms
  p50): one BLAKE3 call per write instead of two, yet more than half of the
  two-call cost remains.
- `hash` is *higher* in `partitioned/acc` than in the baseline (46.6 vs
  36.4 ms) for the same number of BLAKE3 calls. Not explained by these
  measurements (`merge` already touches the in-place record before `hash`).
- `compute` still includes one keyed BLAKE3 per write: the rule's own
  entropy (100,000 per tick), a cost of the synthetic rule, not of the storage.

### B / delta: the commit tail follows the drive, not the variant

|Variant|`commit` p50 (1st / 2nd)|`commit` p99 (1st / 2nd)|`commit` max (1st / 2nd)|Position (1st / 2nd)|
|---|---:|---:|---:|---|
|btree/acc|16.80 / 17.94|32.91 / 32.92|36.44 / 175.06|1 / 4|
|partitioned/acc|15.78 / 17.24|44.38 / 243.55|157.20 / 302.30|2 / 3|
|partitioned/acc-cached|33.38 / 17.88|54.94 / 197.75|445.71 / 218.51|3 / 2|
|partitioned/rehash|117.47 / 16.41|400.12 / 26.38|689.37 / 27.79|4 / 1|

The same 14,400,089-byte write + `sync_data` has a stable median
(≈ 16–18 ms) but a tail that varies by 10× between runs of the same variant
(rehash: p99 400 ms in one series, 26 ms in the other). The tail tracks
neither the variant nor, strictly, the position. A drive-level effect such
as SLC cache or flush scheduling is plausible but not measured. On this
drive, B/delta p99 cannot be compared across variants with two series per
variant. Even the best series of the fastest in-memory variant exceeds
100 ms at p99.

## Load B — recovery from the tick-100 checkpoint

Fresh process, OS cache not purged, effective cache temperature unknown.

|Variant|Mode|Checkpoint load ms|Checkpoint verify ms|Replay ms / tick|Ratio (target < 0.5)|
|---|---|---:|---:|---:|---:|
|btree/acc|root|255.42|209.10|126.50|1.27|
|partitioned/acc|root|125.32|177.46|80.13|0.80|
|partitioned/acc-cached|root|127.16|191.63|61.79|0.62|
|partitioned/rehash|root|142.58|37.44|75.26|0.75|
|btree/acc|delta (1st / 2nd)|249.97 / —|201.47 / —|114.56 / 129.44|1.15 / 1.29|
|partitioned/acc|delta (1st / 2nd)|121.43 / —|174.35 / —|69.85 / 82.34|0.70 / 0.82|
|partitioned/acc-cached|delta (1st / 2nd)|125.26 / —|188.76 / —|54.62 / 54.67|**0.55 / 0.55**|
|partitioned/rehash|delta (1st / 2nd)|134.63 / —|36.46 / —|77.71 / 73.65|0.78 / 0.74|

(Load/verify for the second series are in `results-layout-rerun/`.) Replay
does the same in-memory work as a live tick minus the commit, so it stays
just under the live p50. Verifying a rehash checkpoint (144 MB hashed as
contiguous partitions) takes 36–53 ms against 174–226 ms for 1,000,000
separate record digests.

## Loads A and C (control)

|Case|btree/acc p50 / p99|partitioned/acc-cached p50 / p99|partitioned/rehash p50 / p99|rehash `hash` p50|
|---|---:|---:|---:|---:|
|A / delta|8.26 / 10.83|6.64 / 8.62|115.06 / 162.58|106.27|
|A / root|7.70 / 9.20|6.15 / 7.55|114.85 / 166.81|106.52|
|C / delta|9.95 / 12.36|8.38 / 21.74|162.59 / 181.89|152.74|
|C / root|8.72 / 11.69|7.59 / 29.42|169.28 / 180.37|159.91|

- `acc-cached` lowers A and C medians by 1–2 ms and their replay to
  2.06–3.74 ms/tick. C's p99 rose (12.36 → 21.74 and 11.69 → 29.42 ms) while
  its median fell. That was measured once and is not explained.
- Rehash with per-resolution roots costs ≈ 106 ms per tick in A: 1,000
  commands each rehash up to two whole partitions (up to 288 MB of BLAKE3
  input per tick). Partition rehashing pays off only when the root is needed
  once per tick and the writes are dense within partitions. B's writes are
  spread across all partitions, so even there the per-tick rehash (51 ms)
  costs more than cached digests (29 ms).

## What this changes, and what it does not

- For B the state layout matters more than the hash scheme: contiguous,
  in-place partitions bring B/root from 147 to 79–98 ms p99 under identical
  rules, journal and roots. The baseline's failure on B was largely
  `BTreeMap` + overlay cost, not the B workload itself.
- The replay-ratio target (< 0.5) is not met by any variant. After the
  layout change, the remaining cost per replayed B tick is about 22 ms of rule
  work (including 100,000 keyed BLAKE3 calls of entropy), about 29 ms of state
  hashing and about 12 ms of in-place writes. Further levers (parallel
  hashing, a cheaper per-write fingerprint, multiple ticks per publication
  during replay) were **not tested**.
- The delta journal for B stays outside the budget on this drive. Its cost is
  disk-tail latency at 14.4 MB/tick, outside what layout or hashing can
  change.
- The full 144 MB checkpoint pause (≈ 237–435 ms) is untouched. B writes
  10 % of every partition per tick, so a dirty-partition or
  dirty-record incremental checkpoint cannot shrink it (argued from the
  workload, not measured); moving the checkpoint off the tick path was not
  tested.
- Not covered: storage engines (redb, Fjall), incremental checkpoints,
  multi-threading, networking or quorum, other hash schemes beyond these two.
  The additive digest sum remains an experimental fingerprint.
