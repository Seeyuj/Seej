# Phase 1 — How to run the code (binaries)

All commands below assume you are in:

```bash
cd seeyuj/server
```

## Build / test

```bash
cargo build --workspace
cargo test --workspace
```

## `server_d` — headless daemon

### Create a world (seed is REQUIRED)

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42
```

Optional initial population:

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42 --resources 10 --creatures 5
```

### List worlds

```bash
cargo run --bin server_d -- list
```

### Run an existing world (tick loop)

```bash
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
```

Notes:
- `--ticks 0` means “run forever”.
- `Ctrl+C` triggers a graceful shutdown and saves the world.

### Storage directory

By default, both binaries use `--data-dir ./data`.
You can override it:

```bash
cargo run --bin server_d -- --data-dir ./my_data create --name "MyWorld" --seed 42
```

## `sy_cli` — admin inspection CLI (no UI)

### Status (includes crash-recovery cursor)

```bash
cargo run --bin sy_cli -- status world_42
```

### List recent WAL events

```bash
cargo run --bin sy_cli -- events world_42 --count 50
```

Filter from a tick:

```bash
cargo run --bin sy_cli -- events world_42 --from-tick 1000 --count 50
```

### Dump the snapshot as JSON

```bash
cargo run --bin sy_cli -- dump world_42 --pretty
```

### Inspect entities and zones

```bash
cargo run --bin sy_cli -- entities world_42
cargo run --bin sy_cli -- entities world_42 --kind creature
cargo run --bin sy_cli -- entity world_42 1
cargo run --bin sy_cli -- zones world_42
```

