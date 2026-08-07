//! Security Types
//!
//! Shared types for security operations.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A trusted device record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    /// Unique device identifier.
    pub device_id: String,
    /// Human-readable device name.
    pub device_name: String,
    /// Public key fingerprint.
    pub fingerprint: String,
    /// When this device was first trusted.
    pub trusted_since: DateTime<Utc>,
    /// Last time this device was seen.
    pub last_seen: DateTime<Utc>,
    /// Whether this device is currently blocked.
    pub blocked: bool,
}

/// A session token with expiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    /// Session identifier.
    pub session_id: Uuid,
    /// When this session was created.
    pub created_at: DateTime<Utc>,
    /// When this session expires.
    pub expires_at: DateTime<Utc>,
    /// Device ID of the remote peer.
    pub remote_device_id: String,
}

impl SessionToken {
    /// Check if this session has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// QR invitation data for secure pairing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrInvitation {
    /// Invitation token.
    pub token: String,
    /// Sender device name.
    pub device_name: String,
    /// Sender's public key.
    pub public_key: Vec<u8>,
    /// IP address/port for connection.
    pub address: Option<String>,
    /// When this invitation expires.
    pub expires_at: DateTime<Utc>,
    /// Available transports.
    pub transports: Vec<String>,
}

impl QrInvitation {
    /// Check if this invitation has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_session_token_not_expired() {
        let token = SessionToken {
            session_id: Uuid::new_v4(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
            remote_device_id: "device-1".to_string(),
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn test_session_token_expired() {
        let token = SessionToken {
            session_id: Uuid::new_v4(),
            created_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::hours(1),
            remote_device_id: "device-1".to_string(),
        };
        assert!(token.is_expired());
    }

    #[test]
    fn test_qr_invitation_not_expired() {
        let invitation = QrInvitation {
            token: "test-token".to_string(),
            device_name: "Test Device".to_string(),
            public_key: vec![1, 2, 3],
            address: Some("192.168.1.1:8080".to_string()),
            expires_at: Utc::now() + Duration::minutes(5),
            transports: vec!["tcp_lan".to_string()],
        };
        assert!(!invitation.is_expired());
    }

    #[test]
    fn test_qr_invitation_expired() {
        let invitation = QrInvitation {
            token: "old-token".to_string(),
            device_name: "Old Device".to_string(),
            public_key: vec![],
            address: None,
            expires_at: Utc::now() - Duration::minutes(1),
            transports: vec![],
        };
        assert!(invitation.is_expired());
    }

    #[test]
    fn test_trusted_device_serialization() {
        let device = TrustedDevice {
            device_id: "dev-123".to_string(),
            device_name: "My Phone".to_string(),
            fingerprint: "AA:BB:CC:DD".to_string(),
            trusted_since: Utc::now(),
            last_seen: Utc::now(),
            blocked: false,
        };
        let json = serde_json::to_string(&device).unwrap();
        let deserialized: TrustedDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.device_id, "dev-123");
        assert!(!deserialized.blocked);
    }
}
