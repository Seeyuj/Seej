use std::collections::BTreeMap;

use crate::state::{
    counter, key_at, seed_key, set_buffer, set_counter, Key, State, Value, PARTITION_SIZE,
};
use crate::Config;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Command {
    pub(crate) seq: u32,
    pub(crate) keys: [u64; 4],
    pub(crate) amount: u64,
}

pub(crate) struct Write {
    pub(crate) key: Key,
    pub(crate) old: Value,
    pub(crate) new: Value,
}

pub(crate) struct View<'a> {
    pub(crate) state: &'a State,
    pub(crate) overlay: &'a BTreeMap<Key, Value>,
}

impl View<'_> {
    pub(crate) fn get(&self, key: &Key) -> Result<&Value, String> {
        self.overlay
            .get(key)
            .or_else(|| self.state.get(key))
            .ok_or_else(|| format!("unknown key {key:?}"))
    }
}

pub(crate) struct Rules<'a> {
    cfg: &'a Config,
    pub(crate) seed_key: [u8; 32],
}

impl<'a> Rules<'a> {
    pub(crate) fn new(cfg: &'a Config) -> Self {
        Self {
            cfg,
            seed_key: seed_key(cfg.seed),
        }
    }

    fn entropy(&self, domain: u8, a: u64, b: u64) -> [u8; 32] {
        let mut input = [0; 17];
        input[0] = domain;
        input[1..9].copy_from_slice(&a.to_le_bytes());
        input[9..].copy_from_slice(&b.to_le_bytes());
        *blake3::keyed_hash(&self.seed_key, &input).as_bytes()
    }

    pub(crate) fn command(&self, tick: u64, seq: u32) -> Command {
        let entropy = self.entropy(3, tick, u64::from(seq));
        let e: [u64; 4] = std::array::from_fn(|i| {
            u64::from_le_bytes(entropy[8 * i..8 * (i + 1)].try_into().unwrap())
        });
        let mut keys = e.map(|n| n % self.cfg.keys);
        let amount = (e[0] >> 32) % 100 + 1;
        let hot = self.cfg.hot_keys;
        if hot > 0 && (e[1] >> 32) % 100 < self.cfg.hot_percent {
            keys[1] = ((e[1] & 0xFFFF_FFFF) % hot) * (self.cfg.keys / hot);
        }
        for j in 1..keys.len() {
            while keys[..j].contains(&keys[j]) {
                keys[j] = (keys[j] + 1) % self.cfg.keys;
            }
        }
        Command { seq, keys, amount }
    }

    pub(crate) fn autonomous(
        &self,
        tick: u64,
        view: &View<'_>,
        writes: &mut Vec<Write>,
    ) -> Result<(), String> {
        writes.clear();
        if self.cfg.auto_writes == 0 {
            return Ok(());
        }
        let partitions = self.cfg.keys / PARTITION_SIZE;
        let count = self.cfg.auto_writes / partitions;
        let stride = PARTITION_SIZE / count;
        for part in 0..partitions {
            let entropy = self.entropy(2, tick, part);
            let phase = u64::from_le_bytes(entropy[..8].try_into().unwrap()) % stride;
            for i in 0..count {
                let index = part * PARTITION_SIZE + phase + i * stride;
                let key = key_at(index);
                let old = *view.get(&key)?;
                let mut new = old;
                set_counter(
                    &mut new,
                    counter(&old).checked_add(1).ok_or("counter overflow")?,
                );
                set_buffer(&mut new, tick);
                new[16..48].copy_from_slice(&self.entropy(1, tick, index));
                writes.push(Write { key, old, new });
            }
        }
        Ok(())
    }
}

pub(crate) fn resolve_command(
    command: &Command,
    view: &View<'_>,
    writes: &mut Vec<Write>,
) -> Result<(), String> {
    writes.clear();
    let keys = command.keys.map(key_at);
    let old0 = *view.get(&keys[0])?;
    let old1 = *view.get(&keys[1])?;
    let old2 = view.get(&keys[2])?;
    let old3 = view.get(&keys[3])?;
    let moved = command.amount.min(counter(&old0));
    let buffer = counter(old2) ^ counter(old3) ^ u64::from(command.seq);
    let mut new0 = old0;
    let mut new1 = old1;
    set_counter(&mut new0, counter(&old0) - moved);
    set_counter(
        &mut new1,
        counter(&old1)
            .checked_add(moved)
            .ok_or("counter overflow")?,
    );
    set_buffer(&mut new0, buffer);
    set_buffer(&mut new1, buffer);
    let first = Write {
        key: keys[0],
        old: old0,
        new: new0,
    };
    let second = Write {
        key: keys[1],
        old: old1,
        new: new1,
    };
    if first.key < second.key {
        writes.extend([first, second]);
    } else {
        writes.extend([second, first]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JournalMode, Load};

    #[test]
    fn autonomous_counter_overflow_fails_without_applying() {
        let mut cfg = Config::preset(Load::B, JournalMode::Root);
        cfg.keys = 1000;
        cfg.auto_writes = 1;
        let rules = Rules::new(&cfg);
        let state = State::genesis(cfg.keys, 1, &rules.seed_key);
        let mut overlay = BTreeMap::new();
        let mut writes = Vec::new();
        let view = View {
            state: &state,
            overlay: &overlay,
        };
        rules.autonomous(1, &view, &mut writes).unwrap();
        let key = writes[0].key;
        let original = *state.get(&key).unwrap();
        let mut value = original;
        set_counter(&mut value, u64::MAX);
        overlay.insert(key, value);
        let view = View {
            state: &state,
            overlay: &overlay,
        };
        assert_eq!(
            rules.autonomous(1, &view, &mut writes).unwrap_err(),
            "counter overflow"
        );
        assert!(writes.is_empty());
        assert_eq!(*view.get(&key).unwrap(), value);
        assert_eq!(*state.get(&key).unwrap(), original);

        set_counter(&mut value, u64::MAX - 1);
        overlay.insert(key, value);
        let view = View {
            state: &state,
            overlay: &overlay,
        };
        rules.autonomous(1, &view, &mut writes).unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].key, key);
        assert_eq!(counter(&writes[0].new), u64::MAX);
        assert_eq!(counter(view.get(&key).unwrap()), u64::MAX - 1);
    }
}
