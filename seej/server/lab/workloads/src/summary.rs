use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{HashMode, JournalMode, Load, StateLayout};

pub(crate) const BUDGET_MS: u64 = 100;

#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseStats {
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub load: Load,
    pub journal: JournalMode,
    pub layout: StateLayout,
    pub hash_mode: HashMode,
    pub seed: u64,
    pub keys: u64,
    pub ticks: u64,
    pub measured_ticks: u64,
    pub auto_writes: u64,
    pub commands: u64,
    pub final_root: String,
    pub acc_root: String,
    pub genesis_ms: f64,
    pub gen: PhaseStats,
    pub compute: PhaseStats,
    pub hash: PhaseStats,
    pub encode: PhaseStats,
    pub merge: PhaseStats,
    pub commit: PhaseStats,
    pub apply: PhaseStats,
    pub total: PhaseStats,
    pub budget_ms: u64,
    pub budget_met: bool,
    pub journal_bytes_total: u64,
    pub journal_bytes_per_tick_mean: f64,
    pub journal_mb_per_s_at_10hz: f64,
    pub coalesced_bytes_per_tick_mean: f64,
    pub resolutions_per_tick: u64,
    pub checkpoint_write_ms: f64,
    pub checkpoint_tick_total_ms: f64,
    pub checkpoint_bytes: u64,
    pub full_scan_ms: f64,
    pub full_recompute_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverSummary {
    pub checkpoint_load_ms: f64,
    pub checkpoint_verify_ms: f64,
    pub replay_ms: f64,
    pub replayed_ticks: u64,
    pub replayed_resolutions: u64,
    pub replay_ms_per_tick: f64,
    pub replay_ratio_to_budget: f64,
    pub journal_bytes_read: u64,
    pub final_root: String,
    pub acc_root: String,
    pub root_matches_run: Option<bool>,
}

#[derive(Default)]
pub(crate) struct TickTimes {
    pub(crate) gen: Duration,
    pub(crate) compute: Duration,
    pub(crate) hash: Duration,
    pub(crate) encode: Duration,
    pub(crate) merge: Duration,
    pub(crate) commit: Duration,
    pub(crate) apply: Duration,
    pub(crate) total: Duration,
}

pub(crate) struct Sample {
    pub(crate) times: TickTimes,
    pub(crate) journal_bytes: u64,
    pub(crate) coalesced_bytes: u64,
}

pub(crate) fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

pub(crate) fn phase_stats(
    samples: &[Sample],
    phase: impl Fn(&TickTimes) -> Duration,
    scratch: &mut Vec<Duration>,
) -> PhaseStats {
    scratch.clear();
    scratch.extend(samples.iter().map(|sample| phase(&sample.times)));
    scratch.sort_unstable();
    let n = scratch.len();
    PhaseStats {
        mean_ms: scratch.iter().copied().map(ms).sum::<f64>() / n as f64,
        p50_ms: ms(scratch[n.div_ceil(2) - 1]),
        p99_ms: ms(scratch[n - n / 100 - 1]),
        max_ms: ms(scratch[n - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_ceiling_rank() {
        let samples: Vec<_> = (1..=290)
            .rev()
            .map(|n| Sample {
                times: TickTimes {
                    total: Duration::from_millis(n),
                    ..TickTimes::default()
                },
                journal_bytes: 0,
                coalesced_bytes: 0,
            })
            .collect();
        let stats = phase_stats(&samples, |times| times.total, &mut Vec::new());
        assert_eq!(stats.p50_ms, 145.0);
        assert_eq!(stats.p99_ms, 288.0);
        assert_eq!(stats.max_ms, 290.0);
    }
}
