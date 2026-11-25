use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write as _};
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::checkpoint;
use crate::journal::{self, Action, Decoded, JournalReader, Resolution};
use crate::rules::{self, Command, Rules, View, Write};
use crate::state::{key_at, root_hex, root_of, AccDelta, Key, Root, State, Value, ENTRY_BYTES};
use crate::summary::{ms, phase_stats, Sample, TickTimes, BUDGET_MS};
use crate::{Config, JournalMode, Layout, RecoverSummary, RunSummary, INVALID_CONFIG};

struct TickWork {
    overlay: BTreeMap<Key, Value>,
    delta: AccDelta,
    writes: Vec<Write>,
}

impl TickWork {
    fn new(partitions: usize, resolved_capacity: usize) -> Self {
        Self {
            overlay: BTreeMap::new(),
            delta: AccDelta::new(partitions),
            writes: Vec::with_capacity(resolved_capacity),
        }
    }

    fn resolve(
        &mut self,
        state: &State,
        rules: &Rules<'_>,
        action: Action,
        tick: u64,
    ) -> Result<(), String> {
        let view = View {
            state,
            overlay: &self.overlay,
        };
        match action {
            Action::Command(cmd) => rules::resolve_command(&cmd, &view, &mut self.writes),
            Action::Closure { .. } => rules.autonomous(tick, &view, &mut self.writes),
        }
    }

    fn hash_resolved(&mut self, state: &State) -> Root {
        for write in &self.writes {
            self.delta.record(&write.key, Some(&write.old), &write.new);
        }
        state.root_after(&self.delta)
    }

    fn merge_resolved(&mut self) {
        for write in &self.writes {
            self.overlay.insert(write.key, write.new);
        }
    }

    fn replay_resolution(
        &mut self,
        state: &State,
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
        let post_root = match mode {
            JournalMode::Delta => {
                if let Action::Command(command) = frame.action {
                    let [source, target, ..] = command.keys;
                    let expected = [key_at(source.min(target)), key_at(source.max(target))];
                    if !decoded.entries().map(|(key, _)| key).eq(&expected) {
                        return Err(format!(
                            "journal writes do not match command keys at tick {}, seq {}",
                            frame.tick, command.seq
                        ));
                    }
                }
                let view = View {
                    state,
                    overlay: &self.overlay,
                };
                let mut prev_key = None;
                for (key, value) in decoded.entries() {
                    if prev_key.is_some_and(|prev| prev >= key) {
                        return Err(format!(
                            "journal writes not strictly ordered at tick {}",
                            frame.tick
                        ));
                    }
                    let old = view.get(key)?;
                    self.delta.record(key, Some(old), value);
                    prev_key = Some(key);
                }
                state.root_after(&self.delta)
            }
            JournalMode::Root => {
                self.resolve(state, rules, frame.action, frame.tick)?;
                self.hash_resolved(state)
            }
        };
        require_root(
            &post_root,
            &frame.post_root,
            format_args!("post-root at tick {}", frame.tick),
        )?;
        match mode {
            JournalMode::Delta => {
                for (key, value) in decoded.entries() {
                    self.overlay.insert(*key, *value);
                }
            }
            JournalMode::Root => self.merge_resolved(),
        }
        Ok(post_root)
    }

    fn apply(&mut self, state: &mut State) {
        state.apply(std::mem::take(&mut self.overlay).into_iter(), &self.delta);
        self.delta.reset();
    }
}

struct LiveTick<'a> {
    layout: &'a Layout,
    mode: JournalMode,
    rules: Rules<'a>,
    work: TickWork,
    commands: Vec<Command>,
    bytes: Vec<u8>,
}

impl<'a> LiveTick<'a> {
    fn new(cfg: &'a Config, layout: &'a Layout, rules: Rules<'a>) -> Self {
        Self {
            layout,
            mode: cfg.journal,
            rules,
            work: TickWork::new(layout.partitions, layout.resolved_capacity),
            commands: Vec::with_capacity(layout.command_count as usize),
            bytes: Vec::with_capacity(layout.tick_bytes as usize),
        }
    }

    fn stage(&mut self, state: &State, tick: u64, times: &mut TickTimes) -> Result<(), String> {
        let start = Instant::now();
        self.commands.clear();
        self.bytes.clear();
        self.commands
            .extend((0..self.layout.command_count).map(|seq| self.rules.command(tick, seq)));
        times.gen = start.elapsed();
        let start = Instant::now();
        let mut root = state.root();
        times.hash = start.elapsed();
        for index in 0..self.commands.len() {
            root = self.stage_resolution(
                state,
                tick,
                Action::Command(self.commands[index]),
                root,
                times,
            )?;
        }
        self.stage_resolution(
            state,
            tick,
            Action::Closure {
                seq: self.layout.command_count,
            },
            root,
            times,
        )?;
        debug_assert_eq!(self.bytes.len() as u64, self.layout.tick_bytes);
        Ok(())
    }

    fn stage_resolution(
        &mut self,
        state: &State,
        tick: u64,
        action: Action,
        pre_root: Root,
        times: &mut TickTimes,
    ) -> Result<Root, String> {
        let start = Instant::now();
        self.work.resolve(state, &self.rules, action, tick)?;
        times.compute += start.elapsed();
        let start = Instant::now();
        let post_root = self.work.hash_resolved(state);
        times.hash += start.elapsed();
        let start = Instant::now();
        let frame = Resolution {
            action,
            tick,
            pre_root,
            post_root,
        };
        let layout = match action {
            Action::Command(_) => self.layout.command,
            Action::Closure { .. } => self.layout.closure,
        };
        journal::encode(
            &mut self.bytes,
            &frame,
            layout,
            self.mode,
            &self.work.writes,
        );
        times.encode += start.elapsed();
        let start = Instant::now();
        self.work.merge_resolved();
        times.merge += start.elapsed();
        Ok(post_root)
    }
}

pub fn run(cfg: &Config, dir: &Path) -> Result<RunSummary, String> {
    let layout = config_layout(cfg)?;
    prepare_run_dir(dir)?;
    write_json(&dir.join("config.json"), cfg)?;
    let path = dir.join("journal.bin");
    let mut file = File::create_new(&path).map_err(|e| file_error(&path, e))?;
    let start = Instant::now();
    let rules = Rules::new(cfg);
    let mut state = State::genesis(cfg.keys, layout.partitions, &rules.seed_key);
    let genesis_ms = ms(start.elapsed());
    let mut live = LiveTick::new(cfg, &layout, rules);
    let mut samples = Vec::with_capacity(layout.measured_ticks);
    let mut offset = 0;
    let mut checkpoint_write_ms = 0.0;
    let mut checkpoint_tick_total_ms = 0.0;
    let mut checkpoint_bytes = 0;
    for tick in 1..=cfg.ticks {
        let tick_start = Instant::now();
        let mut times = TickTimes::default();
        live.stage(&state, tick, &mut times)?;
        let bytes = live.bytes.len() as u64;
        let coalesced_bytes = live.work.overlay.len() as u64 * ENTRY_BYTES as u64;
        let start = Instant::now();
        file.write_all(&live.bytes)
            .and_then(|()| file.sync_data())
            .map_err(|e| file_error(&path, e))?;
        times.commit = start.elapsed();
        offset += bytes;
        let start = Instant::now();
        live.work.apply(&mut state);
        times.apply = start.elapsed();
        if tick == cfg.checkpoint_at {
            let start = Instant::now();
            checkpoint_bytes = checkpoint::write(dir, tick, offset, &state)?;
            checkpoint_write_ms = ms(start.elapsed());
            if checkpoint_bytes != layout.checkpoint_bytes {
                return Err("checkpoint size does not match config".into());
            }
        }
        times.total = tick_start.elapsed();
        if tick == cfg.checkpoint_at {
            checkpoint_tick_total_ms = ms(times.total);
        }
        if tick > cfg.warmup_ticks {
            samples.push(Sample {
                times,
                journal_bytes: bytes,
                coalesced_bytes,
            });
        }
    }
    let journal_bytes_total = file.metadata().map_err(|e| file_error(&path, e))?.len();
    if journal_bytes_total != offset || journal_bytes_total != layout.journal_bytes {
        return Err("journal size does not match committed ticks".into());
    }
    let (full_scan_ms, full_recompute_ms) = verify_state(&state, &layout)?;
    let mut scratch = Vec::with_capacity(samples.len());
    let total = phase_stats(&samples, |t| t.total, &mut scratch);
    let budget_met = total.p99_ms <= BUDGET_MS as f64;
    let journal_bytes_per_tick_mean = byte_mean(&samples, |s| s.journal_bytes);
    let summary = RunSummary {
        load: cfg.load,
        journal: cfg.journal,
        seed: cfg.seed,
        keys: cfg.keys,
        ticks: cfg.ticks,
        measured_ticks: samples.len() as u64,
        auto_writes: cfg.auto_writes,
        commands: cfg.commands,
        final_root: root_hex(&state.root()),
        genesis_ms,
        gen: phase_stats(&samples, |t| t.gen, &mut scratch),
        compute: phase_stats(&samples, |t| t.compute, &mut scratch),
        hash: phase_stats(&samples, |t| t.hash, &mut scratch),
        encode: phase_stats(&samples, |t| t.encode, &mut scratch),
        merge: phase_stats(&samples, |t| t.merge, &mut scratch),
        commit: phase_stats(&samples, |t| t.commit, &mut scratch),
        apply: phase_stats(&samples, |t| t.apply, &mut scratch),
        total,
        budget_ms: BUDGET_MS,
        budget_met,
        journal_bytes_total,
        journal_bytes_per_tick_mean,
        journal_mb_per_s_at_10hz: journal_bytes_per_tick_mean * 10.0 / 1_000_000.0,
        coalesced_bytes_per_tick_mean: byte_mean(&samples, |s| s.coalesced_bytes),
        resolutions_per_tick: cfg.commands + 1,
        checkpoint_write_ms,
        checkpoint_tick_total_ms,
        checkpoint_bytes,
        full_scan_ms,
        full_recompute_ms,
    };
    write_json(&dir.join("summary.json"), &summary)?;
    Ok(summary)
}

pub fn recover(dir: &Path) -> Result<RecoverSummary, String> {
    let config_path = dir.join("config.json");
    let config_file = File::open(&config_path).map_err(|e| file_error(&config_path, e))?;
    let cfg: Config = serde_json::from_reader(BufReader::new(config_file)).map_err(|e| {
        if e.is_io() {
            file_error(&config_path, e)
        } else {
            format!("{INVALID_CONFIG}{}: {e}", config_path.display())
        }
    })?;
    let layout = config_layout(&cfg)?;
    let start = Instant::now();
    let checkpoint = checkpoint::read(dir, &cfg, &layout)?;
    let checkpoint_load_ms = ms(start.elapsed());
    let start = Instant::now();
    let mut state = State::from_checkpoint(checkpoint.entries, layout.partitions);
    require_root(&state.root(), &checkpoint.root, "checkpoint")?;
    let checkpoint_verify_ms = ms(start.elapsed());
    let path = dir.join("journal.bin");
    let source = File::open(&path).map_err(|e| file_error(&path, e))?;
    let mut reader =
        JournalReader::new(source, checkpoint.journal_offset).map_err(|e| file_error(&path, e))?;
    let rules = Rules::new(&cfg);
    let capacity = match cfg.journal {
        JournalMode::Delta => 0,
        JournalMode::Root => layout.resolved_capacity,
    };
    let mut work = TickWork::new(layout.partitions, capacity);
    let start = Instant::now();
    for tick in (cfg.checkpoint_at + 1)..=cfg.ticks {
        replay_tick(
            &cfg,
            &layout,
            &rules,
            &mut work,
            &mut state,
            &mut reader,
            tick,
        )?;
    }
    let journal_end = reader.finish()?;
    let replay_ms = ms(start.elapsed());
    verify_state(&state, &layout)?;
    let replayed_ticks = cfg.ticks - cfg.checkpoint_at;
    let replay_ms_per_tick = replay_ms / replayed_ticks as f64;
    let final_root = root_hex(&state.root());
    let root_matches_run = compare_witness(dir, &final_root)?;
    let summary = RecoverSummary {
        checkpoint_load_ms,
        checkpoint_verify_ms,
        replay_ms,
        replayed_ticks,
        replayed_resolutions: replayed_ticks * (cfg.commands + 1),
        replay_ms_per_tick,
        replay_ratio_to_budget: replay_ms_per_tick / BUDGET_MS as f64,
        journal_bytes_read: journal_end - checkpoint.journal_offset,
        final_root,
        root_matches_run,
    };
    write_json(&dir.join("recover_summary.json"), &summary)?;
    Ok(summary)
}

fn replay_tick(
    cfg: &Config,
    layout: &Layout,
    rules: &Rules<'_>,
    work: &mut TickWork,
    state: &mut State,
    reader: &mut JournalReader,
    tick: u64,
) -> Result<(), String> {
    let mut pre = state.root();
    for seq in 0..=layout.command_count {
        let frame_layout = if seq < layout.command_count {
            layout.command
        } else {
            layout.closure
        };
        let decoded = reader.next(cfg.keys, frame_layout, tick, seq)?;
        pre = work.replay_resolution(state, rules, cfg.journal, &pre, &decoded)?;
    }
    work.apply(state);
    Ok(())
}

fn require_root(expected: &Root, actual: &Root, location: impl Display) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "RECOVERY ROOT MISMATCH ({location}): expected {}, got {}",
            root_hex(expected),
            root_hex(actual)
        ))
    }
}

fn verify_state(state: &State, layout: &Layout) -> Result<(f64, f64), String> {
    let start = Instant::now();
    let sum = state.full_scan();
    let full_scan_ms = ms(start.elapsed());
    if sum != u128::from(layout.expected_sum) {
        return Err(format!(
            "counter sum mismatch: expected {}, got {sum}",
            layout.expected_sum
        ));
    }
    let start = Instant::now();
    let (world, parts, count, sum) = state.full_recompute();
    let full_recompute_ms = ms(start.elapsed());
    require_root(
        &root_of(&world, count),
        &state.root(),
        "full-state recompute",
    )?;
    if !state.matches_recomputed(&world, &parts, count) || sum != layout.expected_sum {
        return Err("recomputed state accumulators or counter sum mismatch".into());
    }
    Ok((full_scan_ms, full_recompute_ms))
}

fn config_layout(cfg: &Config) -> Result<Layout, String> {
    cfg.layout().map_err(|err| format!("{INVALID_CONFIG}{err}"))
}

fn prepare_run_dir(dir: &Path) -> Result<(), String> {
    match fs::read_dir(dir) {
        Ok(mut entries) => {
            if entries
                .next()
                .transpose()
                .map_err(|e| file_error(dir, e))?
                .is_some()
            {
                return Err(format!("run directory is not empty: {}", dir.display()));
            }
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(dir).map_err(|e| file_error(dir, e))
        }
        Err(e) => Err(file_error(dir, e)),
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let file = File::create(path).map_err(|e| file_error(path, e))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|e| file_error(path, e))?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|e| file_error(path, e))
}

fn compare_witness(dir: &Path, root: &str) -> Result<Option<bool>, String> {
    #[derive(Deserialize)]
    struct Witness {
        final_root: String,
    }
    let path = dir.join("summary.json");
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(file_error(&path, e)),
    };
    let witness: Witness =
        serde_json::from_reader(BufReader::new(file)).map_err(|e| file_error(&path, e))?;
    if witness.final_root != root {
        return Err(format!(
            "RECOVERY ROOT MISMATCH (summary.json): expected {}, got {root}",
            witness.final_root
        ));
    }
    Ok(Some(true))
}

fn file_error(path: &Path, err: impl Display) -> String {
    format!("{}: {err}", path.display())
}

fn byte_mean(samples: &[Sample], bytes: impl Fn(&Sample) -> u64) -> f64 {
    samples
        .iter()
        .map(|sample| u128::from(bytes(sample)))
        .sum::<u128>() as f64
        / samples.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{counter, key_at, set_counter, INITIAL_COUNTER};
    use crate::Load;

    #[test]
    fn second_command_reads_the_first_commands_write() {
        let cfg = Config::preset(Load::A, JournalMode::Delta);
        let layout = cfg.layout().unwrap();
        let rules = Rules::new(&cfg);
        let fixture = || State::genesis(8, 1, &rules.seed_key);
        let actions = [
            Action::Command(Command {
                seq: 0,
                keys: [0, 1, 2, 3],
                amount: 10,
            }),
            Action::Command(Command {
                seq: 1,
                keys: [1, 0, 2, 3],
                amount: 5,
            }),
        ];
        let mut witness = fixture();
        let mut delta = AccDelta::new(1);
        let first_updates =
            [(0, INITIAL_COUNTER - 10), (1, INITIAL_COUNTER + 10)].map(|(index, count)| {
                let key = key_at(index);
                let old = witness.get(&key).unwrap();
                let mut new = *old;
                set_counter(&mut new, count);
                delta.record(&key, Some(old), &new);
                (key, new)
            });
        witness.apply(first_updates.into_iter(), &delta);
        let mut direct = fixture();
        let initial_root = direct.root();
        let mut live = LiveTick {
            layout: &layout,
            mode: JournalMode::Delta,
            rules: Rules::new(&cfg),
            work: TickWork::new(1, 2),
            commands: Vec::new(),
            bytes: Vec::new(),
        };
        let mut times = TickTimes::default();
        let first = live
            .stage_resolution(&direct, 1, actions[0], initial_root, &mut times)
            .unwrap();
        assert_eq!(first, witness.root());
        let second = live
            .stage_resolution(&direct, 1, actions[1], first, &mut times)
            .unwrap();
        let delta_bytes = std::mem::take(&mut live.bytes);
        live.work.apply(&mut direct);
        assert_eq!(counter(direct.get(&key_at(0)).unwrap()), 999_995);
        assert_eq!(counter(direct.get(&key_at(1)).unwrap()), 1_000_005);
        assert_eq!(direct.root(), second);

        let root_layout = journal::FrameLayout::new(true, 0).unwrap();
        let mut root_bytes = Vec::new();
        for (action, pre_root, post_root) in [
            (actions[0], initial_root, first),
            (actions[1], first, second),
        ] {
            let frame = Resolution {
                action,
                tick: 1,
                pre_root,
                post_root,
            };
            journal::encode(&mut root_bytes, &frame, root_layout, JournalMode::Root, &[]);
        }
        for (mode, bytes, frame_layout) in [
            (JournalMode::Delta, delta_bytes, layout.command),
            (JournalMode::Root, root_bytes, root_layout),
        ] {
            let mut source = tempfile::tempfile().unwrap();
            source.write_all(&bytes).unwrap();
            let mut reader = JournalReader::new(source, 0).unwrap();
            let mut replayed = fixture();
            let mut work = TickWork::new(1, 2);
            let frame = reader.next(8, frame_layout, 1, 0).unwrap();
            let pre = work
                .replay_resolution(&replayed, &rules, mode, &initial_root, &frame)
                .unwrap();
            assert_eq!(pre, first);
            let frame = reader.next(8, frame_layout, 1, 1).unwrap();
            assert_eq!(frame.frame.pre_root, first);
            let post = work
                .replay_resolution(&replayed, &rules, mode, &pre, &frame)
                .unwrap();
            assert_eq!(post, second);
            reader.finish().unwrap();
            work.apply(&mut replayed);
            assert_eq!(replayed.root(), direct.root());
            assert!(replayed.entries().eq(direct.entries()));
            assert_eq!(replayed.full_recompute(), direct.full_recompute());
        }
    }

    #[test]
    fn truncated_tick_does_not_publish_its_commands() {
        for mode in [JournalMode::Delta, JournalMode::Root] {
            let mut cfg = Config::preset(Load::A, mode);
            cfg.keys = 1000;
            cfg.commands = 1;
            let layout = cfg.layout().unwrap();
            let rules = Rules::new(&cfg);
            let mut state = State::genesis(cfg.keys, layout.partitions, &rules.seed_key);
            let initial_root = state.root();
            let mut live = LiveTick::new(&cfg, &layout, Rules::new(&cfg));
            live.stage(&state, 1, &mut TickTimes::default()).unwrap();
            let mut source = tempfile::tempfile().unwrap();
            source
                .write_all(&live.bytes[..layout.command.bytes() as usize])
                .unwrap();
            let mut reader = JournalReader::new(source, 0).unwrap();
            let mut work = TickWork::new(layout.partitions, layout.resolved_capacity);
            let err = replay_tick(&cfg, &layout, &rules, &mut work, &mut state, &mut reader, 1)
                .unwrap_err();
            assert!(err.contains("missing closure"), "{err}");
            assert_eq!(state.root(), initial_root);
            assert!(!work.overlay.is_empty());
        }
    }
}
