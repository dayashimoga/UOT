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
