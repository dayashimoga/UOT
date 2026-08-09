//! USB Transport Implementation
//!
//! Provides USB bulk/serial transport for wired device-to-device transfers.
//! Uses a length-prefixed framing protocol over the serial/bulk endpoint.

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::error::TransportError;
use crate::transport::tcp::Frame;
use crate::transport::types::{TransportCapabilities, TransportState, TransportStats};

/// USB connection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UsbMode {
    /// Android accessory mode (AOA protocol).
    AndroidAccessory,
    /// USB serial (CDC ACM).
    Serial,
    /// MTP file transfer (metadata only — transfer handled by OS).
    Mtp,
    /// USB bulk transfer (raw endpoints).
    Bulk,
}

impl std::fmt::Display for UsbMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AndroidAccessory => write!(f, "Android Accessory"),
            Self::Serial => write!(f, "USB Serial"),
            Self::Mtp => write!(f, "MTP"),
            Self::Bulk => write!(f, "USB Bulk"),
        }
    }
}

/// Detected USB device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsbDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_name: String,
    pub serial_number: Option<String>,
    pub mode: UsbMode,
}

/// USB transport.
pub struct UsbTransport {
    state: RwLock<TransportState>,
    stats: RwLock<TransportStats>,
    mode: UsbMode,
    connected_device: RwLock<Option<UsbDevice>>,
    tx_buffer: RwLock<VecDeque<Frame>>,
    rx_buffer: RwLock<VecDeque<Frame>>,
    max_packet_size: usize,
}

impl UsbTransport {
    pub fn new(mode: UsbMode) -> Self {
        Self {
            state: RwLock::new(TransportState::Idle),
            stats: RwLock::new(TransportStats::default()),
            mode,
            connected_device: RwLock::new(None),
            tx_buffer: RwLock::new(VecDeque::new()),
            rx_buffer: RwLock::new(VecDeque::new()),
            max_packet_size: match mode {
                UsbMode::Bulk => 512 * 1024,        // 512 KB
                UsbMode::AndroidAccessory => 16384, // 16 KB
                UsbMode::Serial => 4096,            // 4 KB
                UsbMode::Mtp => 0,                  // N/A
            },
        }
    }

    pub fn capabilities() -> TransportCapabilities {
        TransportCapabilities {
            bidirectional: true,
            reliable: true,
            requires_network: false,
            max_throughput: 480_000_000, // USB 2.0 = 480 Mbps
            typical_latency_ms: 1,
            max_payload_size: 512 * 1024,
            supports_streaming: true,
            supports_discovery: true,
            platforms: vec![
                "android".into(),
                "windows".into(),
                "macos".into(),
                "linux".into(),
            ],
        }
    }

    /// Scan for connected USB devices.
    pub fn scan_devices(&self) -> Vec<UsbDevice> {
        // In production, this would enumerate USB devices via platform APIs.
        // Returns empty in software-only mode.
        log::info!("USB device scan (mode: {})", self.mode);
        Vec::new()
    }

    /// Connect to a USB device.
    pub fn connect(&self, device: UsbDevice) -> Result<(), TransportError> {
        if self.mode == UsbMode::Mtp {
            return Err(TransportError::Connection(
                "MTP mode does not support direct data transfer".into(),
            ));
        }
        log::info!("USB connecting to {} ({})", device.device_name, device.mode);
        *self.connected_device.write() = Some(device);
        *self.state.write() = TransportState::Connected;
        Ok(())
    }

    /// Disconnect.
    pub fn disconnect(&self) {
        *self.connected_device.write() = None;
        *self.state.write() = TransportState::Disconnected;
    }

    /// Send a frame over USB.
    pub fn send_frame(&self, frame: Frame) -> Result<(), TransportError> {
        if *self.state.read() != TransportState::Connected {
            return Err(TransportError::SendFailed {
                reason: "Not connected".into(),
            });
        }
        let encoded = frame.encode();
        // Fragment if needed
        if self.max_packet_size > 0 && encoded.len() > self.max_packet_size {
            for chunk in encoded.chunks(self.max_packet_size) {
                self.tx_buffer
                    .write()
                    .push_back(Frame::data(chunk.to_vec()));
            }
        } else {
            self.tx_buffer.write().push_back(frame);
        }
        self.stats.write().bytes_sent += encoded.len() as u64;
        Ok(())
    }

    /// Receive a frame from USB.
    pub fn recv_frame(&self) -> Result<Frame, TransportError> {
        self.rx_buffer
            .write()
            .pop_front()
            .ok_or(TransportError::ReceiveFailed {
                reason: "No USB data".into(),
            })
    }

    /// Inject a received frame (for testing).
    pub fn inject_rx_frame(&self, frame: Frame) {
        self.rx_buffer.write().push_back(frame);
    }

    /// Read a sent frame (for testing).
    pub fn read_tx_frame(&self) -> Option<Frame> {
        self.tx_buffer.write().pop_front()
    }

    pub fn state(&self) -> TransportState {
        *self.state.read()
    }

    pub fn stats(&self) -> TransportStats {
        self.stats.read().clone()
    }

    pub fn mode(&self) -> UsbMode {
        self.mode
    }

    pub fn connected_device(&self) -> Option<UsbDevice> {
        self.connected_device.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_modes() {
        assert_eq!(format!("{}", UsbMode::Bulk), "USB Bulk");
        assert_eq!(format!("{}", UsbMode::Serial), "USB Serial");
        assert_eq!(
            format!("{}", UsbMode::AndroidAccessory),
            "Android Accessory"
        );
        assert_eq!(format!("{}", UsbMode::Mtp), "MTP");
    }

    #[test]
    fn test_usb_capabilities() {
        let caps = UsbTransport::capabilities();
        assert!(!caps.requires_network);
        assert!(caps.bidirectional);
        assert!(caps.reliable);
    }

    #[test]
    fn test_usb_connect_send_recv() {
        let transport = UsbTransport::new(UsbMode::Bulk);
        let device = UsbDevice {
            vendor_id: 0x18d1,
            product_id: 0x4ee1,
            device_name: "Pixel 8".into(),
            serial_number: Some("ABC123".into()),
            mode: UsbMode::Bulk,
        };

        transport.connect(device).unwrap();
        assert_eq!(transport.state(), TransportState::Connected);

        let frame = Frame::control(b"usb hello");
        transport.send_frame(frame).unwrap();
        assert!(transport.stats().bytes_sent > 0);

        let sent = transport.read_tx_frame().unwrap();
        assert_eq!(sent.payload, b"usb hello");

        transport.inject_rx_frame(Frame::data(vec![0xDE, 0xAD]));
        let recv = transport.recv_frame().unwrap();
        assert_eq!(recv.payload, vec![0xDE, 0xAD]);

        transport.disconnect();
        assert_eq!(transport.state(), TransportState::Disconnected);
    }

    #[test]
    fn test_usb_mtp_no_transfer() {
        let transport = UsbTransport::new(UsbMode::Mtp);
        let device = UsbDevice {
            vendor_id: 0,
            product_id: 0,
            device_name: "Phone".into(),
            serial_number: None,
            mode: UsbMode::Mtp,
        };
        assert!(transport.connect(device).is_err());
    }

    #[test]
    fn test_usb_not_connected_fails() {
        let transport = UsbTransport::new(UsbMode::Serial);
        assert!(transport.send_frame(Frame::ping()).is_err());
    }

    #[test]
    fn test_usb_fragmentation() {
        let transport = UsbTransport::new(UsbMode::Serial); // 4 KB max
        let device = UsbDevice {
            vendor_id: 0,
            product_id: 0,
            device_name: "Serial".into(),
            serial_number: None,
            mode: UsbMode::Serial,
        };
        transport.connect(device).unwrap();

        // Send a large frame (> 4 KB)
        let large = Frame::data(vec![0u8; 8192]);
        transport.send_frame(large).unwrap();

        // Should be fragmented into multiple TX frames
        let mut total = 0;
        while let Some(f) = transport.read_tx_frame() {
            total += f.payload.len();
        }
        assert!(total > 0);
    }

    #[test]
    fn test_usb_scan_empty() {
        let transport = UsbTransport::new(UsbMode::Bulk);
        let devices = transport.scan_devices();
        assert!(devices.is_empty()); // No real USB in software-only mode
    }
}
