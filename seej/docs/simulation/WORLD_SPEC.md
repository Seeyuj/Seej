# World Specification

```text
status: TARGET MODEL — CURRENT PROTOTYPE IS MORE SPECIFIC
```

This document defines what a Seej world means independently of any one storage
provider, plugin, deployment topology, or Rust representation. The short
binding subset is [../CONTRACT.md](../CONTRACT.md).

## 1. World identity

A world is one persistent canonical history identified by a stable `world_id`.
The identifier is independent of:

- the seed and genesis hash;
- a node or authority group;
- a filesystem location;
- a current coordinator;
- the plugins presently available;
- a checkpoint or storage-provider format.

A replica is another holder of the same world and identity. A clone begins a
new world identity while recording its provenance when desired.

A versioned `GenesisSpec` declares the initial execution profile and initial
data. Its semantic hash detects mismatch; the hash is not the identity itself.

## 2. Canonical and operational state

Canonical world state contains only values that affect future canonical
transitions or the meaning of established history. It includes:

- tick and command-order cursors;
- deterministic randomness state when used;
- versioned owner-domain data;
- active laws, contracts, limits, and approximation policy;
- active obligations needed for safe plugin lifecycle;
- semantic references required for recovery and migration.

Operational state includes node membership, local file paths, transport
connections, caches, leader leases, diagnostics, and storage cursors. It can be
durable and security-sensitive without becoming application world state.

The canonical history has a `confirmed_cursor`. A retained checkpoint binds
one state and semantic hash to its `checkpoint_cursor`, while each node-local
state instance tracks the highest materialized `applied_cursor`. These cursor
roles cannot be substituted: a node may know a later confirmed cursor than it
has applied, and cannot present the lagging state as current.

A semantic state hash excludes deployment-specific bytes and authority
metadata. Storage checksums and authenticated network messages have separate
purposes.

## 3. Time

Each world has a monotonically increasing canonical `Tick`. Tick advancement
is an admitted command or an equivalently explicit confirmed input. Replicas do
not choose their own ticks.

The mapping between ticks and real time is a world policy. A world may run as
fast as possible, be operator-driven, or request a real-time cadence. A
real-time world declares bounded behavior for overload, pause, and catch-up.
No cadence policy may create undurable mutations, bypass quorum, or let
observers influence canonical decisions.

There is no global tick that orders independent worlds.

## 4. Data domains

World data is organized into owner domains. Each domain has:

- a stable owner capability;
- format version;
- payload, either structured by a declared schema or opaque;
- public commands and queries;
- deterministic validators;
- declared links to other domains;
- limits, retention needs, and active-obligation semantics;
- migration steps.

The kernel does not require an `Entity`, `Zone`, position, creature,
resource, item, player, or human type. Plugins can define these concepts.

Opaque payloads do not exempt a domain from ownership, limits, versioning,
integrity, migration, or lifecycle declarations. The kernel can preserve an
opaque payload without being able to display its fields.

## 5. Laws

A world's laws are the active set of declared validator contracts and
cross-domain invariants. The kernel executes and enforces them; it does not
supply universal gameplay rules.

A valid world can allow or forbid resource creation, distinguish humans from
other characters, model mortality differently by role, or have no economy at
all. These are explicit world choices.

Validation is deterministic for canonical state. It returns a typed decision
and cannot depend directly on wall-clock, network arrival order, observer
presence, or an unrecorded external service result.

## 6. Commands, transactions, and events

Commands express intentions against an owner domain at a target tick. Admission
binds them to a stable identity, payload fingerprint, active contracts, and a
canonical order.

A transaction can contain commands for several owners. It binds to one
confirmed base cursor and gives its owner sub-commands a canonical order. Each
owner observes the candidate state produced by preceding sub-commands, but may
propose changes only to its own domains. Cross-domain laws validate the
complete candidate. If any part is invalid or the complete result cannot be
durably confirmed, no owner effects occur.

If another confirmed transition makes the base cursor stale, the candidate is
discarded and the entire transaction is recomputed from its canonical
predecessor. Execution failure or unavailable plugin code is not a world-law
refusal and cannot create partial effects. An authorized operator may instead
resolve a still-pending command as `Cancelled` through the confirmed kernel
control contract. Cancellation has no application effects and does not resolve
world obligations.

Events record the accepted causal result. A state change without an admitted
command and confirmed event chain is invalid. Diagnostic messages and
observer-specific projections are not events.

Ordinary recovery materializes recorded confirmed transitions after a
checkpoint without asking owner plugins to decide resolved commands again.
Re-simulation is the separate operation that recomputes them with exact
components. Stable event identifiers permit redelivery without creating a new
canonical event.

External side effects and other worlds are outside the atomic boundary. Their
coordination uses explicit messages and compensating or asynchronous
application protocols. Recovering kernel state does not directly repeat an
external call; consumers of durable effect intentions handle redelivery by
stable identity.

## 7. Randomness and external computation

Randomness is a world capability with a declared deterministic algorithm and
persisted state or persisted selected result. Re-execution uses the exact same
random input.

External computation, including an LLM, search service, sensor, or wall-clock
observation, is not executed inside the pure transition. Its result becomes an
untrusted command payload, is bounded and validated, and is persisted if it
causes canonical effects.

## 8. Approximation and multiresolution

Multiresolution is optional community behavior. A world that enables it
declares:

- facts and identities that must remain exact;
- active obligations and laws that cannot be approximated;
- data that may be aggregated;
- permitted approximation error or qualitative freedom;
- materialization and reconciliation rules;
- deterministic inputs to the policy.

Established facts and past history remain true. The future may differ from a
hypothetical fully detailed simulation when the declared policy permits that
difference.

Observers never select fidelity by their presence or camera position. An
authorized world command may change the active simulation policy, but that
change is canonical, validated, ordered, and persisted.

Seej supplies persistence, validation, and ownership primitives. It does not
ship an official multiresolution scheduler, scoring formula, or aggregation
engine.

## 9. Retention and obligations

A world declares how long detailed commands, events, command results, snapshots,
and component artifacts are retained. Compaction must preserve a coherent
recovery point, current state, active obligations, command-ID expiration
safety, and migration metadata.

Command-result expiry is an exact canonical tick recorded with the terminal
resolution. Detailed status is unavailable from that tick onward, while a
compact tombstone or closed-generation floor continues to prevent ID reuse.
Finite producer and world limits force confirmed generation rotation or
admission backpressure so an active producer cannot make retained terminal
results grow without bound.

An obligation is durable lifecycle state whose owner cannot disappear without
a declared outcome. Examples include debts, scheduled effects, references held
by another domain, and pending cross-domain transactions. Its envelope must be
identifiable even when the payload is opaque.

An active obligation is detached from its current owner only by a confirmed
transfer to an active compatible successor or a confirmed resolution. Merely
archiving its bytes is not an outcome while the world can still advance.

Full archival and genesis re-simulation are optional. A bounded-retention world
can still promise recovery from its retained checkpoint.

## 10. Deployment independence

World semantics do not encode the current node count, coordinator, transport,
or file layout. Those belong to the node-network and provider configuration.
A deployment change preserves world identity and confirmed history.

The authority group records the execution profile required to interpret and
advance the world. A node lacking compatible kernel, contracts, providers, or
plugins cannot claim authority.

## 11. Lifecycle operations

- **Create:** allocate a new `world_id`, validate genesis, and confirm initial
  state.
- **Recover:** load a retained compatible checkpoint and confirmed suffix.
- **Replicate:** add an authorized copy of the same world through catch-up.
- **Migrate:** drain or explicitly cancel affected pending commands, fix the
  exact source state at a confirmed barrier, and transform explicit data
  versions for atomic activation.
- **Re-simulate:** recompute retained transitions with exact components.
- **Clone:** create a new identity from an exported state.
- **Archive:** preserve state without requiring it to remain executable; an
  archived world cannot advance until all required components and obligations
  are restored through an explicit lifecycle transition.
- **Delete:** an operator operation outside the simulation contract.

These operations cannot be substituted for each other silently.

## 12. Current prototype mapping

The current Phase 1 world uses `world_<seed>`, an origin zone, typed entities,
positions, resource/creature categories, float properties, and built-in
degradation rules. Its snapshot and WAL are local reference formats.

Those choices remain documented for recovery of the current format, but they
are not universal requirements of the target world model. Their removal or
migration must be explicit and tested before the first stable contract.
