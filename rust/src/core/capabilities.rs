//! Platform Capabilities Detection
//!
//! Provides runtime detection of available transport and feature capabilities
//! based on the current platform. This ensures the UI never shows unsupported
//! options and downstream code receives truthful capability information.
//!
//! # Platform Support Matrix
//!
//! | Feature           | Android | iOS | Windows | macOS | Linux | Web |
//! |-------------------|---------|-----|---------|-------|-------|-----|
//! | TCP/LAN           | ✅      | ✅  | ✅      | ✅    | ✅    | ❌  |
//! | mDNS Discovery    | ✅      | ✅  | ✅      | ✅    | ✅    | ❌  |
//! | Subnet Scanner    | ✅      | ✅  | ✅      | ✅    | ✅    | ❌  |
//! | BLE GATT          | ✅      | ✅  | ❌      | ❌    | ❌    | ❌  |
//! | Wi-Fi Direct      | ✅      | ❌  | ❌      | ❌    | ❌    | ❌  |
//! | Camera QR Scanner | ✅      | ✅  | ❌      | ❌    | ❌    | ❌  |
//! | Hotspot Creation  | ✅      | ❌  | ✅      | ✅    | ✅    | ❌  |
//! | Hardware Codecs   | ✅      | ✅  | ❌      | ❌    | ❌    | ❌  |

use serde::{Deserialize, Serialize};

/// Detected platform capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// Current platform name.
    pub platform: String,
    /// TCP/LAN transport is available.
    pub tcp_transport: bool,
    /// mDNS device discovery is available.
    pub mdns_discovery: bool,
    /// IPv4 subnet scanner fallback is available.
    pub subnet_scanner: bool,
    /// BLE GATT transport is available (requires Bluetooth hardware).
    pub ble_gatt: bool,
    /// Wi-Fi Direct P2P is available (Android only).
    pub wifi_direct: bool,
    /// Camera QR code scanner is available (mobile only).
    pub camera_qr_scanner: bool,
    /// Hotspot creation is available.
    pub hotspot_creation: bool,
    /// Hardware H.264/AAC codec acceleration is available (mobile only).
    pub hardware_codecs: bool,
    /// Animated QR fountain code transport is available.
    pub fountain_qr: bool,
    /// AES-256-GCM + X25519 encryption is available.
    pub encryption: bool,
}

impl PlatformCapabilities {
    /// Detect capabilities for the current compile-time platform.
    pub fn detect() -> Self {
        let platform = Self::current_platform();

        Self {
            platform: platform.to_string(),
            // Core features available on all desktop/mobile platforms
            tcp_transport: !cfg!(target_arch = "wasm32"),
            mdns_discovery: !cfg!(target_arch = "wasm32"),
            subnet_scanner: !cfg!(target_arch = "wasm32"),
            encryption: true, // Software-only, always available

            // Fountain QR: always available (software encoder/decoder)
            fountain_qr: true,

            // BLE GATT: only on Android and iOS
            ble_gatt: cfg!(target_os = "android") || cfg!(target_os = "ios"),

            // Wi-Fi Direct: Android only
            wifi_direct: cfg!(target_os = "android"),

            // Camera QR scanner: mobile only
            camera_qr_scanner: cfg!(target_os = "android") || cfg!(target_os = "ios"),

            // Hotspot creation: available on Android, Windows, macOS, Linux
            hotspot_creation: cfg!(target_os = "android")
                || cfg!(target_os = "windows")
                || cfg!(target_os = "macos")
                || cfg!(target_os = "linux"),

            // Hardware codecs: mobile only
            hardware_codecs: cfg!(target_os = "android") || cfg!(target_os = "ios"),
        }
    }

    /// Returns the current platform as a string.
    fn current_platform() -> &'static str {
        if cfg!(target_os = "android") {
            "android"
        } else if cfg!(target_os = "ios") {
            "ios"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_arch = "wasm32") {
            "web"
        } else {
            "unknown"
        }
    }

    /// Get a list of supported transport names for the current platform.
    pub fn supported_transports(&self) -> Vec<&'static str> {
        let mut transports = Vec::new();
        if self.tcp_transport {
            transports.push("tcp");
        }
        if self.ble_gatt {
            transports.push("ble_gatt");
        }
        if self.wifi_direct {
            transports.push("wifi_direct");
        }
        if self.fountain_qr {
            transports.push("fountain_qr");
        }
        transports
    }

    /// Get a list of unsupported features with reasons.
    pub fn unsupported_features(&self) -> Vec<(&'static str, &'static str)> {
        let mut unsupported = Vec::new();
        if !self.ble_gatt {
            unsupported.push(("BLE GATT", "Requires Android/iOS with Bluetooth hardware"));
        }
        if !self.wifi_direct {
            unsupported.push(("Wi-Fi Direct", "Requires Android with Wi-Fi Direct support"));
        }
        if !self.camera_qr_scanner {
            unsupported.push((
                "Camera QR Scanner",
                "Requires mobile device with camera access",
            ));
        }
        if !self.hardware_codecs {
            unsupported.push((
                "Hardware Codecs",
                "Requires mobile device with MediaCodec/VideoToolbox",
            ));
        }
        unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let caps = PlatformCapabilities::detect();

        // Core features always available (we're not on wasm32 in tests)
        assert!(caps.tcp_transport);
        assert!(caps.mdns_discovery);
        assert!(caps.subnet_scanner);
        assert!(caps.encryption);
        assert!(caps.fountain_qr);

        // Platform string should be one of the known values
        assert!(
            ["android", "ios", "windows", "macos", "linux", "web", "unknown"]
                .contains(&caps.platform.as_str())
        );
    }

    #[test]
    fn test_supported_transports() {
        let caps = PlatformCapabilities::detect();
        let transports = caps.supported_transports();
        assert!(transports.contains(&"tcp"));
        assert!(transports.contains(&"fountain_qr"));
    }

    #[test]
    fn test_unsupported_features_desktop() {
        let caps = PlatformCapabilities::detect();
        if !cfg!(target_os = "android") && !cfg!(target_os = "ios") {
            let unsupported = caps.unsupported_features();
            // On desktop, BLE, camera, hardware codecs should be unsupported
            let names: Vec<&str> = unsupported.iter().map(|(name, _)| *name).collect();
            assert!(names.contains(&"BLE GATT"));
            assert!(names.contains(&"Camera QR Scanner"));
            assert!(names.contains(&"Hardware Codecs"));
        }
    }
}
