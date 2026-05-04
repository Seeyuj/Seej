# Schemas

This directory contains the source of truth for all data structures exchanged between client and server.

## Format

We use FlatBuffers (or Protobuf) for:
- Efficient binary serialization
- Schema evolution with backward compatibility
- Code generation for Rust and client languages

## Directory Structure

```
/schemas
├── common.fbs       # Shared types (Vec3, EntityId, Timestamp)
├── commands.fbs     # Client → Server messages
├── events.fbs       # Server → Client messages
└── README.md
```

## Generating Code

### Rust (sy_protocol crate)

```bash
flatc --rust -o ../server/crates/sy_protocol/src/generated/ *.fbs
```

### TypeScript (web client)

```bash
flatc --ts -o ../clients/web/src/generated/ *.fbs
```

## Versioning

- Schema files are versioned with the repository
- Breaking changes require a protocol version bump
- Use `@deprecated` annotations before removing fields
- Always add new fields at the end with default values
