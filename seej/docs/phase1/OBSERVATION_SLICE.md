# Confirmed Observation Slice

```text
status: TARGET — GAP K-02
```

Observation is the first public read surface of the kernel. It demonstrates
that a world can evolve, persist, replicate, and be understood without a
client owning its state.

## Contract boundary

An observer can:

- obtain a coherent confirmed state;
- subscribe to confirmed events after a cursor;
- inspect world identity, tick, execution profile, and freshness;
- recover from reconnect and cursor expiration;
- filter or project data within declared permissions.

An observer cannot:

- submit mutations through the read surface;
- read unconfirmed local proposals as canonical;
- change tick rate, ordering, fidelity, or rules by subscribing;
- trigger canonical materialization through camera or query activity;
- bypass plugin ownership or data permissions.

## Initial state response

A coherent initial response identifies at least:

- `world_id`;
- confirmed tick and history cursor;
- authority term, configuration version, and freshness classification;
- execution-profile identifier;
- selected readable owner domains or projection;
- a replacement stream cursor.

Opaque domains can be included as authorized envelopes without promising
field-level interpretation.

## Event stream

Events after the cursor are ordered by confirmed world position and have stable
identifiers. Reconnection may redeliver events, so consumers deduplicate.
Delivery is not advertised as exactly once.

A node serves one of two explicit read classes:

- **current confirmed:** backed by a quorum read barrier in the active term and
  configuration, with the serving node applied through the barrier cursor;
- **known confirmed but stale:** tied to a confirmed cursor with freshness
  marked stale or unknown because current authority was not proved.

The API never silently presents a raw local WAL tail as confirmed.

## Retention and resynchronization

If the requested cursor remains retained, streaming resumes after it. If it
expired:

1. return a typed cursor-expired result;
2. provide or reference a coherent confirmed state and cursor from the same
   cut;
3. retain that cut through stream handoff, or restart explicitly from a newer
   replacement cut;
4. state that removed events are unavailable.

This behavior preserves usefulness without promising permanent history.

## Neutrality and overload

Observer identity, number, latency, filters, and region of interest have no
canonical effect. A node applies backpressure, rate limits, disconnection, or
request refusal before changing world rules or resolution.

If a world requests real-time cadence and observation competes for resources,
observation yields. The world may still miss its cadence under total overload;
it cannot alter canonical semantics to satisfy readers.

## Security and limits

The read surface authenticates the caller when the world policy requires it,
authorizes domains and projections, bounds response and stream sizes, and does
not expose operator secrets or opaque payloads without permission.

Network framing for this surface is separate from ingame netcode. It may share
transport libraries with the node-network implementation but has a distinct
contract and authority.

## Deterministic fixture

Gap K-03 supplies fixed genesis, commands, expected events, confirmed cursors,
and final semantic hash. The fixture includes:

- an empty-world case;
- extension-owned data without spatial fields;
- a cross-owner transaction;
- a deterministic rejected command;
- enough confirmed events to exercise reconnect and compaction.

Assertions use fixed expected outcomes. They do not depend on a statistically
likely event from random population behavior.

## External consumer evidence

K-02 requires a consumer using the observation contract without internal Rust
state. K-04 separately requires a third-party **write extension** that declares
data and commands. An observer does not close K-04.

## Acceptance scenarios

- opening and closing an observer leaves the semantic state hash unchanged;
- a slow observer is limited without delaying or altering canonical order;
- reconnecting after a response loss produces no semantic duplicates;
- a stale replica announces its freshness;
- a minority partition cannot claim a current read;
- an expired cursor resynchronizes from coherent state;
- compaction advancing during resynchronization cannot create a silent event
  gap;
- unauthorized owner data is omitted or refused;
- events are published only after distributed confirmation.

Until those scenarios have executable evidence, this document is a target
contract rather than an implemented network API.
