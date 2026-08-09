//! BLE, Wi-Fi Direct, QR Fountain, and Streaming Simulators
//!
//! Deterministic fake implementations of hardware adapters for testing
//! protocol contracts without physical hardware.

use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};

use crate::core::error::TransportError;
use crate::testing::adapters::*;

// ═══════════════════════════════════════════════════════════════════
// FAKE BLE ADAPTER
// ═══════════════════════════════════════════════════════════════════

/// Deterministic fake BLE adapter for testing BLE protocol contracts.
pub struct FakeBleAdapter {
    available: bool,
    mtu: usize,
    advertising: RwLock<bool>,
    scanning: RwLock<bool>,
    connected_devices: RwLock<Vec<String>>,
    discovered: RwLock<Vec<BleDevice>>,
    /// Simulated data buffers per device (device_id -> incoming data queue).
    rx_buffers: RwLock<std::collections::HashMap<String, VecDeque<Vec<u8>>>>,
    /// Simulated TX buffers per device.
    tx_buffers: RwLock<std::collections::HashMap<String, VecDeque<Vec<u8>>>>,
    /// Fault injection: drop every Nth send (0 = never).
    drop_every_n: u32,
    send_count: RwLock<u32>,
}

impl FakeBleAdapter {
    pub fn new(available: bool, mtu: usize) -> Self {
        Self {
            available,
            mtu,
            advertising: RwLock::new(false),
            scanning: RwLock::new(false),
            connected_devices: RwLock::new(Vec::new()),
            discovered: RwLock::new(Vec::new()),
            rx_buffers: RwLock::new(std::collections::HashMap::new()),
            tx_buffers: RwLock::new(std::collections::HashMap::new()),
            drop_every_n: 0,
            send_count: RwLock::new(0),
        }
    }

    /// Create with fault injection.
    pub fn with_faults(available: bool, mtu: usize, drop_every_n: u32) -> Self {
        let mut adapter = Self::new(available, mtu);
        adapter.drop_every_n = drop_every_n;
        adapter
    }

    /// Inject a discovered device (simulating a real scan result).
    pub fn inject_device(&self, device: BleDevice) {
        self.discovered.write().push(device);
    }

    /// Inject incoming data for a device (simulating GATT notification).
    pub fn inject_rx_data(&self, device_id: &str, data: Vec<u8>) {
        self.rx_buffers
            .write()
            .entry(device_id.to_string())
            .or_default()
            .push_back(data);
    }

    /// Read TX data sent by the adapter (for verification).
    pub fn read_tx_data(&self, device_id: &str) -> Option<Vec<u8>> {
        self.tx_buffers
            .write()
            .get_mut(device_id)
            .and_then(|q| q.pop_front())
    }
}

#[async_trait]
impl BleAdapter for FakeBleAdapter {
    async fn start_advertising(&self, _payload: &[u8]) -> Result<(), TransportError> {
        if !self.available {
            return Err(TransportError::Connection("BLE unavailable".into()));
        }
        *self.advertising.write() = true;
        Ok(())
    }

    async fn stop_advertising(&self) -> Result<(), TransportError> {
        *self.advertising.write() = false;
        Ok(())
    }

    async fn start_scan(&self) -> Result<(), TransportError> {
        if !self.available {
            return Err(TransportError::Connection("BLE unavailable".into()));
        }
        *self.scanning.write() = true;
        Ok(())
    }

    async fn stop_scan(&self) -> Result<(), TransportError> {
        *self.scanning.write() = false;
        Ok(())
    }

    fn discovered_devices(&self) -> Vec<BleDevice> {
        self.discovered.read().clone()
    }

    async fn connect(&self, device_id: &str) -> Result<(), TransportError> {
        if !self.available {
            return Err(TransportError::Connection("BLE unavailable".into()));
        }
        self.connected_devices.write().push(device_id.to_string());
        self.rx_buffers
            .write()
            .entry(device_id.to_string())
            .or_default();
        self.tx_buffers
            .write()
            .entry(device_id.to_string())
            .or_default();
        Ok(())
    }

    async fn disconnect(&self, device_id: &str) -> Result<(), TransportError> {
        self.connected_devices.write().retain(|d| d != device_id);
        Ok(())
    }

    async fn send_data(&self, device_id: &str, data: &[u8]) -> Result<(), TransportError> {
        if !self
            .connected_devices
            .read()
            .contains(&device_id.to_string())
        {
            return Err(TransportError::Connection("Not connected".into()));
        }
        // Fragment by MTU
        let mut count = *self.send_count.read();
        for chunk in data.chunks(self.mtu) {
            count += 1;
            if self.drop_every_n > 0 && count % self.drop_every_n == 0 {
                continue; // Simulate dropped fragment
            }
            self.tx_buffers
                .write()
                .get_mut(device_id)
                .unwrap()
                .push_back(chunk.to_vec());
        }
        *self.send_count.write() = count;
        Ok(())
    }

    async fn recv_data(&self, device_id: &str) -> Result<Vec<u8>, TransportError> {
        self.rx_buffers
            .write()
            .get_mut(device_id)
            .and_then(|q| q.pop_front())
            .ok_or_else(|| TransportError::Timeout { timeout_ms: 0 })
    }

    fn mtu(&self) -> usize {
        self.mtu
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

// ═══════════════════════════════════════════════════════════════════
// FAKE WI-FI DIRECT ADAPTER
// ═══════════════════════════════════════════════════════════════════

/// Deterministic fake Wi-Fi Direct adapter.
pub struct FakeWifiDirectAdapter {
    available: bool,
    peers: RwLock<Vec<WifiDirectPeer>>,
    connected: RwLock<Option<WifiDirectPeer>>,
    fail_on_connect: bool,
}

impl FakeWifiDirectAdapter {
    pub fn new(available: bool) -> Self {
        Self {
            available,
            peers: RwLock::new(Vec::new()),
            connected: RwLock::new(None),
            fail_on_connect: false,
        }
    }

    pub fn with_failure(available: bool) -> Self {
        Self {
            available,
            peers: RwLock::new(Vec::new()),
            connected: RwLock::new(None),
            fail_on_connect: true,
        }
    }

    pub fn inject_peer(&self, peer: WifiDirectPeer) {
        self.peers.write().push(peer);
    }
}

#[async_trait]
impl WifiDirectAdapter for FakeWifiDirectAdapter {
    async fn discover_peers(&self) -> Result<Vec<WifiDirectPeer>, TransportError> {
        if !self.available {
            return Err(TransportError::Connection(
                "Wi-Fi Direct unavailable".into(),
            ));
        }
        Ok(self.peers.read().clone())
    }

    async fn create_group(&self) -> Result<WifiDirectPeer, TransportError> {
        if !self.available {
            return Err(TransportError::Connection(
                "Wi-Fi Direct unavailable".into(),
            ));
        }
        let go = WifiDirectPeer {
            device_id: "self".to_string(),
            device_name: "UOT-GO".to_string(),
            is_group_owner: true,
            ip_address: Some("192.168.49.1".to_string()),
        };
        *self.connected.write() = Some(go.clone());
        Ok(go)
    }

    async fn connect_peer(&self, device_id: &str) -> Result<WifiDirectPeer, TransportError> {
        if self.fail_on_connect {
            return Err(TransportError::Connection(
                "Simulated connection failure".into(),
            ));
        }
        let peer = self
            .peers
            .read()
            .iter()
            .find(|p| p.device_id == device_id)
            .cloned()
            .ok_or_else(|| TransportError::Connection("Peer not found".into()))?;
        *self.connected.write() = Some(peer.clone());
        Ok(peer)
    }

    async fn disconnect(&self) -> Result<(), TransportError> {
        *self.connected.write() = None;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

// ═══════════════════════════════════════════════════════════════════
// QR FOUNTAIN CODE SIMULATOR
// ═══════════════════════════════════════════════════════════════════

/// Simple LT-code inspired fountain encoder/decoder for QR data transfer.
pub struct FountainEncoder {
    blocks: Vec<Vec<u8>>,
    block_size: usize,
    total_blocks: usize,
    frame_count: u32,
}

impl FountainEncoder {
    /// Create encoder from raw data, splitting into `block_size` chunks.
    pub fn new(data: &[u8], block_size: usize) -> Self {
        let blocks: Vec<Vec<u8>> = data
            .chunks(block_size)
            .map(|c| {
                let mut block = c.to_vec();
                block.resize(block_size, 0); // Pad to block_size
                block
            })
            .collect();
        let total_blocks = blocks.len();
        Self {
            blocks,
            block_size,
            total_blocks,
            frame_count: 0,
        }
    }

    /// Generate the next fountain-encoded frame.
    /// Returns (frame_index, xor_indices, encoded_block).
    pub fn next_frame(&mut self) -> FountainFrame {
        let degree = 1 + (self.frame_count as usize % 3).min(self.total_blocks - 1);
        let mut indices: Vec<usize> = Vec::new();
        let mut block = vec![0u8; self.block_size];

        for d in 0..degree {
            let idx = (self.frame_count as usize + d * 7) % self.total_blocks;
            if !indices.contains(&idx) {
                indices.push(idx);
                for (i, b) in self.blocks[idx].iter().enumerate() {
                    block[i] ^= b;
                }
            }
        }

        self.frame_count += 1;
        FountainFrame {
            frame_index: self.frame_count - 1,
            total_blocks: self.total_blocks as u32,
            block_size: self.block_size as u32,
            indices,
            data: block,
        }
    }

    pub fn total_blocks(&self) -> usize {
        self.total_blocks
    }
}

/// A single fountain-coded frame.
#[derive(Debug, Clone)]
pub struct FountainFrame {
    pub frame_index: u32,
    pub total_blocks: u32,
    pub block_size: u32,
    pub indices: Vec<usize>,
    pub data: Vec<u8>,
}

/// Fountain decoder — reconstructs data from received frames.
pub struct FountainDecoder {
    total_blocks: usize,
    block_size: usize,
    decoded: Vec<Option<Vec<u8>>>,
    frames_received: u32,
}

impl FountainDecoder {
    pub fn new(total_blocks: usize, block_size: usize) -> Self {
        Self {
            total_blocks,
            block_size,
            decoded: vec![None; total_blocks],
            frames_received: 0,
        }
    }

    /// Process a received frame. Returns true if new data was decoded.
    pub fn process_frame(&mut self, frame: &FountainFrame) -> bool {
        self.frames_received += 1;

        // Simple: degree-1 frames directly decode a block
        if frame.indices.len() == 1 {
            let idx = frame.indices[0];
            if idx < self.total_blocks && self.decoded[idx].is_none() {
                self.decoded[idx] = Some(frame.data.clone());
                return true;
            }
        }
        // For degree > 1: XOR out known blocks to recover unknown
        else {
            let unknown: Vec<usize> = frame
                .indices
                .iter()
                .filter(|&&i| i < self.total_blocks && self.decoded[i].is_none())
                .copied()
                .collect();

            if unknown.len() == 1 {
                let mut recovered = frame.data.clone();
                for &idx in &frame.indices {
                    if idx != unknown[0] {
                        if let Some(ref block) = self.decoded[idx] {
                            for (i, b) in block.iter().enumerate() {
                                recovered[i] ^= b;
                            }
                        }
                    }
                }
                self.decoded[unknown[0]] = Some(recovered);
                return true;
            }
        }
        false
    }

    /// Check if all blocks are decoded.
    pub fn is_complete(&self) -> bool {
        self.decoded.iter().all(|b| b.is_some())
    }

    /// Get number of decoded blocks.
    pub fn decoded_count(&self) -> usize {
        self.decoded.iter().filter(|b| b.is_some()).count()
    }

    /// Reconstruct the original data (only valid after is_complete() == true).
    pub fn reconstruct(&self, original_size: usize) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }
        let mut data = Vec::new();
        for block in &self.decoded {
            data.extend_from_slice(block.as_ref()?);
        }
        data.truncate(original_size);
        Some(data)
    }
}

// ═══════════════════════════════════════════════════════════════════
// SYNTHETIC MEDIA SOURCES
// ═══════════════════════════════════════════════════════════════════

/// Deterministic synthetic video source generating test patterns.
pub struct SyntheticVideoSource {
    width: u32,
    height: u32,
    fps: u32,
    frame_count: u64,
    max_frames: u64,
}

impl SyntheticVideoSource {
    pub fn new(width: u32, height: u32, fps: u32, max_frames: u64) -> Self {
        Self {
            width,
            height,
            fps,
            frame_count: 0,
            max_frames,
        }
    }
}

#[async_trait]
impl VideoSource for SyntheticVideoSource {
    async fn next_frame(&mut self) -> Option<VideoFrame> {
        if self.frame_count >= self.max_frames {
            return None;
        }
        let pts = self.frame_count * 1_000_000 / self.fps as u64;
        let is_keyframe = self.frame_count % (self.fps as u64) == 0;

        // Generate deterministic pattern (just frame number bytes)
        let mut data = Vec::with_capacity(self.width as usize * self.height as usize);
        let pattern = (self.frame_count % 256) as u8;
        data.resize(self.width as usize * self.height as usize, pattern);

        self.frame_count += 1;
        Some(VideoFrame {
            pts_us: pts,
            width: self.width,
            height: self.height,
            data,
            is_keyframe,
        })
    }

    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn fps(&self) -> u32 {
        self.fps
    }
}

/// Deterministic synthetic audio source generating sine wave samples.
pub struct SyntheticAudioSource {
    sample_rate: u32,
    channels: u16,
    frame_count: u64,
    max_frames: u64,
    samples_per_frame: usize,
}

impl SyntheticAudioSource {
    pub fn new(sample_rate: u32, channels: u16, max_frames: u64) -> Self {
        Self {
            sample_rate,
            channels,
            frame_count: 0,
            max_frames,
            samples_per_frame: (sample_rate / 50) as usize, // 20ms frames
        }
    }
}

#[async_trait]
impl AudioSource for SyntheticAudioSource {
    async fn next_frame(&mut self) -> Option<AudioFrame> {
        if self.frame_count >= self.max_frames {
            return None;
        }
        let pts = self.frame_count * 20_000; // 20ms per frame

        let mut data = Vec::with_capacity(self.samples_per_frame * 2);
        for i in 0..self.samples_per_frame {
            let t = (self.frame_count as f64 * self.samples_per_frame as f64 + i as f64)
                / self.sample_rate as f64;
            let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
            let s16 = (sample * 32000.0) as i16;
            data.extend_from_slice(&s16.to_le_bytes());
        }

        self.frame_count += 1;
        Some(AudioFrame {
            pts_us: pts,
            sample_rate: self.sample_rate,
            channels: self.channels,
            data,
        })
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> u16 {
        self.channels
    }
}

// ═══════════════════════════════════════════════════════════════════
// FAKE CAMERA ADAPTER (QR)
// ═══════════════════════════════════════════════════════════════════

/// Fake camera that returns injected QR frames.
pub struct FakeCameraAdapter {
    available: bool,
    frames: RwLock<VecDeque<Option<Vec<u8>>>>,
}

impl FakeCameraAdapter {
    pub fn new(available: bool) -> Self {
        Self {
            available,
            frames: RwLock::new(VecDeque::new()),
        }
    }

    /// Inject a QR frame result.
    pub fn inject_frame(&self, data: Option<Vec<u8>>) {
        self.frames.write().push_back(data);
    }
}

#[async_trait]
impl CameraAdapter for FakeCameraAdapter {
    async fn scan_qr_frame(&self) -> Result<Option<Vec<u8>>, TransportError> {
        if !self.available {
            return Err(TransportError::Connection("Camera unavailable".into()));
        }
        Ok(self.frames.write().pop_front().flatten())
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BLE Simulator Tests ──

    #[tokio::test]
    async fn test_ble_full_lifecycle() {
        let ble = FakeBleAdapter::new(true, 20);
        ble.start_advertising(b"UOT").await.unwrap();
        ble.start_scan().await.unwrap();

        ble.inject_device(BleDevice {
            device_id: "dev-1".into(),
            device_name: "Phone".into(),
            rssi: -65,
            service_uuids: vec!["UOT-SVC".into()],
        });
        assert_eq!(ble.discovered_devices().len(), 1);

        ble.connect("dev-1").await.unwrap();

        // Send data (fragmented by MTU=20)
        let data = vec![0xABu8; 50]; // 50 bytes = 3 fragments
        ble.send_data("dev-1", &data).await.unwrap();
        assert!(ble.read_tx_data("dev-1").is_some()); // Fragment 1

        // Inject RX data
        ble.inject_rx_data("dev-1", vec![1, 2, 3]);
        let rx = ble.recv_data("dev-1").await.unwrap();
        assert_eq!(rx, vec![1, 2, 3]);

        ble.disconnect("dev-1").await.unwrap();
        assert!(ble.send_data("dev-1", &[1]).await.is_err()); // Not connected
    }

    #[tokio::test]
    async fn test_ble_unavailable() {
        let ble = FakeBleAdapter::new(false, 20);
        assert!(!ble.is_available());
        assert!(ble.start_advertising(b"x").await.is_err());
        assert!(ble.start_scan().await.is_err());
        assert!(ble.connect("dev").await.is_err());
    }

    #[tokio::test]
    async fn test_ble_mtu_fragmentation() {
        let ble = FakeBleAdapter::new(true, 10);
        ble.connect("dev-1").await.unwrap();
        ble.send_data("dev-1", &[0u8; 25]).await.unwrap(); // 3 fragments of 10,10,5

        let mut total = 0;
        while let Some(fragment) = ble.read_tx_data("dev-1") {
            assert!(fragment.len() <= 10);
            total += fragment.len();
        }
        assert_eq!(total, 25);
    }

    #[tokio::test]
    async fn test_ble_with_drops() {
        let ble = FakeBleAdapter::with_faults(true, 10, 2); // Drop every 2nd fragment
        ble.connect("dev-1").await.unwrap();
        ble.send_data("dev-1", &[0u8; 30]).await.unwrap(); // 3 fragments, 1 dropped

        let mut count = 0;
        while ble.read_tx_data("dev-1").is_some() {
            count += 1;
        }
        assert_eq!(count, 2); // 1 of 3 dropped
    }

    // ── Wi-Fi Direct Simulator Tests ──

    #[tokio::test]
    async fn test_wifi_direct_lifecycle() {
        let wfd = FakeWifiDirectAdapter::new(true);
        wfd.inject_peer(WifiDirectPeer {
            device_id: "peer-1".into(),
            device_name: "Laptop".into(),
            is_group_owner: false,
            ip_address: None,
        });

        let peers = wfd.discover_peers().await.unwrap();
        assert_eq!(peers.len(), 1);

        let connected = wfd.connect_peer("peer-1").await.unwrap();
        assert_eq!(connected.device_name, "Laptop");

        wfd.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_wifi_direct_group_creation() {
        let wfd = FakeWifiDirectAdapter::new(true);
        let go = wfd.create_group().await.unwrap();
        assert!(go.is_group_owner);
        assert_eq!(go.ip_address, Some("192.168.49.1".to_string()));
    }

    #[tokio::test]
    async fn test_wifi_direct_unavailable() {
        let wfd = FakeWifiDirectAdapter::new(false);
        assert!(wfd.discover_peers().await.is_err());
    }

    #[tokio::test]
    async fn test_wifi_direct_connection_failure() {
        let wfd = FakeWifiDirectAdapter::with_failure(true);
        wfd.inject_peer(WifiDirectPeer {
            device_id: "peer-1".into(),
            device_name: "X".into(),
            is_group_owner: false,
            ip_address: None,
        });
        assert!(wfd.connect_peer("peer-1").await.is_err());
    }

    // ── QR Fountain Tests ──

    #[test]
    fn test_fountain_encode_decode_small() {
        let original = b"Hello, QR Fountain Code transfer!";
        let mut encoder = FountainEncoder::new(original, 8);
        let total = encoder.total_blocks();
        let mut decoder = FountainDecoder::new(total, 8);

        // Send enough frames to decode all blocks
        for _ in 0..(total * 3) {
            let frame = encoder.next_frame();
            decoder.process_frame(&frame);
            if decoder.is_complete() {
                break;
            }
        }

        assert!(decoder.is_complete());
        let reconstructed = decoder.reconstruct(original.len()).unwrap();
        assert_eq!(reconstructed, original);
    }

    #[test]
    fn test_fountain_with_frame_loss() {
        let original = vec![42u8; 100];
        let mut encoder = FountainEncoder::new(&original, 10);
        let total = encoder.total_blocks();
        let mut decoder = FountainDecoder::new(total, 10);

        // Drop every 3rd frame, but send extra frames to compensate
        let mut frame_idx = 0;
        for _ in 0..(total * 10) {
            let frame = encoder.next_frame();
            frame_idx += 1;
            if frame_idx % 3 == 0 {
                continue; // Drop
            }
            decoder.process_frame(&frame);
            if decoder.is_complete() {
                break;
            }
        }

        assert!(decoder.is_complete());
        let result = decoder.reconstruct(original.len()).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_fountain_sha256_integrity() {
        let original = b"Integrity check for fountain transfer";
        let expected_hash = {
            let mut h = Sha256::new();
            h.update(original);
            hex::encode(h.finalize())
        };

        let mut encoder = FountainEncoder::new(original, 8);
        let total = encoder.total_blocks();
        let mut decoder = FountainDecoder::new(total, 8);

        for _ in 0..(total * 5) {
            let frame = encoder.next_frame();
            decoder.process_frame(&frame);
            if decoder.is_complete() {
                break;
            }
        }

        let result = decoder.reconstruct(original.len()).unwrap();
        let actual_hash = {
            let mut h = Sha256::new();
            h.update(&result);
            hex::encode(h.finalize())
        };
        assert_eq!(expected_hash, actual_hash);
    }

    // ── Synthetic Media Tests ──

    #[tokio::test]
    async fn test_synthetic_video_source() {
        let mut src = SyntheticVideoSource::new(320, 240, 30, 90);
        let mut count = 0;
        let mut keyframes = 0;
        while let Some(frame) = src.next_frame().await {
            count += 1;
            if frame.is_keyframe {
                keyframes += 1;
            }
            assert_eq!(frame.width, 320);
            assert_eq!(frame.height, 240);
        }
        assert_eq!(count, 90);
        assert_eq!(keyframes, 3); // Every 30 frames
    }

    #[tokio::test]
    async fn test_synthetic_audio_source() {
        let mut src = SyntheticAudioSource::new(48000, 1, 50);
        let mut count = 0;
        while let Some(frame) = src.next_frame().await {
            count += 1;
            assert_eq!(frame.sample_rate, 48000);
            assert!(!frame.data.is_empty());
        }
        assert_eq!(count, 50);
    }

    // ── Fake Camera Tests ──

    #[tokio::test]
    async fn test_fake_camera_qr_scan() {
        let cam = FakeCameraAdapter::new(true);
        cam.inject_frame(None);
        cam.inject_frame(Some(b"QR_DATA_HERE".to_vec()));
        cam.inject_frame(None);

        assert!(cam.scan_qr_frame().await.unwrap().is_none());
        assert_eq!(cam.scan_qr_frame().await.unwrap().unwrap(), b"QR_DATA_HERE");
        assert!(cam.scan_qr_frame().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_fake_camera_unavailable() {
        let cam = FakeCameraAdapter::new(false);
        assert!(cam.scan_qr_frame().await.is_err());
    }
}
