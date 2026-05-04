//! # Migrations
//!
//! Schema and data migrations for storage.
//!
//! ## Phase 1
//! Minimal migration support - just version tracking.

use sy_types::WorldMeta;

/// Check if a world needs migration.
pub fn needs_migration(meta: &WorldMeta) -> bool {
    meta.format_version < WorldMeta::CURRENT_FORMAT_VERSION
}

/// Migrate world metadata to current version.
/// Returns true if migration was needed.
pub fn migrate_meta(meta: &mut WorldMeta) -> bool {
    if meta.format_version >= WorldMeta::CURRENT_FORMAT_VERSION {
        return false;
    }

    // Future migrations would go here
    // match meta.format_version {
    //     0 => { /* migrate v0 -> v1 */ }
    //     _ => {}
    // }

    meta.format_version = WorldMeta::CURRENT_FORMAT_VERSION;
    true
}
