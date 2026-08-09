//! Session Cipher — Wire-level encryption for UOT transfers
//!
//! Wraps `SoftwareCryptoProvider` to encrypt/decrypt every data frame payload
//! using AES-256-GCM with incrementing nonce counters for replay protection.

use crate::core::error::SecurityError;
use crate::security::crypto::{SoftwareCryptoProvider, NONCE_LEN};
use crate::security::CryptoProvider;

/// Session cipher for encrypting/decrypting wire frames.
///
/// Uses a shared AES-256-GCM key derived from X25519 key exchange,
/// with a monotonically incrementing 64-bit nonce counter to prevent replay attacks.
pub struct SessionCipher {
    /// AES-256-GCM session key (32 bytes).
    session_key: Vec<u8>,
    /// Nonce counter for replay protection (increments per frame).
    nonce_counter: u64,
    /// Crypto provider for encrypt/decrypt operations.
    crypto: SoftwareCryptoProvider,
}

impl SessionCipher {
    /// Create a new session cipher from a shared key.
    ///
    /// The key MUST be exactly 32 bytes (derived from X25519 + HKDF-SHA256).
    pub fn new(session_key: Vec<u8>) -> Result<Self, SecurityError> {
        if session_key.len() != 32 {
            return Err(SecurityError::EncryptionFailed {
                reason: format!("Session key must be 32 bytes, got {}", session_key.len()),
            });
        }
        Ok(Self {
            session_key,
            nonce_counter: 0,
            crypto: SoftwareCryptoProvider::new(),
        })
    }

    /// Create a pair of session ciphers from X25519 key exchange.
    ///
    /// Returns `(our_cipher, their_public_key_bytes)` — send the public key
    /// to the remote peer so they can derive the same shared secret.
    pub fn create_key_exchange() -> Result<(Vec<u8>, Vec<u8>), SecurityError> {
        let crypto = SoftwareCryptoProvider::new();
        let kp = crypto.generate_key_pair()?;
        Ok((kp.private_key, kp.public_key))
    }

    /// Derive a session cipher from our private key and their public key.
    pub fn from_key_exchange(
        our_private: &[u8],
        their_public: &[u8],
    ) -> Result<Self, SecurityError> {
        let crypto = SoftwareCryptoProvider::new();
        let shared = crypto.derive_shared_secret(our_private, their_public)?;
        Self::new(shared)
    }

    /// Build a 12-byte nonce from the counter value.
    ///
    /// Format: [0u8; 4] ++ counter.to_be_bytes() — ensures unique nonce per frame.
    fn build_nonce(&self) -> Vec<u8> {
        let mut nonce = vec![0u8; NONCE_LEN];
        nonce[4..12].copy_from_slice(&self.nonce_counter.to_be_bytes());
        nonce
    }

    /// Encrypt a plaintext payload, returning `nonce_counter (8 bytes) || ciphertext`.
    ///
    /// The nonce counter is prepended so the receiver can reconstruct the nonce.
    /// Increments the internal counter after each call.
    pub fn encrypt_frame(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let nonce = self.build_nonce();
        let ciphertext = self.crypto.encrypt(&self.session_key, plaintext, &nonce)?;

        // Prepend the 8-byte counter so receiver can reconstruct nonce
        let mut frame = Vec::with_capacity(8 + ciphertext.len());
        frame.extend_from_slice(&self.nonce_counter.to_be_bytes());
        frame.extend_from_slice(&ciphertext);

        self.nonce_counter += 1;
        Ok(frame)
    }

    /// Decrypt an encrypted frame (expected format: `nonce_counter (8 bytes) || ciphertext`).
    ///
    /// Verifies that the nonce counter matches the expected sequence to prevent replay.
    pub fn decrypt_frame(&mut self, encrypted: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if encrypted.len() < 8 {
            return Err(SecurityError::DecryptionFailed {
                reason: "Encrypted frame too short for nonce counter".to_string(),
            });
        }

        let received_counter = u64::from_be_bytes(encrypted[..8].try_into().unwrap());

        // Replay protection: counter must match or be ahead (allow for packet reordering)
        if received_counter < self.nonce_counter {
            return Err(SecurityError::DecryptionFailed {
                reason: format!(
                    "Replay detected: received counter {} < expected {}",
                    received_counter, self.nonce_counter
                ),
            });
        }

        // Advance our counter to match
        self.nonce_counter = received_counter;

        let mut nonce = vec![0u8; NONCE_LEN];
        nonce[4..12].copy_from_slice(&received_counter.to_be_bytes());

        let ciphertext = &encrypted[8..];
        let plaintext = self.crypto.decrypt(&self.session_key, ciphertext, &nonce)?;

        self.nonce_counter += 1;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_cipher_roundtrip() {
        let key = vec![0x42u8; 32];
        let mut encryptor = SessionCipher::new(key.clone()).unwrap();
        let mut decryptor = SessionCipher::new(key).unwrap();

        let plaintext = b"Hello, UOT encrypted transfer!";
        let encrypted = encryptor.encrypt_frame(plaintext).unwrap();

        assert_ne!(&encrypted[8..], plaintext); // ciphertext differs from plaintext
        assert!(encrypted.len() > plaintext.len()); // includes nonce counter + tag

        let decrypted = decryptor.decrypt_frame(&encrypted).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_session_cipher_multi_frame() {
        let key = vec![0xAB; 32];
        let mut enc = SessionCipher::new(key.clone()).unwrap();
        let mut dec = SessionCipher::new(key).unwrap();

        for i in 0..10 {
            let msg = format!("Frame {i}");
            let encrypted = enc.encrypt_frame(msg.as_bytes()).unwrap();
            let decrypted = dec.decrypt_frame(&encrypted).unwrap();
            assert_eq!(String::from_utf8(decrypted).unwrap(), msg);
        }
    }

    #[test]
    fn test_session_cipher_replay_detection() {
        let key = vec![0xCD; 32];
        let mut enc = SessionCipher::new(key.clone()).unwrap();
        let mut dec = SessionCipher::new(key).unwrap();

        let frame0 = enc.encrypt_frame(b"first").unwrap();
        let frame1 = enc.encrypt_frame(b"second").unwrap();

        // Decrypt frame 0 first
        dec.decrypt_frame(&frame0).unwrap();
        // Decrypt frame 1
        dec.decrypt_frame(&frame1).unwrap();

        // Replay frame 0 — should fail (counter too low)
        let result = dec.decrypt_frame(&frame0);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_cipher_tamper_detection() {
        let key = vec![0xEF; 32];
        let mut enc = SessionCipher::new(key.clone()).unwrap();
        let mut dec = SessionCipher::new(key).unwrap();

        let mut encrypted = enc.encrypt_frame(b"sensitive data").unwrap();
        // Tamper with ciphertext byte
        if encrypted.len() > 10 {
            encrypted[10] ^= 0xFF;
        }

        let result = dec.decrypt_frame(&encrypted);
        assert!(result.is_err()); // GCM tag verification fails
    }

    #[test]
    fn test_session_cipher_key_exchange_roundtrip() {
        let (private_a, public_a) = SessionCipher::create_key_exchange().unwrap();
        let (private_b, public_b) = SessionCipher::create_key_exchange().unwrap();

        let mut cipher_a = SessionCipher::from_key_exchange(&private_a, &public_b).unwrap();
        let mut cipher_b = SessionCipher::from_key_exchange(&private_b, &public_a).unwrap();

        let msg = b"Key exchange works!";
        let encrypted = cipher_a.encrypt_frame(msg).unwrap();
        let decrypted = cipher_b.decrypt_frame(&encrypted).unwrap();
        assert_eq!(&decrypted, msg);
    }

    #[test]
    fn test_session_cipher_wrong_key_fails() {
        let mut enc = SessionCipher::new(vec![0x11; 32]).unwrap();
        let mut dec = SessionCipher::new(vec![0x22; 32]).unwrap(); // Different key

        let encrypted = enc.encrypt_frame(b"wrong key test").unwrap();
        let result = dec.decrypt_frame(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_cipher_invalid_key_length() {
        let result = SessionCipher::new(vec![0x11; 16]); // 16 bytes instead of 32
        assert!(result.is_err());
    }
}
