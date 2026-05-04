//! # sy_config (NIV 0)
//!
//! Configuration parsing and validation.
//!
//! ## Phase 1
//! Minimal configuration - mostly defaults.

use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Data directory for world storage
    pub data_dir: PathBuf,
    /// Log level
    pub log_level: String,
    /// Ticks per second (0 = unlimited)
    pub ticks_per_second: u32,
    /// Auto-save interval in ticks (0 = disabled)
    pub auto_save_interval: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            data_dir: PathBuf::from("./data"),
            log_level: "info".to_string(),
            ticks_per_second: 0, // Unlimited for headless
            auto_save_interval: 100,
        }
    }
}

impl ServerConfig {
    /// Load config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(dir) = std::env::var("SEEYUJ_DATA_DIR") {
            config.data_dir = PathBuf::from(dir);
        }
        if let Ok(level) = std::env::var("SEEYUJ_LOG_LEVEL") {
            config.log_level = level;
        }
        if let Ok(tps) = std::env::var("SEEYUJ_TPS") {
            if let Ok(n) = tps.parse() {
                config.ticks_per_second = n;
            }
        }

        config
    }
}
