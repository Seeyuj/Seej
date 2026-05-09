# Server (Rust Workspace)

This is the Rust workspace containing all server-side crates.

## Constitution

### Active External Dependencies

- `serde`, `serde_json` - persistence DTO serialization and inspection output
- `tracing`, `tracing-subscriber` - logging and observability
- `clap` - CLI parsing for `server_d` and `sy_cli`
- `ctrlc` - graceful shutdown handling in `server_d`
- `crc32fast`, `byteorder` - WAL record encoding and validation
- `xxhash-rust` - deterministic state hashing for tests/diagnostics
- `windows-sys` - Windows atomic replacement support in filesystem persistence
- `assert_cmd`, `predicates`, `tempfile`, `proptest` - tests and recovery scenarios

No async runtime or wire-protocol dependency is active in the Phase 1 workspace.
`sy_protocol` is preserved as a Phase 2+ placeholder and is excluded from the
workspace members.

Workspace-level declarations that are not used by active crates should be
treated as cleanup candidates, not as architectural commitments.

### Forbidden in sy_core

The simulation core (`sy_core`) MUST NOT use:
- Any async runtime (tokio, async-std)
- Any I/O (std::fs, std::net)
- Any randomness from std (use injected IRng)
- Any time from std (use injected ISimClock)

### Building

```bash
cargo build --workspace
```

### Testing

```bash
cargo test --workspace
```

### Running

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
cargo run --bin sy_cli -- status world_42
```

## Crate Overview

| Crate       | Level | Purpose                              |
|-------------|-------|--------------------------------------|
| sy_types    | NIV 0 | Stable primitive types               |
| sy_config   | NIV 0 | Configuration parsing                |
| sy_protocol | NIV 1 | Phase 2+ wire protocol (excluded)    |
| sy_api      | NIV 1 | Internal API (commands/events)       |
| sy_core     | NIV 2 | Pure simulation logic                |
| sy_infra    | NIV 3 | I/O implementations                  |
| sy_tools    | NIV 3 | Phase 1 placeholder for operator utilities |
| sy_testkit  | NIV 3 | Testing harness and mocks            |
| sy_loader   | NIV 4 | Phase 1 placeholder; `server_d` wires runtime directly |
