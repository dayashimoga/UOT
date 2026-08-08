//! Settings Persistence
//!
//! Saves/loads user preferences to/from JSON file on disk.
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Persisted user settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub device_name: String,
    pub theme_mode: String, // "dark", "light", "system"
    pub chunk_size_kb: u32,
    pub verify_sha256: bool,
    pub auto_accept_trusted: bool,
    pub require_pin: bool,
    pub save_directory: String,
    pub network_port: u16,
    pub scan_interval_secs: u32,
    pub show_hidden_files: bool,
    pub max_concurrent_transfers: u32,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "UOT Device".to_string()),
            theme_mode: "dark".to_string(),
            chunk_size_kb: 256,
            verify_sha256: true,
            auto_accept_trusted: false,
            require_pin: false,
            save_directory: dirs_next::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("UOT")
                .to_string_lossy()
                .to_string(),
            network_port: 42000,
            scan_interval_secs: 5,
            show_hidden_files: false,
            max_concurrent_transfers: 3,
        }
    }
}

impl UserSettings {
    /// Load settings from disk.
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to disk.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Create dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Write: {e}"))?;
        Ok(())
    }

    /// Get the settings file path.
    pub fn default_path() -> PathBuf {
        dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("uot")
            .join("settings.json")
    }
}
