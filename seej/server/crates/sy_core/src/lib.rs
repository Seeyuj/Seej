//! # `sy_core` — Pure Deterministic Transition Engine
//!
//! Architectural baseline: **D-015** (`doc/DECISIONS.md`).
//!
//! `sy_core` is the pure, headless, deterministic heart of Seej. It depends
//! **strictly on `sy_types` and `sy_api`** (zero external crates) and is
//! guarded on every commit by `scripts/check_sy_core_purity.py` and CI.
//!
//! ## Purity Guarantees (`P0-01`)
//!
//! Code in `sy_core` must **never**:
//!
//! - read wall-clock or process time (`SystemTime`, `Instant`);
//! - access environment variables, filesystem, paths, generic I/O, or network;
//! - spawn OS threads or depend on thread scheduling;
//! - use OS/ambient randomness or randomized hash collections (`HashMap`,
//!   `HashSet`, `RandomState`, `DefaultHasher`);
//! - make canonical decisions with unconstrained floating-point math (`P0-03`);
//! - embed application ontologies (space, entities, zones, economies, agents).
//!
//! ## Ordered Implementation Plan — Phase 1.3 (and Pure Logic for Phases 2–3)
//!
//! Build immediately after `sy_api` (Phase 1.2):
//!
//! 1. **Derived integer entropy (`EntropyScope`, `P0-03`):** counter-based
//!    deterministic PRF/PRNG derived from canonical coordinates
//!    `(world_seed, entropy_epoch, tick, admission_seq | phase, domain_id)`
//!    with `.fork(stable_key)` and pure integer draws.
//! 2. **Single canonical mutation path (`D-015`, `K-06`):** immutable
//!    `StateView` + `CandidateOverlay` + deterministic `WriteSet` applied via
//!    **one** `apply(WriteSet)` function shared identically by live execution
//!    and crash recovery (eliminates full-world cloning and dual `sim`/`replay`
//!    code).
//! 3. **Cross-owner atomic transactions (`K-06`):** canonical sub-command
//!    ordering, per-owner write isolation, cross-domain delta validation,
//!    full recomputation on stale `base_cursor`, and strict rule that plugin
//!    failure never fabricates a deterministic `Refused` decision.
//! 4. **Obligation lifecycle & pure integrity validator (`K-06`, `P0-12`):**
//!    block owner removal while active obligations exist; pure world integrity
//!    validation invoked after decode, recovery, migration, and before
//!    authority claims.
//! 5. **Incremental semantic root & causal chain (`P1-04`, `P1-20`):**
//!    storage-layout-independent incremental per-record state root (never full
//!    per-partition rehash on scattered commands), `events_root`, profile hash
//!    binding, and distinct `confirmed_cursor` / `checkpoint_cursor` /
//!    `applied_cursor` invariants.
//! 6. **Command bounds, lifecycle state machine & control plane (`K-08`,
//!    `P1-13`, `P1-14`, Phases 1.3 & 2.2):** pre-execution validation, finite
//!    persisted quotas, isolated **reserved control-plane budget** for
//!    `CancelCommand` and migration control, explicit rule blocking
//!    `CloseTick(T)` while a `Pending` command targets `T`, exact checked
//!    expiry, and `Open` / `Sealed` / `Closed` generation rules.
//! 7. **Deterministic contract fixtures (`K-03`, `P2-06`):** empty world,
//!    non-spatial plugin, cross-owner transaction, obligation block, and
//!    declared $T-1$ autonomous pass fixtures proving order independence.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use sy_types::LayerDescriptor;

/// Returns the architectural descriptor for `sy_core`.
#[must_use]
pub fn layer_descriptor() -> LayerDescriptor {
    LayerDescriptor {
        crate_name: "sy_core",
        entry_phase: "Phase 1.3",
        pure: true,
    }
}

/// Returns the lower-layer descriptors required by `sy_core` (`sy_types`, `sy_api`).
#[must_use]
pub fn required_pure_layers() -> [LayerDescriptor; 2] {
    [sy_types::layer_descriptor(), sy_api::layer_descriptor()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_and_dependencies_are_pure() {
        let core = layer_descriptor();
        assert_eq!(core.crate_name, "sy_core");
        assert_eq!(core.entry_phase, "Phase 1.3");
        assert!(core.pure);

        let deps = required_pure_layers();
        assert_eq!(deps[0].crate_name, "sy_types");
        assert_eq!(deps[1].crate_name, "sy_api");
        assert!(deps.iter().all(|layer| layer.pure));
    }
}
