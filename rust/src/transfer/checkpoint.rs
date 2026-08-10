//! Transfer Checkpoint — Persistent Resume State
//!
//! Saves transfer progress to disk so interrupted transfers can be
//! resumed after app restart. Uses a JSON checkpoint file per transfer.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Checkpoint state for a single transfer — persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCheckpoint {
    /// Transfer ID.
    pub transfer_id: Uuid,
    /// Direction: "send" or "receive".
    pub direction: String,
    /// Remote device name.
    pub remote_device: String,
    /// Total bytes expected.
    pub total_size: u64,
    /// Bytes transferred so far.
    pub transferred_bytes: u64,
    /// Per-item checkpoints.
    pub items: Vec<ItemCheckpoint>,
    /// When this checkpoint was saved.
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

/// Per-item checkpoint within a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemCheckpoint {
    /// Item name.
    pub name: String,
    /// Relative path.
    pub relative_path: String,
    /// Total item size.
    pub size: u64,
    /// Bytes of this item already transferred.
    pub transferred_bytes: u64,
    /// Whether this item is complete.
    pub complete: bool,
    /// SHA-256 hash if complete.
    pub sha256: Option<String>,
}

/// Manages checkpoint persistence for resume-after-restart.
#[derive(Debug)]
pub struct CheckpointStore {
    /// Directory where checkpoint files are stored.
    checkpoint_dir: PathBuf,
}

impl CheckpointStore {
    /// Create a new checkpoint store in the given directory.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let checkpoint_dir = dir.into();
        std::fs::create_dir_all(&checkpoint_dir).ok();
        Self { checkpoint_dir }
    }

    /// Default checkpoint directory.
    pub fn default_path() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(base);
        path.push("uot");
        path.push("checkpoints");
        path
    }

    /// Save a checkpoint for a transfer.
    pub fn save(&self, checkpoint: &TransferCheckpoint) -> Result<(), std::io::Error> {
        let filename = format!("{}.checkpoint.json", checkpoint.transfer_id);
        let path = self.checkpoint_dir.join(filename);
        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)?;
        log::debug!("Saved checkpoint for transfer {}", checkpoint.transfer_id);
        Ok(())
    }

    /// Load a checkpoint for a specific transfer.
    pub fn load(&self, transfer_id: &Uuid) -> Result<TransferCheckpoint, std::io::Error> {
        let filename = format!("{transfer_id}.checkpoint.json");
        let path = self.checkpoint_dir.join(filename);
        let json = std::fs::read_to_string(&path)?;
        let checkpoint: TransferCheckpoint = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(checkpoint)
    }

    /// List all incomplete transfer checkpoints.
    pub fn list_incomplete(&self) -> Vec<TransferCheckpoint> {
        let mut checkpoints = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.checkpoint_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(json) = std::fs::read_to_string(&path) {
                        if let Ok(cp) = serde_json::from_str::<TransferCheckpoint>(&json) {
                            if cp.transferred_bytes < cp.total_size {
                                checkpoints.push(cp);
                            }
                        }
                    }
                }
            }
        }
        checkpoints
    }

    /// Remove a checkpoint (called when transfer completes).
    pub fn remove(&self, transfer_id: &Uuid) -> Result<(), std::io::Error> {
        let filename = format!("{transfer_id}.checkpoint.json");
        let path = self.checkpoint_dir.join(filename);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Get the checkpoint directory path.
    pub fn dir(&self) -> &Path {
        &self.checkpoint_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path());

        let tid = Uuid::new_v4();
        let checkpoint = TransferCheckpoint {
            transfer_id: tid,
            direction: "send".to_string(),
            remote_device: "TestDevice".to_string(),
            total_size: 1_000_000,
            transferred_bytes: 500_000,
            items: vec![ItemCheckpoint {
                name: "file1.txt".to_string(),
                relative_path: "file1.txt".to_string(),
                size: 1_000_000,
                transferred_bytes: 500_000,
                complete: false,
                sha256: None,
            }],
            saved_at: chrono::Utc::now(),
        };

        store.save(&checkpoint).unwrap();

        let loaded = store.load(&tid).unwrap();
        assert_eq!(loaded.transfer_id, tid);
        assert_eq!(loaded.transferred_bytes, 500_000);
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].name, "file1.txt");
    }

    #[test]
    fn test_checkpoint_list_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path());

        // Save an incomplete checkpoint
        let cp1 = TransferCheckpoint {
            transfer_id: Uuid::new_v4(),
            direction: "receive".to_string(),
            remote_device: "Dev1".to_string(),
            total_size: 2000,
            transferred_bytes: 1000,
            items: vec![],
            saved_at: chrono::Utc::now(),
        };
        store.save(&cp1).unwrap();

        // Save a complete checkpoint
        let cp2 = TransferCheckpoint {
            transfer_id: Uuid::new_v4(),
            direction: "send".to_string(),
            remote_device: "Dev2".to_string(),
            total_size: 500,
            transferred_bytes: 500, // complete
            items: vec![],
            saved_at: chrono::Utc::now(),
        };
        store.save(&cp2).unwrap();

        let incomplete = store.list_incomplete();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].remote_device, "Dev1");
    }

    #[test]
    fn test_checkpoint_remove() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path());

        let tid = Uuid::new_v4();
        let cp = TransferCheckpoint {
            transfer_id: tid,
            direction: "send".to_string(),
            remote_device: "Dev".to_string(),
            total_size: 100,
            transferred_bytes: 50,
            items: vec![],
            saved_at: chrono::Utc::now(),
        };
        store.save(&cp).unwrap();
        assert!(store.load(&tid).is_ok());

        store.remove(&tid).unwrap();
        assert!(store.load(&tid).is_err());
    }

    #[test]
    fn test_checkpoint_load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path());
        let result = store.load(&Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn test_checkpoint_default_path_and_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path());
        assert_eq!(store.dir(), dir.path());

        let default_p = CheckpointStore::default_path();
        assert!(default_p.to_string_lossy().contains("checkpoints"));
    }
}
