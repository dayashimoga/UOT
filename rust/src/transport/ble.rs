//! Bluetooth Low Energy (BLE) Transport Module
//!
//! Provides GATT service definitions and framing for offline BLE discovery and fallback data transfer.
use serde::{Deserialize, Serialize};

/// Standard UOT GATT Service UUID string.
pub const UOT_BLE_SERVICE_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";
/// Characteristic for incoming control frames.
pub const UOT_BLE_CHAR_CONTROL: &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E";
/// Characteristic for incoming data chunks.
pub const UOT_BLE_CHAR_DATA: &str = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E";

/// BLE Advertisement Payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleAdvertisement {
    /// Short device display name.
    pub device_name: String,
    /// Truncated device ID hash.
    pub device_hash: String,
    /// Primary IP address if available for Wi-Fi upgrade.
    pub wifi_ip: Option<String>,
    /// Port for TCP upgrade.
    pub port: u16,
}

impl BleAdvertisement {
    /// Encode payload for BLE advertisement packet (max 31 bytes).
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Decode BLE advertisement packet.
    pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
