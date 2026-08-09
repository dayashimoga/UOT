//! Hardware Abstraction Interfaces
//!
//! Common traits for all transport/peripheral adapters. Production code and
//! simulation code MUST use identical interfaces so tests validate real contracts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::error::TransportError;
use crate::transport::tcp::Frame;

// ═══════════════════════════════════════════════════════════════════
// TRANSPORT ADAPTER TRAIT
// ═══════════════════════════════════════════════════════════════════

/// Universal transport adapter trait. Every transport (TCP, BLE, Wi-Fi Direct,
/// QR, USB) implements this so the engine can treat them uniformly.
#[async_trait]
pub trait TransportAdapter: Send + Sync {
    /// Human-readable transport name.
    fn name(&self) -> &str;

    /// Whether this transport is currently available on this platform/device.
    fn is_available(&self) -> bool;

    /// Send a frame to the connected peer.
    async fn send_frame(&self, frame: Frame) -> Result<(), TransportError>;

    /// Receive the next frame from the connected peer.
    async fn recv_frame(&self) -> Result<Frame, TransportError>;

    /// Close the connection gracefully.
    async fn close(&self) -> Result<(), TransportError>;
}

// ═══════════════════════════════════════════════════════════════════
// BLE ADAPTER TRAIT
// ═══════════════════════════════════════════════════════════════════

/// BLE scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleDevice {
    pub device_id: String,
    pub device_name: String,
    pub rssi: i32,
    pub service_uuids: Vec<String>,
}

/// BLE adapter trait for both real and simulated BLE.
#[async_trait]
pub trait BleAdapter: Send + Sync {
    /// Start BLE advertising with the given service UUID and payload.
    async fn start_advertising(&self, payload: &[u8]) -> Result<(), TransportError>;
    /// Stop advertising.
    async fn stop_advertising(&self) -> Result<(), TransportError>;
    /// Start scanning for BLE peripherals.
    async fn start_scan(&self) -> Result<(), TransportError>;
    /// Stop scanning.
    async fn stop_scan(&self) -> Result<(), TransportError>;
    /// Get discovered devices.
    fn discovered_devices(&self) -> Vec<BleDevice>;
    /// Connect to a peripheral by ID.
    async fn connect(&self, device_id: &str) -> Result<(), TransportError>;
    /// Disconnect from a peripheral.
    async fn disconnect(&self, device_id: &str) -> Result<(), TransportError>;
    /// Send data over BLE GATT characteristic.
    async fn send_data(&self, device_id: &str, data: &[u8]) -> Result<(), TransportError>;
    /// Receive data from BLE GATT characteristic.
    async fn recv_data(&self, device_id: &str) -> Result<Vec<u8>, TransportError>;
    /// Get the negotiated MTU.
    fn mtu(&self) -> usize;
    /// Check if BLE is available on this platform.
    fn is_available(&self) -> bool;
}

// ═══════════════════════════════════════════════════════════════════
// WI-FI DIRECT ADAPTER TRAIT
// ═══════════════════════════════════════════════════════════════════

/// Wi-Fi Direct peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiDirectPeer {
    pub device_id: String,
    pub device_name: String,
    pub is_group_owner: bool,
    pub ip_address: Option<String>,
}

/// Wi-Fi Direct adapter trait.
#[async_trait]
pub trait WifiDirectAdapter: Send + Sync {
    /// Start peer discovery.
    async fn discover_peers(&self) -> Result<Vec<WifiDirectPeer>, TransportError>;
    /// Create a P2P group (become group owner).
    async fn create_group(&self) -> Result<WifiDirectPeer, TransportError>;
    /// Connect to a peer.
    async fn connect_peer(&self, device_id: &str) -> Result<WifiDirectPeer, TransportError>;
    /// Disconnect.
    async fn disconnect(&self) -> Result<(), TransportError>;
    /// Check availability.
    fn is_available(&self) -> bool;
}

// ═══════════════════════════════════════════════════════════════════
// CAMERA / QR ADAPTER TRAIT
// ═══════════════════════════════════════════════════════════════════

/// Camera/QR adapter trait.
#[async_trait]
pub trait CameraAdapter: Send + Sync {
    /// Capture a QR code frame and return decoded bytes (or None if no QR found).
    async fn scan_qr_frame(&self) -> Result<Option<Vec<u8>>, TransportError>;
    /// Check if camera is available.
    fn is_available(&self) -> bool;
}

// ═══════════════════════════════════════════════════════════════════
// MEDIA SOURCE TRAITS
// ═══════════════════════════════════════════════════════════════════

/// A video frame.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub pts_us: u64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub is_keyframe: bool,
}

/// An audio frame.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub pts_us: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub data: Vec<u8>,
}

/// Video source trait (camera, screen capture, or synthetic).
#[async_trait]
pub trait VideoSource: Send + Sync {
    async fn next_frame(&mut self) -> Option<VideoFrame>;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn fps(&self) -> u32;
}

/// Audio source trait (microphone or synthetic).
#[async_trait]
pub trait AudioSource: Send + Sync {
    async fn next_frame(&mut self) -> Option<AudioFrame>;
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER SESSION MODEL
// ═══════════════════════════════════════════════════════════════════

/// Universal transfer session — common across all transports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    /// Unique session ID.
    pub session_id: String,
    /// Authentication state.
    pub authenticated: bool,
    /// Encryption key (derived from key exchange).
    pub encryption_key: Option<Vec<u8>>,
    /// Transfer manifest (files, sizes, hashes).
    pub manifest: Vec<ManifestEntry>,
    /// Per-chunk verification map: chunk_index -> verified.
    pub chunk_map: HashMap<u32, bool>,
    /// Number of verified chunks.
    pub verified_chunks: u32,
    /// Total chunks.
    pub total_chunks: u32,
    /// Current transport being used.
    pub transport_id: String,
    /// Retry count.
    pub retry_count: u32,
    /// Bytes transferred.
    pub bytes_transferred: u64,
    /// Total bytes.
    pub total_bytes: u64,
}

/// Manifest entry for a file in a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub chunks: u32,
    pub completed_chunks: u32,
}

impl TransferSession {
    /// Create a new transfer session.
    pub fn new(session_id: &str, transport_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            authenticated: false,
            encryption_key: None,
            manifest: Vec::new(),
            chunk_map: HashMap::new(),
            verified_chunks: 0,
            total_chunks: 0,
            transport_id: transport_id.to_string(),
            retry_count: 0,
            bytes_transferred: 0,
            total_bytes: 0,
        }
    }

    /// Mark a chunk as verified.
    pub fn verify_chunk(&mut self, index: u32) {
        if self.chunk_map.insert(index, true) != Some(true) {
            self.verified_chunks += 1;
        }
    }

    /// Check if a chunk is already verified (for resume — skip retransmit).
    pub fn is_chunk_verified(&self, index: u32) -> bool {
        self.chunk_map.get(&index).copied().unwrap_or(false)
    }

    /// Progress as fraction (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            0.0
        } else {
            self.verified_chunks as f64 / self.total_chunks as f64
        }
    }

    /// Whether the transfer is complete.
    pub fn is_complete(&self) -> bool {
        self.verified_chunks == self.total_chunks
    }

    /// Migrate to a different transport (preserving session state).
    pub fn migrate_transport(&mut self, new_transport_id: &str) {
        self.transport_id = new_transport_id.to_string();
        self.retry_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_session_lifecycle() {
        let mut session = TransferSession::new("sess-1", "tcp");
        session.total_chunks = 10;
        session.total_bytes = 10000;

        assert!(!session.is_complete());
        assert_eq!(session.progress(), 0.0);

        for i in 0..10 {
            session.verify_chunk(i);
        }
        assert!(session.is_complete());
        assert_eq!(session.progress(), 1.0);
        assert_eq!(session.verified_chunks, 10);
    }

    #[test]
    fn test_transfer_session_no_duplicate_verify() {
        let mut session = TransferSession::new("sess-2", "tcp");
        session.total_chunks = 5;
        session.verify_chunk(0);
        session.verify_chunk(0); // Duplicate
        assert_eq!(session.verified_chunks, 1);
    }

    #[test]
    fn test_transfer_session_transport_migration() {
        let mut session = TransferSession::new("sess-3", "tcp");
        session.total_chunks = 100;
        for i in 0..50 {
            session.verify_chunk(i);
        }
        assert_eq!(session.verified_chunks, 50);

        // Migrate to BLE
        session.migrate_transport("ble");
        assert_eq!(session.transport_id, "ble");
        assert_eq!(session.retry_count, 1);
        assert_eq!(session.verified_chunks, 50); // Progress preserved

        // Verified chunks should not be retransmitted
        assert!(session.is_chunk_verified(0));
        assert!(!session.is_chunk_verified(50));
    }

    #[test]
    fn test_manifest_entry() {
        let entry = ManifestEntry {
            name: "file.txt".to_string(),
            relative_path: "dir/file.txt".to_string(),
            size: 1024,
            sha256: Some("abc".to_string()),
            chunks: 4,
            completed_chunks: 2,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ManifestEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "file.txt");
        assert_eq!(parsed.chunks, 4);
    }
}
