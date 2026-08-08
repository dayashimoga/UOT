//! File Transfer Engine Implementation
//!
//! Handles chunked file I/O, SHA-256 integrity verification,
//! progress tracking, and multi-file/folder transfers.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use uuid::Uuid;

use crate::core::error::TransferError;
use crate::transfer::types::{
    TransferDirection, TransferItemRecord, TransferProgress, TransferRecord, TransferStatus,
};

/// A file item to be transferred.
#[derive(Debug, Clone)]
pub struct TransferItem {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Relative path for the receiver (preserves folder structure).
    pub relative_path: String,
    /// File name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
}

impl TransferItem {
    /// Create from an absolute path.
    pub async fn from_path(path: &Path) -> Result<Self, TransferError> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| TransferError::FileIo(format!("Cannot read {}: {e}", path.display())))?;

        if !metadata.is_file() {
            return Err(TransferError::FileIo(format!(
                "Not a file: {}",
                path.display()
            )));
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Self {
            path: path.to_path_buf(),
            relative_path: name.clone(),
            name,
            size: metadata.len(),
        })
    }
}

/// Collect all files from a directory recursively.
pub async fn collect_files(dir: &Path) -> Result<Vec<TransferItem>, TransferError> {
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("folder");

    let mut items = Vec::new();
    collect_files_recursive(dir, dir_name, &mut items).await?;
    Ok(items)
}

async fn collect_files_recursive(
    dir: &Path,
    prefix: &str,
    items: &mut Vec<TransferItem>,
) -> Result<(), TransferError> {
    let mut entries = fs::read_dir(dir)
        .await
        .map_err(|e| TransferError::FileIo(format!("Cannot read dir {}: {e}", dir.display())))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| TransferError::FileIo(format!("Dir entry error: {e}")))?
    {
        let path = entry.path();
        let name = entry.file_name().to_str().unwrap_or("unknown").to_string();
        let relative = format!("{prefix}/{name}");

        let metadata = entry
            .metadata()
            .await
            .map_err(|e| TransferError::FileIo(format!("Metadata error: {e}")))?;

        if metadata.is_file() {
            items.push(TransferItem {
                path: path.clone(),
                relative_path: relative,
                name,
                size: metadata.len(),
            });
        } else if metadata.is_dir() {
            // Skip symlinks for security
            if metadata.file_type().is_symlink() {
                log::warn!("Skipping symlink: {}", path.display());
                continue;
            }
            Box::pin(collect_files_recursive(&path, &relative, items)).await?;
        }
    }

    Ok(())
}

/// Read a file chunk at the given offset.
pub async fn read_chunk(
    path: &Path,
    offset: u64,
    chunk_size: usize,
) -> Result<(Vec<u8>, u32), TransferError> {
    let mut file = fs::File::open(path)
        .await
        .map_err(|e| TransferError::FileIo(format!("Cannot open {}: {e}", path.display())))?;

    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|e| TransferError::FileIo(format!("Seek error: {e}")))?;

    let mut buf = vec![0u8; chunk_size];
    let bytes_read = file
        .read(&mut buf)
        .await
        .map_err(|e| TransferError::FileIo(format!("Read error: {e}")))?;

    buf.truncate(bytes_read);

    // CRC32 for chunk integrity
    let crc = crc32fast::hash(&buf);

    Ok((buf, crc))
}

/// Write a file chunk at the given offset.
pub async fn write_chunk(
    path: &Path,
    offset: u64,
    data: &[u8],
    expected_crc: u32,
) -> Result<(), TransferError> {
    // Verify CRC
    let actual_crc = crc32fast::hash(data);
    if actual_crc != expected_crc {
        return Err(TransferError::IntegrityFailed(format!(
            "Chunk CRC mismatch at offset {offset}: expected {expected_crc:08x}, got {actual_crc:08x}"
        )));
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| TransferError::FileIo(format!("Cannot create dir: {e}")))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .await
        .map_err(|e| TransferError::FileIo(format!("Cannot open for write: {e}")))?;

    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|e| TransferError::FileIo(format!("Seek error: {e}")))?;

    file.write_all(data)
        .await
        .map_err(|e| TransferError::FileIo(format!("Write error: {e}")))?;

    file.flush()
        .await
        .map_err(|e| TransferError::FileIo(format!("Flush error: {e}")))?;

    Ok(())
}

/// Compute SHA-256 hash of a file.
pub async fn compute_sha256(path: &Path) -> Result<String, TransferError> {
    let mut file = fs::File::open(path)
        .await
        .map_err(|e| TransferError::FileIo(format!("Cannot open {}: {e}", path.display())))?;

    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024]; // 256KB read buffer

    loop {
        let bytes_read = file
            .read(&mut buf)
            .await
            .map_err(|e| TransferError::FileIo(format!("Read error: {e}")))?;

        if bytes_read == 0 {
            break;
        }
        hasher.update(&buf[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Progress tracker for an active transfer.
pub struct ProgressTracker {
    /// Transfer ID.
    transfer_id: Uuid,
    /// Total bytes to transfer.
    total_bytes: u64,
    /// Bytes transferred so far.
    transferred: Arc<RwLock<u64>>,
    /// Items completed.
    items_completed: Arc<RwLock<usize>>,
    /// Total items.
    items_total: usize,
    /// Current item name.
    current_item: Arc<RwLock<Option<String>>>,
    /// Speed samples for averaging (timestamp_ms, bytes).
    speed_samples: Arc<RwLock<Vec<(u64, u64)>>>,
    /// Start time.
    start_time: std::time::Instant,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new(transfer_id: Uuid, total_bytes: u64, items_total: usize) -> Self {
        Self {
            transfer_id,
            total_bytes,
            transferred: Arc::new(RwLock::new(0)),
            items_completed: Arc::new(RwLock::new(0)),
            items_total,
            current_item: Arc::new(RwLock::new(None)),
            speed_samples: Arc::new(RwLock::new(Vec::new())),
            start_time: std::time::Instant::now(),
        }
    }

    /// Record bytes transferred.
    pub fn add_bytes(&self, bytes: u64) {
        *self.transferred.write() += bytes;

        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        let total = *self.transferred.read();
        self.speed_samples.write().push((elapsed_ms, total));

        // Keep only last 20 samples
        let mut samples = self.speed_samples.write();
        let len = samples.len();
        if len > 20 {
            samples.drain(..len - 20);
        }
    }

    /// Mark an item as completed.
    pub fn complete_item(&self) {
        *self.items_completed.write() += 1;
    }

    /// Set the current item name.
    pub fn set_current_item(&self, name: &str) {
        *self.current_item.write() = Some(name.to_string());
    }

    /// Get the current progress snapshot.
    pub fn snapshot(&self) -> TransferProgress {
        let transferred = *self.transferred.read();
        let progress = if self.total_bytes > 0 {
            transferred as f64 / self.total_bytes as f64
        } else {
            1.0
        };

        let speed = self.calculate_speed();
        let eta = if speed > 0 {
            let remaining = self.total_bytes.saturating_sub(transferred);
            Some(remaining as f64 / speed as f64)
        } else {
            None
        };

        TransferProgress {
            transfer_id: self.transfer_id,
            progress,
            transferred_bytes: transferred,
            total_bytes: self.total_bytes,
            speed_bytes_per_sec: speed,
            eta_secs: eta,
            current_item: self.current_item.read().clone(),
            items_completed: *self.items_completed.read(),
            items_total: self.items_total,
        }
    }

    /// Calculate current speed (bytes/sec) using sliding window average.
    fn calculate_speed(&self) -> u64 {
        let samples = self.speed_samples.read();
        if samples.len() < 2 {
            return 0;
        }

        let first = &samples[0];
        let last = &samples[samples.len() - 1];
        let time_diff_ms = last.0.saturating_sub(first.0);

        if time_diff_ms == 0 {
            return 0;
        }

        let byte_diff = last.1.saturating_sub(first.1);
        (byte_diff * 1000) / time_diff_ms
    }
}

/// Create a TransferRecord for tracking.
pub fn create_transfer_record(
    items: &[TransferItem],
    direction: TransferDirection,
    remote_device: &str,
) -> TransferRecord {
    let total_size: u64 = items.iter().map(|i| i.size).sum();
    let item_records: Vec<TransferItemRecord> = items
        .iter()
        .map(|item| TransferItemRecord {
            item_id: Uuid::new_v4(),
            name: item.name.clone(),
            relative_path: item.relative_path.clone(),
            size: item.size,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
            hash: None,
        })
        .collect();

    TransferRecord {
        transfer_id: Uuid::new_v4(),
        direction,
        status: TransferStatus::Queued,
        remote_device: remote_device.to_string(),
        items: item_records,
        total_size,
        transferred_bytes: 0,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transfer_item_from_path() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello world").await.unwrap();

        let item = TransferItem::from_path(&file_path).await.unwrap();
        assert_eq!(item.name, "test.txt");
        assert_eq!(item.size, 11);
    }

    #[tokio::test]
    async fn test_transfer_item_not_a_file() {
        let dir = TempDir::new().unwrap();
        let result = TransferItem::from_path(dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_write_chunk() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("data.bin");

        // Create test file
        let data = vec![42u8; 1024];
        fs::write(&file_path, &data).await.unwrap();

        // Read chunk
        let (chunk, crc) = read_chunk(&file_path, 0, 512).await.unwrap();
        assert_eq!(chunk.len(), 512);
        assert!(chunk.iter().all(|&b| b == 42));

        // Write chunk to new file
        let out_path = dir.path().join("out.bin");
        write_chunk(&out_path, 0, &chunk, crc).await.unwrap();

        let written = fs::read(&out_path).await.unwrap();
        assert_eq!(written, chunk);
    }

    #[tokio::test]
    async fn test_write_chunk_crc_mismatch() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("bad.bin");

        let data = vec![1, 2, 3, 4];
        let wrong_crc = 0xDEADBEEF;

        let result = write_chunk(&file_path, 0, &data, wrong_crc).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC mismatch"));
    }

    #[tokio::test]
    async fn test_compute_sha256() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("hash_test.txt");
        fs::write(&file_path, "hello").await.unwrap();

        let hash = compute_sha256(&file_path).await.unwrap();
        // Known SHA-256 of "hello"
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[tokio::test]
    async fn test_collect_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "aaa").await.unwrap();
        fs::create_dir(dir.path().join("sub")).await.unwrap();
        fs::write(dir.path().join("sub/b.txt"), "bbb")
            .await
            .unwrap();

        let items = collect_files(dir.path()).await.unwrap();
        assert_eq!(items.len(), 2);

        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));

        // Check relative paths preserve folder structure
        let sub_item = items.iter().find(|i| i.name == "b.txt").unwrap();
        assert!(sub_item.relative_path.contains("sub/b.txt"));
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::new(Uuid::new_v4(), 1000, 2);

        tracker.set_current_item("file1.txt");
        tracker.add_bytes(500);

        let snap = tracker.snapshot();
        assert_eq!(snap.transferred_bytes, 500);
        assert_eq!(snap.total_bytes, 1000);
        assert!((snap.progress - 0.5).abs() < f64::EPSILON);
        assert_eq!(snap.current_item, Some("file1.txt".to_string()));
        assert_eq!(snap.items_completed, 0);
        assert_eq!(snap.items_total, 2);

        tracker.complete_item();
        let snap2 = tracker.snapshot();
        assert_eq!(snap2.items_completed, 1);
    }

    #[test]
    fn test_create_transfer_record() {
        let items = vec![
            TransferItem {
                path: PathBuf::from("/tmp/a.txt"),
                relative_path: "a.txt".to_string(),
                name: "a.txt".to_string(),
                size: 100,
            },
            TransferItem {
                path: PathBuf::from("/tmp/b.txt"),
                relative_path: "b.txt".to_string(),
                name: "b.txt".to_string(),
                size: 200,
            },
        ];

        let record = create_transfer_record(&items, TransferDirection::Send, "Test Phone");
        assert_eq!(record.total_size, 300);
        assert_eq!(record.items.len(), 2);
        assert_eq!(record.direction, TransferDirection::Send);
        assert_eq!(record.status, TransferStatus::Queued);
        assert_eq!(record.remote_device, "Test Phone");
    }
}
