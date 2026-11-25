use std::collections::BTreeMap;

pub(crate) const PARTITION_SIZE: u64 = 1000;
pub(crate) const INITIAL_COUNTER: u64 = 1_000_000;
pub(crate) const ENTRY_BYTES: usize = 144;
pub(crate) type Key = [u8; 16];
pub(crate) type Value = [u8; 128];
pub(crate) type Acc = [u64; 4];
pub(crate) type Root = [u8; 32];

pub(crate) struct State {
    map: BTreeMap<Key, Value>,
    world: Acc,
    parts: Vec<Acc>,
    count: u64,
}

pub(crate) struct AccDelta {
    world: Acc,
    parts: Vec<Acc>,
    count: u64,
}

pub(crate) fn key_at(index: u64) -> Key {
    let mut key = [0; 16];
    key[..2].copy_from_slice(&1u16.to_be_bytes());
    key[2..4].copy_from_slice(&1u16.to_be_bytes());
    key[4..8].copy_from_slice(&(index / PARTITION_SIZE).to_be_bytes()[4..]);
    key[8..].copy_from_slice(&(index % PARTITION_SIZE).to_be_bytes());
    key
}

pub(crate) fn counter(value: &Value) -> u64 {
    u64::from_le_bytes(value[..8].try_into().unwrap())
}

pub(crate) fn set_counter(value: &mut Value, next: u64) {
    value[..8].copy_from_slice(&next.to_le_bytes());
}

pub(crate) fn set_buffer(value: &mut Value, next: u64) {
    value[8..16].copy_from_slice(&next.to_le_bytes());
}

pub(crate) fn seed_key(seed: u64) -> [u8; 32] {
    blake3::derive_key("seej lab workloads v1 seed", &seed.to_le_bytes())
}

pub(crate) fn genesis_value(seed_key: &[u8; 32], index: u64) -> Value {
    let mut value = [0; 128];
    set_counter(&mut value, INITIAL_COUNTER);
    blake3::Hasher::new_keyed(seed_key)
        .update(&[0])
        .update(&index.to_le_bytes())
        .finalize_xof()
        .fill(&mut value[16..]);
    value
}

fn partition(key: &Key) -> usize {
    u32::from_be_bytes(key[4..8].try_into().unwrap()) as usize
}

fn digest(key: &Key, value: &Value) -> Acc {
    let mut entry = [0; ENTRY_BYTES];
    entry[..16].copy_from_slice(key);
    entry[16..].copy_from_slice(value);
    let hash = blake3::hash(&entry);
    std::array::from_fn(|i| {
        u64::from_le_bytes(hash.as_bytes()[8 * i..8 * (i + 1)].try_into().unwrap())
    })
}

fn add(lhs: &mut Acc, rhs: &Acc) {
    let mut carry = false;
    for (l, &r) in lhs.iter_mut().zip(rhs) {
        let (sum, overflow) = l.overflowing_add(r);
        let (sum, carry_overflow) = sum.overflowing_add(u64::from(carry));
        *l = sum;
        carry = overflow || carry_overflow;
    }
}

fn subtract(lhs: &mut Acc, rhs: &Acc) {
    let mut borrow = false;
    for (l, &r) in lhs.iter_mut().zip(rhs) {
        let (diff, underflow) = l.overflowing_sub(r);
        let (diff, borrow_underflow) = diff.overflowing_sub(u64::from(borrow));
        *l = diff;
        borrow = underflow || borrow_underflow;
    }
}

pub(crate) fn root_of(world: &Acc, count: u64) -> Root {
    const DOMAIN: &[u8; 16] = b"seej lab root v1";
    let mut bytes = [0; 56];
    bytes[..16].copy_from_slice(DOMAIN);
    for (dst, limb) in bytes[16..48].chunks_exact_mut(8).zip(world) {
        dst.copy_from_slice(&limb.to_le_bytes());
    }
    bytes[48..].copy_from_slice(&count.to_le_bytes());
    *blake3::hash(&bytes).as_bytes()
}

pub(crate) fn root_hex(root: &Root) -> String {
    blake3::Hash::from_bytes(*root).to_hex().to_string()
}

impl AccDelta {
    pub(crate) fn new(partitions: usize) -> Self {
        Self {
            world: [0; 4],
            parts: vec![[0; 4]; partitions],
            count: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.world = [0; 4];
        self.parts.fill([0; 4]);
        self.count = 0;
    }

    pub(crate) fn record(&mut self, key: &Key, old: Option<&Value>, new: &Value) {
        let mut change = digest(key, new);
        if let Some(old) = old {
            subtract(&mut change, &digest(key, old));
        } else {
            self.count += 1;
        }
        add(&mut self.world, &change);
        add(&mut self.parts[partition(key)], &change);
    }
}

impl State {
    pub(crate) fn genesis(keys: u64, partitions: usize, seed_key: &[u8; 32]) -> Self {
        let mut map = BTreeMap::new();
        let mut delta = AccDelta::new(partitions);
        for i in 0..keys {
            let key = key_at(i);
            let value = genesis_value(seed_key, i);
            delta.record(&key, None, &value);
            map.insert(key, value);
        }
        Self::from_accumulated(map, delta)
    }

    pub(crate) fn from_checkpoint(map: BTreeMap<Key, Value>, partitions: usize) -> Self {
        let mut delta = AccDelta::new(partitions);
        for (key, value) in &map {
            delta.record(key, None, value);
        }
        Self::from_accumulated(map, delta)
    }

    fn from_accumulated(map: BTreeMap<Key, Value>, delta: AccDelta) -> Self {
        Self {
            map,
            world: delta.world,
            parts: delta.parts,
            count: delta.count,
        }
    }

    pub(crate) fn get(&self, key: &Key) -> Option<&Value> {
        self.map.get(key)
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.map.iter()
    }

    pub(crate) fn count(&self) -> u64 {
        self.count
    }

    pub(crate) fn root(&self) -> Root {
        root_of(&self.world, self.count)
    }

    pub(crate) fn root_after(&self, delta: &AccDelta) -> Root {
        let mut world = self.world;
        add(&mut world, &delta.world);
        root_of(&world, self.count + delta.count)
    }

    pub(crate) fn apply(&mut self, writes: impl Iterator<Item = (Key, Value)>, delta: &AccDelta) {
        for (key, value) in writes {
            self.map.insert(key, value);
        }
        add(&mut self.world, &delta.world);
        for (part, change) in self.parts.iter_mut().zip(&delta.parts) {
            add(part, change);
        }
        self.count += delta.count;
    }

    pub(crate) fn full_scan(&self) -> u128 {
        self.map
            .values()
            .map(|value| u128::from(counter(value)))
            .sum()
    }

    pub(crate) fn full_recompute(&self) -> (Acc, Vec<Acc>, u64, u64) {
        let mut delta = AccDelta::new(self.parts.len());
        let mut sum = 0u64;
        for (key, value) in self.entries() {
            delta.record(key, None, value);
            // Callers first verify the independently scanned sum fits in u64.
            sum += counter(value);
        }
        (delta.world, delta.parts, delta.count, sum)
    }

    pub(crate) fn matches_recomputed(&self, world: &Acc, parts: &[Acc], count: u64) -> bool {
        &self.world == world && self.parts == parts && self.count == count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_wraps_with_carry_and_borrow() {
        let mut limbs = [u64::MAX; 4];
        let one = [1, 0, 0, 0];
        add(&mut limbs, &one);
        assert_eq!(limbs, [0; 4]);
        subtract(&mut limbs, &one);
        assert_eq!(limbs, [u64::MAX; 4]);
    }

    #[test]
    fn root_is_independent_of_insertion_order() {
        let seed = seed_key(42);
        let entries: Vec<_> = (0..3000)
            .map(|i| (key_at(i), genesis_value(&seed, i)))
            .collect();
        let mut direct = State::genesis(3000, 3, &seed);
        let mut inverse = State::genesis(0, 3, &seed);
        let mut blocks = State::genesis(0, 3, &seed);
        let mut delta = AccDelta::new(3);
        for (key, value) in entries.iter().rev() {
            delta.record(key, None, value);
        }
        inverse.apply(entries.iter().rev().copied(), &delta);
        for block in entries.chunks(500).rev() {
            delta.reset();
            for (key, value) in block {
                delta.record(key, None, value);
            }
            blocks.apply(block.iter().copied(), &delta);
        }
        assert_eq!(direct.root(), inverse.root());
        assert_eq!(direct.root(), blocks.root());
        for state in [&mut direct, &mut inverse, &mut blocks] {
            let updates = [key_at(22), key_at(1499), key_at(2001)].map(|key| {
                let mut new = *state.get(&key).unwrap();
                set_counter(&mut new, 7);
                (key, new)
            });
            delta.reset();
            for (key, new) in &updates {
                delta.record(key, state.get(key), new);
            }
            let predicted = state.root_after(&delta);
            state.apply(updates.into_iter(), &delta);
            let (world, parts, count, _) = state.full_recompute();
            assert!(state.matches_recomputed(&world, &parts, count));
            for (p, actual) in state.parts.iter().enumerate() {
                let start = p as u64 * PARTITION_SIZE;
                let mut sum = [0u8; 32];
                for i in start..start + PARTITION_SIZE {
                    let key = key_at(i);
                    let value = state.get(&key).unwrap();
                    let hash = blake3::Hasher::new().update(&key).update(value).finalize();
                    let mut carry = 0u16;
                    for (dst, byte) in sum.iter_mut().zip(hash.as_bytes()) {
                        let total = u16::from(*dst) + u16::from(*byte) + carry;
                        *dst = total as u8;
                        carry = total >> 8;
                    }
                }
                let expected: Acc = std::array::from_fn(|i| {
                    u64::from_le_bytes(sum[8 * i..8 * (i + 1)].try_into().unwrap())
                });
                assert_eq!(*actual, expected, "partition {p}");
            }
            assert_eq!(state.root(), predicted);
        }
        assert_eq!(direct.root(), inverse.root());
        assert_eq!(direct.root(), blocks.root());
    }
}
