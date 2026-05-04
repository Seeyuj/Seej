# Security Policy

## Purpose

This document defines the security policy for the **Seej** project.

## Navigation

- [`README.md`](../README.md)
- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`DECISIONS.md`](DECISIONS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)

The objectives of this policy are to:

- protect the integrity of the **simulation core**;
- guarantee the **coherence and persistence** of simulated worlds;
- frame the **responsible disclosure** of vulnerabilities;
- clarify the **scope of responsibility** of the project.

Security is considered a **structural requirement**, on par with persistence, determinism, and maintainability.

---

## Security Scope

### Included in Scope

The security policy covers:

- the **server simulation core**;
- **disk persistence** mechanisms;
- public APIs exposed by the server;
- world state, entity, and rule management;
- official tools provided with the core (CLI, scripts, server utilities).

### Out of Scope

**Not** covered by this policy:

- graphical clients (Unreal, Godot, Web, etc.);
- third-party modules or extensions;
- specific rules defined by a community server;
- custom deployments (cloud, bare metal, containers);
- content, scripts, or assets produced by third parties.

Each world operator is responsible for the security of their execution environment.

---

## Threat Model (Principles)

The project is based on the following assumptions:

- the **server is authoritative** and trusts no client;
- the client is considered **potentially hostile**;
- all incoming data is **untrusted by default**;
- persistence is a **critical asset** (loss or corruption = major incident).

Contributions must respect these principles and never introduce logic that weakens this model.

---

## Expected Security Best Practices

Contributors are required to follow these rules:
These requirements complement those described in `doc/CONTRIBUTING.md`.

### Server Side

- strict input validation;
- absence of critical logic on the client side;
- clear separation between persistent and transient data;
- refusal of any uncontrolled network dependency;
- no hardcoded secrets (tokens, keys, identifiers).

### Code and Architecture

- code readability and auditability;
- determinism of critical systems;
- clear logging of errors impacting world state;
- explicit failure handling (I/O, corruption, invalid states).

Complex but opaque code will be rejected, even if functional.

---

## Vulnerability Reporting

### ⚠️ Do Not Open a Public Issue

Security vulnerabilities **must not** be reported via public issues, discussions, or pull requests.

This includes notably:

- corruption or loss of persistent data;
- privilege escalation;
- bypass of server authority;
- arbitrary code execution;
- denial of service impacting persistent simulation.

### Reporting Channel

Please report any vulnerability responsibly via:

**Email:** `security@seej.org`

For general contribution rules (issues, PR, workflow), see `doc/CONTRIBUTING.md` — except for vulnerabilities, which must follow this document.

The message must include:

- a clear description of the problem;
- the impacted scope;
- reproduction steps if possible;
- potential impact on simulation or persistence.

---

## Processing Process

Maintainers commit to:

1. acknowledge receipt of the report;
2. analyze impact and severity;
3. propose a fix or mitigation;
4. publish a correction within a reasonable timeframe.

No correction date is guaranteed, but critical vulnerabilities are treated as a priority.

---

## Responsible Disclosure

Contributors and security researchers commit to:

- not exploit the vulnerability publicly;
- not disclose details before correction;
- cooperate with maintainers if necessary.

Any premature disclosure or voluntary public exploit may result in exclusion from the project.

---

## Supported Versions

As the project is in the foundation phase:

- only the **active main branch** is supported;
- no security backward compatibility is guaranteed on obsolete versions;
- fixes are applied to the current development version.

World operators are responsible for keeping their instance up to date.

---

## Server Operator Responsibility

Server administrators must:

- secure their operating system;
- control network access;
- manage offline backups;
- audit third-party modules used;
- monitor logs and metrics.

The project cannot be held responsible for an incident related to poor configuration or an external module.

---

## Philosophy

The project's security rests on a simple principle:

> **A corrupted persistent world is worse than an unavailable world.**

Any security decision will prioritize:
- data integrity;
- simulation coherence;
- long-term robustness.

Security is not an optional feature.
