//! Discovery Types
//!
//! Shared types for device discovery.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A discovered device on the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// Unique device identifier.
    pub device_id: String,
    /// Human-readable device name.
    pub device_name: String,
    /// Device type (phone, tablet, laptop, desktop).
    pub device_type: DeviceType,
    /// How this device was discovered.
    pub discovery_method: DiscoveryMethod,
    /// Network address (IP:port, BLE address, etc.).
    pub address: Option<String>,
    /// Available transport capabilities.
    pub capabilities: Vec<String>,
    /// Signal strength indicator (for wireless, 0-100).
    pub signal_strength: Option<u8>,
    /// When this device was first discovered.
    pub first_seen: DateTime<Utc>,
    /// When this device was last seen.
    pub last_seen: DateTime<Utc>,
    /// Whether this is a trusted/paired device.
    pub is_trusted: bool,
}

/// Type of device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Phone,
    Tablet,
    Laptop,
    Desktop,
    Tv,
    Unknown,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Phone => write!(f, "Phone"),
            Self::Tablet => write!(f, "Tablet"),
            Self::Laptop => write!(f, "Laptop"),
            Self::Desktop => write!(f, "Desktop"),
            Self::Tv => write!(f, "TV"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Discovery method/technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Multicast DNS / Network Service Discovery.
    Mdns,
    /// Bluetooth Low Energy scanning.
    BluetoothLe,
    /// Bluetooth Classic discovery.
    BluetoothClassic,
    /// QR code scan.
    QrCode,
    /// Manual IP entry.
    Manual,
}

impl std::fmt::Display for DiscoveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mdns => write!(f, "mDNS"),
            Self::BluetoothLe => write!(f, "Bluetooth LE"),
            Self::BluetoothClassic => write!(f, "Bluetooth"),
            Self::QrCode => write!(f, "QR Code"),
            Self::Manual => write!(f, "Manual"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Phone.to_string(), "Phone");
        assert_eq!(DeviceType::Desktop.to_string(), "Desktop");
        assert_eq!(DeviceType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_discovery_method_display() {
        assert_eq!(DiscoveryMethod::Mdns.to_string(), "mDNS");
        assert_eq!(DiscoveryMethod::BluetoothLe.to_string(), "Bluetooth LE");
        assert_eq!(DiscoveryMethod::QrCode.to_string(), "QR Code");
    }

    #[test]
    fn test_discovered_device_serialization() {
        let device = DiscoveredDevice {
            device_id: "dev-1".to_string(),
            device_name: "My Phone".to_string(),
            device_type: DeviceType::Phone,
            discovery_method: DiscoveryMethod::Mdns,
            address: Some("192.168.1.100:42000".to_string()),
            capabilities: vec!["file_transfer".to_string(), "streaming".to_string()],
            signal_strength: Some(85),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            is_trusted: false,
        };
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: DiscoveredDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.device_name, "My Phone");
        assert_eq!(deserialized.device_type, DeviceType::Phone);
    }
}
