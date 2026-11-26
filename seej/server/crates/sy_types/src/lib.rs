//! # `sy_types` — Stable Primitives and Canonical Identifiers
//!
//! Architectural baseline: **D-015** (`doc/DECISIONS.md`).
//!
//! `sy_types` sits at the bottom of the dependency DAG with **zero workspace
//! and external dependencies**. It defines pure, domain-agnostic value types
//! shared by the frozen public contract (`sy_api`), the pure deterministic
//! engine (`sy_core`), and runtime infrastructure (`sy_infra`).
//!
//! ## Non-Negotiable Invariants
//!
//! - **Zero application ontology:** no `Entity`, `Zone`, spatial coordinates,
//!   economy types, or player categories ever belong here.
//! - **Fail-closed checked arithmetic (`P0-04`):** canonical counters (`Tick`,
//!   `EventId`, `AdmissionSeq`, `AuthorityTerm`, `ConfigVersion`, producer
//!   incarnations, expirations) must return overflow errors and never wrap or
//!   saturate silently.
//! - **Integer-only canonical decisions (`P0-03`):** no unconstrained `f32`/`f64`
//!   fields in canonical decision types.
//! - **Decoupled identity (`P0-07`):** `WorldId` is independent of seed,
//!   `GenesisHash`, node identity, filesystem path, and membership.
//! - **Versioned persistence separation (`P1-06`, `P1-07`):** persisted wire
//!   and storage DTOs remain separate from internal runtime Rust enums.
//!
//! ## Ordered Implementation Plan — Phase 1.1
//!
//! Build this crate first (before `sy_api` and `sy_core`):
//!
//! 1. **Identity & checked counters (`P0-04`, `P0-07`):** `WorldId`, `NodeId`,
//!    `Tick`, `EventId`, `AdmissionSeq`, `AuthorityTerm`, `ConfigVersion`,
//!    `Cursor`, `ProducerSlot`, `ProducerIncarnation`, `AdmissionGeneration`,
//!    and `CounterOverflowError`.
//! 2. **Logical state addressing (`K-06`):** `DomainId`, `TableId`, `TableKind`
//!    (extensible tag starting with `Ordered`), `PartitionKey`, `RecordKey`,
//!    `OpaqueBytes`, and `ObligationId`.
//! 3. **Digests & causal cursors (`P1-04`, `P1-20`):** strongly typed
//!    `StateRoot`, `EventsRoot`, `GenesisHash`, `ProfileHash`,
//!    `IntentionFingerprint`, and distinct cursor wrappers (`ConfirmedCursor`,
//!    `CheckpointCursor`, `AppliedCursor`).
//! 4. **Genesis, execution profile & limits (`P0-05`, `P0-06`, `P1-14`,
//!    `P1-15`):** versioned `GenesisSpec`, `ProofMode` (`Delta` v1 default,
//!    future opt-in `Root`), hash/entropy profile IDs, `ExecutionProfile`, and
//!    `LimitsProfile` (including the isolated reserved control-plane budget for
//!    `CancelCommand` and migration control).
//! 5. **Versioned persisted DTO boundary (`P1-06`, `P1-07`):** explicit schema
//!    tags and typed refusal for unknown mandatory variants.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Architectural decision baseline governing the clean-slate kernel.
pub const ARCHITECTURE_BASELINE: &str = "D-015";

/// Static description of a workspace layer's responsibility and coding phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerDescriptor {
    /// Cargo crate name.
    pub crate_name: &'static str,
    /// First implementation phase targeting this crate in `doc/ROADMAP.md`.
    pub entry_phase: &'static str,
    /// Whether this crate must remain strictly free of I/O and ambient state.
    pub pure: bool,
}

/// Returns the architectural descriptor for `sy_types`.
#[must_use]
pub fn layer_descriptor() -> LayerDescriptor {
    LayerDescriptor {
        crate_name: "sy_types",
        entry_phase: "Phase 1.1",
        pure: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_matches_clean_slate_baseline() {
        let desc = layer_descriptor();
        assert_eq!(desc.crate_name, "sy_types");
        assert_eq!(desc.entry_phase, "Phase 1.1");
        assert!(desc.pure);
        assert_eq!(ARCHITECTURE_BASELINE, "D-015");
    }
}
