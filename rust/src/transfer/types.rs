//! Transfer Types
//!
//! Types for tracking transfer state, progress, and history.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A record of a transfer (active or historical).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Unique transfer identifier.
    pub transfer_id: Uuid,
    /// Direction of the transfer.
    pub direction: TransferDirection,
    /// Current status.
    pub status: TransferStatus,
    /// Remote device name.
    pub remote_device: String,
    /// Items in this transfer.
    pub items: Vec<TransferItemRecord>,
    /// Total size in bytes.
    pub total_size: u64,
    /// Bytes transferred so far.
    pub transferred_bytes: u64,
    /// When the transfer was created.
    pub created_at: DateTime<Utc>,
    /// When the transfer started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the transfer completed/failed/cancelled.
    pub finished_at: Option<DateTime<Utc>>,
    /// Error message if transfer failed.
    pub error: Option<String>,
}

/// Individual item within a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferItemRecord {
    /// Item identifier.
    pub item_id: Uuid,
    /// File name.
    pub name: String,
    /// Relative path.
    pub relative_path: String,
    /// Size in bytes.
    pub size: u64,
    /// Bytes transferred.
    pub transferred_bytes: u64,
    /// Status of this item.
    pub status: TransferStatus,
    /// SHA-256 hash (computed after transfer).
    pub hash: Option<String>,
    /// Saved absolute or canonical path on local disk (if available).
    #[serde(default)]
    pub saved_path: Option<String>,
}

/// Direction of a transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Sending to a remote device.
    Send,
    /// Receiving from a remote device.
    Receive,
}

/// Status of a transfer or transfer item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Waiting to start.
    Queued,
    /// Waiting for acceptance.
    Pending,
    /// Actively transferring.
    InProgress,
    /// Transfer paused.
    Paused,
    /// Verifying integrity.
    Verifying,
    /// Successfully completed.
    Completed,
    /// Transfer failed.
    Failed,
    /// Transfer cancelled.
    Cancelled,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "In Progress"),
            Self::Paused => write!(f, "Paused"),
            Self::Verifying => write!(f, "Verifying"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Real-time progress of an active transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// Transfer identifier.
    pub transfer_id: Uuid,
    /// Overall progress (0.0 - 1.0).
    pub progress: f64,
    /// Bytes transferred so far.
    pub transferred_bytes: u64,
    /// Total bytes to transfer.
    pub total_bytes: u64,
    /// Current transfer speed in bytes/sec.
    pub speed_bytes_per_sec: u64,
    /// Estimated time remaining in seconds.
    pub eta_secs: Option<f64>,
    /// Currently transferring item name.
    pub current_item: Option<String>,
    /// Items completed / total items.
    pub items_completed: usize,
    /// Total items in transfer.
    pub items_total: usize,
}

impl TransferProgress {
    /// Get a formatted speed string (e.g., "72 MB/s").
    pub fn speed_display(&self) -> String {
        format_bytes_per_sec(self.speed_bytes_per_sec)
    }

    /// Get a formatted ETA string (e.g., "2m 30s").
    pub fn eta_display(&self) -> String {
        match self.eta_secs {
            Some(secs) if secs < 60.0 => format!("{}s", secs as u64),
            Some(secs) if secs < 3600.0 => {
                format!("{}m {}s", (secs / 60.0) as u64, (secs % 60.0) as u64)
            }
            Some(secs) => format!(
                "{}h {}m",
                (secs / 3600.0) as u64,
                ((secs % 3600.0) / 60.0) as u64
            ),
            None => "calculating…".to_string(),
        }
    }

    /// Get clamped percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            ((self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
        }
    }
}

/// Format bytes/sec into a human-readable string.
fn format_bytes_per_sec(bps: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bps >= GB {
        format!("{:.1} GB/s", bps as f64 / GB as f64)
    } else if bps >= MB {
        format!("{:.1} MB/s", bps as f64 / MB as f64)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps as f64 / KB as f64)
    } else {
        format!("{bps} B/s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_status_display() {
        assert_eq!(TransferStatus::InProgress.to_string(), "In Progress");
        assert_eq!(TransferStatus::Completed.to_string(), "Completed");
        assert_eq!(TransferStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_format_bytes_per_sec() {
        assert_eq!(format_bytes_per_sec(500), "500 B/s");
        assert_eq!(format_bytes_per_sec(1024), "1.0 KB/s");
        assert_eq!(format_bytes_per_sec(72 * 1024 * 1024), "72.0 MB/s");
        assert_eq!(format_bytes_per_sec(2 * 1024 * 1024 * 1024), "2.0 GB/s");
    }

    #[test]
    fn test_transfer_progress_speed_display() {
        let progress = TransferProgress {
            transfer_id: Uuid::new_v4(),
            progress: 0.5,
            transferred_bytes: 50 * 1024 * 1024,
            total_bytes: 100 * 1024 * 1024,
            speed_bytes_per_sec: 72 * 1024 * 1024,
            eta_secs: Some(0.7),
            current_item: Some("photo.jpg".to_string()),
            items_completed: 3,
            items_total: 10,
        };
        assert_eq!(progress.speed_display(), "72.0 MB/s");
    }

    #[test]
    fn test_transfer_progress_eta_display() {
        let mut progress = TransferProgress {
            transfer_id: Uuid::new_v4(),
            progress: 0.0,
            transferred_bytes: 0,
            total_bytes: 100,
            speed_bytes_per_sec: 0,
            eta_secs: None,
            current_item: None,
            items_completed: 0,
            items_total: 1,
        };

        assert_eq!(progress.eta_display(), "calculating…");

        progress.eta_secs = Some(30.0);
        assert_eq!(progress.eta_display(), "30s");

        progress.eta_secs = Some(150.0);
        assert_eq!(progress.eta_display(), "2m 30s");

        progress.eta_secs = Some(3750.0);
        assert_eq!(progress.eta_display(), "1h 2m");
    }

    #[test]
    fn test_transfer_record_serialization() {
        let record = TransferRecord {
            transfer_id: Uuid::new_v4(),
            direction: TransferDirection::Send,
            status: TransferStatus::Completed,
            remote_device: "Test Phone".to_string(),
            items: vec![],
            total_size: 1024,
            transferred_bytes: 1024,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            finished_at: Some(Utc::now()),
            error: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: TransferRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.remote_device, "Test Phone");
        assert_eq!(deserialized.direction, TransferDirection::Send);
    }
}
