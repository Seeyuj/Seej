# Contribution Guide

Thank you for your interest in this project.

Before any contribution (code, documentation, ideas, issues, discussions), please read the [`README.md`](README.md) and ensure that your proposal aligns with the repository's objective: **providing a simulation core for persistent sandbox worlds**, maintainable in the long term.

This repository is neither a video game, nor a graphics engine, nor a narrative RPG. The project's priority is **systemic coherence**, **core stability**, **real persistence** of the simulated world, and **maintainability over several years**.

Some contributions, even technically correct, may be rejected if they do not serve the project's vision and architecture.

## Navigation

- [`README.md`](README.md)
- [`ARCHITECTURE.md`](doc/ARCHITECTURE.md)
- [`DECISIONS.md`](doc/DECISIONS.md)
- [`ROADMAP.md`](doc/ROADMAP.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](doc/SECURITY.md)
- [`CODE_OF_CONDUCT.md`](doc/CODE_OF_CONDUCT.md)

## Table of Contents

- [Before Contributing](#before-contributing)
- [Security (Responsible Disclosure)](#security-responsible-disclosure)
- [Project Philosophy and Principles](#project-philosophy-and-principles)
  - [Simulation Before Narration](#simulation-before-narration)
  - [Autonomous World Not Centered on the Player](#autonomous-world-not-centered-on-the-player)
  - [Authoritative Server and Real Persistence](#authoritative-server-and-real-persistence)
  - [Pragmatic Deterministic and Explainable AI](#pragmatic-deterministic-and-explainable-ai)
  - [Strict Modularity and Controlled Extensibility](#strict-modularity-and-controlled-extensibility)
  - [Strict Separation Between Simulation and Rendering](#strict-separation-between-simulation-and-rendering)
  - [Platform Before Content](#platform-before-content)
- [Accepted Contribution Types](#accepted-contribution-types)
- [Good First Contributions](#good-first-contributions)
- [Explicitly Rejected Contributions](#explicitly-rejected-contributions)
- [Architecture Technical Rules and Constraints](#architecture-technical-rules-and-constraints)
- [Contribution Workflow](#contribution-workflow)
- [Validation Process and Governance](#validation-process-and-governance)
- [Code of Conduct and Communication](#code-of-conduct-and-communication)

## Before Contributing

By contributing to this repository, you accept that:

- **simulation** and **persistence** take precedence over narration or rendering;
- the **server** remains the sole authority on world state;
- systems must be **explainable**, **deterministic**, and **decoupled**;
- project governance prioritizes **coherence** over speed.

If these principles do not match your expectations or approach, this project is probably not suited to your contribution — and this choice is perfectly legitimate.

To maximize acceptance chances:

- **For a significant modification** (new feature, core change, major refactoring), first open an issue or discussion. *(Exception: security vulnerabilities — see `SECURITY.md`.)*
- **One Pull Request = one objective**: avoid "catch-all" changes.
- **Explain the why**: context and systemic impact matter as much as code.

## Security (Responsible Disclosure)

Security vulnerabilities **must not** be reported via public issues, discussions, or pull requests.

Please follow the procedure and reporting channel described in [`SECURITY.md`](doc/SECURITY.md).

## Project Philosophy and Principles

This platform is based on a set of structural principles that guide all technical, architectural, and organizational decisions. Any contribution is evaluated against these principles.

### Simulation Before Narration

The project's heart is a **persistent simulation engine**.

The world exists, evolves, and transforms independently of any human or player presence. No scenario, no imposed narration, no predefined "path" structures the world's functioning.

Stories, narratives, and situations emerge exclusively from simulated systems (time, resources, entities, conflicts, interactions).

Any contribution introducing imposed, linear, or player-centered narrative logic will be rejected.

### Autonomous World Not Centered on the Player

The player is never a central, indispensable, or privileged entity.

The world:

- continues to evolve in the absence of players;
- does not pause;
- does not artificially adapt to human presence.

Players, like NPCs, are actors among others, subject to the same systemic rules.

Any contribution assuming a "hero", a unique role, or special treatment of the player is incompatible with this project.

### Authoritative Server and Real Persistence

The server is the sole authority on world state.

This implies:

- no critical logic must be executed on the client side;
- the client is a consumer of world state, never a decision-maker;
- persistence is ensured on disk, explicitly and traceably.

Mechanisms relying on volatile, temporary, or implicitly recomputed states are prohibited for the simulation core.

### Pragmatic Deterministic and Explainable AI

Non-player entities are **agents** defined by:

- needs;
- objectives;
- memory;
- explainable decision rules.

The project does not aim to create "magical" or opaque AI. Behaviors must be reproducible, observable, and debuggable.

Probabilistic or generative approaches may exist **on the periphery**, but never at the heart of persistent simulation.

### Strict Modularity and Controlled Extensibility

The project core must remain:

- minimal;
- stable;
- decoupled;
- maintainable in the long term.

Any non-essential feature must be implemented as an **optional module**, clearly isolated from the simulation core.

Modules:

- use public and versioned APIs;
- do not bypass the core;
- can be activated, deactivated, or replaced without compromising the world.

### Strict Separation Between Simulation and Rendering

World simulation is totally independent of any rendering technology.

The project may define a reference graphics standard, but this standard:

- never dictates simulation rules;
- introduces no graphics dependency in the server;
- remains a consumer of the simulated world.

Any contribution directly coupling simulation logic to a graphics engine will be rejected.

### Platform Before Content

The project provides **systems**, **rules**, and **tools**, not ready-made experiences.

Content (worlds, factions, scripts, assets, specific rules) is left to servers, creators, and communities.

Contributions adding "default" content to the detriment of platform robustness are not prioritized.

## Accepted Contribution Types

The project accepts and encourages contributions that strengthen the platform's stability, coherence, and sustainability.

## Good First Contributions

If you want to contribute quickly with high acceptance probability, start with small, verifiable work directly aligned with Phase 1:

- replay validation tests (snapshot cursor + WAL boundary cases);
- persistence documentation improvements (snapshot/WAL/recovery behavior);
- CLI inspection improvements (`sy_cli status`, `events`, `dump`, `entities`, `zones`);
- burn-in scenarios and reproducibility checks;
- structured logging and diagnostic clarity improvements.

These contributions are valuable because they strengthen determinism, persistence confidence, and operational observability without changing the architectural direction.

### Bug Fixes

Any fix improving:

- server stability;
- simulation coherence;
- persistence reliability;
- behavior reproducibility;

is considered a priority.

Fixes must be accompanied by a clear description of the problem, its systemic impact, and, when possible, a test reproducing the case.

### Performance and Scalability Improvements

Contributions on:

- CPU / memory consumption;
- region or chunk simulation management;
- server scaling;
- disk access and persistence optimization;

are strongly encouraged.

Any optimization must prioritize readability, maintainability, and stability over an isolated marginal gain.

### Technical Documentation

Documentation is a contribution in its own right.

Particularly encouraged:

- architecture documentation;
- simulation flow diagrams;
- system invariant descriptions;
- server deployment guides;
- public API and module documentation.

Clear documentary contribution is often more valuable than an additional feature.

### Tests Validation and Observability

The project values contributions related to:

- unit and integration tests;
- persistence validation tools;
- replay or deterministic simulation mechanisms;
- metrics, structured logs, and diagnostic tools.

Any improvement facilitating debugging of a persistent world is considered strategic.

### Optional Modules and Extensions

Features non-essential to the core may be proposed as **optional modules**, provided they:

- strictly respect public APIs;
- do not introduce a dependency toward the core;
- remain deactivatable without impact on simulation.

Experimental modules are accepted as long as they remain clearly identified as such.

### Development and Operational Tools

Tools facilitating:

- local development;
- profiling;
- server monitoring;
- persistent world management;
- administration and maintenance;

are welcome contributions, even if not directly visible on the client side.

## Explicitly Rejected Contributions

To preserve the platform's coherence, maintainability, and long-term viability, certain categories of contributions are explicitly rejected.

### Game-Oriented or Immediate Fun Features

Contributions primarily aiming to:

- improve short-term game experience;
- add isolated gameplay mechanics;
- enrich content without systemic impact;

are not accepted in the project core.

The project provides systems, not ready-made game experiences.

### Imposed Narration or Scripted Logic

Any contribution introducing:

- linear quests;
- mandatory scripted events;
- imposed narrative progression;

is incompatible with the principle of emergent narration.

Stories must stem from simulated systems, never precede them.

### Player Centrality or Special Treatment

Contributions assuming:

- a unique or privileged player role;
- specific rules applicable only to players;
- artificial world adaptation to human presence;

are rejected.

The player is an actor among others, subject to the same rules as simulated entities.

### Authoritative Client Logic or Coupled to Simulation

Any critical logic executed on the client side is prohibited.

This notably includes:

- simulation decisions made on the client side;
- calculations impacting the world's persistent state;
- direct dependency between client and server core.

The client is never a source of truth.

### Opaque Non-Deterministic or Magical AI

Contributions relying on:

- unexplainable decisions;
- non-reproducible models;
- strong dependency on external generative systems;

are not accepted in the simulation core.

Behaviors must be understandable, observable, and reproducible.

### Strong Coupling with a Specific Technology or Engine

Any contribution:

- linking the simulation core to a graphics engine;
- introducing an unjustified dependency on a closed technology;
- preventing headless server execution;

will be rejected.

The project must remain independent of any particular client or engine.

### Adding Default Content to the Core

The core is not intended to contain:

- predefined worlds;
- "official" factions;
- imposed game rules;
- mandatory graphical assets.

Content belongs to servers, not the platform.

## Architecture Technical Rules and Constraints

Any contribution must respect the project's overall architecture and technical constraints, designed to guarantee system stability, persistence, and evolutivity in the long term.

### Strict Separation Between Core and Extensions

The project is structured around:

- a **minimal and stable simulation core**;
- **optional modules and extensions**.

The core:

- contains exclusively mechanisms essential to persistent simulation;
- exposes public, documented, and versioned APIs;
- depends on no external module.

Any non-strictly essential feature must be implemented as a module.

### API Stability and Breaking Change Management

The core's public APIs are considered **stable**.

Consequently:

- any incompatible modification must be explicitly justified;
- no breaking change will be accepted without prior discussion;
- impacts on existing modules must be clearly documented.

Backward compatibility is a priority.

### Determinism and Reproducibility

Core systems must be:

- deterministic given equal inputs;
- reproducible over time;
- observable and debuggable.

Any logic introducing non-reproducible behaviors must be isolated, documented, and justified.

### Explicit and Traceable Persistence

Persistent world data must:

- be explicitly written to disk;
- be replayable or inspectable;
- not depend on implicit or transient states.

Persistence mechanisms must be designed to survive restarts, crashes, and version migrations.

### Mandatory Headless Server Execution

The server must be able to run:

- without a graphical interface;
- without client dependency;
- without a rendering engine.

Any contribution introducing a direct or indirect graphics dependency on the server side will be rejected.

### Readability Simplicity and Maintainability

Code readability is a functional requirement.

Contributions must prioritize:

- simple abstractions;
- minimal dependencies;
- code understandable without implicit context.

"Smart" but hard-to-maintain code will be rejected in favor of a simpler and more robust solution.

## Contribution Workflow

Any contribution to the project follows a standardized process to ensure quality, coherence, and traceability of changes.

### Recommended Prior Discussion

For any significant contribution (new feature, core modification, major refactoring), it is strongly recommended to open an issue or discussion before writing code.

This allows:

- verifying alignment with project philosophy;
- avoiding unnecessary work;
- anticipating systemic impacts.

### Fork and Branches

Contributions are made via a fork of the main repository.

Branches must follow a clear convention:

- `fix/short-description`
- `feature/short-description`
- `doc/short-description`
- `refactor/short-description`

Each branch must be limited to a specific objective.

### Commits

Commits must:

- be atomic;
- have a clear and descriptive message;
- explain the *why* of the change, not just the *what*.

Corrective or temporary commits must be cleaned before Pull Request submission.

### Pull Requests

Any contribution goes through a Pull Request.

The Pull Request must include:

- a description of the problem or objective;
- the systemic justification for the change;
- impacts on simulation, persistence, and compatibility;
- tests added or adapted, if applicable.

Incomplete or insufficiently justified Pull Requests may be closed without thorough review.

### Review and Iterations

Contributions are reviewed by project maintainers.

Modifications or clarifications may be requested. Prolonged lack of response may result in Pull Request closure.

Review covers architecture as much as the code itself.

## Validation Process and Governance

This project is open-source, but it is not without governance.

Technical and architectural decisions are made in the project's long-term interest, not based on contribution volume or proposal popularity.

### Maintainers and Responsibility

The project is led by a small group of maintainers responsible for the core and overall vision.

Maintainers:

- define and protect the project's architecture;
- validate or reject contributions;
- guarantee the platform's long-term coherence.

Core responsibility cannot be implicitly delegated by a Pull Request.

### Contribution Validation Criteria

Contributions are evaluated according to the following criteria:

- alignment with project philosophy;
- impact on stability and persistence;
- code clarity and maintainability;
- compatibility with existing architecture;
- long-term systemic benefit.

A technically correct contribution may be rejected if it does not meet these criteria.

### Rejection Disagreements and Arbitration

Rejecting a contribution is a normal decision and part of the governance process.

Maintainers may reject a contribution:

- without obligation to propose an alternative;
- without entering into a prolonged debate;
- without exhaustive justification beyond established principles.

Disagreements must remain technical, argued, and respectful.

### Project Evolution

The project's vision and principles may evolve, but only:

- collectively;
- via structured discussions;
- with clearly measured impact on existing work.

No isolated Pull Request can alone modify the project's foundations.

## Code of Conduct and Communication

The project aims to maintain a healthy, respectful collaboration environment oriented toward constructive technical exchanges.

### Respect and Professionalism

All interactions (issues, discussions, code reviews, comments) must remain:

- respectful;
- factual;
- focused on technical content.

Personal attacks, value judgments, and aggressive behaviors are not tolerated.

### Technical Disagreements

Disagreements are normal and expected in an infrastructure project.

They must:

- be technically argued;
- rely on facts, measurements, or established principles;
- avoid any personalization of the debate.

Disagreement never implies questioning intentions or skills.

### Communication and Expectations

Maintainers contribute on their time and according to their priorities.

There is:

- no obligation for immediate response;
- no guarantee of contribution acceptance;
- no commitment to a public roadmap.

Any pressure, insistence, or attempt to force a decision is contrary to the project's spirit.

### Enforcement

Maintainers reserve the right to:

- moderate discussions;
- close issues or Pull Requests;
- limit or suspend access to contributions;

when a participant's behavior harms the project's proper functioning.
