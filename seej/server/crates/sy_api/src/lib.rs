//! # `sy_api` — Draft v1 Public Contract Boundary
//!
//! **Status:** empty Phase 0 clean-slate skeleton (D-015; contracts are
//! `DRAFT — NOT FROZEN` until `K-01`).
//!
//! `sy_api` depends strictly on `sy_types` (zero external crates). It defines
//! the target v1 public plugin, command, and observation boundary specified in
//! `seej/docs/CONTRACT.md` (`0.4.0-draft`) and `seej/docs/PLUGIN_CONTRACT.md`
//! (`0.3.0-draft`).
//!
//! ## Enforced Invariants
//!
//! - Zero `unsafe` code (`#![forbid(unsafe_code)]`).
//! - Zero floating-point math (`#![deny(clippy::float_arithmetic)]` and purity
//!   gate).
//! - Every public item documented (`#![deny(missing_docs)]`).
//! - Pure and headless: no I/O, wall-clock time, OS randomness, or threads.
//!
//! Authoritative phase ordering and acceptance criteria live solely in
//! `doc/ROADMAP.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]
