//! Lifetime Transfer Statistics & Analytics Manager
//!
//! Tracks cumulative statistics across application lifetime (total bytes, total transfers, peak speed).
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cumulative transfer statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LifetimeStats {
    pub total_transfers: u64,
    pub successful_transfers: u64,
    pub failed_transfers: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub peak_speed_bytes_per_sec: u64,
}

impl LifetimeStats {
    /// Load stats from disk.
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save stats to disk.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Write: {e}"))?;
        Ok(())
    }

    /// Record a completed transfer.
    pub fn record_success(&mut self, bytes: u64, is_send: bool, speed: u64) {
        self.total_transfers += 1;
        self.successful_transfers += 1;
        if is_send {
            self.total_bytes_sent += bytes;
        } else {
            self.total_bytes_received += bytes;
        }
        if speed > self.peak_speed_bytes_per_sec {
            self.peak_speed_bytes_per_sec = speed;
        }
    }

    /// Record a failed transfer.
    pub fn record_failure(&mut self) {
        self.total_transfers += 1;
        self.failed_transfers += 1;
    }

    /// Default storage path.
    pub fn default_path() -> PathBuf {
        dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("uot")
            .join("stats.json")
    }
}
