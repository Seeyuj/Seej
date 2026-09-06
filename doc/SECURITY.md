# Security Policy

Security is a structural property of Seej's authority, persistence, provider,
and compatibility contracts. A corrupted canonical world is worse than an
explicitly unavailable world.

## Scope

The project security scope includes maintained Seej code and bundled
fundamental providers:

- pure transition and validation logic;
- local and distributed persistence;
- node identity, admission, membership, authority epochs, and replication;
- public command, observation, provider, and plugin boundaries;
- migrations, retention, backup, recovery, and operator tools;
- build and dependency controls maintained by the repository.

Community plugins, renderers, ingame clients, custom deployment systems, and
third-party services are not maintained by Seej unless explicitly adopted.
Vulnerabilities in Seej's isolation, permission, validation, or plugin boundary
remain in scope even when a third-party plugin triggers them.

Operators remain responsible for host security, network policy, credentials,
backup custody, approved plugins, and deployment-specific availability.

## Current and target security posture

The current repository is a local, single-writer prototype. It does not
implement distributed node authentication, consensus, generic plugin
isolation, or hot loading.

The first distributed reference profile trusts admitted operators and handles
crash faults, storage failures, and network partitions. It is not Byzantine or
permissionless. A malicious authorized kernel node is outside that first fault
model, while privilege bypass, unauthenticated admission, message tampering,
and stale-authority writes remain security defects.

The first stable release may execute only trusted providers explicitly approved
by the operator at startup. Isolated application-plugin execution is later.
Until it exists, native plugins have process-level privileges and must not be
described as sandboxed.

## Assets and invariants

Protect at least:

- one confirmed history and stable identity per world;
- command identity, order, status, and non-reuse after expiration;
- checkpoint, retained history, active obligations, and migration metadata;
- authority membership and monotonic fencing epochs;
- component manifests, versions, and artifact fingerprints;
- credentials, signing material, and transport keys;
- operator diagnostics that may contain sensitive world metadata.

Security-sensitive code must fail explicitly. Diagnostic logs are not a
canonical store and must not contain secrets.

## Trust boundaries

### Nodes

Discovery does not grant authority. Membership is an explicit confirmed
operation. Connections bind to an authenticated `node_id`, authorized world,
authority epoch, and compatible execution profile.

A stale or minority node cannot confirm mutations or claim a current read.
Messages are bounded, authenticated, replay-protected as required, and checked
against world and profile identity.

### Producers and observers

Every external input is untrusted. Authenticate and authorize as required by
world policy, enforce payload/cardinality limits, validate before canonical
execution, and avoid error details that expose secrets.

Observers receive only permitted domains and a confirmed cursor with freshness.
Read activity cannot mutate state or weaken simulation behavior.

### Providers and plugins

Installation does not imply permission. Manifests declare capabilities,
ownership, dependencies, limits, permissions, versions, migrations, and
fingerprints.

A plugin cannot directly mutate another owner's data. Opaque payloads remain
bounded and integrity-protected. Removal cannot discard or merely archive
active obligations while a world continues: they require a confirmed transfer
or resolution. Missing or mismatched required artifacts block activation,
migration, or re-simulation.

### Storage

Persistence errors fail closed. Full/read-only media, append, synchronization,
rename, and generic I/O failures cannot produce reported success or accepted
memory-only state. Torn-tail recovery is distinct from suspicious corruption
and operator repair.

Storage CRC and semantic state hashes are integrity tools, not peer
authentication or signatures.

## Security requirements

Contributions affecting a trust boundary must consider:

- authentication and authorization;
- least privilege and explicit permissions;
- input and resource bounds;
- denial of service and backpressure;
- duplicate, delayed, reordered, and replayed messages;
- command retry and timeout ambiguity;
- stale authority and split-brain resistance;
- storage failure and crash consistency;
- migration rollback and mixed-version refusal;
- dependency and artifact provenance;
- secret handling and diagnostic redaction.

Do not hardcode keys, tokens, credentials, or stable deployment identities.
CORS or network location is not authorization.

## Vulnerability reporting

Do not report a vulnerability through a public issue, discussion, pull request,
or other public channel.

Report privately by email:

**security@seej.org**

Include:

- a clear description and affected version or commit;
- impacted boundary and potential world-state consequences;
- reproduction steps or proof of concept when safe;
- whether confidentiality, integrity, authority, or availability is affected;
- suggested mitigation when available.

Examples include persistent-data loss or corruption, authority bypass,
unauthorized node admission, stale writes, command duplication, permission
escape, arbitrary code execution, secret disclosure, or denial of service that
can compromise durable state.

Maintainers will acknowledge, assess, coordinate a fix or mitigation, and
publish information when appropriate. No fixed response or correction time is
guaranteed.

Researchers should avoid accessing unrelated data, damaging worlds, retaining
secrets, or disclosing exploitable details before coordinated remediation.

## Supported versions and migrations

During the foundation phase, only the active development line is supported for
security fixes. After stable releases exist, the project will publish an
explicit supported-version policy.

The contract promise to migrate kernel-owned data from every stable format does
not promise indefinite security maintenance of every old executable. Operators
may need to perform supported stepwise migrations to reach a maintained
release.

Plugin security support and data migrations remain the plugin publisher's
responsibility. Seej refuses incompatible activation but cannot supply missing
third-party fixes or artifacts.

## Operator responsibilities

Operators must:

- secure hosts, accounts, credentials, and transport endpoints;
- authorize nodes and plugins using least privilege;
- retain recovery data and required historical plugin artifacts;
- test restore and migration procedures;
- monitor quorum, storage health, authority changes, and failed validation;
- keep maintained components updated;
- isolate native plugins at the deployment level until Seej sandboxing exists;
- follow the documented recovery process instead of silently repairing
  suspicious evidence.

## Related documents

- [Decision D-012](DECISIONS.md#d-012--distributed-extensible-world-kernel)
- [Node-network contract](../seej/docs/NODE_NETWORK.md)
- [Plugin contract](../seej/docs/PLUGIN_CONTRACT.md)
- [Persistence behavior](../seej/docs/phase1/PERSISTENCE.md)
- [Exit checklist](../seej/docs/phase1/EXIT_CHECKLIST.md)
- [Contribution guide](../CONTRIBUTING.md)
