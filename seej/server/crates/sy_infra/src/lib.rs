//! # `sy_infra` — Persistence, Authority, Execution, and Observation Adapters
//!
//! **Status:** empty Phase 0 clean-slate skeleton (D-015).
//!
//! `sy_infra` will host all real I/O and runtime infrastructure outside the
//! pure engine (`sy_core`): durable local storage, fail-closed WAL and
//! single-world authority replication, batched execution, single-threaded
//! Deterministic Simulation Testing (DST), confirmed observation, and backup
//! procedures.
//!
//! ## Enforced Invariants
//!
//! - Every public item documented (`#![deny(missing_docs)]`).
//! - External crates are added only with explicit approval when their
//!   implementing phase starts and must pass `deny.toml` license/advisory/ban
//!   checks.
//!
//! Authoritative phase ordering and acceptance criteria live solely in
//! `doc/ROADMAP.md`.

#![deny(missing_docs)]
