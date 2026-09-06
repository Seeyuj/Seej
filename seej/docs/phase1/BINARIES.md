# Phase 1 — Current Binaries

```text
status: IMPLEMENTED LOCAL PROTOTYPE COMMANDS
```

All commands below run from:

```bash
cd seej/server
```

They operate on the current local FileStore. No cluster, node-admission,
provider-loading, migration, or network-observation command exists yet.

## Build and test

```bash
cargo build --workspace
cargo test --workspace
```

## `server_d`

Create a world; the seed is required:

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42
```

Create the current spatial demonstration population:

```bash
cargo run --bin server_d -- create --name "MyWorld" --seed 42 --resources 10 --creatures 5
```

These population flags exercise prototype application rules. They are not part
of the target generic genesis contract.

List local worlds:

```bash
cargo run --bin server_d -- list
```

Run an existing local world:

```bash
cargo run --bin server_d -- run --world world_42 --ticks 1000 --save-interval 100
```

`--ticks 0` runs until interruption. `Ctrl+C` requests graceful shutdown
and saves the current world. `--save-interval` is expressed in simulation
ticks.

Both binaries default to `--data-dir ./data`. Override it before the
subcommand:

```bash
cargo run --bin server_d -- --data-dir ./my_data create --name "MyWorld" --seed 42
```

## `sy_cli`

Inspect local status and recovery cursor:

```bash
cargo run --bin sy_cli -- status world_42
```

Read recent local WAL events:

```bash
cargo run --bin sy_cli -- events world_42 --count 50
cargo run --bin sy_cli -- events world_42 --from-tick 1000 --count 50
```

Dump the recovered world state (snapshot plus valid later WAL events):

```bash
cargo run --bin sy_cli -- dump world_42 --pretty
```

Inspect the current prototype entity and zone model:

```bash
cargo run --bin sy_cli -- entities world_42
cargo run --bin sy_cli -- entities world_42 --kind creature
cargo run --bin sy_cli -- entity world_42 1
cargo run --bin sy_cli -- zones world_42
```

`sy_cli` opens the WAL through a read-only path, but it first constructs
`FilesystemStore`, whose current initialization can create
`<data-dir>/worlds`. The `dump --output` option writes the requested export
file, and the code does not prevent that destination from resolving inside, or
aliasing a file in, the canonical data directory. Until P1-11 is implemented,
operators must point inspection at an already existing `<data-dir>/worlds`
tree and choose an export destination outside the world store.

P1-11 also remains open for coherent reads while `server_d` writes
concurrently. `sy_cli` is not the K-02 confirmed network observation API.

## Future command documentation rule

Distributed, provider, migration, and plugin commands are documented here only
after their binaries and help output exist. Architecture examples elsewhere
are conceptual and must not be copied as runnable commands.
