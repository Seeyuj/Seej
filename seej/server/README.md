# Server (Rust Workspace)

This is the Rust workspace containing all server-side crates.

## Constitution

### Allowed External Dependencies

- `serde` - Serialization (config, debugging)
- `tokio` - Async runtime (sy_infra only)
- `tracing` - Logging and observability
- `thiserror` - Error types
- `flatbuffers` - Protocol serialization (sy_protocol only)

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
cargo run --bin server_d -- --config config.toml
```

## Crate Overview

| Crate       | Level | Purpose                              |
|-------------|-------|--------------------------------------|
| sy_types    | NIV 0 | Stable primitive types               |
| sy_config   | NIV 0 | Configuration parsing                |
| sy_protocol | NIV 1 | Wire protocol (generated code)       |
| sy_api      | NIV 1 | Internal API (commands/events)       |
| sy_core     | NIV 2 | Pure simulation logic                |
| sy_infra    | NIV 3 | I/O implementations                  |
| sy_tools    | NIV 3 | Operator utilities                   |
| sy_testkit  | NIV 3 | Testing harness and mocks            |
| sy_loader   | NIV 4 | Dependency injection and boot        |
