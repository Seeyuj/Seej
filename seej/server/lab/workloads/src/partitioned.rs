use std::time::Instant;

use crate::checkpoint::Snapshot;
use crate::engine::{require_root, validate_command_writes, Work};
use crate::journal::{Action, Decoded};
use crate::rules::{self, ReadView, Rules, Write};
use crate::state::{
    add, counter, genesis_value, key_at, record_digest, root_of, subtract, Acc, Key, Root, Value,
    ENTRY_BYTES, PARTITION_SIZE,
};
use crate::summary::TickTimes;
use crate::{allocation_count, Config, HashMode, JournalMode, Layout};

type Record = [u8; ENTRY_BYTES];

#[derive(Debug, PartialEq, Eq)]
struct Partition {
    prefix: [u8; 8],
    suffixes: Vec<[u8; 8]>,
    records: Vec<Record>,
    digests: Vec<Acc>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Records {
    partitions: Vec<Partition>,
    count: u64,
    partition_capacity: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Slot {
    partition: usize,
    record: usize,
}

impl Records {
    pub(crate) fn new(partitions: usize, partition_capacity: usize) -> Self {
        Self {
            partitions: Vec::with_capacity(partitions),
            partition_capacity,
            count: 0,
        }
    }

    pub(crate) fn push_sorted(&mut self, record: Record) {
        let prefix: [u8; 8] = record[..8].try_into().unwrap();
        if self.partitions.last().is_none_or(|p| p.prefix != prefix) {
            debug_assert!(self.partitions.last().is_none_or(|p| p.prefix < prefix));
            self.partitions.push(Partition {
                prefix,
                suffixes: Vec::with_capacity(self.partition_capacity),
                records: Vec::with_capacity(self.partition_capacity),
                digests: Vec::new(),
            });
        }
        let part = self.partitions.last_mut().unwrap();
        let suffix = record[8..16].try_into().unwrap();
        debug_assert!(part.suffixes.last().is_none_or(|last| *last < suffix));
        part.suffixes.push(suffix);
        part.records.push(record);
        self.count += 1;
    }

    fn slot(&self, key: &Key) -> Result<Slot, String> {
        let prefix: &[u8; 8] = key[..8].try_into().unwrap();
        let suffix: &[u8; 8] = key[8..].try_into().unwrap();
        let missing = |_| format!("unknown key {key:?}");
        let partition = self
            .partitions
            .binary_search_by(|p| p.prefix.cmp(prefix))
            .map_err(missing)?;
        let record = self.partitions[partition]
            .suffixes
            .binary_search(suffix)
            .map_err(missing)?;
        Ok(Slot { partition, record })
    }

    fn record(&self, slot: Slot) -> &Record {
        &self.partitions[slot.partition].records[slot.record]
    }

    fn part_hash(&self, index: usize) -> Root {
        let index_le = (index as u32).to_le_bytes();
        *blake3::Hasher::new()
            .update(b"seej lab part v1")
            .update(&index_le)
            .update(self.partitions[index].records.as_flattened())
            .finalize()
            .as_bytes()
    }

    fn all_part_hashes(&self) -> Vec<Root> {
        (0..self.partitions.len())
            .map(|p| self.part_hash(p))
            .collect()
    }

    fn chunks(&self) -> impl Iterator<Item = &[Record]> {
        self.partitions.iter().map(|p| p.records.as_slice())
    }
}

fn key(record: &Record) -> &Key {
    record[..16].try_into().unwrap()
}

fn value(record: &Record) -> &Value {
    record[16..].try_into().unwrap()
}

fn rehash_root(parts: &[Root], count: u64) -> Root {
    *blake3::Hasher::new()
        .update(b"seej lab root v2")
        .update(parts.as_flattened())
        .update(&count.to_le_bytes())
        .finalize()
        .as_bytes()
}

#[derive(Debug, PartialEq, Eq)]
enum Hashes {
    Acc { world: Acc, parts: Vec<Acc> },
    Rehash(Vec<Root>),
}

pub(crate) struct Partitioned {
    records: Records,
    hashes: Hashes,
}

impl Partitioned {
    pub(crate) fn genesis(cfg: &Config, layout: &Layout, seed_key: &[u8; 32]) -> Self {
        let mut records = Records::new(layout.partitions, PARTITION_SIZE as usize);
        for index in 0..cfg.keys {
            let mut record = [0; ENTRY_BYTES];
            record[..16].copy_from_slice(&key_at(index));
            record[16..].copy_from_slice(&genesis_value(seed_key, index));
            records.push_sorted(record);
        }
        Self::new(records, cfg.hash)
    }

    pub(crate) fn new(mut records: Records, mode: HashMode) -> Self {
        let hashes = match mode {
            HashMode::Acc | HashMode::AccCached => {
                let mut world = [0; 4];
                let mut parts = Vec::with_capacity(records.partitions.len());
                for part in &mut records.partitions {
                    let mut sum = [0; 4];
                    if mode == HashMode::AccCached {
                        part.digests.reserve_exact(part.records.len());
                    }
                    for record in &part.records {
                        let digest = record_digest(record);
                        if mode == HashMode::AccCached {
                            part.digests.push(digest);
                        }
                        add(&mut sum, &digest);
                    }
                    add(&mut world, &sum);
                    parts.push(sum);
                }
                Hashes::Acc { world, parts }
            }
            HashMode::Rehash => Hashes::Rehash(records.all_part_hashes()),
        };
        Self { records, hashes }
    }
}

impl ReadView for Records {
    type Prior = Slot;

    fn get(&self, key: &Key) -> Result<&Value, String> {
        Ok(value(self.record(self.slot(key)?)))
    }

    fn for_write(&self, key: &Key) -> Result<(&Value, Slot), String> {
        let slot = self.slot(key)?;
        Ok((value(self.record(slot)), slot))
    }
}

impl Snapshot for Partitioned {
    fn count(&self) -> u64 {
        self.records.count
    }

    fn root(&self) -> Root {
        match &self.hashes {
            Hashes::Acc { world, .. } => root_of(world, self.count()),
            Hashes::Rehash(parts) => rehash_root(parts, self.count()),
        }
    }

    fn entries(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.records
            .chunks()
            .flatten()
            .map(|record| (key(record), value(record)))
    }

    fn full_scan(&self) -> u128 {
        self.records
            .chunks()
            .flatten()
            .map(|record| u128::from(counter(value(record))))
            .sum()
    }

    fn checked_acc_root(&self, expected_sum: u64) -> Result<Root, String> {
        let mut world = [0; 4];
        let mut count = 0;
        let mut sum = 0;
        for (index, part) in self.records.partitions.iter().enumerate() {
            let mut acc = [0; 4];
            for (slot, record) in part.records.iter().enumerate() {
                let digest = record_digest(record);
                if !part.digests.is_empty() && part.digests.get(slot) != Some(&digest) {
                    return Err("recomputed record digest does not match cache".into());
                }
                add(&mut acc, &digest);
                sum += counter(value(record));
            }
            if let Hashes::Acc { parts, .. } = &self.hashes {
                if parts[index] != acc {
                    return Err("recomputed partition accumulator mismatch".into());
                }
            }
            add(&mut world, &acc);
            count += part.records.len() as u64;
        }
        if count != self.count() || sum != expected_sum {
            return Err("recomputed record count or counter sum mismatch".into());
        }
        let acc_root = root_of(&world, count);
        match &self.hashes {
            Hashes::Acc { .. } => require_root(&acc_root, &self.root(), "full-state recompute")?,
            Hashes::Rehash(parts) => {
                let recomputed = self.records.all_part_hashes();
                let expected = rehash_root(&recomputed, count);
                require_root(&expected, &self.root(), "full-state recompute")?;
                if *parts != recomputed {
                    return Err("recomputed partition hashes mismatch".into());
                }
            }
        }
        Ok(acc_root)
    }
}

struct Undo {
    slot: Slot,
    old: Record,
}

struct AccChanges {
    world: Acc,
    parts: Vec<Acc>,
    old_digests: Option<Vec<Acc>>,
}

enum HashChanges {
    Acc(AccChanges),
    Rehash(Vec<(usize, Root)>),
}

#[derive(Clone, Copy)]
struct ResolutionStart {
    records: usize,
    part_hashes: usize,
}

pub(crate) struct PartitionWork {
    undo: Vec<Undo>,
    hash: HashChanges,
    writes: Vec<Write<Slot>>,
    last_tick: Option<Vec<Undo>>,
}

pub(crate) fn validate_capacity(cfg: &Config, partitions: u64) -> Result<usize, String> {
    allocation_count::<Partition>(partitions, "partitions")?;
    allocation_count::<Write<Slot>>(
        cfg.auto_writes.max(if cfg.commands > 0 { 2 } else { 0 }),
        "partitioned resolution buffer",
    )?;
    let command_writes = cfg
        .commands
        .checked_mul(2)
        .ok_or("command write count overflow")?;
    let entries = command_writes
        .checked_add(cfg.auto_writes)
        .ok_or("tick write count overflow")?;
    if cfg.hash == HashMode::Rehash {
        allocation_count::<(usize, Root)>(
            command_writes + cfg.auto_writes.min(partitions),
            "partition hash undo log",
        )?;
    }
    if cfg.hash == HashMode::AccCached {
        allocation_count::<Acc>(entries, "digest undo log")?;
    }
    allocation_count::<Undo>(entries, "record undo log")
}

impl PartitionWork {
    fn stage_record(
        state: &mut Partitioned,
        hash: &mut HashChanges,
        undo: &mut Vec<Undo>,
        start: ResolutionStart,
        slot: Slot,
        value: &Value,
    ) {
        let part = &mut state.records.partitions[slot.partition];
        match (hash, &state.hashes) {
            (HashChanges::Acc(delta), Hashes::Acc { .. }) => {
                if let Some(digests) = &mut delta.old_digests {
                    digests.push(part.digests[slot.record]);
                }
            }
            (HashChanges::Rehash(old), Hashes::Rehash(parts)) => {
                // Write sets are strictly sorted; log each partition once per resolution.
                if old.len() == start.part_hashes || old.last().unwrap().0 != slot.partition {
                    old.push((slot.partition, parts[slot.partition]));
                }
            }
            _ => unreachable!("partition hash state matches its work buffer"),
        }
        let record = &mut part.records[slot.record];
        undo.push(Undo { slot, old: *record });
        record[16..].copy_from_slice(value);
    }

    fn new(cfg: &Config, layout: &Layout, live: bool) -> Self {
        let hash = if cfg.hash == HashMode::Rehash {
            let capacity =
                2 * cfg.commands as usize + layout.partitions.min(cfg.auto_writes as usize);
            HashChanges::Rehash(Vec::with_capacity(capacity))
        } else {
            HashChanges::Acc(AccChanges {
                world: [0; 4],
                parts: vec![[0; 4]; layout.partitions],
                old_digests: (cfg.hash == HashMode::AccCached)
                    .then(|| Vec::with_capacity(layout.undo_capacity)),
            })
        };
        Self {
            undo: Vec::with_capacity(layout.undo_capacity),
            hash,
            writes: if live || cfg.journal == JournalMode::Root {
                Vec::with_capacity(layout.resolved_capacity)
            } else {
                Vec::new()
            },
            last_tick: live.then(|| Vec::with_capacity(layout.undo_capacity)),
        }
    }

    fn resolution_start(&self) -> ResolutionStart {
        ResolutionStart {
            records: self.undo.len(),
            part_hashes: match &self.hash {
                HashChanges::Acc(_) => 0,
                HashChanges::Rehash(old) => old.len(),
            },
        }
    }

    fn merge(&mut self, state: &mut Partitioned) -> ResolutionStart {
        let start = self.resolution_start();
        for write in &self.writes {
            Self::stage_record(
                state,
                &mut self.hash,
                &mut self.undo,
                start,
                write.old,
                &write.new,
            );
        }
        start
    }

    #[cfg(test)]
    pub(crate) fn assert_same_state(state: &Partitioned, other: &Partitioned) {
        assert_eq!(state.records, other.records);
        assert_eq!(state.hashes, other.hashes);
        assert_eq!(state.root(), other.root());
    }

    fn hash_resolution(&mut self, state: &mut Partitioned, start: ResolutionStart) -> Root {
        let count = state.count();
        match (&mut self.hash, &mut state.hashes) {
            (HashChanges::Acc(delta), Hashes::Acc { world, .. }) => {
                for (offset, undo) in self.undo[start.records..].iter().enumerate() {
                    let Slot { partition, record } = undo.slot;
                    let part = &mut state.records.partitions[partition];
                    let mut change = record_digest(&part.records[record]);
                    let old = if let Some(digests) = &delta.old_digests {
                        part.digests[record] = change;
                        digests[start.records + offset]
                    } else {
                        record_digest(&undo.old)
                    };
                    subtract(&mut change, &old);
                    add(&mut delta.world, &change);
                    add(&mut delta.parts[partition], &change);
                }
                let mut world = *world;
                add(&mut world, &delta.world);
                root_of(&world, count)
            }
            (HashChanges::Rehash(old_hashes), Hashes::Rehash(parts)) => {
                for &(index, _) in &old_hashes[start.part_hashes..] {
                    parts[index] = state.records.part_hash(index);
                }
                rehash_root(parts, count)
            }
            _ => unreachable!("partition hash state matches its work buffer"),
        }
    }
}

impl Work for PartitionWork {
    type State = Partitioned;
    type Prior = Slot;

    fn genesis(cfg: &Config, layout: &Layout, seed_key: &[u8; 32]) -> Partitioned {
        Partitioned::genesis(cfg, layout, seed_key)
    }

    fn start(cfg: &Config, layout: &Layout, live: bool) -> Self {
        Self::new(cfg, layout, live)
    }

    fn resolve(
        &mut self,
        state: &Partitioned,
        rules: &Rules<'_>,
        action: Action,
        tick: u64,
    ) -> Result<(), String> {
        match action {
            Action::Command(cmd) => rules::resolve_command(&cmd, &state.records, &mut self.writes),
            Action::Closure { .. } => rules.autonomous(tick, &state.records, &mut self.writes),
        }
    }

    fn writes(&self) -> &[Write<Slot>] {
        &self.writes
    }

    fn stage_root(&mut self, state: &mut Partitioned, times: &mut TickTimes) -> Root {
        let start = Instant::now();
        let index = self.merge(state);
        times.merge += start.elapsed();
        let start = Instant::now();
        let root = self.hash_resolution(state, index);
        times.hash += start.elapsed();
        root
    }

    fn replay_resolution(
        &mut self,
        state: &mut Partitioned,
        rules: &Rules<'_>,
        mode: JournalMode,
        pre_root: &Root,
        decoded: &Decoded<'_>,
    ) -> Result<Root, String> {
        let frame = &decoded.frame;
        require_root(
            pre_root,
            &frame.pre_root,
            format_args!("pre-root at tick {}", frame.tick),
        )?;
        let start = match mode {
            JournalMode::Delta => {
                validate_command_writes(decoded)?;
                let start = self.resolution_start();
                let mut previous = None;
                for (key, value) in decoded.entries() {
                    if previous.is_some_and(|prev| prev >= key) {
                        return Err(format!(
                            "journal writes not strictly ordered at tick {}",
                            frame.tick
                        ));
                    }
                    let slot = state.records.slot(key)?;
                    Self::stage_record(state, &mut self.hash, &mut self.undo, start, slot, value);
                    previous = Some(key);
                }
                start
            }
            JournalMode::Root => {
                self.resolve(state, rules, frame.action, frame.tick)?;
                self.merge(state)
            }
        };
        let post_root = self.hash_resolution(state, start);
        require_root(
            &post_root,
            &frame.post_root,
            format_args!("post-root at tick {}", frame.tick),
        )?;
        Ok(post_root)
    }

    fn coalesced_after_publish(&mut self) -> usize {
        let last_tick = self
            .last_tick
            .as_mut()
            .expect("live tick keeps its undo log for reporting");
        last_tick.sort_unstable_by_key(|entry| entry.slot);
        last_tick.dedup_by_key(|entry| entry.slot);
        last_tick.len()
    }

    fn apply(&mut self, state: &mut Partitioned) {
        match (&mut self.hash, &mut state.hashes) {
            (HashChanges::Acc(delta), Hashes::Acc { world, parts }) => {
                add(world, &delta.world);
                delta.world = [0; 4];
                for (part, change) in parts.iter_mut().zip(&mut delta.parts) {
                    add(part, change);
                    *change = [0; 4];
                }
                if let Some(digests) = &mut delta.old_digests {
                    digests.clear();
                }
            }
            (HashChanges::Rehash(old), Hashes::Rehash(_)) => old.clear(),
            _ => unreachable!("partition hash state matches its work buffer"),
        }
        if let Some(last_tick) = &mut self.last_tick {
            last_tick.clear();
            std::mem::swap(last_tick, &mut self.undo);
        } else {
            self.undo.clear();
        }
    }

    fn rollback(&mut self, state: &mut Partitioned) {
        match (&mut self.hash, &mut state.hashes) {
            (HashChanges::Acc(delta), Hashes::Acc { .. }) => {
                if let Some(digests) = &mut delta.old_digests {
                    for (undo, digest) in self.undo.iter().zip(digests.drain(..)).rev() {
                        let Slot { partition, record } = undo.slot;
                        state.records.partitions[partition].digests[record] = digest;
                    }
                }
                delta.world = [0; 4];
                delta.parts.fill([0; 4]);
            }
            (HashChanges::Rehash(old), Hashes::Rehash(parts)) => {
                for (index, hash) in old.drain(..).rev() {
                    parts[index] = hash;
                }
            }
            _ => unreachable!("partition hash state matches its work buffer"),
        }
        for Undo {
            slot: Slot { partition, record },
            old,
        } in self.undo.iter().rev()
        {
            let part = &mut state.records.partitions[*partition];
            part.records[*record] = *old;
        }
        self.undo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{seed_key, set_counter, INITIAL_COUNTER};
    use crate::{Load, StateLayout};

    #[test]
    fn sparse_keys_are_found_by_prefix_and_suffix() {
        let mut records = Records::new(2, 2);
        for (index, marker) in [(5_013, 1), (5_999, 2), (19_277, 3)] {
            let mut record = [0; ENTRY_BYTES];
            record[..16].copy_from_slice(&key_at(index));
            record[16] = marker;
            records.push_sorted(record);
        }
        for (index, marker) in [(5_013, 1), (5_999, 2), (19_277, 3)] {
            assert_eq!(records.get(&key_at(index)).unwrap()[0], marker);
        }
        for index in [13, 5_012, 5_014, 6_013, 19_276] {
            assert!(records
                .get(&key_at(index))
                .unwrap_err()
                .contains("unknown key"));
        }
        let mut other_domain = key_at(5_013);
        other_domain[1] = 2;
        assert!(records.get(&other_domain).is_err());
    }

    #[test]
    fn incremental_partition_hashes_match_full_recompute() {
        let mut cfg = Config::preset(Load::C, JournalMode::Root);
        cfg.layout = StateLayout::Partitioned;
        cfg.hash = HashMode::Rehash;
        cfg.keys = 3000;
        cfg.commands = 3;
        cfg.auto_writes = 3;
        let layout = cfg.layout().unwrap();
        let mut state = Partitioned::genesis(&cfg, &layout, &seed_key(cfg.seed));
        let mut work = PartitionWork::new(&cfg, &layout, true);
        let initial = state.records.all_part_hashes();
        let mut changed = *state.records.get(&key_at(1073)).unwrap();
        changed[58] ^= 1;
        let original = *state.records.get(&key_at(31)).unwrap();
        let mut extra = original;
        set_counter(&mut extra, counter(&original) + 1);
        for (index, new) in [(1073, changed), (31, extra), (31, original)] {
            let key = key_at(index);
            work.writes.clear();
            work.writes.push(Write {
                key,
                new,
                old: state.records.slot(&key).unwrap(),
            });
            let start = work.merge(&mut state);
            let root = work.hash_resolution(&mut state, start);
            let all = state.records.all_part_hashes();
            assert_eq!(root, rehash_root(&all, state.count()));
            assert_eq!(state.hashes, Hashes::Rehash(all));
        }
        work.apply(&mut state);
        assert_eq!(work.coalesced_after_publish(), 2);
        let final_hashes = state.records.all_part_hashes();
        assert_eq!(final_hashes[0], initial[0]);
        assert_ne!(final_hashes[1], initial[1]);
        assert_eq!(final_hashes[2], initial[2]);
        assert!(state.checked_acc_root(cfg.keys * INITIAL_COUNTER).is_ok());
    }
}
