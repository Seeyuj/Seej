# Node-Network Contract

```text
contract_version: 0.1.0-draft
status:           DRAFT — TARGET, NOT IMPLEMENTED
```

This document defines the network that joins Seej kernel nodes. It is part of
the first stable kernel contract. It does not define player sessions,
prediction, interpolation, rendering replication, or any other ingame
netcode.

## 1. Scope and fault model

A logical world runs on an authorized authority group of one or more nodes.
The reference distributed profile uses three nodes and a quorum of two. It
assumes authenticated operators, crash failures, storage failures, message
loss, reordering, duplication, delay, and network partitions. Byzantine nodes,
permissionless admission, global consensus across worlds, and automatic
cross-world transactions are outside the first stable contract.

A consensus provider may be replaced only by another provider that satisfies
the observable guarantees in this document. The reference implementation may
use a Raft-style protocol; its wire format is not the public plugin contract.

## 2. Identities and authority

- `world_id` identifies one canonical history and is independent of the seed,
  genesis hash, node identity, and storage location.
- `node_id` identifies an operator-authorized node. Discovering a node never
  admits it to an authority group.
- Each world records two independent monotonic values: an authority term for
  coordinator elections and a configuration version for membership. Every
  authority message binds both values. Only the active term and configuration
  can confirm mutations.
- A replica preserves `world_id`. An independent clone must use a new
  `world_id`, even if it copies state and genesis.
- Membership changes are confirmed transitions. A new node first joins as a
  non-voting learner, verifies the execution profile, and catches up before it
  can enter a voting configuration. A returning voter recovers its durable
  term, vote, configuration, and confirmed log before participating under the
  consensus protocol. A configured voter that is offline or temporarily
  incompatible does not become a learner implicitly: it remains part of the
  quorum denominator but cannot vote until compatible and caught up. Removing
  or demoting it requires the confirmed membership-transition protocol. No node
  serves current reads until it has applied the required confirmed prefix.

Authentication, credential formats, and transport encryption are deployment
concerns described by the security policy. They must bind a connection to an
authorized `node_id` and world membership.

## 3. Reference membership transition

The first Raft-style provider changes membership without allowing disjoint old
and new groups to confirm independently:

1. New nodes enter as non-voting learners and catch up to a confirmed cursor.
2. The current configuration confirms a joint configuration containing the old
   and new voter sets.
3. While joint configuration is active, elections and confirmation require a
   majority of the old set and a majority of the new set.
4. The final new configuration is confirmed under the joint transition rule.
   Only after that commit can the removed voters lose authority and the new
   configuration act alone.

The configuration version and transition state are durable confirmed data.
After a full restart, recovered voters use them with their persisted authority
term and vote; local configuration files cannot override confirmed membership.
Other consensus providers may use a different mechanism only if conformance
evidence proves equivalent quorum intersection, fencing, and recovery.

## 4. One confirmed history

For each world, the authority group establishes one monotonically ordered
confirmed log. A node may stage or durably append a proposal locally, but that
local append is not a distributed commit.

A mutation becomes canonical only after the active authority group confirms
it according to the selected provider's quorum rule. Only then may the runtime:

1. report durable success;
2. apply or expose its canonical effects;
3. advance the published commit cursor;
4. emit it to observers.

The pure transition result must be reproducible from the prior confirmed
state, the ordered admitted input, the tick, and the active contract profile.
Network arrival order and leader scheduling are not transition inputs until
captured in the confirmed order.

Confirmation and local materialization are separate positions. The group
publishes a canonical `confirmed_cursor`; each node records the
`applied_cursor` of its current state instance. A checkpoint atomically binds
its state and semantic hash to a `checkpoint_cursor`. A node can lag and catch
up by materializing recorded confirmed transitions in order, but it cannot
re-evaluate an already resolved command during ordinary recovery or claim a
current read before applying the required barrier.

## 5. Availability and failure behavior

- **Quorum available:** the group may admit and confirm mutations.
- **Quorum lost:** mutation pauses for that world. Nodes may expose a
  read-only confirmed state with its commit cursor and freshness status.
- **Stale coordinator:** term and configuration fencing prevent it from
  confirming or publishing mutations.
- **Local persistence failure:** the affected node stops or is quarantined
  before accepting an undurable canonical mutation. Other nodes continue only
  if the configured quorum remains.
- **Returning node:** it validates identity, authority term, configuration
  version, contract profile, and confirmed prefix before catch-up. The
  consensus provider may explicitly replace a local unconfirmed suffix. A
  conflict in a prefix already known as confirmed is quarantined and never
  overwritten as ordinary catch-up.
- **Uncertain client result:** a timeout or disconnected response remains
  uncertain. The caller uses the command-status contract; it must not infer
  rejection.

The kernel never creates two canonical branches to remain writable through a
partition, and it provides no automatic branch merge.

## 6. Time and ordering

Each world has its own canonical tick and command order. The authority group
admits commands into an order independent of packet arrival races. Replicas do
not advance ticks independently, and Seej defines no global clock shared by
all worlds.

A command targeting a completed tick is refused explicitly. A world that maps
real time to ticks declares its cadence and catch-up or degradation policy as
part of its world contract. That policy cannot weaken quorum, persistence, or
observation neutrality.

## 7. Reads and observation

Canonical observation reads from a confirmed prefix, never an unconfirmed
local WAL tail. Every response identifies:

- `world_id`;
- authority term and configuration version;
- confirmed cursor;
- state `applied_cursor` or event position;
- freshness, including whether the node is currently caught up.

A **current confirmed** read requires a quorum-backed authority read barrier in
the active term and configuration. The serving node applies at least through
that barrier before replying and binds the response to that applied cursor. A
confirmed state without such a barrier is **known confirmed but stale** and
reports its cursor and freshness as stale or unknown. These modes cannot be
confused silently; an isolated former coordinator cannot claim current state.

History compaction may expire an observer cursor. The response identifies the
expiration and offers a coherent state with a new cursor; removed history is
not recreated. Reconnection can redeliver identified events, so consumers
deduplicate by their stable identifiers.

The replacement state and cursor describe the same confirmed cut. The provider
pins that cut, or an equivalent retained recovery point, until stream handoff
completes. If it cannot, it returns a new expiration result and cut rather than
silently skipping events.

## 8. Compatibility and migration

Before participating in authority, nodes prove compatibility with the world's
execution profile:

- kernel contract and state-format versions;
- simulation contract and deterministic execution profile;
- command, event, and ownership schemas;
- required provider and plugin identifiers, versions, and fingerprints;
- active limits and retention policies.

An incompatible node cannot vote, confirm mutations, or claim a current read.

A coordinated migration has explicit confirmed phases:

1. `MigrationPrepare` identifies the old and target profiles, affected
   capabilities, conversion steps, required artifacts, and active configuration
   version under the old profile. New affected application admissions pause.
   The prepare record does not yet choose the source state for conversion.
2. Every pending command bound to the affected old profile reaches
   `Succeeded`, `Refused`, or an explicitly authorized `Cancelled` resolution.
   Cancellation is a confirmed kernel control operation and never an inferred
   result of plugin failure.
3. `MigrationBarrier` fixes the exact old-profile state, semantic hash, and
   confirmed cursor after the drain. Affected ticks and application transitions
   cannot pass this barrier.
4. Nodes needed for the active configuration's quorum verify the target
   artifacts and deterministically derive the same candidate state and semantic
   hash from that barrier.
5. `MigrationActivate` confirms the barrier reference, target profile, and
   migrated state together. This is the activation commit point;
   mixed-profile mutation remains refused.

Membership changes and migration are serialized. The configured voter set and
configuration version remain fixed from preparation through activation or
confirmed abort; if membership must change, the migration aborts and restarts
under the new confirmed configuration. Elections may still change the
authority term. An absent configured voter remains a voter in the confirmed
configuration and therefore in quorum calculation, but cannot participate or
serve current reads until it installs the active profile and catches up. A
confirmed joint membership transition is required to remove or demote it.

Before `MigrationActivate` commits, a failed preparation or barrier can be
abandoned by a confirmed abort under the old profile, releasing the admission
and progression pause while leaving the old state authoritative. After
activation commits, the new profile and state are authoritative and recovery
must finish installing or loading them. An uncertain response is resolved by
reading confirmed history. Returning to the old profile requires a separately
declared and confirmed migration; it is never an automatic rollback.

## 9. First stable evidence

Gap `K-07` requires executable evidence for:

- one-node operation through the same contracts;
- three nodes with quorum two;
- coordinator failure and replacement;
- minority partition remaining read-only;
- stale-authority fencing;
- node catch-up after interruption;
- local disk failure with and without remaining quorum;
- learner catch-up and joint membership change, including interruption between
  the joint and final configurations;
- full authority-group restart from durable term, vote, membership, and log;
- incompatible node or plugin profile refusal;
- current-read refusal by an isolated former coordinator;
- observation resynchronization without a gap while compaction advances;
- migration failure before activation and recovery after activation commit;
- cancellation racing normal resolution without two terminal outcomes;
- migration draining a blocked command through authorized cancellation;
- a mutation finishing after preparation but before the final source barrier;
- an absent configured voter returning under the activated profile without an
  implicit membership change.

Until that evidence exists, every distributed behavior in this document is a
target rather than an implemented claim.
