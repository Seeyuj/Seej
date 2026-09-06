# Phase 1 — Kernel Foundation

```text
status: LOCAL PROTOTYPE IMPLEMENTED; STABLE KERNEL GATE OPEN
```

Phase 1 now closes the first stable distributed kernel contract established by
D-012. The existing local recovery slice is retained as evidence inside that
larger goal.

## Implemented local slice

The current repository can:

- create a seeded local demonstration world;
- advance deterministic ticks in a headless process;
- run current resource/creature degradation rules;
- write a local snapshot, metadata, and event WAL;
- recover by replaying WAL events after the snapshot cursor;
- inspect durable state through `sy_cli`;
- run deterministic, forced-kill, burn-in, and WAL fuzz checks.

The current world identity, zones, entity kinds, float properties, event-only
WAL, and direct runtime wiring are implementation facts. They are not the
frozen target contract.

## Phase 1 target

Phase 1 must produce:

- an empty or extension-defined world without mandatory space or entity kinds;
- replaceable fundamental providers and trusted startup extensions;
- durable command lifecycle and atomic cross-owner transitions;
- fail-closed behavior for common local-storage failures;
- the same world contract on one or three authorized nodes;
- one confirmed history under coordinator failure and network partition;
- confirmed read-only observation, bounded retention, and resynchronization;
- compatible kernel-owned migrations;
- a third-party extension proving the public surface.

The full requirements and evidence are in
[EXIT_CHECKLIST.md](EXIT_CHECKLIST.md).

## Explicitly later

These capabilities do not block the first stable contract:

- arbitrary hot loading or hot replacement;
- isolated application-plugin execution;
- ingame netcode and player prediction;
- official agents, economies, renderers, or multiresolution engines;
- world sharding or Byzantine consensus;
- synchronous atomic transactions between worlds.

A future implementation may prepare clean boundaries for these capabilities
without documenting them as available.

## Terminology

- **Kernel networking:** inter-node membership, authority, replication, and
  confirmed reads. It is required in Phase 1.
- **Ingame netcode:** player/client sessions, prediction, interpolation, and
  application synchronization. It is a later plugin.
- **Provider:** replaceable implementation of a fundamental kernel capability.
- **Application plugin:** optional world or consumer capability.
- **Implemented:** code and evidence currently exist.
- **Target:** required by an unchecked stable gap.
- **Later:** outside the first stable contract.

## Documents

- [Kernel contract](../CONTRACT.md)
- [Node-network contract](../NODE_NETWORK.md)
- [Plugin contract](../PLUGIN_CONTRACT.md)
- [Persistence behavior](PERSISTENCE.md)
- [Determinism behavior](DETERMINISM.md)
- [Observation target](OBSERVATION_SLICE.md)
- [Binary usage](BINARIES.md)
- [Exit checklist](EXIT_CHECKLIST.md)

## Exit rule

Phase 1 is complete only when every mandatory Gate 1-K item is closed with
reproducible evidence. Existing tests, a convincing design, or a local demo
cannot substitute for the missing distributed and extension proofs.
