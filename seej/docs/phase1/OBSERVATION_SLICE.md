# Observation Slice — the First Userland Brick (K-02, K-03)

## Status

Design specification for Gate 1-K gaps `K-02` (observation surface), `K-03`
(anchor world), and the exit criterion `K-04` (first external consumer). See
[`EXIT_CHECKLIST.md`](EXIT_CHECKLIST.md). Nothing in this document is
implemented until its checklist gap is checked.

## Why this brick is first

The kernel becomes a platform the day someone else can consume a world
**without reading the Rust source**. Among all possible bricks (observation,
agents, human netcode), observation is first because it is:

- genre-neutral: every consumer (renderer, dashboard, bot, analysis script)
  starts by watching;
- read-only: it cannot violate causal closure, so it needs no new kernel
  invariants beyond a concurrent-read policy (P1-11);
- nearly free: everything it emits already exists in the WAL;
- the proof artifact: a world evolving in front of an external tool is the
  demonstration that the "world exists without players" claim is real.

Human-facing netcode is explicitly the **third** brick (after an
agents/command-injection brick), and remains out of Phase 1 scope.

## K-02 — Observation surface

### Deliverable

A follow-mode, machine-readable event stream over an existing world:

```bash
sy_cli tail <world_id> [--from-event <id> | --from-tick <t>] [--follow] --json
```

### Output contract

- **JSON Lines**: exactly one `SimEvent` per line, matching the shapes of
  [`../CONTRACT.md`](../CONTRACT.md) section 6 (`{ "event_id": ..., "tick":
  ..., "data": { ... } }`). No wrapper object, no progress decoration on
  stdout; diagnostics go to stderr.
- Events are emitted in `event_id` order, without gaps, starting from the
  requested cursor (default: the current durable tail when `--follow`, else
  `event_id 1`).
- The stream is **read-only evidence**: the command never creates, repairs,
  truncates, or locks-for-write any world file.

### Concurrent-read policy (implements P1-11 for this path)

While `server_d` owns the world (single writer, P0-10):

- The reader opens the WAL in read-only mode (`open_read_only` semantics —
  never the repairing open).
- On reaching a torn or incomplete tail record, the reader treats it as
  **end-of-stream-for-now**: it does not report corruption, does not repair,
  and (in `--follow` mode) retries from the same offset after a poll
  interval.
- A CRC failure on a *non-tail* record is reported as suspected corruption
  and terminates the stream with a nonzero exit code: durable evidence
  disagreeing mid-file is never skipped over silently.
- Poll interval and reopen strategy are implementation details (out of
  contract); the visible guarantee is: every durably committed event is
  eventually emitted exactly once, in order.

### Acceptance criteria (evidence for closing K-02)

1. A world running under `server_d` can be tailed concurrently by `sy_cli`
   from another process; the tail emits every committed event exactly once,
   in `event_id` order.
2. A test appends a deliberately torn tail record and proves the reader
   pauses (follow mode) or exits cleanly at the tail (non-follow), without
   modifying the file (byte-identical before/after).
3. Consuming the stream requires no Seej code: a documented example pipes the
   output through a generic JSON tool (e.g. `jq`) to answer one question
   ("how many entities died this run?").
4. Two consumers tailing the same world receive identical streams.

## K-03 — Anchor world

### Purpose

The observation surface needs something worth observing. The anchor world is
a documented, reproducible genesis whose event stream shows visible systemic
life using only the Phase 1 rules — no new gameplay, no new scope.

### Definition

- Genesis: `server_d create --name <name> --seed <seed> --resources 10
  --creatures 5` (population recorded as part of genesis once P0-06 lands;
  until then, the creation command line is the documented genesis).
- Run: `server_d run --world <world_id> --ticks 1000 --save-interval 100`.
- Under the v0 systemic rules (1%/tick resource depletion, 0.5%/tick creature
  degradation, dead cleanup every 100 ticks), a 1000-tick run over this
  population is statistically guaranteed to emit systemic events
  (`ResourceDepleted`, `EntityDegraded`, `EntityStateChanged`,
  `EntityDespawned`) — the stream visibly changes without any human input.

### Acceptance criteria (evidence for closing K-03)

1. The scenario above is documented (this file) and reproducible with two
   commands.
2. **Stream-level determinism:** two fresh runs from the same genesis produce
   byte-identical event streams (same events, same order, same payloads),
   proven by a test or scripted comparison. This is the observable form of
   the Tier 2 guarantee.
3. An observer following the stream for the 1000-tick run sees systemic
   events without issuing any command.

## K-04 — First external consumer

The exit criterion of a kernel: at least one consumer of the observation
surface or contract **not maintained by the kernel author** — a terminal
visualizer, a Discord bot, a plotting script, anything. Closing evidence is a
link to the consumer plus the contract surface it uses. This item cannot be
closed by writing code in this repository; that is the point.

## Out of scope for this slice

- Any network transport, subscription protocol, or push mechanism
  (`sy_protocol` remains Phase 2+).
- Any write path for consumers (command injection is the second brick and
  requires P0-08/P0-09 first).
- Any UI, rendering, or graphical dependency in this repository.
- Filtering/query languages beyond the cursor flags above (consumers filter
  downstream; that is what JSON Lines is for).
