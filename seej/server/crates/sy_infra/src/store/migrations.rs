//! # Migrations
//!
//! Schema and data migrations for storage.
//!
//! ## Phase 1
//! Supported formats are read as-is; there are no implicit migrations.

use sy_types::WorldMeta;

/// Check if a world needs migration.
pub fn needs_migration(meta: &WorldMeta) -> bool {
    !WorldMeta::supports_format(meta.format_version)
}

/// Migrate world metadata to current version.
/// Returns true if migration was needed.
pub fn migrate_meta(meta: &mut WorldMeta) -> bool {
    let _ = meta;
    false
}
