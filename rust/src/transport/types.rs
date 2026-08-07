//! Transport Types
//!
//! Shared types used across all transport implementations.
use serde::{Deserialize, Serialize};

/// Unique identifier for a transport technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportId {
    /// TCP over LAN/Wi-Fi.
    TcpLan,
    /// Wi-Fi Direct (P2P).
    WifiDirect,
    /// Bluetooth Classic.
    BluetoothClassic,
    /// Bluetooth Low Energy.
    BluetoothLe,
    /// QR Code (animated visual data transport).
    QrCode,
    /// USB wired connection.
    Usb,
    /// Temporary hotspot.
    Hotspot,
    /// Future internet/relay transport.
    Relay,
}

impl std::fmt::Display for TransportId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TcpLan => write!(f, "Wi-Fi"),
            Self::WifiDirect => write!(f, "Wi-Fi Direct"),
            Self::BluetoothClassic => write!(f, "Bluetooth"),
            Self::BluetoothLe => write!(f, "Bluetooth LE"),
            Self::QrCode => write!(f, "QR Code"),
            Self::Usb => write!(f, "USB"),
            Self::Hotspot => write!(f, "Hotspot"),
            Self::Relay => write!(f, "Relay"),
        }
    }
}

/// Current state of a transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    /// Transport is available but not active.
    Idle,
    /// Transport is scanning/listening.
    Listening,
    /// Transport is establishing a connection.
    Connecting,
    /// Transport has an active connection.
    Connected,
    /// Transport connection was lost, attempting reconnect.
    Reconnecting,
    /// Transport is shutting down.
    Disconnecting,
    /// Transport is not available on this platform.
    Unavailable,
    /// Transport encountered an error.
    Error,
}

impl std::fmt::Display for TransportState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Listening => write!(f, "Listening"),
            Self::Connecting => write!(f, "Connecting…"),
            Self::Connected => write!(f, "Connected"),
            Self::Reconnecting => write!(f, "Reconnecting…"),
            Self::Disconnecting => write!(f, "Disconnecting…"),
            Self::Unavailable => write!(f, "Unavailable"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Capabilities of a transport implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapabilities {
    /// Whether this transport supports bidirectional data.
    pub bidirectional: bool,
    /// Whether this transport supports reliable (ordered, guaranteed) delivery.
    pub reliable: bool,
    /// Whether this transport requires an existing network.
    pub requires_network: bool,
    /// Maximum theoretical throughput in bytes/sec (0 = unknown).
    pub max_throughput: u64,
    /// Typical latency in milliseconds.
    pub typical_latency_ms: u32,
    /// Maximum payload size per message (0 = unlimited/stream).
    pub max_payload_size: u64,
    /// Whether this transport supports streaming.
    pub supports_streaming: bool,
    /// Whether discovery is supported through this transport.
    pub supports_discovery: bool,
    /// Platforms where this transport is available.
    pub platforms: Vec<String>,
}

/// Runtime statistics for a transport connection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportStats {
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Total bytes received.
    pub bytes_received: u64,
    /// Current throughput in bytes/sec (send).
    pub send_throughput: u64,
    /// Current throughput in bytes/sec (receive).
    pub receive_throughput: u64,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u32,
    /// Number of retransmissions.
    pub retransmissions: u64,
    /// Connection uptime in seconds.
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_id_display() {
        assert_eq!(TransportId::TcpLan.to_string(), "Wi-Fi");
        assert_eq!(TransportId::WifiDirect.to_string(), "Wi-Fi Direct");
        assert_eq!(TransportId::BluetoothLe.to_string(), "Bluetooth LE");
        assert_eq!(TransportId::QrCode.to_string(), "QR Code");
        assert_eq!(TransportId::Usb.to_string(), "USB");
    }

    #[test]
    fn test_transport_state_display() {
        assert_eq!(TransportState::Connected.to_string(), "Connected");
        assert_eq!(TransportState::Connecting.to_string(), "Connecting…");
        assert_eq!(TransportState::Reconnecting.to_string(), "Reconnecting…");
    }

    #[test]
    fn test_transport_stats_default() {
        let stats = TransportStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.latency_ms, 0);
    }

    #[test]
    fn test_transport_id_serialization() {
        let id = TransportId::TcpLan;
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: TransportId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_transport_capabilities_serialization() {
        let caps = TransportCapabilities {
            bidirectional: true,
            reliable: true,
            requires_network: true,
            max_throughput: 100_000_000,
            typical_latency_ms: 5,
            max_payload_size: 0,
            supports_streaming: true,
            supports_discovery: true,
            platforms: vec!["android".to_string(), "windows".to_string()],
        };
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: TransportCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps.bidirectional, deserialized.bidirectional);
        assert_eq!(caps.max_throughput, deserialized.max_throughput);
    }
}
