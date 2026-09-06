# Plugin and Provider Contract

```text
contract_version: 0.2.0-draft
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
kernel-issued producer slot and incarnation, admission generation, and producer
sequence), target tick, a positive finite status window, canonical order after
admission, source metadata, `intention_fingerprint`, owning command contract and
version, and correlation metadata. The fingerprint binds the complete intention
but excludes transport-only correlation data. The producer incarnation is a
bounded command namespace, distinct from the authenticated external principal.

Admission rules:

- only a generation opened for the producer by a confirmed kernel operation is
  valid;
- a request for an older incarnation of a reassigned producer slot returns
  `Expired` before compacted generation records are consulted;
- retrying a pending or unexpired identifier with the same fingerprint returns
  its durable status without repeating effects, even when its target tick has
  since closed;
- a pending or unexpired identifier with a different fingerprint is a conflict;
- a new command for an already completed tick is refused;
- a timeout is not a terminal status;
- after lookup of previously admitted IDs, a new application submission during
  a confirmed migration admission pause returns `WorldPaused` without creating
  an Admission;
- a terminal result remains queryable only while the confirmed world tick is
  below its recorded
  `expiration_tick = resolution_tick + status_window_ticks`, calculated with
  checked canonical arithmetic; at that tick and afterward, the identifier
  returns `Expired` and cannot be reused as a new command.

Admission creates durable `Pending` state without application effects.
Resolution records `Succeeded`, `Refused`, or `Cancelled`; a successful
resolution confirms all events and state effects together. `Conflict`,
`Expired`, `GenerationSealed`, `LimitExceeded`, `NotFound`, `Unavailable`, and
`WorldPaused` are request results rather than stored terminal states. A pending
command is retained until resolution. Closed kernel-issued generations preserve
non-reuse after individual terminal results are compacted.

A producer generation is `Open`, `Sealed`, or `Closed`. Opening the next
generation seals the previous open one; declared finite command-count,
byte-count, or canonical-tick-age limits can also seal it through confirmed
order. Sealed generations accept no new identities but keep resolving admitted
commands. Expired terminal details become non-reuse tombstones. A generation
closes only after it is sealed and all commands are terminal and expired, after
which the active incarnation's closed-generation floor is sufficient. An unused
ID in a sealed generation returns `GenerationSealed` on submission; any ID in a
closed generation returns `Expired` before fingerprint comparison.

Each world exposes a finite number of kernel-issued producer slots. Retiring a
producer incarnation requires all its generations to be closed. Reusing that
slot confirms a strictly greater incarnation; a durable per-slot high-water
mark rejects every earlier incarnation without retaining its individual
generation floors. External callers cannot choose slots or incarnation values.

If producer activation, generation opening, or admission would exceed a
persisted producer or world bound, the kernel creates no producer, generation,
or Admission record and returns `LimitExceeded`. The producer may retry after
an incarnation retires or older generations close; the kernel never frees
capacity by dropping pending work or extending an expiration promise.

An authorized operator can request a confirmed `CancelCommand` kernel control
operation for a command that is still `Pending`. It records the target,
requester, reason, and its own stable identity. Cancellation produces no
application effects and does not settle application obligations. Normal
execution and cancellation race only through confirmed order: the first
terminal resolution wins, and retrying either request returns the durable
outcome. The kernel control path remains usable when application code is
unavailable, subject to its own authorization, quorum, limits, compatibility,
and persistence requirements. Its envelope follows the same bounded identity,
fingerprint, retry, and result-retention rules.

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
the authority unavailable until it can resume safely. An authorized
`Cancelled` decision remains available as the explicit administrative escape
path; it is never synthesized from a timeout or crash.

The existing resource and creature degradation behavior is a prototype
fixture. Before the first stable contract it must execute through the same
public extension boundary as any third-party world rule.

## 6. Lifecycle

Startup activation verifies manifests, permissions, fingerprints,
dependencies, data versions, and execution compatibility before mutation is
allowed.

The first-stable migration protocol pauses every new application admission for
the world, then lets every application command already admitted under the old
profile reach a normal terminal result or an authorized cancellation. A
confirmed barrier fixes the exact source state and cursor before conversion. No
world tick or application transition may pass that barrier; inspection,
authority elections, and migration-control operations remain available. The
migrated candidate and target execution profile activate together or the old
profile remains authoritative.

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

Gap `K-05` requires a public-interface substitution fixture for every
fundamental capability boundary. An alternative used only by the conformance
harness is sufficient; Seej does not require two production implementations of
each capability. One test component may cover several boundaries, but each row
below needs independent success and failure evidence:

| Capability boundary | Minimum substitution evidence |
| --- | --- |
| State storage and recovery | Replace the bundled provider; preserve durable ordering, checkpoint recovery, integrity checks, and fail-closed injected I/O failures. |
| Command admission and status | Preserve producer incarnation, identity, deduplication, ordering, terminal states, cancellation, generation lifecycle, expiry, and limits. |
| Tick scheduling | Advance explicit canonical ticks without making wall-clock or local scheduling order a transition input. |
| Deterministic validation | Produce the same typed decisions from the same explicit state, command, tick, and contract without bypassing owner rules. |
| Transaction coordination | Preserve ordered candidate construction, final cross-domain validation, stale-base recomputation, and atomic confirmation. |
| Membership and authority terms | Preserve explicit admission, voter configuration, joint transitions, durable terms, and stale-authority fencing. |
| Consensus and replication | Preserve one confirmed order in one- and three-node profiles, quorum loss behavior, catch-up, and crash recovery. |
| Observation and retained cursors | Preserve confirmed/stale read classes, neutrality, authorization, cursor expiry, and coherent resynchronization. |
| Component loading and compatibility | Preserve manifest, permission, version, fingerprint, dependency, migration, and incompatible-activation checks. |

Every fixture imports only public contracts and is run against both the bundled
and alternative implementation. Hot loading and sandbox isolation do not block
the first stable contract.

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
