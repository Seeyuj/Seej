# Architecture

## Purpose and authority

This document explains the durable project architecture established by
[decision D-012](DECISIONS.md#d-012--distributed-extensible-world-kernel).
The normative behavior is defined by the contracts under `seej/docs/`.
The roadmap schedules that work, and the Phase 1 documents describe the
prototype and its evidence.

Seej is a distributed, extensible world kernel. It is infrastructure on which
communities can define worlds and applications. It is not itself a game,
renderer, agent framework, spatial engine, or ingame networking stack.

## One logical kernel, one or more nodes

A world is owned by one logical kernel authority group. That group can contain
one node or several explicitly authorized nodes. The number of processes does
not change the world contract.

```text
                           WORLD
                             |
                  confirmed canonical history
                             |
           +-----------------+-----------------+
           |                 |                 |
        node A            node B            node C
     engine + providers engine + providers engine + providers
           |                 |                 |
           +---------- authorized group -------+
```

The kernel is decentralized in the precise sense that Seej requires no global
coordinator or global registry and an operator can form independent authority
groups. Within one world, a temporary coordinator may order work. The authority
group still produces one confirmed history; a network partition never creates
two writable canonical worlds.

The first distributed reference profile uses three authorized nodes, quorum
two, and a Raft-style provider under a crash/partition fault model. Other
providers may conform to the same public guarantees. Byzantine consensus,
permissionless validators, and automatic merging of divergent histories are
outside the first stable contract.

## Kernel, engine, runtime, and providers

These terms have distinct meanings:

- **World kernel:** the logical distributed system owning identity, time,
  command admission, validation coordination, atomic transitions, durability,
  and replication.
- **Transition engine:** the pure deterministic state-transition function.
  `sy_core` currently fills this role for the prototype.
- **Node runtime:** the process that admits work, coordinates providers, and
  participates in a world's authority group.
- **Provider:** a replaceable implementation of a fundamental kernel capability,
  such as persistence, scheduling, validation, membership, or replication.
- **Application plugin:** an optional capability defining world data or an
  experience, such as space, agents, economics, multiresolution, rendering, or
  ingame netcode.
- **Observer:** a read-only consumer of confirmed state and events.

The project ships a minimal distribution of providers capable of running an
empty world. Replaceability does not make the guarantees optional: a provider
must preserve the contract of the capability it implements.

## Pure transition boundary

Canonical transition logic is a pure function of:

```text
confirmed world state
  + ordered admitted command
  + target tick
  + active deterministic contracts
  = transition decision and events
```

It never depends on wall-clock time, packet arrival timing, local filesystem
ordering, thread scheduling, ambient randomness, connected observers, or
application UI. Nondeterministic external results become explicit validated
commands before they can affect state.

I/O, consensus, authentication, and storage stay outside the pure engine. A
node runtime may execute those operations, but must capture the canonical input
and order before invoking the transition.

## World freedom and data ownership

The kernel has no mandatory spatial model or application ontology. A world can
contain positions and creatures, or consist only of organizations,
relationships, and obligations.

Each mutable data domain declares one logical owner. Other plugins change it
through public commands. A kernel transaction can coordinate several owners
inside one world so that all validated effects are confirmed together or none
are applied. This does not imply atomicity across worlds or external services.

World law is supplied through declared validators. The kernel enforces those
validators without defining universal economics, mortality, equality between
humans and NPCs, or another application policy.

## Persistence and replication

The durable model separates:

1. a proposed command;
2. local staging or WAL append;
3. confirmed admission and durable `Pending` state;
4. deterministic resolution computation;
5. confirmed resolution containing decision, events, and transition;
6. one-time application and publication to observers.

A local append is sufficient for parts of the current one-node prototype but
is not a distributed commit. The distributed contract reports success and
publishes canonical effects only after the configured confirmation rule.

Every canonical mutation has a recoverable cause. Recovery uses a compatible
checkpoint and the retained confirmed history after its cursor. Re-simulation
from commands is a stronger operation and additionally requires the exact
contracts and plugin artifacts. Migration transforms data between explicit
stable formats and is neither recovery nor re-simulation.

## Observation

Observers read a confirmed prefix and receive its cursor and freshness. Opening,
closing, slowing, or multiplying observers never changes canonical rules,
resolution, ordering, or fidelity. Under pressure, nodes limit reads before
altering world semantics.

History can be compacted under a declared retention policy. An observer whose
cursor expired receives a coherent replacement state and new cursor. The
deleted event range remains unavailable.

## Compatibility and evolution

A world records one execution profile: kernel contract, state formats,
simulation rules, schemas, provider/plugin identities and fingerprints, limits,
and retention policy. Nodes with incompatible profiles cannot participate in
authority.

Every stable version of kernel-owned data has a supported migration path to the
current stable version, possibly through intermediate versions. Plugin authors
own their data migrations. Activation occurs at a confirmed transition
boundary and fails closed when required code or data conversion is missing.

The first stable contract permits only operator-approved trusted providers
loaded at startup. Hot loading, safe hot replacement, and isolated application
plugins are subsequent capabilities.

## Scalability boundaries

Replication improves availability and recoverability. It does not by itself
divide one tick across more machines. The first distributed implementation
replicates the complete world.

Sharding a world, distributed execution of one transition, Byzantine authority,
and synchronous cross-world transactions require new contracts. They cannot be
inferred from the baseline decentralized deployment.

## Architectural rule

A change belongs in the kernel only when it is required to preserve the
declared contracts of identity, causality, deterministic transition,
durability, authority, compatibility, or extension coordination. Everything
else belongs behind a provider or application-plugin boundary.
