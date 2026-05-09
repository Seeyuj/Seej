# Dependency Rules

This document defines the allowed dependencies between crates to prevent architectural drift.

## Dependency Matrix

| Crate        | Can depend on                                      |
|--------------|---------------------------------------------------|
| sy_types     | (none - leaf crate)                               |
| sy_config    | (none - leaf crate)                               |
| sy_protocol  | sy_types (Phase 2+, outside active Phase 1 build) |
| sy_api       | sy_types                                          |
| sy_core      | sy_types, sy_api                                  |
| sy_infra     | sy_types, sy_config, sy_api, sy_core             |
| sy_loader    | ALL crates (Phase 1 placeholder; no active wiring) |
| sy_tools     | sy_types, sy_config, sy_api, sy_core              |
| sy_testkit   | sy_types, sy_api, sy_core                         |
| mod_*        | sy_types, sy_api                                  |

## Forbidden Dependencies

- **sy_core** MUST NOT depend on sy_infra or sy_protocol
- **sy_api** MUST NOT depend on sy_protocol (protocol is wire format only)
- **sy_infra** MUST NOT depend on sy_protocol in the active Phase 1 workspace
- **mod_*** MUST NOT depend on sy_core directly (only via sy_api)
- **sy_protocol** is Phase 2+ and must remain excluded from the active Phase 1 workspace build

## Enforcement

CI enforces these rules with both `cargo deny` and a custom metadata check.
The supply-chain job runs the quiet `cargo deny` checks that are meaningful for
Phase 1:

```text
cargo deny check advisories licenses sources
```

`cargo deny check bans` is intentionally not part of CI in Phase 1 because the
current manifests use internal `workspace = true` dependencies and cargo-deny
emits noisy `unresolved-workspace-dependency` diagnostics while still exiting
successfully. Internal dependency boundaries are enforced by the metadata check
below instead.

The custom check reads `cargo metadata --no-deps` and fails if:

- `sy_core` direct dependencies are anything other than `sy_types` and `sy_api`
- `sy_core` owns persistence port files such as `event_log.rs` or `store.rs`
- `sy_api` depends on `sy_protocol`
- `sy_infra` depends on `sy_protocol` in Phase 1
- any `mod_*` crate depends directly on `sy_core`

## Rationale

These rules ensure:
1. The simulation core remains pure and testable
2. Protocol changes don't leak into business logic
3. Modules are loosely coupled and can evolve independently
4. Phase 1 remains a minimal deterministic persistence build without an active wire protocol
