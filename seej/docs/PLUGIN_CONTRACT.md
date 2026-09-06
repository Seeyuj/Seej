# Plugin and Provider Contract

```text
contract_version: 0.1.0-draft
status:           DRAFT — PARTLY IMPLEMENTED
```

Seej uses a kernel-and-ecosystem model. Every capability beyond the smallest
coordination kernel is supplied by a provider or application plugin. This
document defines the guarantees those components must respect.

The current repository has internal Rust ports and one statically wired local
runtime. It does not yet implement the generic contract described here.

## 1. Capability classes

### Fundamental providers

The minimal Seej distribution includes replaceable providers sufficient to:

- create and identify an empty world;
- admit and order commands;
- advance simulated time;
- validate and apply transitions;
- persist, recover, and inspect state;
- manage node membership and replicate confirmed history.

Replacement cannot weaken the public guarantees. A storage provider, for
example, may choose its physical format but must preserve durability,
integrity, ordering, recovery, and migration behavior.

### Application plugins

World data models, spatial systems, agents, economies, multiresolution engines,
renderers, interfaces, and ingame netcode are application capabilities. Seej
does not select an official implementation of any of them.

The first stable contract supports operator-approved trusted components loaded
at process startup. Hot loading, hot replacement, and isolated execution are
later capabilities. When isolation is implemented, application plugins run
isolated by default with explicit permissions; native execution remains an
operator-approved mode.

## 2. Manifest and compatibility

Every provider or plugin declares a manifest containing at least:

- stable component and publisher identifiers;
- semantic version and immutable artifact fingerprint;
- implemented contract and capability versions;
- owned data domains and public command types;
- required and optional capabilities;
- permissions and resource limits;
- deterministic execution requirements;
- data-format versions and available migration steps;
- lifecycle obligations that can block removal.

The operator selects and approves the active set. Seej records the exact
manifest and fingerprint set in the world execution profile. Missing,
incompatible, or differently fingerprinted required components block
activation and re-simulation.

## 3. Data ownership

Every mutable application data domain has one declared logical owner.
Other components may read it only through allowed queries or observations and
may change it only through commands exposed by its owner.

Plugin data may be opaque to the kernel. Its durable envelope still identifies
the world, owner, format version, payload length or limits, integrity data, and
causal position. Seej guarantees envelope and payload preservation but does
not promise generic interpretation of opaque fields.

A world model may omit space entirely. The kernel does not reserve categories
such as zone, creature, resource, player, or human. Plugins can define those
concepts and their laws.

## 4. Commands and transactions

A command envelope contains the stable command identity (`world_id`,
kernel-issued admission generation, producer identity, and producer sequence),
target tick, canonical order after admission, source metadata, payload
fingerprint, owning command contract and version, and correlation metadata. The
fingerprint binds the complete intention but excludes transport-only correlation
data.

Admission rules:

- only a generation opened for the producer by a confirmed kernel operation is
  valid;
- retrying the same identifier with the same fingerprint returns its durable
  status without repeating effects, even when its target tick has since closed;
- the same identifier with a different fingerprint is a conflict;
- a new command for an already completed tick is refused;
- a timeout is not a terminal status;
- after the declared status-retention window, the identifier is recognized as
  expired and cannot be reused as a new command.

Admission creates durable `Pending` state without application effects.
Resolution records `Succeeded` or `Refused`; a successful resolution confirms
all events and state effects together. `Conflict`, `Expired`, `NotFound`, and
`Unavailable` are request results rather than stored terminal states. A pending
command is retained until resolution. Closed kernel-issued generations preserve
non-reuse after individual terminal results are compacted.

One in-world transaction may call commands owned by multiple plugins. The
transaction binds to one confirmed base cursor and gives owner sub-commands a
canonical order. Each owner validates against the candidate state produced by
the preceding sub-commands and may propose changes only to its own domains.
Cross-domain validators then inspect the complete candidate.

The kernel confirms the decision, events, and complete transition atomically,
or applies none of it. A candidate whose base cursor is no longer the confirmed
predecessor is discarded and recomputed in full. Effects in other worlds and
external services require explicit asynchronous protocols and are not part of
this atomicity guarantee.

## 5. Laws and validation

Each world declares its laws through active validation capabilities. The
kernel coordinates and enforces their results; it does not invent universal
economic conservation, mortality, spatial, or agent rules.

Validation is deterministic for canonical transitions and returns typed,
persistable decisions. An external nondeterministic computation affects the
world only after its result becomes a bounded, validated, persisted command
input.

Only deterministic contract evaluation can produce a durable `Refused`
decision. A plugin crash, timeout, missing artifact, or temporarily unavailable
capability produces no application effects and leaves the admission pending or
the authority unavailable until it can resume safely.

The existing resource and creature degradation behavior is a prototype
fixture. Before the first stable contract it must execute through the same
public extension boundary as any third-party world rule.

## 6. Lifecycle

Startup activation verifies manifests, permissions, fingerprints,
dependencies, data versions, and execution compatibility before mutation is
allowed.

A component with owned data or active obligations cannot be removed merely by
unregistering code. Removal or replacement migrates its owned data and either
transfers each active obligation to an activated compatible successor or
resolves it according to the world contract. Archival permits detachment only
for terminal obligations, or as part of archiving the entire world so that it
cannot advance. The server process may stay available while the affected
world's progression pauses at a confirmed boundary.

Stable kernel-owned data always has a migration path from each stable version.
Plugins own their data migrations. A missing step or component blocks the
world migration. The operator archives historical artifacts; Seej records
their references and fingerprints and refuses unsupported replay.

## 7. Observation and multiresolution

Read-only observers have no lifecycle hook that can alter canonical state,
priority, tick cadence, or simulation detail. Under load, the runtime limits
or refuses reads before changing world semantics.

A community multiresolution plugin may aggregate and materialize data only
under a world-declared approximation policy. It preserves protected identities,
facts, obligations, and past history exactly. Future outcomes may differ from
a hypothetical fully detailed run when that difference is explicitly allowed.
Seej supplies persistence and validation primitives, not an official
multiresolution scheduler or scoring model.

## 8. First stable evidence

Gap `K-04` requires a real external extension, maintained outside the kernel
implementation, that:

- declares and persists its own non-spatial data;
- owns commands and validators;
- participates in an atomic transaction with another owner;
- recovers without relying on internal Rust domain types;
- proves that a consumer can use the contract without reading `sy_core`.

Gap `K-05` requires at least one fundamental provider replacement in tests.
Hot loading and sandbox isolation do not block the first stable contract.

Transaction and lifecycle fixtures before freeze include:

- with a declared non-negative-balance law, two ordered withdrawals of seven
  from a balance of ten cannot both succeed;
- payment and item transfer either appear in one confirmed resolution or not
  at all;
- a candidate made stale before confirmation is discarded and recomputed in
  full;
- a participant crash cannot become a fabricated business refusal or partial
  effect;
- a debt with a future due tick prevents owner removal until confirmed transfer
  or resolution.
