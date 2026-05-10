# Causal Resolution Policy

## Status

This document defines target architecture for future Seej phases. It is not a
claim that causal multi-resolution scheduling exists in the current
implementation, and it is not an immediate Rust API requirement.

The current Phase 1 implementation validates a minimal deterministic headless
server with snapshot + WAL recovery. This document describes the policy family
needed for future durable worlds that cannot simulate every fact at full detail
forever.

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

Causal resolution belongs to the advanced simulation/scalability roadmap. Phase
1 may only prepare contract foundations needed to make it safe later.

## Blocking Foundations

Causal resolution is blocked until the following foundations exist:

1. `WorldSpec` hash and identity binding.
2. `SimulationContract` persistence.
3. Canonical `CommandEnvelope` log.
4. Durable ordering within ticks.
5. Replay oracle.
6. World integrity validator.
7. Limits manifest.
8. Contract-bound WAL and snapshot metadata.

Implementation must not start with `ResolutionScheduler`.

Resolution scheduling depends on world identity, contract persistence, command
journaling, durable ordering, replay validation, integrity validation, limits,
and contract-bound storage metadata. Implementing scheduling before those
foundations would create unauditable state transitions.

## Purpose

This document formalizes how Seej can preserve persistent world causality
without simulating the entire world at high resolution at all times.

It defines architecture constraints for deterministic multi-resolution
simulation: how resolution is prioritized, bounded, persisted or replayed, and
validated without making players, clients, rendering, or LLM output authoritative
over world state.

## Doctrine

Everything exists permanently.

Everything may influence the world.

Not everything is simulated at the same resolution all the time.

Seej preserves persistent causal reality through deterministic
multi-resolution simulation, not exhaustive high-resolution simulation.

High resolution is not a player privilege. It is a bounded server resource
allocated by explicit causal policy.

## Problem Statement

Persistent sandbox worlds must survive beyond the visible scene. Suspending
regions because no player is nearby breaks the project model: the world would
exist only around observers, and persistence would become a rendering illusion.

At the same time, simulating every entity, relationship, market, disease,
conflict, resource, decision, and local interaction at full detail forever would
explode:

- CPU cost;
- memory use;
- WAL volume;
- snapshot size;
- replay duration;
- audit cost;
- recovery risk.

The required distinction is:

- systemic existence: facts remain persistent and causally valid;
- simulation detail: the current resolution used to evolve those facts.

The solution is deterministic causal multi-resolution simulation. Low-resolution
state is real state, not absence. High-resolution state is a temporary refined
view of causally important parts of the world.

## Non-Negotiable Principles

- The server is the only source of truth.
- Resolution changes do not create client authority.
- The world is not centered on players.
- The server must run headless.
- Resolution decisions must be deterministic.
- Resolution decisions must be replayable.
- Persistent state must be explicit.
- Low-resolution regions are not nonexistent.
- There is no fake spawn around the player.
- There is no hidden world generation around the player.
- Clients observe state and submit intentions; they do not define world state.
- LLM output is not a source of truth.
- Resolution changes must be deterministic, explainable, bounded, and either
  persisted or reproducible from explicit state and contract.

## Resolution Levels

The following levels are conceptual. They can be refined by future design, but
they define the intended direction:

- global world state;
- region-level aggregate;
- group, faction, or population state;
- named important agents;
- local high-resolution scene;
- bounded full-detail simulation.

These levels are not rendering levels. They are simulation and persistence
levels.

A region can be low resolution and still contain real population, resources,
ownership, conflicts, disease state, debts, information, and pending events. A
small scene can be high resolution because its causal risk is high, even if no
player observes it.

## CausalPriorityScore

`CausalPriorityScore` estimates how important it is to simulate a target at
higher resolution during a scheduling window.

Possible factors include:

- player, client, or tool observation;
- important named agent;
- political instability;
- active conflict;
- rare resources;
- trade or logistics connectivity;
- disease or contamination propagation;
- information propagation;
- economic shock;
- faction leadership;
- recent event cascade;
- strategic location;
- historical importance;
- pending high-impact decision;
- module-declared causal factors.

The score must be:

- deterministic;
- calculated from explicit world state;
- versioned in `SimulationContract`;
- auditable;
- reproducible.

`CausalPriorityScore` must be computed as:

```text
score = Σ(versioned_factor_value * versioned_weight)
```

Every factor must declare:

- `factor_id`;
- input state fields;
- normalization rule;
- weight;
- tie-break behavior;
- module owner;
- contract version.

Opaque heuristics are rejected. A scheduler must not hide scattered rules like
`if unrest > 0.7` in runtime code. The factor, input, normalization, weight, and
tie-break behavior belong in the versioned causal policy.

The weighted score may be combined with explicit deterministic gates:

- mandatory promotion gates;
- forbidden promotion gates;
- minimum invariant-preservation gates;
- budget override rejection gates.

All gates must be versioned, declared, deterministic, and replayable.

Observation may contribute to priority because observed state may need more
detail for inspection and command validation. Observation must not dominate the
policy by default, and must not be equivalent to authority.

## SimulationBudget

The engine does not freely decide what is "interesting".

It applies explicit causal policies under limited budgets.

A future `SimulationBudget` should define:

- max high-resolution regions;
- max high-resolution entities;
- max events per tick;
- max WAL bytes per tick;
- max materializations per tick;
- max non-canonical advisory calls, if enabled;
- max CPU and memory envelope;
- max replay cost envelope.

Budgets are part of the world contract. They protect recovery, audit,
determinism, and operator expectations. If a budget is exceeded, the system must
record or reproduce the decision path; it must not silently drop canonical
events.

LLM budgets are outside canonical simulation budgets unless the call produces
only non-authoritative advisory output. Canonical simulation must remain valid
with zero LLM calls.

## Future ResolutionScheduler

`ResolutionScheduler` is the conceptual component that allocates simulation
resolution.

It:

- collects candidates;
- calculates causal priority scores;
- applies budgets;
- promotes or degrades resolution;
- emits traces or events;
- respects deterministic replay.

Conceptual scheduling loop:

```text
for each scheduling window:
  collect candidates
  compute causal priority scores
  sort deterministically
  apply simulation budgets
  promote high-impact targets
  degrade low-impact targets
  persist or replay decisions
  validate invariants
```

Candidate collection, scoring, sorting, tie-breaking, and budget enforcement
must be stable. Runtime load, thread scheduling, map iteration order, network
arrival timing, or client frame timing must not decide canonical resolution
state.

## Promotion and Degradation

Resolution transitions include:

- Aggregate -> Medium;
- Medium -> High;
- High -> Aggregate.

Transitions must preserve causality.

Forbidden transition behavior:

- arbitrary creation;
- loss of causal state;
- duplicated resources;
- duplicated entities;
- disappearance of history;
- mutation outside validated events;
- replacing canonical state with narrative summary;
- changing ownership, inventory, population, disease, debt, or conflict without
  a valid state transition.

Promotion refines explicit state. Degradation compresses explicit state. Neither
one is permission to invent or forget durable facts.

## MaterializationRules

Materialization is deterministic refinement, not generation.

Example aggregate region state:

```text
population = 900
food = 1200
unrest = 0.72
faction_control = IronGuild
leaders = [npc_17]
rare_resource = mana_crystal
```

When the region moves to a higher resolution, materialization may:

- materialize important named agents;
- materialize representative groups;
- materialize important stocks;
- materialize causal locations;
- derive local details only from aggregate state, seed, history, and
  `SimulationContract`.

Materialization must not:

- create an extra leader because a client arrived;
- duplicate the rare resource into multiple locations;
- erase previous conflicts;
- convert aggregate unrest into unrelated local facts;
- use LLM text as canonical state;
- depend on wall-clock time or nondeterministic runtime conditions.

If a detail was not previously represented individually, the materialization
rule must explain how it is derived from aggregate state and why the result is
replayable.

## AggregationRules

Aggregation converts detailed state back into compact state.

It must preserve:

- population totals;
- resources;
- ownership;
- faction control;
- important named agents;
- unresolved conflicts;
- causal debts;
- important memories or history;
- pending events;
- module-defined invariants.

Aggregation must not average away facts that remain causally unique. A unique
artifact, faction leader, infection source, signed debt, strategic message, or
pending succession crisis cannot become anonymous mass unless the ontology and
module contract explicitly allow it without losing causal meaning.

## Ontology Role

Ontology defines what state means well enough to compress it.

It allows Seej to:

- know what can be compressed;
- know what must remain individualized;
- connect modules through declared semantics;
- avoid hardcoded special cases;
- preserve systemic meaning during aggregation and materialization.

Without ontology, the scheduler cannot know whether `100 units` means fungible
grain, a unique artifact stack, debt obligations, infected carriers, or military
strength. Those facts have different causal preservation requirements.

Ontology is therefore a structural dependency of durable multi-resolution
simulation, not an optional performance trick.

Ontology is not required to be a general semantic-web system. Phase-appropriate
ontology may begin as versioned domain schemas and explicit
compression/materialization annotations.

Rejected ontology patterns:

- generic RDF/OWL dependency in the core;
- runtime semantic inference required for ticks;
- ontology rules that cannot be replayed deterministically;
- ontology stored only as documentation.

## Player-Neutral Prioritization

High resolution is a server resource allocation decision, not a player
privilege.

A non-player entity may be prioritized over a player if its causal potential is
higher.

Examples of high-priority non-player targets:

- political leader;
- spy;
- disease carrier;
- inventor;
- general;
- strategic merchant;
- messenger with critical information;
- mage preparing an unstable ritual;
- systemic economic actor.

Player, client, or tool observation can be a causal factor because observation
may require validation and inspectable detail. It is one factor among others,
not the center of the world.

## Module-Specific Causal Factors

Seej should not hardcode one universal causal policy in the kernel.

The kernel provides:

- scheduling framework;
- deterministic evaluation;
- persistence and replay;
- budget enforcement;
- invariant validation.

World specs and modules provide:

- causal factors;
- scoring weights;
- aggregation rules;
- materialization rules;
- invariants;
- limits.

Examples:

- economics: scarcity, debt, trade volume, market shock;
- politics: legitimacy, unrest, succession, faction hostility;
- disease: infection rate, density, mobility, mutation risk;
- magic: mana density, artifact uniqueness, ritual instability.

Modules declare domain meaning. The kernel enforces that declared meaning
deterministically and persistently.

## LLM and Generative AI

LLM usage is peripheral.

An LLM may propose:

- dialogue;
- plans;
- summaries;
- intentions;
- operator-facing explanations.

The server validates all effects. Only structured commands and events become
canonical state.

LLM text is secondary trace, not source of truth.

There is no continuous LLM brain for every NPC. Important scenes may be
temporarily refined under strict budget, but all canonical outcomes must be
validated, structured, persisted, and replayable.

## Persistence and Replay Implications

Causal resolution must integrate with:

- `SimulationContract`;
- `GenesisSpec`;
- `CommandEnvelope`;
- durable ordering;
- replay oracle;
- causality hashes;
- deterministic flight recorder;
- limits manifest;
- world integrity validator.

Resolution decisions must be reproducible.

If resolution decisions are persisted as events, replay must apply those events.
If they are recalculated, recalculation must be deterministic from explicit
state and contract.

Both paths require clear versioning. A world must not recover under a different
resolution policy unless an explicit migration declares the change and proves
compatibility.

## Failure Modes

Incorrect causal resolution can cause:

- duplicated unique entities;
- lost debts or ownership;
- replay divergence;
- snapshot/WAL incompatibility;
- invisible state corruption;
- player-proximity world bias;
- unbounded WAL growth;
- unrecoverable materialization history.

These failures are persistence integrity failures, not gameplay bugs.

## Conceptual Events

The following event names are architectural concepts, not immediate Rust API
requirements:

- `ResolutionPromoted`;
- `ResolutionDegraded`;
- `RegionMaterialized`;
- `RegionAggregated`;
- `CausalPriorityEvaluated`;
- `BudgetExceeded`;
- `MaterializationRejected`;
- `AggregationValidated`;
- `InvariantViolationDetected`.

These concepts name transitions and diagnostics that future designs may need to
persist, replay, or emit through a deterministic flight recorder. Final API
shape belongs to later implementation design.

## Rejected Patterns

The following patterns are rejected:

- `if player_nearby => high_res else fake`;
- spawning content only because the player arrived;
- client-driven world generation;
- LLM directly mutating world state;
- simulating everything at full detail forever;
- storing only narrative summaries as canonical state;
- aggregating unique artifacts, leaders, or critical debts into anonymous
  averages;
- changing resolution based on nondeterministic runtime conditions;
- silently dropping events because budget is exceeded;
- treating low-resolution regions as nonexistent;
- allowing high-resolution scenes to bypass module invariants;
- treating rendering detail as simulation resolution.

## Relationship to WorldSpec

The causal resolution policy must be declared or referenced by the
[`WorldSpec`](WORLD_SPEC.md).

Different worlds or modules may use different causal policies, but they must
remain deterministic, versioned, bounded, and replayable.

The `WorldSpec` binds the causal policy to genesis, ontology, module contracts,
limits, persistence compatibility, and replay semantics. Without that binding,
resolution changes would be hidden runtime behavior and could not be safely
audited or recovered.

## Relationship to Phase 1 Gaps

This document does not close Phase 1 gaps. It provides target architecture for
future work related to:

- simulation contract versioning;
- formal genesis;
- durable input ordering;
- command and event causality;
- causality hashes;
- deterministic flight recorder;
- limits manifest;
- world integrity validator;
- replay oracle;
- storage DTO separation;
- WAL/world/contract binding.

The causal resolution policy depends on those foundations. It must not be
implemented as a scheduler shortcut, player-proximity heuristic, or hidden
runtime optimization.

## Implementation Checklist

Before modifying code based on this document, identify:

- the exact Phase 1 gap being closed;
- the persistent artifact affected;
- the compatibility check added;
- the rejection test added;
- why the change does not implement advanced simulation prematurely.

If those cannot be stated, do not modify code.
