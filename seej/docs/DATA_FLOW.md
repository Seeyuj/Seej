# Data Flow

This document describes how data flows through the Seej platform.

## Command Processing Flow

```
Operator (server_d / sy_cli)
     │
     ▼
┌─────────────┐
│   sy_api    │  (SimCommand build + validation)
└─────────────┘
     │
     ▼
┌─────────────┐
│   sy_core   │  (Simulation: command → SimEvent batch)
└─────────────┘
     │
     ▼
┌─────────────┐
│  sy_infra   │  (WAL append-batch → state commit | rollback)
└─────────────┘
```

> Phase 2+: a `sy_protocol` step deserializes wire requests into `SimCommand`
> before `sy_api` validation. Phase 1 has no wire layer.

## Snapshot + WAL Commit Flow

1. Commands are validated and converted to internal API commands
2. The simulation core processes commands and emits events
3. Events are persisted to the Write-Ahead Log (WAL)
4. In-memory state is accepted only after the WAL append succeeds
5. Recovery loads `snapshot.json` and replays WAL events after the snapshot cursor

Phase 1 does not yet persist canonical external command envelopes. Full
re-simulation from `GenesisSpec + CommandEnvelope[]` is tracked as Phase 1
hardening work in `phase1/EXIT_CHECKLIST.md`.

## Tick Loop

```rust
loop {
    let checkpoint = sim.checkpoint();
    let events = sim.process_command(cmd)?;       // pure, in-memory
    match event_log.append_batch(events) {        // WAL durable
        Ok(persisted) => persisted,               // keep state
        Err(e) => {
            sim.restore_checkpoint(checkpoint);
            return Err(e);
        }
    };
}
```
