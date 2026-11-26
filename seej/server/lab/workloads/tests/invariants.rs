use std::fs;
use std::ops::Range;
use std::path::Path;
use std::process::Command;

use seej_lab_workloads::{recover, run, Config, HashMode, JournalMode, Load, StateLayout};
use tempfile::tempdir;

const LOADS: [Load; 3] = [Load::A, Load::B, Load::C];
const MODES: [JournalMode; 2] = [JournalMode::Delta, JournalMode::Root];
const COMBOS: [(StateLayout, HashMode); 4] = [
    (StateLayout::Btree, HashMode::Acc),
    (StateLayout::Partitioned, HashMode::Acc),
    (StateLayout::Partitioned, HashMode::AccCached),
    (StateLayout::Partitioned, HashMode::Rehash),
];
const ENTRY_BYTES: usize = 144;
const EMPTY_CLOSURE_BYTES: usize = 81 + 8;

fn scaled(load: Load, journal: JournalMode) -> Config {
    let mut cfg = Config::preset(load, journal);
    cfg.keys = 10_000;
    cfg.ticks = 12;
    cfg.warmup_ticks = 0;
    cfg.checkpoint_at = 5;
    match load {
        Load::A => cfg.commands = 100,
        Load::B => cfg.auto_writes = 1000,
        Load::C => {
            cfg.auto_writes = 10;
            cfg.commands = 100;
            cfg.hot_keys = 4;
        }
    }
    cfg
}

fn checkpoint_offset(bytes: &[u8]) -> usize {
    usize::try_from(u64::from_le_bytes(bytes[48..56].try_into().unwrap())).unwrap()
}

fn read_offset(dir: &Path) -> usize {
    checkpoint_offset(&fs::read(dir.join("checkpoint.bin")).unwrap())
}

fn frame_body(bytes: &[u8], start: usize) -> Range<usize> {
    let body_start = start + 4;
    let size = u32::from_le_bytes(bytes[start..body_start].try_into().unwrap()) as usize;
    body_start..body_start + size
}

fn rewrite_crc(bytes: &mut [u8], body: Range<usize>) {
    let crc = crc32fast::hash(&bytes[body.clone()]);
    bytes[body.end..body.end + 4].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn same_seed_same_journal_bytes() {
    for load in [Load::B, Load::C] {
        let cfg = scaled(load, JournalMode::Delta);
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        run(&cfg, first.path()).unwrap();
        run(&cfg, second.path()).unwrap();
        let journal = |d: &Path| fs::read(d.join("journal.bin")).unwrap();
        assert!(journal(first.path()) == journal(second.path()), "{load:?}");
    }
}

#[test]
fn delta_and_root_journals_reach_the_same_root() {
    for load in LOADS {
        let delta = tempdir().unwrap();
        let root = tempdir().unwrap();
        let a = run(&scaled(load, JournalMode::Delta), delta.path()).unwrap();
        let b = run(&scaled(load, JournalMode::Root), root.path()).unwrap();
        assert_eq!(a.final_root, b.final_root, "{load:?}");
    }
}

fn campaign_one_journal_digest(load: Load, mode: JournalMode) -> &'static str {
    // Produced by the released workload executable at commit 1c98282 using scaled().
    match (load, mode) {
        (Load::A, JournalMode::Delta) => {
            "715ba6184877afe24a75f9bd9dac1eaefad20521af6bed5498a25ed705000137"
        }
        (Load::A, JournalMode::Root) => {
            "5771336af098276c670d3b188d65a017b64dc0009e64e5d33846447925fefa54"
        }
        (Load::B, JournalMode::Delta) => {
            "a89dd91c869caeab6313d9bc796a8aabb2739695b495781b43d0cf2312694150"
        }
        (Load::B, JournalMode::Root) => {
            "4010ccb9dda0abb00d72151a8e7e369bb63fd30e16ff9c78a8e41b7b1b5ae648"
        }
        (Load::C, JournalMode::Delta) => {
            "4ddd60c9f7c6f61a2758a6ff9b9aa658e94fc6304b56d073c17a6a5b434caa59"
        }
        (Load::C, JournalMode::Root) => {
            "f5a7a91309074fad026c64d797d2939b93d19dec28f560bcdee73994b6bbc631"
        }
    }
}

#[test]
fn recovery_from_checkpoint_matches_live_root() {
    for load in LOADS {
        let mut additive = None;
        let mut rehash_root = None;
        for mode in MODES {
            let mut baseline_bytes = None;
            let mut baseline_coalesced = None;
            for (layout, hash) in COMBOS {
                let dir = tempdir().unwrap();
                let mut cfg = scaled(load, mode);
                cfg.layout = layout;
                cfg.hash = hash;
                let live = run(&cfg, dir.path()).unwrap();
                let recovered = recover(dir.path()).unwrap();
                assert_eq!(live.layout, layout);
                assert_eq!(live.hash_mode, hash);
                assert_eq!(recovered.root_matches_run, Some(true));
                assert_eq!(recovered.final_root, live.final_root);
                assert_eq!(recovered.acc_root, live.acc_root);
                assert_eq!(recovered.replayed_ticks, 7);
                assert_eq!(recovered.replayed_resolutions, 7 * (cfg.commands + 1));
                assert_eq!(
                    recovered.journal_bytes_read,
                    live.journal_bytes_total - read_offset(dir.path()) as u64
                );
                assert_eq!(live.measured_ticks, cfg.ticks);
                assert_eq!(
                    *baseline_coalesced.get_or_insert(live.coalesced_bytes_per_tick_mean),
                    live.coalesced_bytes_per_tick_mean,
                    "{load:?}/{mode:?}/{hash:?}"
                );
                assert_eq!(
                    additive.get_or_insert_with(|| live.acc_root.clone()),
                    &live.acc_root
                );
                if hash == HashMode::Rehash {
                    assert_ne!(live.final_root, live.acc_root);
                    assert_eq!(
                        rehash_root.get_or_insert_with(|| live.final_root.clone()),
                        &live.final_root
                    );
                } else {
                    assert_eq!(live.final_root, live.acc_root);
                    let bytes = fs::read(dir.path().join("journal.bin")).unwrap();
                    if let Some(baseline) = &baseline_bytes {
                        assert!(baseline == &bytes, "{load:?}/{mode:?}/{hash:?}");
                    } else {
                        assert_eq!(
                            blake3::hash(&bytes).to_hex().as_str(),
                            campaign_one_journal_digest(load, mode),
                            "{load:?}/{mode:?}"
                        );
                        baseline_bytes = Some(bytes);
                    }
                }
            }
        }
    }
}

#[test]
fn configuration_without_layout_and_hash_recovers_as_btree_acc() {
    let dir = tempdir().unwrap();
    let live = run(&scaled(Load::B, JournalMode::Delta), dir.path()).unwrap();
    assert_eq!(
        live.final_root,
        "88a5bc758c83db5fcc966767b4db5ebc88ffc373891c0d36352a1051efff5b90"
    );
    let path = dir.path().join("config.json");
    let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    old.as_object_mut().unwrap().remove("layout").unwrap();
    old.as_object_mut().unwrap().remove("hash").unwrap();
    fs::write(path, serde_json::to_vec(&old).unwrap()).unwrap();
    let recovered = recover(dir.path()).unwrap();
    assert_eq!(recovered.root_matches_run, Some(true));
    assert_eq!(recovered.acc_root, live.final_root);
}

#[test]
fn unsupported_layout_hash_pairs_exit_with_invalid_configuration() {
    let dir = tempdir().unwrap();
    for hash in ["acc-cached", "rehash"] {
        let output = Command::new(env!("CARGO_BIN_EXE_workloads"))
            .args([
                "run",
                "--load",
                "b",
                "--journal",
                "delta",
                "--layout",
                "btree",
                "--hash",
                hash,
                "--dir",
            ])
            .arg(dir.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("btree layout requires --hash acc")
        );
        assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
    }
}

#[test]
fn recovery_without_a_witness_still_verifies_the_journal() {
    let dir = tempdir().unwrap();
    let mut cfg = scaled(Load::B, JournalMode::Delta);
    cfg.warmup_ticks = 2;
    let live = run(&cfg, dir.path()).unwrap();
    fs::remove_file(dir.path().join("summary.json")).unwrap();
    let recovered = recover(dir.path()).unwrap();
    assert_eq!(recovered.root_matches_run, None);
    assert_eq!(recovered.final_root, live.final_root);
    let bytes_per_tick = 144 * cfg.auto_writes + 81 + 8;
    assert_eq!(live.measured_ticks, 10);
    assert_eq!(live.journal_bytes_per_tick_mean, bytes_per_tick as f64);
    assert_eq!(live.journal_bytes_total, bytes_per_tick * cfg.ticks);
    assert_eq!(live.checkpoint_bytes, 144 * cfg.keys + 68);
    assert_eq!(
        live.journal_bytes_total,
        fs::metadata(dir.path().join("journal.bin")).unwrap().len()
    );
}

#[test]
fn recovery_rejects_a_flipped_journal_byte() {
    let dir = tempdir().unwrap();
    run(&scaled(Load::B, JournalMode::Delta), dir.path()).unwrap();
    let path = dir.path().join("journal.bin");
    let mut bytes = fs::read(&path).unwrap();
    let start = read_offset(dir.path());
    bytes[start + 20] ^= 1;
    fs::write(&path, &bytes).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(err.contains("journal CRC mismatch"), "{err}");
    assert!(!dir.path().join("recover_summary.json").exists());
    assert!(fs::read(path).unwrap() == bytes);
}

#[test]
fn recovery_rejects_invalid_delta_command_envelopes_with_valid_crc() {
    let dir = tempdir().unwrap();
    run(&scaled(Load::A, JournalMode::Delta), dir.path()).unwrap();
    let path = dir.path().join("journal.bin");
    let original = fs::read(&path).unwrap();
    let body = frame_body(&original, read_offset(dir.path()));
    assert_eq!(original[body.start], 1);
    for amount in [0u64, 101] {
        let mut malformed = original.clone();
        malformed[body.start + 109..body.start + 117].copy_from_slice(&amount.to_le_bytes());
        rewrite_crc(&mut malformed, body.clone());
        fs::write(&path, malformed).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(err.contains("invalid command amount"), "{err}");
    }

    let mut malformed = original;
    let keys = &mut malformed[body.start + 77..body.start + 109];
    let (first_two, last_two) = keys.split_at_mut(16);
    first_two[..8].swap_with_slice(&mut last_two[..8]);
    rewrite_crc(&mut malformed, body);
    fs::write(&path, malformed).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(
        err.contains("journal writes do not match command keys"),
        "{err}"
    );
}

#[test]
fn recovery_rejects_a_different_seed() {
    let dir = tempdir().unwrap();
    let mut cfg = scaled(Load::B, JournalMode::Root);
    run(&cfg, dir.path()).unwrap();
    cfg.seed += 1;
    fs::write(
        dir.path().join("config.json"),
        serde_json::to_vec(&cfg).unwrap(),
    )
    .unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(err.contains("RECOVERY ROOT MISMATCH"), "{err}");
    assert!(err.contains("post-root at tick 6"), "{err}");
}

#[test]
fn recovery_rejects_incomplete_or_out_of_order_ticks_without_a_witness() {
    for mode in MODES {
        let dir = tempdir().unwrap();
        let cfg = scaled(Load::A, mode);
        run(&cfg, dir.path()).unwrap();
        fs::remove_file(dir.path().join("summary.json")).unwrap();
        let path = dir.path().join("journal.bin");
        let original = fs::read(&path).unwrap();
        let last_closure = original.len() - EMPTY_CLOSURE_BYTES;
        let closure_body = frame_body(&original, last_closure);
        assert_eq!(original[closure_body.start], 2);
        assert_eq!(original[closure_body.end - 4..closure_body.end], [0; 4]);
        fs::write(&path, &original[..last_closure]).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(
            err.contains("missing closure at tick 12"),
            "{mode:?}: {err}"
        );

        let tick_bytes = original.len() / cfg.ticks as usize;
        fs::write(&path, &original[..original.len() - tick_bytes]).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(
            err.contains("missing command at tick 12"),
            "{mode:?}: {err}"
        );

        let suffix = read_offset(dir.path());
        let first = frame_body(&original, suffix);
        let mut duplicated = original.clone();
        duplicated.splice(
            first.end + 4..first.end + 4,
            original[suffix..first.end + 4].iter().copied(),
        );
        fs::write(&path, duplicated).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(
            err.contains("expected command at tick 6, seq 1"),
            "{mode:?}: {err}"
        );

        let mut wrong_seq = original.clone();
        wrong_seq[first.start + 9..first.start + 13].copy_from_slice(&99u32.to_le_bytes());
        rewrite_crc(&mut wrong_seq, first);
        fs::write(&path, wrong_seq).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(
            err.contains("expected command at tick 6, seq 0"),
            "{mode:?}: {err}"
        );

        let mut extra = original;
        extra.push(0);
        fs::write(&path, extra).unwrap();
        let err = recover(dir.path()).unwrap_err();
        assert!(err.contains("extra journal bytes"), "{mode:?}: {err}");
        assert!(!dir.path().join("recover_summary.json").exists());
    }
}

#[test]
fn recovery_checks_checkpoint_and_intermediate_roots() {
    let dir = tempdir().unwrap();
    run(&scaled(Load::A, JournalMode::Delta), dir.path()).unwrap();
    let checkpoint_path = dir.path().join("checkpoint.bin");
    let checkpoint = fs::read(&checkpoint_path).unwrap();
    fs::write(&checkpoint_path, &checkpoint[..checkpoint.len() - 1]).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(err.contains("invalid checkpoint length"), "{err}");
    let mut corrupt_checkpoint = checkpoint.clone();
    corrupt_checkpoint[16] ^= 1;
    let crc_pos = corrupt_checkpoint.len() - 4;
    fs::write(&checkpoint_path, &corrupt_checkpoint).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(err.contains("checkpoint CRC mismatch"), "{err}");
    rewrite_crc(&mut corrupt_checkpoint, 0..crc_pos);
    fs::write(&checkpoint_path, &corrupt_checkpoint).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(err.contains("RECOVERY ROOT MISMATCH (checkpoint)"), "{err}");
    fs::write(&checkpoint_path, &checkpoint).unwrap();

    let journal_path = dir.path().join("journal.bin");
    let mut journal = fs::read(&journal_path).unwrap();
    let body = frame_body(&journal, checkpoint_offset(&checkpoint));
    journal[body.start + 45] ^= 1;
    rewrite_crc(&mut journal, body);
    fs::write(&journal_path, journal).unwrap();
    let err = recover(dir.path()).unwrap_err();
    assert!(
        err.contains("RECOVERY ROOT MISMATCH (post-root at tick 6)"),
        "{err}"
    );
}

#[test]
fn counters_are_conserved_with_hot_keys() {
    let dir = tempdir().unwrap();
    let cfg = scaled(Load::C, JournalMode::Delta);
    run(&cfg, dir.path()).unwrap();
    let checkpoint = fs::read(dir.path().join("checkpoint.bin")).unwrap();
    let counter = |entry: &[u8]| u64::from_le_bytes(entry[16..24].try_into().unwrap());
    let mut counters: Vec<_> = checkpoint[64..checkpoint.len() - 4]
        .chunks_exact(ENTRY_BYTES)
        .map(counter)
        .collect();
    assert_eq!(counters.len(), cfg.keys as usize);
    assert_eq!(
        counters.iter().sum::<u64>(),
        cfg.keys * 1_000_000 + cfg.checkpoint_at * cfg.auto_writes
    );

    let journal = fs::read(dir.path().join("journal.bin")).unwrap();
    let mut start = checkpoint_offset(&checkpoint);
    while start < journal.len() {
        let body = frame_body(&journal, start);
        start = body.end + 4;
        let entries_start = if journal[body.start] == 1 { 121 } else { 81 };
        for entry in journal[body.start + entries_start..body.end].chunks_exact(ENTRY_BYTES) {
            let partition = u32::from_be_bytes(entry[4..8].try_into().unwrap()) as usize;
            let record =
                usize::try_from(u64::from_be_bytes(entry[8..16].try_into().unwrap())).unwrap();
            counters[partition * 1000 + record] = counter(entry);
        }
    }
    assert_eq!(start, journal.len());
    assert_eq!(
        counters.iter().sum::<u64>(),
        cfg.keys * 1_000_000 + cfg.ticks * cfg.auto_writes
    );
}
