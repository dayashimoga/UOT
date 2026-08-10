//! Wi-Fi Direct & Hotspot Transport Module
//!
//! Provides Wi-Fi Direct P2P Group negotiation metadata and local AP hotspot creation payload.
use serde::{Deserialize, Serialize};

/// Wi-Fi Direct P2P Group Information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiDirectGroupInfo {
    /// Service Set Identifier (SSID).
    pub ssid: String,
    /// WPA2/WPA3 Pre-Shared Key.
    pub passphrase: String,
    /// Operating frequency / channel.
    pub frequency_mhz: u32,
    /// IPv4 Group Owner address.
    pub group_owner_ip: String,
    /// Port for incoming TCP connections.
    pub port: u16,
}

impl WifiDirectGroupInfo {
    /// Generate a temporary P2P group configuration.
    pub fn new_group(device_name: &str, port: u16) -> Self {
        let mut rng = rand::rng();
        let suffix: u16 = rand::Rng::random_range(&mut rng, 1000..9999);
        let ssid = format!("DIRECT-UOT-{device_name}-{suffix}");
        let passphrase = format!(
            "{:08}",
            rand::Rng::random_range(&mut rng, 10_000_000..99_999_999u32)
        );

        Self {
            ssid,
            passphrase,
            frequency_mhz: 5180, // 5GHz Band (Channel 36) default
            group_owner_ip: "192.168.49.1".to_string(),
            port,
        }
    }

    /// Serialize group info to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize group info from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wifidirect_group_info_inline() {
        let group = WifiDirectGroupInfo::new_group("TestDevice", 42000);
        assert!(group.ssid.contains("DIRECT-UOT-TestDevice"));
        assert_eq!(group.port, 42000);

        let json = group.to_json().unwrap();
        let parsed = WifiDirectGroupInfo::from_json(&json).unwrap();
        assert_eq!(parsed.ssid, group.ssid);
    }
}
