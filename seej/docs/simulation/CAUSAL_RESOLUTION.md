# Causal Resolution Extension Contract

```text
status: OPTIONAL COMMUNITY CAPABILITY — NOT A KERNEL IMPLEMENTATION
```

Causal resolution is the optional process of representing inactive detail in a
coarser form and materializing it later. Seej provides the contracts needed to
do this safely but does not ship an official engine, scheduler, scoring model,
or spatial policy.

## Kernel guarantees

The kernel provides:

- versioned owner domains and opaque payload preservation;
- deterministic commands, validation, transactions, and events;
- declared world laws and protected-fact metadata;
- checkpoints, retention, and confirmed-history cursors;
- observer neutrality;
- component identities, fingerprints, and migrations.

These primitives let a plugin implement multiresolution without gaining a
second path to mutate world state.

## Required world policy

A world enabling causal resolution declares:

- which domains the plugin owns or may query;
- exact facts, identities, relationships, and obligations to preserve;
- what may be aggregated or omitted;
- permitted approximation behavior;
- how aggregate state is validated;
- materialization and reconciliation commands;
- limits and resource budgets;
- compatible plugin and data versions.

No default weighted score, distance rule, camera rule, or player-priority rule
is part of the Seej contract.

## Causal behavior

Aggregation and materialization occur only through admitted canonical commands.
The active policy and its version are part of the execution profile. For equal
confirmed inputs and contracts, the plugin produces equal transitions.

Past facts are not rewritten. Protected people, identities, balances,
obligations, and links remain exact according to the world declaration. The
future after an approximation may differ from a hypothetical fully detailed
future when the declared policy explicitly allows it.

A plugin can decline materialization when required historical detail or a
compatible artifact is missing. It cannot invent a result and call it replay.

## Observation neutrality

Opening an observer, moving a camera, subscribing to a region, or requesting a
projection cannot trigger canonical materialization or change simulation
priority. Under load, the runtime delays, limits, or refuses observation.

An operator or world agent may request a fidelity transition through an
authorized command. That cause is then visible, validated, ordered, and
persisted like any other mutation.

## Distribution

The authority group confirms one resolution transition. Replicas do not choose
independent detail levels. The transition cannot be published from a local
unconfirmed branch.

The first distributed baseline replicates the complete confirmed world. Causal
resolution does not imply sharding, parallel ownership across nodes, or
automatic scaling of tick computation.

## Compatibility and lifecycle

A world records the exact causal-resolution plugin and policy versions needed
to interpret retained state. The plugin owns its payload migrations. Removing
it requires materializing, transferring, resolving, or explicitly archiving
its owned state and obligations.

Operators retain artifacts needed for re-simulation. If they are missing, Seej
may still preserve and recover an opaque checkpoint while refusing to
recalculate transitions that require the plugin.

## Acceptance scenarios for an extension

A conforming community implementation should demonstrate:

- deterministic aggregate transitions for equal inputs;
- preservation of every declared protected fact;
- rejection when an obligation cannot be represented safely;
- materialization through public commands only;
- identical confirmed results across replicas;
- no canonical effect from observer activity;
- explicit behavior when history or plugin artifacts are unavailable;
- migration of its own data without modifying kernel-owned state directly.

These are extension conformance cases. They do not make causal resolution a
first-stable Seej deliverable.
