# Data Flow

This document describes how data flows through the See-Yuj platform.

## Command Processing Flow

```
Client Request
     │
     ▼
┌─────────────┐
│ sy_protocol │  (deserialize wire format)
└─────────────┘
     │
     ▼
┌─────────────┐
│   sy_api    │  (validate + sanitize)
└─────────────┘
     │
     ▼
┌─────────────┐
│   sy_core   │  (apply command → events)
└─────────────┘
     │
     ▼
┌─────────────┐
│  sy_infra   │  (persist events, broadcast)
└─────────────┘
```

## Event Sourcing

1. Commands are validated and converted to internal API commands
2. The simulation core processes commands and emits events
3. Events are persisted to the Write-Ahead Log (WAL)
4. State is reconstructed by replaying events

## Tick Loop

```rust
loop {
    let tick = clock.next_tick();
    let commands = collect_pending_commands();
    let events = core.apply(tick, commands);
    event_log.append(events);
    broadcast_to_clients(events);
}
```
