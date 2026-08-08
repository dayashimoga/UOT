//! QR Invitation & Optical Pairing Module
//!
//! Generates and parses secure encrypted QR codes for out-of-band device pairing.
use serde::{Deserialize, Serialize};

/// QR invitation content for device pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrInvitation {
    /// Device display name.
    pub device_name: String,
    /// Unique device ID.
    pub device_id: String,
    /// Ephemeral public key (base64 encoded).
    pub public_key: String,
    /// Socket address (IP:port) for direct connection.
    pub address: String,
    /// Single-use OTP PIN for authentication.
    pub pin: String,
    /// Timestamp when invitation expires (seconds since UNIX epoch).
    pub expires_at: u64,
}

impl QrInvitation {
    /// Create a new QR invitation.
    pub fn new(
        device_name: String,
        device_id: String,
        public_key: String,
        address: String,
        pin: String,
        ttl_secs: u64,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            device_name,
            device_id,
            public_key,
            address,
            pin,
            expires_at: now + ttl_secs,
        }
    }

    /// Serialize to compact JSON string suitable for QR code generation.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse QR invitation from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Check if invitation has expired.
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at
    }
}
