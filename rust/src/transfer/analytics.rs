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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_stats_inline_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("stats.json");

        let mut stats = LifetimeStats::load(&path);
        assert_eq!(stats.total_transfers, 0);

        stats.record_success(1024, true, 50000);
        stats.record_success(2048, false, 80000);
        stats.record_failure();

        assert_eq!(stats.total_transfers, 3);
        assert_eq!(stats.successful_transfers, 2);
        assert_eq!(stats.failed_transfers, 1);
        assert_eq!(stats.total_bytes_sent, 1024);
        assert_eq!(stats.total_bytes_received, 2048);
        assert_eq!(stats.peak_speed_bytes_per_sec, 80000);

        stats.save(&path).unwrap();
        let reloaded = LifetimeStats::load(&path);
        assert_eq!(reloaded.total_transfers, 3);

        let def_path = LifetimeStats::default_path();
        assert!(def_path.to_string_lossy().contains("stats.json"));
    }
}
