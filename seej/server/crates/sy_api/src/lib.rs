//! # `sy_api` — Frozen Public Contract & Internal Engineering Seams
//!
//! Architectural baseline: **D-015** (`doc/DECISIONS.md`).
//! Contracts: `seej/docs/CONTRACT.md` (`0.4.0-draft`) and
//! `seej/docs/PLUGIN_CONTRACT.md` (`0.3.0-draft`).
//!
//! `sy_api` depends strictly on `sy_types`. Following the Linux / Cordis
//! discipline established in D-015, it keeps a strict separation between:
//!
//! 1. **The single frozen public boundary (v1):** the application plugin,
//!    command, and observation contract. Third-party plugins depend *only* on
//!    these public definitions — never on `sy_core` or `sy_infra`.
//! 2. **Three internal engineering seams (tested, NOT publicly frozen in v1):**
//!    `StateStore`, `AuthorityLog`, and `Executor`. No internal seam may be
//!    frozen as a public ABI without a real second provider and real consumer.
//!
//! ## Ordered Implementation Plan — Phase 1.2 (and Extensions in Phases 2–3)
//!
//! Build immediately after `sy_types` (Phase 1.1):
//!
//! 1. **Public logical state & obligations (`K-06`, Phase 1.2):** immutable
//!    `StateView` over partitioned `OrderedTable` keys, `WriteSet` / `WriteOp`,
//!    and `ObligationEnvelope` / `ObligationRegistry` view traits.
//! 2. **Public pure plugin contract (`K-04`, `K-06`, Phase 1.2):** immutable
//!    `PluginManifest`, flat-batch entrypoint
//!    `(StateView, InputBatch, EntropyScope, Context) -> Decision + WriteSet + EventBatch + OutboxIntents`,
//!    explicit read-semantics declaration (`Sequential` vs. `SnapshotTMinus1`
//!    for autonomous passes), optional `AccessHint`, and cross-domain
//!    delta-oriented validator traits.
//! 3. **Public command, resolution & control schemas (`K-08`, Phase 1.2 / 2.2):**
//!    `CommandEnvelope`, `Admission`, `Resolution` (`Succeeded`, `Refused`,
//!    `Cancelled`), `CancelCommand` (reserved control-plane budget),
//!    `CloseTick`, and lookup/transient status codes (`Conflict`, `Expired`,
//!    `GenerationSealed`, `LimitExceeded`, `NotFound`, `Unavailable`,
//!    `WorldPaused`).
//! 4. **Public observation & migration schemas (`K-02`, `K-09`, Phase 3):**
//!    confirmed prefix queries with freshness (`CurrentConfirmed` vs. `Stale`),
//!    pinned cursor resynchronization, and 5-phase migration control messages.
//! 5. **Internal testable traits (`K-05`, Phase 1.2 / 1.4 / 2.1):**
//!    - `StateStore`: MVCC `pin_cut`, atomic `apply(WriteSet)`, incremental
//!      per-record semantic root maintenance, and checkpoint chunk streaming.
//!    - `AuthorityLog`: group-committed `TickCommit`, persistent fencing state
//!      (`authority_term`, vote, `config_version`), joint membership, and quorum
//!      read barriers.
//!    - `Executor`: batched sequential executor in v1; parallel `Block-STM` /
//!      `BSP` and isolated WASM execution in Gate 1-D without changing plugin
//!      semantics.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use sy_types::LayerDescriptor;

/// Draft kernel contract version aligned with `seej/docs/CONTRACT.md`.
pub const KERNEL_CONTRACT_VERSION: &str = "0.4.0-draft";

/// Draft plugin contract version aligned with `seej/docs/PLUGIN_CONTRACT.md`.
pub const PLUGIN_CONTRACT_VERSION: &str = "0.3.0-draft";

/// Returns the architectural descriptor for `sy_api`.
#[must_use]
pub fn layer_descriptor() -> LayerDescriptor {
    LayerDescriptor {
        crate_name: "sy_api",
        entry_phase: "Phase 1.2",
        pure: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_and_draft_versions_match_contracts() {
        let desc = layer_descriptor();
        assert_eq!(desc.crate_name, "sy_api");
        assert_eq!(desc.entry_phase, "Phase 1.2");
        assert!(desc.pure);
        assert_eq!(KERNEL_CONTRACT_VERSION, "0.4.0-draft");
        assert_eq!(PLUGIN_CONTRACT_VERSION, "0.3.0-draft");
    }
}
