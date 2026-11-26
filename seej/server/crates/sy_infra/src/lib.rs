//! # `sy_infra` — Persistence, Authority, Execution & Observation Adapters
//!
//! Architectural baseline: **D-015** (`doc/DECISIONS.md`).
//!
//! `sy_infra` implements the three internal seams defined in `sy_api`
//! (`StateStore`, `AuthorityLog`, `Executor`) and wires them to the pure
//! transition engine (`sy_core`). All real I/O (filesystem, durable fsync,
//! atomic rename, networking, process locks, structured logging) lives here
//! and never leaks into `sy_core`.
//!
//! ## Ordered Implementation Plan — Phases 1.4 through 3
//!
//! 1. **Phase 1.4 — Durable Local `StateStore`, Fail-Closed WAL & Checkpoints:**
//!    - Partitioned durable `StateStore` (benchmarking candidate engines such
//!      as `redb`, `Fjall`, or a custom paged store against real workloads).
//!    - Fail-closed binary WAL with explicit torn-tail vs. corruption and
//!      quarantine modes (`P0-13`, `P1-18`) plus fresh `cargo-fuzz` targets.
//!    - Tick-bound checkpoints (`P0-02`), artifact/manifest/checkpoint
//!      integrity binding (`P1-09`), durable world creation (`P1-10`), and
//!      common I/O fault injection (`P0-14`).
//! 2. **Phase 1.5 — Replay Oracle, Golden Vectors & Safe Inspection:**
//!    - Independent replay/re-simulation oracle comparing semantic roots
//!      (`P0-15`), golden compatibility fixtures (`P1-05`), and non-mutating
//!      read-only local inspection rejecting storage-aliasing export paths
//!      (`P1-11`).
//! 3. **Phase 2.1 — Internal Seam Doubles & Single-Threaded DST (`K-05`):**
//!    - In-memory and fault-injecting `StateStore`, `AuthorityLog`, network,
//!      and clock doubles powering seeded single-threaded Deterministic
//!      Simulation Testing (DST).
//! 4. **Phase 2.3–2.4 — Single-World Authority & Replication (`K-07`, `P0-10`,
//!    `P0-11`, `P1-12`, `P1-17`):**
//!    - Exclusive local writer lock, persistent `authority_term`/`vote`/
//!      `config_version`, 1-node and 3-node (quorum 2) Raft-style `TickCommit`
//!      group commit, joint consensus membership transitions, quorum read
//!      barriers, and multi-process adversarial crash/partition tests.
//! 5. **Phase 3 — Observation, Migrations, Compaction & Backup (`K-02`,
//!    `K-09`, `K-10`, `P1-19`, `P2-02`, `P2-03`):**
//!    - Confirmed observation streams with freshness and pinned cursor resync;
//!      5-phase coordinated migrations committing a mandatory boundary
//!      checkpoint on `MigrationActivate`; retention-aware compaction; cold
//!      backup/restore and operator recovery runbooks.

#![deny(missing_docs)]

use sy_types::LayerDescriptor;

/// Returns the architectural descriptor for `sy_infra`.
#[must_use]
pub fn layer_descriptor() -> LayerDescriptor {
    LayerDescriptor {
        crate_name: "sy_infra",
        entry_phase: "Phase 1.4",
        pure: false,
    }
}

/// Returns the core engine descriptor wrapped by this infrastructure layer.
#[must_use]
pub fn core_layer_descriptor() -> LayerDescriptor {
    sy_core::layer_descriptor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infra_wraps_pure_core_cleanly() {
        let infra = layer_descriptor();
        assert_eq!(infra.crate_name, "sy_infra");
        assert_eq!(infra.entry_phase, "Phase 1.4");
        assert!(!infra.pure);

        let core = core_layer_descriptor();
        assert_eq!(core.crate_name, "sy_core");
        assert!(core.pure);
    }
}
