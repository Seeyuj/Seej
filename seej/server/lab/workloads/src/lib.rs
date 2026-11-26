//! Standalone workload prototype for comparing exact-delta and replayed-rule journals.

mod checkpoint;
mod engine;
mod journal;
mod partitioned;
mod rules;
mod state;
mod summary;

use std::alloc::Layout as AllocationLayout;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use journal::FrameLayout;
use rules::{Command, Write};
use state::{Acc, ENTRY_BYTES, INITIAL_COUNTER, PARTITION_SIZE};
use summary::Sample;

pub use engine::{recover, run};
pub use summary::{PhaseStats, RecoverSummary, RunSummary};

pub const INVALID_CONFIG: &str = "invalid configuration: ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Load {
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum JournalMode {
    Delta,
    Root,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum StateLayout {
    #[default]
    Btree,
    Partitioned,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum HashMode {
    #[default]
    Acc,
    AccCached,
    Rehash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub load: Load,
    pub seed: u64,
    pub keys: u64,
    pub ticks: u64,
    pub warmup_ticks: u64,
    pub checkpoint_at: u64,
    pub auto_writes: u64,
    pub commands: u64,
    pub hot_keys: u64,
    pub hot_percent: u64,
    pub journal: JournalMode,
    #[serde(default)]
    pub layout: StateLayout,
    #[serde(default)]
    pub hash: HashMode,
}

pub(crate) struct Layout {
    partitions: usize,
    measured_ticks: usize,
    resolved_capacity: usize,
    undo_capacity: usize,
    command_count: u32,
    command: FrameLayout,
    closure: FrameLayout,
    tick_bytes: u64,
    journal_bytes: u64,
    checkpoint_bytes: u64,
    expected_sum: u64,
}

impl Config {
    pub fn preset(load: Load, journal: JournalMode) -> Config {
        let (auto_writes, commands, hot_keys, hot_percent) = match load {
            Load::A => (0, 1000, 0, 0),
            Load::B => (100_000, 0, 0, 0),
            Load::C => (1000, 1000, 32, 80),
        };
        Config {
            load,
            seed: 42,
            keys: 1_000_000,
            ticks: 300,
            warmup_ticks: 10,
            checkpoint_at: 100,
            auto_writes,
            commands,
            hot_keys,
            hot_percent,
            journal,
            layout: StateLayout::Btree,
            hash: HashMode::Acc,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.layout().map(|_| ())
    }

    fn layout(&self) -> Result<Layout, String> {
        if self.layout == StateLayout::Btree && self.hash != HashMode::Acc {
            return Err("btree layout requires --hash acc".into());
        }
        if self.keys < PARTITION_SIZE || !self.keys.is_multiple_of(PARTITION_SIZE) {
            return Err("keys must be a positive multiple of 1000".into());
        }
        let partitions = self.keys / PARTITION_SIZE;
        if u32::try_from(partitions - 1).is_err() {
            return Err("last partition index exceeds u32".into());
        }
        usize::try_from(self.keys).map_err(|_| "keys exceed usize")?;
        if self.auto_writes != 0 {
            if self.auto_writes > self.keys || !self.auto_writes.is_multiple_of(partitions) {
                return Err(
                    "auto-writes must be at most keys and evenly distributed among partitions"
                        .into(),
                );
            }
            let per_partition = self.auto_writes / partitions;
            if per_partition == 0 || !PARTITION_SIZE.is_multiple_of(per_partition) {
                return Err("auto-writes per partition must divide 1000".into());
            }
        }
        if self.checkpoint_at == 0 || self.checkpoint_at >= self.ticks {
            return Err("checkpoint-at must be strictly between zero and ticks".into());
        }
        if self.warmup_ticks >= self.ticks {
            return Err("warmup-ticks must be less than ticks".into());
        }
        let command_count = u32::try_from(self.commands).map_err(|_| "commands exceed u32")?;
        if self.hot_keys > self.keys
            || self.hot_percent > 100
            || (self.hot_percent > 0 && self.hot_keys == 0)
        {
            return Err("hot-keys cannot exceed keys; hot-percent must be 0..=100 and requires hot-keys if positive".into());
        }
        let expected_sum = self
            .keys
            .checked_mul(INITIAL_COUNTER)
            .and_then(|genesis| {
                self.ticks
                    .checked_mul(self.auto_writes)
                    .and_then(|extra| genesis.checked_add(extra))
            })
            .ok_or("counter sum exceeds u64")?;
        let delta = self.journal == JournalMode::Delta;
        let command = FrameLayout::new(true, if delta { 2 } else { 0 })?;
        let closure = FrameLayout::new(false, if delta { self.auto_writes } else { 0 })?;
        let tick_bytes = self
            .commands
            .checked_mul(command.bytes())
            .and_then(|bytes| bytes.checked_add(closure.bytes()))
            .ok_or("journal bytes per tick exceed u64")?;
        let journal_bytes = self
            .ticks
            .checked_mul(tick_bytes)
            .ok_or("journal size exceeds u64")?;
        let checkpoint_bytes = self
            .keys
            .checked_mul(ENTRY_BYTES as u64)
            .and_then(|size| size.checked_add(checkpoint::CHECKPOINT_OVERHEAD))
            .ok_or("checkpoint size exceeds u64")?;
        allocation_count::<Command>(self.commands, "command buffer")?;
        allocation_count::<u8>(tick_bytes, "journal tick buffer")?;
        let undo_capacity = if self.layout == StateLayout::Partitioned {
            partitioned::validate_capacity(self, partitions)?
        } else {
            0
        };
        Ok(Layout {
            partitions: allocation_count::<Acc>(partitions, "partition accumulators")?,
            measured_ticks: allocation_count::<Sample>(
                self.ticks - self.warmup_ticks,
                "measured samples",
            )?,
            resolved_capacity: allocation_count::<Write>(
                self.auto_writes.max(if self.commands > 0 { 2 } else { 0 }),
                "resolution buffer",
            )?,
            undo_capacity,
            command_count,
            command,
            closure,
            tick_bytes,
            journal_bytes,
            checkpoint_bytes,
            expected_sum,
        })
    }
}

fn allocation_count<T>(count: u64, name: &str) -> Result<usize, String> {
    let count = usize::try_from(count).map_err(|_| format!("{name} exceeds usize"))?;
    AllocationLayout::array::<T>(count).map_err(|_| format!("{name} is too large to allocate"))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_presets_and_default_load_b_layout() {
        for load in [Load::A, Load::B, Load::C] {
            for journal in [JournalMode::Delta, JournalMode::Root] {
                Config::preset(load, journal).validate().unwrap();
            }
        }
        let shape = Config::preset(Load::B, JournalMode::Delta)
            .layout()
            .unwrap();
        assert_eq!(shape.tick_bytes, 14_400_089);
        assert_eq!(shape.journal_bytes, 4_320_026_700);
        assert_eq!(shape.checkpoint_bytes, 144_000_068);
    }

    #[test]
    fn validates_bounds_before_allocation() {
        let cfg = Config::preset(Load::B, JournalMode::Root);
        let cases = [
            (
                Config {
                    keys: 999,
                    ..cfg.clone()
                },
                "multiple of 1000",
            ),
            (
                Config {
                    keys: 1001,
                    ..cfg.clone()
                },
                "multiple of 1000",
            ),
            (
                Config {
                    keys: (u64::from(u32::MAX) + 2) * 1000,
                    ..cfg.clone()
                },
                "partition index",
            ),
            (
                Config {
                    auto_writes: 1,
                    ..cfg.clone()
                },
                "evenly distributed",
            ),
            (
                Config {
                    auto_writes: 3000,
                    ..cfg.clone()
                },
                "must divide",
            ),
            (
                Config {
                    checkpoint_at: 0,
                    ..cfg.clone()
                },
                "checkpoint-at",
            ),
            (
                Config {
                    warmup_ticks: cfg.ticks,
                    ..cfg.clone()
                },
                "warmup-ticks",
            ),
            (
                Config {
                    commands: u64::from(u32::MAX) + 1,
                    ..cfg.clone()
                },
                "commands",
            ),
            (
                Config {
                    hot_percent: 1,
                    ..cfg.clone()
                },
                "hot-percent",
            ),
            (
                Config {
                    ticks: u64::MAX,
                    ..cfg.clone()
                },
                "counter sum",
            ),
            (
                Config {
                    auto_writes: 0,
                    ticks: u64::MAX,
                    ..cfg.clone()
                },
                "journal size",
            ),
            (
                Config {
                    keys: 30_000_000,
                    auto_writes: 30_000_000,
                    journal: JournalMode::Delta,
                    ..cfg
                },
                "journal frame body",
            ),
        ];
        for (bad, message) in cases {
            let error = bad.validate().unwrap_err();
            assert!(error.contains(message), "{error}");
        }
    }
}
