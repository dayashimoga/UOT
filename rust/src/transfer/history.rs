//! Persistent Transfer History & State Store
//!
//! Manages persistent history records and query filters across app restarts.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::transfer::types::{TransferRecord, TransferStatus};

/// Persistent transfer history store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransferHistoryStore {
    pub records: Vec<TransferRecord>,
}

impl TransferHistoryStore {
    /// Load history from disk.
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save history to disk.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Write: {e}"))?;
        Ok(())
    }

    /// Add or update a transfer record.
    pub fn upsert(&mut self, record: TransferRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|r| r.transfer_id == record.transfer_id)
        {
            *existing = record;
        } else {
            self.records.push(record);
        }
    }

    /// Search/filter history records.
    pub fn query(&self, query: &str, status_filter: Option<TransferStatus>) -> Vec<TransferRecord> {
        let q = query.to_lowercase();
        self.records
            .iter()
            .filter(|r| {
                let matches_text = q.is_empty()
                    || r.remote_device.to_lowercase().contains(&q)
                    || r.items
                        .iter()
                        .any(|item| item.name.to_lowercase().contains(&q));
                let matches_status = status_filter.is_none_or(|s| r.status == s);
                matches_text && matches_status
            })
            .cloned()
            .collect()
    }

    /// Get default storage path for transfer history.
    pub fn default_path() -> PathBuf {
        dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("uot")
            .join("history.json")
    }
}
