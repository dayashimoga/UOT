//! Temporary Hotspot Creation Assist Module
//!
//! Provides configuration, interface selection, and status monitoring for temporary Wi-Fi Access Points.
use serde::{Deserialize, Serialize};

/// State of local Access Point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotspotState {
    Disabled,
    Enabling,
    Active,
    Error,
}

/// Hotspot configuration details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotConfig {
    pub ssid: String,
    pub password: String,
    pub band_5ghz: bool,
    pub ip_address: String,
    pub port: u16,
    pub state: HotspotState,
}

impl HotspotConfig {
    /// Create temporary hotspot configuration.
    pub fn create_temp(device_name: &str, port: u16) -> Self {
        let mut rng = rand::rng();
        let pass: u32 = rand::Rng::random_range(&mut rng, 10_000_000..99_999_999);
        Self {
            ssid: format!("UOT-{device_name}"),
            password: pass.to_string(),
            band_5ghz: true,
            ip_address: "192.168.43.1".to_string(),
            port,
            state: HotspotState::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotspot_config_inline() {
        let hs = HotspotConfig::create_temp("TestDevice", 42000);
        assert_eq!(hs.ssid, "UOT-TestDevice");
        assert_eq!(hs.state, HotspotState::Disabled);

        let hs_json = serde_json::to_string(&hs).unwrap();
        let hs_parsed: HotspotConfig = serde_json::from_str(&hs_json).unwrap();
        assert_eq!(hs_parsed.ssid, hs.ssid);
    }
}
