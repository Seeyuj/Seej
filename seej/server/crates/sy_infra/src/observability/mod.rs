//! # Observability
//!
//! Tracing and metrics exporters for monitoring.
//!
//! ## Phase 1
//! Basic tracing setup. Uses tracing crate with subscriber.

use tracing_subscriber::EnvFilter;

/// Initialize tracing with default settings.
pub fn init_tracing(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
