# sy_infra WAL fuzzing

This directory is intentionally isolated from the main workspace because
`cargo-fuzz` requires nightly and `libfuzzer`.

Run short local smoke tests from this directory:

```text
cargo +nightly fuzz run decode_record -- -runs=1000
cargo +nightly fuzz run wal_round_trip -- -runs=1000
```

The scheduled CI job uses time-bounded sessions:

```text
cargo +nightly fuzz run --target x86_64-unknown-linux-gnu decode_record -- -max_total_time=180
cargo +nightly fuzz run --target x86_64-unknown-linux-gnu wal_round_trip -- -max_total_time=180
```

Pull requests only build the targets:

```text
cargo +nightly fuzz build --dev --target x86_64-unknown-linux-gnu
```

CI pins the Ubuntu fuzz jobs to the glibc Linux target because AddressSanitizer
is incompatible with statically linked musl libc.

On Windows MSVC, a fuzz target can compile but fail to start with
`STATUS_DLL_NOT_FOUND` if the LLVM/libFuzzer runtime DLL is not on `PATH`.
The scheduled CI smoke job runs these targets on Ubuntu.

The `decode_record` target writes arbitrary bytes to a WAL file and exercises
the same recovery/read path used by `FileEventLog`.

The `wal_round_trip` target generates structured batches, persists them through
`append_batch`, reopens the WAL, and checks that persisted ordering and event
IDs survive decode.
