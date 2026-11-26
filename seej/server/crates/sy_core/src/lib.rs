//! # `sy_core` — Pure Deterministic Transition Engine
//!
//! **Status:** empty Phase 0 clean-slate skeleton (D-015).
//!
//! `sy_core` is the pure, headless, deterministic heart of Seej. It depends
//! **strictly on `sy_types` and `sy_api`** (zero external crates across normal,
//! dev, and build dependencies) and is checked on every PR by
//! `scripts/check_sy_core_purity.py` and `scripts/check_dag.py`.
//!
//! ## Enforced Invariants
//!
//! - Zero `unsafe` code (`#![forbid(unsafe_code)]`).
//! - Zero floating-point types or math (`#![deny(clippy::float_arithmetic)]`
//!   and purity gate blocking `f32`/`f64`).
//! - Zero wall-clock time, environment access, filesystem/I/O, networking,
//!   OS randomness, OS threads, or randomized hash collections.
//! - Every public item documented (`#![deny(missing_docs)]`).
//!
//! Authoritative phase ordering and acceptance criteria live solely in
//! `doc/ROADMAP.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]
