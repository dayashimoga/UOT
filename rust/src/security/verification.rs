//! Security Module — PIN Verification & Session Tokens
//!
//! Provides PIN-based device verification and session token management.
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A 6-digit PIN for device verification.
#[derive(Debug, Clone)]
pub struct VerificationPin {
    pub pin: String,
    pub created_at: Instant,
    pub expires_at: Instant,
}

impl VerificationPin {
    /// Generate a new random 6-digit PIN.
    pub fn generate(ttl_secs: u64) -> Self {
        let mut rng = rand::rng();
        let pin = format!("{:06}", rng.random_range(0..1_000_000u32));
        let now = Instant::now();
        Self {
            pin,
            created_at: now,
            expires_at: now + Duration::from_secs(ttl_secs),
        }
    }

    /// Check if this PIN has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Verify a PIN attempt.
    pub fn verify(&self, attempt: &str) -> bool {
        !self.is_expired() && self.pin == attempt
    }
}

/// Session token for authenticated connections.
#[derive(Debug, Clone)]
pub struct VerificationSession {
    pub token: String,
    pub device_id: String,
    pub created_at: Instant,
    pub expires_at: Instant,
}

impl VerificationSession {
    /// Create a new session token.
    pub fn new(device_id: &str, ttl_secs: u64) -> Self {
        let mut rng = rand::rng();
        let random_bytes: [u8; 32] = rng.random();
        let mut hasher = Sha256::new();
        hasher.update(device_id.as_bytes());
        hasher.update(random_bytes);
        let token = hex::encode(hasher.finalize());

        let now = Instant::now();
        Self {
            token,
            device_id: device_id.to_string(),
            created_at: now,
            expires_at: now + Duration::from_secs(ttl_secs),
        }
    }

    /// Check if this token is still valid.
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

/// Manages trusted devices and sessions.
pub struct TrustManager {
    /// Device IDs that are trusted (persist across sessions).
    trusted_devices: HashMap<String, String>, // device_id -> device_name
    /// Active sessions.
    sessions: HashMap<String, VerificationSession>, // token -> session
    /// Current PIN (if any).
    current_pin: Option<VerificationPin>,
}

impl TrustManager {
    pub fn new() -> Self {
        Self {
            trusted_devices: HashMap::new(),
            sessions: HashMap::new(),
            current_pin: None,
        }
    }

    /// Trust a device.
    pub fn trust_device(&mut self, device_id: &str, device_name: &str) {
        self.trusted_devices
            .insert(device_id.to_string(), device_name.to_string());
    }

    /// Revoke trust.
    pub fn revoke_trust(&mut self, device_id: &str) {
        self.trusted_devices.remove(device_id);
    }

    /// Check if a device is trusted.
    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.trusted_devices.contains_key(device_id)
    }

    /// Get all trusted devices.
    pub fn trusted_devices(&self) -> Vec<(String, String)> {
        self.trusted_devices
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Generate a new PIN.
    pub fn generate_pin(&mut self, ttl_secs: u64) -> &str {
        self.current_pin = Some(VerificationPin::generate(ttl_secs));
        &self.current_pin.as_ref().unwrap().pin
    }

    /// Verify a PIN and create session if valid.
    pub fn verify_pin(&mut self, device_id: &str, attempt: &str) -> Option<String> {
        if let Some(ref pin) = self.current_pin {
            if pin.verify(attempt) {
                let session = VerificationSession::new(device_id, 3600);
                let token = session.token.clone();
                self.sessions.insert(token.clone(), session);
                self.current_pin = None; // Consume the PIN
                return Some(token);
            }
        }
        None
    }

    /// Validate a session token.
    pub fn validate_session(&self, token: &str) -> bool {
        self.sessions
            .get(token)
            .map(|s| s.is_valid())
            .unwrap_or(false)
    }

    /// Clean expired sessions.
    pub fn cleanup(&mut self) {
        self.sessions.retain(|_, s| s.is_valid());
        if let Some(ref pin) = self.current_pin {
            if pin.is_expired() {
                self.current_pin = None;
            }
        }
    }
}

impl Default for TrustManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_pin_lifecycle() {
        let pin = VerificationPin::generate(300);
        assert_eq!(pin.pin.len(), 6);
        assert!(pin.pin.chars().all(|c| c.is_ascii_digit()));
        assert!(!pin.is_expired());
        assert!(pin.verify(&pin.pin));
        assert!(!pin.verify("000000") || pin.pin == "000000");
        assert!(!pin.verify("1234567"));
    }

    #[test]
    fn test_trust_manager_pin_flow_and_single_use() {
        let mut tm = TrustManager::new();
        let pin = tm.generate_pin(300).to_string();

        // Wrong PIN attempt fails
        assert!(tm.verify_pin("device-1", "000000").is_none());

        // Correct PIN attempt succeeds and produces session token
        let token = tm.verify_pin("device-1", &pin);
        assert!(token.is_some());
        let token_str = token.unwrap();
        assert!(tm.validate_session(&token_str));

        // Second attempt with same PIN fails because PIN was consumed (single-use)
        assert!(tm.verify_pin("device-1", &pin).is_none());
    }

    #[test]
    fn test_trust_manager_device_trust_and_revoke() {
        let mut tm = TrustManager::new();
        assert!(!tm.is_trusted("dev-A"));

        tm.trust_device("dev-A", "Alice Phone");
        assert!(tm.is_trusted("dev-A"));
        assert_eq!(tm.trusted_devices().len(), 1);

        tm.revoke_trust("dev-A");
        assert!(!tm.is_trusted("dev-A"));
        assert!(tm.trusted_devices().is_empty());
    }
}
