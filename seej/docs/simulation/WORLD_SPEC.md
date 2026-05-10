# World Specification Contract

## Status

This document defines target architecture for future Seej phases. It is a
contract specification, not a claim about the current implementation and not an
immediate Rust API requirement.

The current Phase 1 implementation proves a minimal headless persistence slice:
seeded world creation, deterministic ticks, snapshot + WAL recovery, and
operator inspection. A complete `WorldSpec` is not yet implemented. This
document formalizes the architecture needed to close known hardening gaps
without adding gameplay scope.

## Implementation Boundary

This document MUST NOT trigger implementation of causal scheduling,
materialization, aggregation, ontology compression, or module-specific scoring
until the Phase 1 P0 hardening backlog is closed.

Allowed Phase 1 work:

- define storage/contract fields needed later;
- add compatibility checks;
- add tests proving rejection of mismatched contracts;
- document invariants.

Forbidden Phase 1 work:

- runtime resolution scheduler;
- high/low resolution entity simulation;
- module causal scoring;
- ontology-driven compression;
- LLM-assisted canonical transitions.

WorldSpec work in Phase 1 should strengthen persistence, compatibility,
recovery, and replay foundations. It must not become an entry point for advanced
simulation systems.

## Purpose

A Seej world is not only runtime state.

A Seej world is defined by a persistent world specification that binds genesis,
ontology, modules, rules, simulation contracts, limits, persistence
compatibility, and replay semantics.

WorldSpec is the technical constitution of a world.

Seej needs a `WorldSpec` to:

- avoid implicit rules hidden in runtime code;
- guarantee replay from explicit inputs and contract versions;
- make persistence compatibility testable instead of inferred;
- allow modded worlds without corrupting the core contract;
- make migrations explicit and auditable;
- prevent the seed from becoming the complete identity of a world;
- protect long-term persisted worlds from silent reinterpretation;
- give operators and maintainers a stable artifact for audit, recovery, and
  debugging.

The `WorldSpec` is not a marketing description, content manifest, or gameplay
configuration dump. It is the durable contract that tells Seej how a world must
be created, interpreted, evolved, validated, recovered, and replayed.

## World Specification vs World State

The following artifacts have different responsibilities and must not be
collapsed into one another:

- `WorldSpec`: the initial and durable contract of the world.
- `WorldState`: the current persistent state of the world.
- `CommandLog`: accepted external intentions in canonical order.
- `EventLog` / WAL: persisted state transitions produced by the server.
- `Snapshot`: recoverable world state at a specific cursor.

`WorldSpec` does not replace world state.

`WorldSpec` defines how world state is created, interpreted, evolved, validated,
and replayed.

`WorldState` changes as the simulation advances. `WorldSpec` changes only
through explicit versioning and migration. Reinterpreting old state under a new
implicit contract is corruption, even when storage decoding succeeds.

## Conceptual Shape

A future `WorldSpec` should be hashable, versioned, persisted, and inspectable.
Conceptually, it includes or references:

- explicit `world_id` policy;
- `GenesisSpec`;
- `SimulationContract`;
- ontology and schema declarations;
- active module contracts;
- time model;
- causal resolution policy;
- limits manifest;
- persistence and replay compatibility policy;
- migration policy;
- canonical hashing semantics.

This list is architectural. It does not prescribe final Rust type names or file
formats.

## Canonical and Non-Canonical Parts

`WorldSpec` should be split conceptually into:

- `CanonicalSpec`: hashable fields that affect replay and world meaning;
- `OperatorManifest`: diagnostic and deployment metadata that does not affect
  simulation;
- `HumanMetadata`: names, descriptions, labels, and documentation references.

Only `CanonicalSpec` participates in `genesis_hash` or `world_spec_hash`.

This separation prevents two opposite failure modes:

- changing a readable name or description invalidates a durable world;
- changing a rule, limit, schema, or compatibility contract fails to change the
  canonical hash.

Every new field must be classified before inclusion:

- canonical, if it affects replay, validation, compatibility, rules, limits, or
  world meaning;
- operator metadata, if it affects diagnostics or deployment only;
- human metadata, if it affects names, labels, descriptions, or documentation
  only.

Unclassified fields must not be added.

Non-canonical metadata may change without migration only if it does not affect
replay, validation, compatibility, rules, limits, identity, or world meaning.

Any change to `CanonicalSpec` requires explicit versioning and, when applied to
an existing world, an explicit migration or compatibility decision.

## Canonical Hashes

`genesis_hash` covers the reproducible initial conditions.

`world_spec_hash` covers the full canonical world contract.

A world may preserve the same `genesis_hash` while changing `world_spec_hash`
only through explicit migration. Recovery must know which hash is being checked.

The two hashes must not be used interchangeably:

- `genesis_hash` proves that the initial world inputs match;
- `world_spec_hash` proves that the full canonical contract matches;
- snapshot, WAL, command-log, and manifest checks must declare which hash they
  bind to.

## Canonical Serialization

Canonical hashing requires canonical serialization:

- stable field ordering;
- stable collection ordering;
- explicit omission rules;
- no locale-dependent formatting;
- no float-dependent formatting;
- no filesystem or path-dependent values;
- no wall-clock timestamps in canonical hashes.

The canonical encoding must be documented and tested. A hash over ad hoc serde
output is not enough unless the output format is itself constrained as a stable
canonical format.

## Minimal Future Shape

A minimal `WorldSpec` candidate may contain:

- `world_spec_version`;
- `world_id_policy`;
- `genesis_spec`;
- `genesis_hash`;
- `simulation_contract`;
- `limits_profile`;
- `persistence_compatibility`;
- `canonical_hashing_policy`.

It does not need to contain full ontology, modules, causal resolution, or
migration machinery in the first implementation step.

The first implementation step should make mismatched contracts rejectable and
testable. It should not attempt to model every future world concept at once.

## GenesisSpec

`GenesisSpec` is the part of `WorldSpec` that defines the reproducible starting
conditions of the world.

Conceptually, `GenesisSpec` contains:

- seed;
- initial topology;
- initial regions or zones;
- initial entities or population templates;
- initial resources;
- initial factions or institutions, when applicable;
- initial ontology and schema assumptions;
- initial active modules;
- initial simulation contract reference;
- deterministic generation rules;
- `genesis_hash`.

The seed is only a parameter of genesis.

The seed is not the complete world identity.

Two worlds can share the same seed and still be different worlds because their
topology, ontology, modules, initial entities, rules, limits, or generation
algorithms differ. The stable `genesis_hash` must cover the canonical
`GenesisSpec`, not only the seed value.

## World Identity

World identity must be explicit and recoverable.

A future Seej world should use either:

- an explicit `world_id` bound to the canonical `WorldSpec`; or
- a `world_id` derived from a stable genesis or world-specification hash.

`world_id` must be decoupled from seed. A naming convention like `world_42` is
acceptable as a local convenience only if it is not treated as the durable
identity contract.

Recovery must reject mismatches between:

- `world_id`;
- `genesis_hash`;
- `simulation_contract`;
- persisted snapshot metadata;
- WAL or event-log metadata;
- command-log metadata;
- manifest metadata.

Failing closed is required. A world loaded under the wrong identity or genesis
is worse than a world that refuses to start.

## SimulationContract

`SimulationContract` describes the rules that produced the world state.

It is not only storage format versioning.

It should include:

- ruleset version;
- deterministic RNG algorithm;
- event schema assumptions;
- command schema assumptions;
- replay semantics;
- scheduling semantics;
- resolution policy version;
- aggregation and materialization policy version;
- module contract versions;
- persistence format compatibility;
- canonical hashing semantics;
- allowed nondeterminism boundaries.

Allowed nondeterminism boundaries must be explicit. For example, wall-clock
time, network timing, client timing, unordered filesystem iteration, and
non-injected randomness are not valid core transition inputs. Any nondeterminism
outside the core must be converted into ordered, validated, persisted input
before it can affect state.

Changing a storage DTO version does not necessarily change the
`SimulationContract`. Changing rules, scheduling, replay semantics, canonical
hashing, or module causal meaning does.

## Ontology and Schema

The world ontology defines the systemic meaning of state.

It exists to:

- define entity types;
- define relationships;
- define which facts can be compressed;
- define which facts must remain individual;
- allow aggregation without losing meaning;
- allow modules to interconnect through declared semantics instead of hacks;
- support multi-resolution causality.

Ontology is not a narrow performance optimization.

It is a systemic complexity optimization.

The ontology must make durable distinctions such as:

- anonymous population count vs named individual;
- fungible resource vs unique artifact;
- local condition vs globally relevant causal factor;
- aggregate debt vs individually traceable obligation;
- transient observation vs canonical state.

Without ontology, aggregation becomes guesswork and modules cannot safely share
state.

## Module Contracts

The kernel provides the framework.

Modules declare domain-specific causal meaning.

Each module that participates in a world must conceptually declare:

- module id;
- module version;
- state schema;
- command types;
- event types;
- invariants;
- causal factors;
- aggregation rules;
- materialization rules;
- validation rules;
- compatibility policy;
- migration policy.

Module behavior must not be implicit runtime magic. If a module affects
persistent state, replay, aggregation, materialization, causal priority, or
validation, that behavior belongs in the world contract.

Modules must use public APIs and must not depend directly on `sy_core`.

## Time Model

Time must be contractual.

The `WorldSpec` or `SimulationContract` must define:

- tick semantics;
- simulated time derivation;
- scheduling window assumptions;
- whether rates are fixed, abstract, regional, or policy-driven;
- how commands target ticks;
- how events are ordered within a tick;
- how replay treats delayed, rejected, or reordered inputs.

Core logic must not depend on wall-clock time. A real-time server loop may
decide when to request the next tick, but state transitions must depend on
explicit simulated time, ordered inputs, and the active contract.

Different worlds may use different time policies, but a persisted world must not
silently change its time policy.

## Causal Resolution Policy

The `WorldSpec` contains or references the causal resolution policy described in
[`CAUSAL_RESOLUTION.md`](CAUSAL_RESOLUTION.md).

That policy covers:

- `ResolutionPolicy`;
- `CausalPriorityScore` definitions;
- `SimulationBudget`;
- `MaterializationRules`;
- `AggregationRules`;
- invariants of resolution changes.

The causal policy must be deterministic, versioned, bounded, and replayable. It
must not be driven by player proximity alone, client authority, wall-clock load
spikes, or opaque LLM output.

## Limits Manifest

Hard limits must not live only as binary constants.

They must be part of the world contract for recovery, validation, and operator
explanation.

A future limits manifest should define, at minimum:

- max entities;
- max events per tick;
- max command size;
- max WAL record size;
- max materializations per tick;
- max high-resolution regions;
- max non-canonical advisory calls, if enabled;
- max replay envelope;
- max snapshot size or snapshot policy.

If a new binary changes limits, recovery must be able to distinguish:

- a valid old world whose contract allowed the persisted state;
- a corrupted world;
- an incompatible world requiring explicit migration;
- an operator policy violation.

Limits are part of durability. They protect replay cost, WAL growth, memory
use, denial-of-service boundaries, and operator diagnostics.

LLM budgets are outside canonical simulation budgets unless the call produces
only non-authoritative advisory output. Canonical simulation must remain valid
with zero LLM calls.

## Persistence and Replay Compatibility

The `WorldSpec` must be bound to persistent artifacts:

- snapshot;
- metadata;
- WAL or event log;
- command log;
- manifest;
- replay oracle inputs;
- canonical state and causality hashes.

Recovery should fail closed when the persisted world does not match its
`WorldSpec`.

Compatibility checks must validate more than storage shape. They must prove
that persisted state was produced under the same world identity, genesis,
simulation contract, command/event schemas, scheduling semantics, and replay
semantics expected by the current recovery path.

The long-term replay target is:

```text
GenesisSpec + SimulationContract + ordered CommandEnvelope[] -> WorldState
```

Snapshot + WAL recovery remains an operational recovery path, but full
re-simulation from canonical intentions is the stronger audit and determinism
oracle.

## Versioning and Migration

World specification versioning must be explicit.

Required rules:

- no silent migration;
- incompatible rulesets are rejected unless an explicit migration exists;
- migrations must declare source and target contracts;
- migrations must preserve audit evidence;
- persisted worlds must not be reinterpreted under different rules without a
  clear migration;
- migration output must be inspectable and testable;
- compatibility fixtures should prove accept and reject behavior.

A migration may change storage layout, simulation semantics, ontology, module
contracts, limits, or all of them. Each kind of change must be declared
directly.

## Security and Audit

`WorldSpec` protects:

- integrity;
- auditability;
- corruption detection;
- module compatibility;
- operator diagnostics;
- reproducibility.

It gives operators and tools concrete answers to questions such as:

- Which rules produced this state?
- Which genesis produced this world?
- Which modules were active?
- Which command schemas and event schemas were valid?
- Which limits applied when the input was accepted?
- Which migration, if any, changed the world contract?
- Why did recovery reject this snapshot or WAL?

The contract should be usable by read-only diagnostic tools before any repair or
migration mutates durable evidence.

## Failure Modes

An incorrect or incomplete `WorldSpec` can cause:

- a seed to be mistaken for full world identity;
- persisted state to load under the wrong rules;
- replay divergence after apparently valid recovery;
- snapshot/WAL/command-log incompatibility;
- silent migration of old worlds;
- missing causality hashes for accepted transitions;
- operator tools to diagnose the wrong contract;
- durable worlds to become unrecoverable after binary changes.

These failures are persistence integrity failures, not gameplay bugs.

## Rejected Patterns

The following patterns are rejected:

- deriving world identity only from seed;
- hiding rules in runtime code;
- leaving module behavior undeclared in the world contract;
- changing ruleset without migration;
- using LLM text as canonical state;
- allowing a client to define world state;
- mixing gameplay configuration with persistence contract without versioning;
- silently accepting mismatched snapshots, WAL records, command logs, or
  contracts;
- treating storage format version as enough to identify simulation semantics;
- treating generated content around an observer as canonical genesis;
- accepting nondeterministic ordering as part of state transition semantics.

## Relationship to Phase 1 Gaps

This document does not close Phase 1 gaps. It formalizes the target architecture
needed to close them.

It directly supports the following known hardening gaps:

- `simulation_contract`;
- formal `GenesisSpec`;
- `world_id` independent from seed;
- canonical `CommandEnvelope`;
- durable input ordering;
- WAL/world/contract binding;
- storage DTO separation;
- world integrity validator;
- replay oracle;
- causality hashes;
- deterministic flight recorder;
- limits manifest.

Future implementation work should use this document as an architectural
reference when designing storage manifests, recovery checks, command journaling,
module contracts, replay tooling, and migration behavior.

No code should treat this document as evidence that those mechanisms already
exist.

## Implementation Checklist

Before modifying code based on this document, identify:

- the exact Phase 1 gap being closed;
- the persistent artifact affected;
- the compatibility check added;
- the rejection test added;
- why the change does not implement advanced simulation prematurely.

If those cannot be stated, do not modify code.
