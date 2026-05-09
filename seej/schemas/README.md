# Schemas

This directory is reserved for Phase 2+ wire-protocol schemas.

There are no active client/server schema files in Phase 1. The current
repository uses Rust command/event types in `sy_api` for the headless server
slice, and `sy_protocol` is intentionally excluded from the active workspace.

When protocol work starts, this directory should become the source of truth for
wire messages only. It must not redefine core simulation state or become a
dependency of `sy_core`.

Planned responsibilities:

- versioned client/server command and event envelopes;
- generated code for `sy_protocol`;
- compatibility rules for schema evolution;
- explicit mapping into `sy_api` commands/events.
