//! # `sy_types` — Stable Primitives and Canonical Identifiers
//!
//! **Status:** empty Phase 0 clean-slate skeleton (D-015).
//!
//! `sy_types` sits at the bottom of the workspace dependency DAG with **zero
//! dependencies** (normal, dev, or build). It will hold pure, domain-agnostic
//! value types shared across the kernel: decoupled world/node identifiers,
//! checked fail-closed canonical counters, logical table addressing, typed
//! digests, distinct cursors, and versioned profile/genesis DTOs.
//!
//! ## Enforced Invariants
//!
//! - Zero `unsafe` code (`#![forbid(unsafe_code)]`).
//! - Zero floating-point math (`#![deny(clippy::float_arithmetic)]` and purity
//!   gate blocking `f32`/`f64`).
//! - Every public item documented (`#![deny(missing_docs)]`).
//! - Zero application ontology (no space, entities, zones, economies, or UI).
//!
//! Authoritative phase ordering and acceptance criteria live solely in
//! `doc/ROADMAP.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]
