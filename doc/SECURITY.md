# Security Policy

Security is a structural property of Seej's authority, persistence, plugin, and
compatibility contracts. A silently corrupted world is worse than an explicitly
unavailable world.

## Scope

In scope: maintained Seej kernel code, storage/consensus adapters, CLI/operator
tools, public contract boundaries, and repository build/supply-chain controls.
Vulnerabilities in Seej's validation, permission, or isolation boundaries remain
in scope even when triggered by a third-party plugin.

Out of scope unless adopted: community plugins, renderers, ingame netcode, and
operator host infrastructure.

## Current and Target Security Posture

- **Current repository state (`seej/server/` — Phase 0 clean slate):**
  empty 4-crate Rust workspace skeleton (`sy_types`, `sy_api`, `sy_core`,
  `sy_infra`) with active build/purity/supply-chain guardrails
  (`check_sy_core_purity.py` across all 3 pure crates, `check_dag.py` across
  normal/dev/build deps, `check_docs.py`, `cargo-audit`, and `cargo deny check
  advisories licenses bans sources`). No runtime daemon, storage engine,
  consensus, authentication, or plugin isolation exists yet.
- **Target v1 (Gate 1-K):** crash-fault-tolerant (CFT) authority groups of 1 or
  3 operator-authenticated nodes (quorum 2). Byzantine authorized nodes are
  outside the v1 fault model; unauthenticated admission, privilege bypass,
  message tampering, and stale-authority writes are security defects.
- **Startup plugins in v1:** trusted, operator-approved native components loaded
  at startup with manifest/fingerprint verification. They run with process
  privileges and must never be described as sandboxed.
- **Later (Gate 1-D / Evolution 2):** isolated application-plugin execution
  (e.g. WASM with deterministic fuel metering and explicit permissions).

## Assets and Invariants

Protect at minimum:

- one confirmed history and stable `world_id` per world;
- command identity, bounded producer slots/incarnations/generations, exact
  expiry, and non-reuse of expired IDs;
- authorization, idempotency, and the **reserved control budget** of kernel
  control operations (`CancelCommand`) so application quota exhaustion cannot
  cause permanent denial of service;
- checkpoints, retained history, active obligations, and migration barriers;
- authority terms, confirmed membership configurations, and component
  fingerprints;
- transport credentials, signing keys, and redacted diagnostics (never log
  secrets).

## Trust Boundaries

- **Nodes:** discovery never grants membership. Inter-node messages and
  connections must be bounded, authenticated, and replay-protected, binding an
  authenticated `node_id`, world, authority term, configuration version, and
  compatible execution profile. Stale or minority nodes cannot confirm mutations
  or claim current reads.
- **Producers & observers:** all external input is untrusted. Authenticate,
  enforce payload/cardinality limits, and validate before pure execution.
  Observers receive only permitted domains and confirmed prefixes with explicit
  freshness; read load triggers rate limiting/refusal before affecting
  canonical simulation.
- **Plugins:** installation does not imply permission. Plugins cannot mutate
  another owner's tables directly, cannot drop active obligations, and cannot
  run with mismatched artifacts/fingerprints.
- **Storage:** fail closed on full/read-only media, append, sync, rename, or
  generic I/O errors. Separate safe torn-tail recovery from corruption and
  quarantine; never silently repair suspicious evidence. Storage CRCs and
  semantic state roots are integrity checks, not peer authentication.

## Vulnerability Reporting

Do **not** report security vulnerabilities through public issues, discussions,
or pull requests.

Report privately to: **security@seej.org**

Include affected commit/version, impacted boundary, potential world-state
impact, safe reproduction steps, and suggested mitigation if available.
Maintainers will acknowledge, assess, and coordinate remediation; no fixed SLA
is guaranteed.

## Operator Responsibilities

Operators secure hosts, credentials, transport endpoints, backup custody,
approved plugin lists, and archived historical artifacts required for
re-simulation. Until plugin isolation exists, isolate native plugins at the OS
or container level and follow documented recovery procedures rather than
manually editing canonical store files.

## Related Documents

- [doc/DECISIONS.md](DECISIONS.md)
- [doc/ARCHITECTURE.md](ARCHITECTURE.md)
- [doc/ROADMAP.md](ROADMAP.md)
- [seej/docs/CONTRACT.md](../seej/docs/CONTRACT.md)
- [seej/docs/PLUGIN_CONTRACT.md](../seej/docs/PLUGIN_CONTRACT.md)
- [seej/docs/BENCHMARKS.md](../seej/docs/BENCHMARKS.md)
