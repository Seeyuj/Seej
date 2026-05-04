# Dependency Rules

This document defines the allowed dependencies between crates to prevent architectural drift.

## Dependency Matrix

| Crate        | Can depend on                                      |
|--------------|---------------------------------------------------|
| sy_types     | (none - leaf crate)                               |
| sy_config    | (none - leaf crate)                               |
| sy_protocol  | sy_types                                          |
| sy_api       | sy_types                                          |
| sy_core      | sy_types, sy_api                                  |
| sy_infra     | sy_types, sy_config, sy_protocol, sy_api, sy_core |
| sy_loader    | ALL crates                                        |
| sy_tools     | sy_types, sy_config, sy_api, sy_core              |
| sy_testkit   | sy_types, sy_api, sy_core                         |
| mod_*        | sy_types, sy_api                                  |

## Forbidden Dependencies

- **sy_core** MUST NOT depend on sy_infra or sy_protocol
- **sy_api** MUST NOT depend on sy_protocol (protocol is wire format only)
- **mod_*** MUST NOT depend on sy_core directly (only via sy_api)

## Enforcement

Use `cargo deny` or custom CI checks to enforce these rules:

```toml
# deny.toml
[bans]
deny = [
    { name = "sy_infra", wrappers = ["sy_core"] },
    { name = "sy_protocol", wrappers = ["sy_api", "sy_core"] },
]
```

## Rationale

These rules ensure:
1. The simulation core remains pure and testable
2. Protocol changes don't leak into business logic
3. Modules are loosely coupled and can evolve independently
