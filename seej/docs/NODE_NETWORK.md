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
- Each world records its authority membership and an increasing authority
  epoch. Only the active epoch can confirm mutations.
- A replica preserves `world_id`. An independent clone must use a new
  `world_id`, even if it copies state and genesis.
- Membership changes are confirmed transitions. A joining or returning node
  catches up to a confirmed state before it can vote or serve current reads.

Authentication, credential formats, and transport encryption are deployment
concerns described by the security policy. They must bind a connection to an
authorized `node_id` and world membership.

## 3. One confirmed history

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

## 4. Availability and failure behavior

- **Quorum available:** the group may admit and confirm mutations.
- **Quorum lost:** mutation pauses for that world. Nodes may expose a
  read-only confirmed state with its commit cursor and freshness status.
- **Stale coordinator:** authority-epoch fencing prevents it from confirming
  or publishing mutations.
- **Local persistence failure:** the affected node stops or is quarantined
  before accepting an undurable canonical mutation. Other nodes continue only
  if the configured quorum remains.
- **Returning node:** it validates identity, authority epoch, contract profile,
  and confirmed prefix before catch-up. It never overwrites a divergent local
  history silently.
- **Uncertain client result:** a timeout or disconnected response remains
  uncertain. The caller uses the command-status contract; it must not infer
  rejection.

The kernel never creates two canonical branches to remain writable through a
partition, and it provides no automatic branch merge.

## 5. Time and ordering

Each world has its own canonical tick and command order. The authority group
admits commands into an order independent of packet arrival races. Replicas do
not advance ticks independently, and Seej defines no global clock shared by
all worlds.

A command targeting a completed tick is refused explicitly. A world that maps
real time to ticks declares its cadence and catch-up or degradation policy as
part of its world contract. That policy cannot weaken quorum, persistence, or
observation neutrality.

## 6. Reads and observation

Canonical observation reads from a confirmed prefix, never an unconfirmed
local WAL tail. Every response identifies:

- `world_id`;
- authority epoch;
- confirmed cursor;
- state or event position;
- freshness, including whether the node is currently caught up.

A deployment may provide a strong current read through the active authority
group or an explicitly stale read from a known confirmed cursor. These modes
cannot be confused silently.

History compaction may expire an observer cursor. The response identifies the
expiration and offers a coherent state with a new cursor; removed history is
not recreated. Reconnection can redeliver identified events, so consumers
deduplicate by their stable identifiers.

## 7. Compatibility and migration

Before participating in authority, nodes prove compatibility with the world's
execution profile:

- kernel contract and state-format versions;
- simulation contract and deterministic execution profile;
- command, event, and ownership schemas;
- required provider and plugin identifiers, versions, and fingerprints;
- active limits and retention policies.

An incompatible node cannot vote, confirm mutations, or claim a current read.
Contract activation and kernel-owned migrations are coordinated at a confirmed
transition boundary. A migration failure leaves the previous compatible state
authoritative; mixed-version mutation is refused.

## 8. First stable evidence

Gap `K-07` requires executable evidence for:

- one-node operation through the same contracts;
- three nodes with quorum two;
- coordinator failure and replacement;
- minority partition remaining read-only;
- stale-authority fencing;
- node catch-up after interruption;
- local disk failure with and without remaining quorum;
- membership change;
- incompatible node or plugin profile refusal;
- coordinated migration and rollback to the prior authoritative state.

Until that evidence exists, every distributed behavior in this document is a
target rather than an implemented claim.
