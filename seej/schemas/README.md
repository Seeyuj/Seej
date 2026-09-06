# Schemas

This directory is reserved for public, versioned contract schemas. It is not a
dump of internal Rust serialization shapes.

Schema families planned before the first stable contract include:

- world identity, genesis, and execution profile;
- command envelope, payload fingerprint, and durable status;
- event envelope and confirmed-history cursor;
- owner-domain envelope for structured or opaque plugin data;
- plugin/provider manifest, permissions, dependencies, and migrations;
- node membership, authority epoch, and compatibility handshake;
- observation state, freshness, cursor expiration, and resynchronization;
- retention, limits, and integrity metadata.

A schema becomes public only when its versioning, unknown-variant behavior,
limits, semantic validation, migration policy, and conformance fixtures are
defined. JSON, binary, and Rust DTO representations may differ while preserving
the same semantic contract.

The current snapshot, metadata, and WAL layouts are documented in
[`../docs/phase1/PERSISTENCE.md`](../docs/phase1/PERSISTENCE.md) as reference
provider formats. They do not automatically become the cross-node wire format
or the public plugin ABI.

Normative behavior lives in:

- [kernel contract](../docs/CONTRACT.md);
- [node-network contract](../docs/NODE_NETWORK.md);
- [plugin contract](../docs/PLUGIN_CONTRACT.md).

Until concrete schema files and compatibility fixtures exist, this directory
makes no stability claim.
