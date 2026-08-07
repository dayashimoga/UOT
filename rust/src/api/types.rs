//! UOT Shared API Types
//!
//! Types that are exposed to Flutter/Dart via flutter_rust_bridge.
//! These are simplified representations of internal types, designed
//! for efficient FFI transport.
use serde::{Deserialize, Serialize};

/// Connection status displayed to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Human-readable connection status.
    pub status: String,
    /// Transport name (e.g., "Wi-Fi", "Bluetooth").
    pub transport: Option<String>,
    /// Current speed display (e.g., "72 MB/s").
    pub speed: Option<String>,
    /// Whether connected.
    pub is_connected: bool,
}

/// Simplified device info for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Device type icon identifier.
    pub device_type: String,
    /// Whether trusted/paired.
    pub is_trusted: bool,
    /// Signal quality (0-100, None if not applicable).
    pub signal: Option<u8>,
}

/// Platform capability summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    /// Whether file transfer is supported.
    pub file_transfer: bool,
    /// Whether folder transfer is supported.
    pub folder_transfer: bool,
    /// Whether clipboard sharing is supported.
    pub clipboard: bool,
    /// Whether camera streaming is supported.
    pub camera_stream: bool,
    /// Whether screen sharing is supported.
    pub screen_share: bool,
    /// Whether video file streaming is supported.
    pub video_stream: bool,
    /// Whether audio file streaming is supported.
    pub audio_stream: bool,
    /// Whether drag-and-drop is supported.
    pub drag_drop: bool,
    /// Whether share sheet integration is supported.
    pub share_sheet: bool,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            file_transfer: true,
            folder_transfer: true,
            clipboard: true,
            camera_stream: false,
            screen_share: false,
            video_stream: false,
            audio_stream: false,
            drag_drop: false,
            share_sheet: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_info_serialization() {
        let info = ConnectionInfo {
            status: "Connected".to_string(),
            transport: Some("Wi-Fi".to_string()),
            speed: Some("72 MB/s".to_string()),
            is_connected: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Connected"));
        assert!(json.contains("Wi-Fi"));
    }

    #[test]
    fn test_platform_capabilities_default() {
        let caps = PlatformCapabilities::default();
        assert!(caps.file_transfer);
        assert!(caps.folder_transfer);
        assert!(caps.clipboard);
        assert!(!caps.camera_stream);
        assert!(!caps.screen_share);
    }
}
