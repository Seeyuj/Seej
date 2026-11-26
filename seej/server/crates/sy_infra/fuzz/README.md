# `sy_infra` WAL Fuzzing

This isolated harness fuzzes the current local prototype's `FileEventLog`
decoder and round-trip persistence. It does **not** exercise the target D-015
command journal, distributed consensus, or retention-aware compaction.

It is intentionally excluded from `seej/server/Cargo.toml` because `cargo-fuzz`
requires nightly Rust and `libFuzzer`.

## Running Locally

From `seej/server/crates/sy_infra/fuzz/`:

```bash
cargo +nightly fuzz run decode_record -- -runs=1000
cargo +nightly fuzz run wal_round_trip -- -runs=1000
```

## CI Behavior

- Pull requests build both targets (`cargo +nightly fuzz build --dev --target x86_64-unknown-linux-gnu`).
- Scheduled runs execute 180-second smoke sessions per target on Ubuntu glibc
  (AddressSanitizer is incompatible with musl; Windows MSVC can hit
  `STATUS_DLL_NOT_FOUND` if the libFuzzer runtime DLL is not on `PATH`).

See [seej/docs/PROTOTYPE.md](../../../../docs/PROTOTYPE.md) for the current WAL v1
format and [doc/ROADMAP.md](../../../../../doc/ROADMAP.md) for release gates.
