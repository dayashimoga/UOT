//! Cryptographic Session Encryption (AES-256-GCM)
//!
//! Provides authenticated encryption and decryption for protocol frames
//! using AES-256-GCM and X25519 Diffie-Hellman key exchange.
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::core::error::SecurityError;
use crate::security::{CryptoProvider, KeyPair};

/// AES-256-GCM key length (32 bytes).
pub const KEY_LEN: usize = 32;
/// AES-256-GCM nonce length (12 bytes).
pub const NONCE_LEN: usize = 12;
/// AES-256-GCM authentication tag length (16 bytes, appended by aes-gcm).
pub const TAG_LEN: usize = 16;

/// Production AES-256-GCM + X25519 implementation of `CryptoProvider`.
#[derive(Debug, Clone, Default)]
pub struct SoftwareCryptoProvider {}

impl SoftwareCryptoProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl CryptoProvider for SoftwareCryptoProvider {
    /// Generate an X25519 key pair for Diffie-Hellman key exchange.
    ///
    /// The private key is a 32-byte X25519 static secret.
    /// The public key is the corresponding 32-byte X25519 public key.
    fn generate_key_pair(&self) -> Result<KeyPair, SecurityError> {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);

        Ok(KeyPair {
            public_key: public.as_bytes().to_vec(),
            private_key: secret.to_bytes().to_vec(),
        })
    }

    /// Derive a shared secret using X25519 Diffie-Hellman, then HKDF-SHA256.
    ///
    /// The resulting 32-byte key is suitable for AES-256-GCM.
    fn derive_shared_secret(
        &self,
        our_private: &[u8],
        their_public: &[u8],
    ) -> Result<Vec<u8>, SecurityError> {
        if our_private.len() != 32 || their_public.len() != 32 {
            return Err(SecurityError::KeyExchangeFailed {
                reason: format!(
                    "Invalid key length: private={}, public={}",
                    our_private.len(),
                    their_public.len()
                ),
            });
        }

        let mut private_bytes = [0u8; 32];
        private_bytes.copy_from_slice(our_private);
        let secret = StaticSecret::from(private_bytes);

        let mut public_bytes = [0u8; 32];
        public_bytes.copy_from_slice(their_public);
        let their_public = PublicKey::from(public_bytes);

        let shared = secret.diffie_hellman(&their_public);

        // HKDF-extract using SHA-256 to derive a uniformly random AES key
        let mut hasher = Sha256::new();
        hasher.update(b"UOT-session-key-v1"); // domain separation
        hasher.update(shared.as_bytes());
        let derived_key = hasher.finalize().to_vec();

        Ok(derived_key)
    }

    /// Encrypt data using AES-256-GCM authenticated encryption.
    ///
    /// Returns ciphertext with 16-byte authentication tag appended.
    fn encrypt(
        &self,
        key: &[u8],
        plaintext: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, SecurityError> {
        if key.len() != KEY_LEN {
            return Err(SecurityError::EncryptionFailed {
                reason: format!("Key must be {} bytes, got {}", KEY_LEN, key.len()),
            });
        }
        if nonce.len() != NONCE_LEN {
            return Err(SecurityError::EncryptionFailed {
                reason: format!("Nonce must be {} bytes, got {}", NONCE_LEN, nonce.len()),
            });
        }

        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| SecurityError::EncryptionFailed {
                reason: format!("Failed to create cipher: {e}"),
            })?;

        let nonce = Nonce::from_slice(nonce);
        cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| SecurityError::EncryptionFailed {
                reason: format!("AES-256-GCM encryption failed: {e}"),
            })
    }

    /// Decrypt data using AES-256-GCM authenticated decryption.
    ///
    /// Expects ciphertext with 16-byte authentication tag appended.
    /// Returns an error if the tag verification fails (tampered data).
    fn decrypt(
        &self,
        key: &[u8],
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, SecurityError> {
        if key.len() != KEY_LEN {
            return Err(SecurityError::DecryptionFailed {
                reason: format!("Key must be {} bytes, got {}", KEY_LEN, key.len()),
            });
        }
        if nonce.len() != NONCE_LEN {
            return Err(SecurityError::DecryptionFailed {
                reason: format!("Nonce must be {} bytes, got {}", NONCE_LEN, nonce.len()),
            });
        }
        if ciphertext.len() < TAG_LEN {
            return Err(SecurityError::DecryptionFailed {
                reason: format!(
                    "Ciphertext too short ({} bytes, need at least {} for tag)",
                    ciphertext.len(),
                    TAG_LEN
                ),
            });
        }

        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| SecurityError::DecryptionFailed {
                reason: format!("Failed to create cipher: {e}"),
            })?;

        let nonce = Nonce::from_slice(nonce);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecurityError::DecryptionFailed {
                reason: format!("AES-256-GCM decryption failed (tampered or wrong key): {e}"),
            })
    }

    /// Generate a cryptographically secure random 12-byte nonce.
    fn generate_nonce(&self) -> Vec<u8> {
        let mut nonce = vec![0u8; NONCE_LEN];
        let mut rng = rand::rng();
        rand::Rng::fill(&mut rng, nonce.as_mut_slice());
        nonce
    }

    /// Compute SHA-256 hash of the given data.
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_pair() {
        let provider = SoftwareCryptoProvider::new();
        let kp = provider.generate_key_pair().unwrap();
        assert_eq!(kp.public_key.len(), 32);
        assert_eq!(kp.private_key.len(), 32);
        // Keys should be different
        assert_ne!(kp.public_key, kp.private_key);
    }

    #[test]
    fn test_key_exchange_shared_secret() {
        let provider = SoftwareCryptoProvider::new();
        let alice = provider.generate_key_pair().unwrap();
        let bob = provider.generate_key_pair().unwrap();

        let secret_a = provider
            .derive_shared_secret(&alice.private_key, &bob.public_key)
            .unwrap();
        let secret_b = provider
            .derive_shared_secret(&bob.private_key, &alice.public_key)
            .unwrap();

        assert_eq!(secret_a, secret_b, "DH shared secrets must match");
        assert_eq!(secret_a.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let provider = SoftwareCryptoProvider::new();
        let kp = provider.generate_key_pair().unwrap();

        // Use private key as session key (32 bytes)
        let key = &kp.private_key;
        let nonce = provider.generate_nonce();
        let plaintext = b"Hello, AES-256-GCM! This is a production-grade test.";

        let ciphertext = provider.encrypt(key, plaintext, &nonce).unwrap();
        assert_ne!(&ciphertext[..plaintext.len()], &plaintext[..]);

        let decrypted = provider.decrypt(key, &ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_empty_data() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0xABu8; KEY_LEN];
        let nonce = provider.generate_nonce();

        let ciphertext = provider.encrypt(&key, b"", &nonce).unwrap();
        // Empty plaintext still produces a tag
        assert_eq!(ciphertext.len(), TAG_LEN);

        let decrypted = provider.decrypt(&key, &ciphertext, &nonce).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_decrypt_tampered_data() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0x42u8; KEY_LEN];
        let nonce = provider.generate_nonce();
        let plaintext = b"Sensitive data";

        let mut ciphertext = provider.encrypt(&key, plaintext, &nonce).unwrap();
        // Tamper with one byte
        if let Some(byte) = ciphertext.get_mut(0) {
            *byte ^= 0xFF;
        }

        let result = provider.decrypt(&key, &ciphertext, &nonce);
        assert!(result.is_err(), "Tampered data must fail decryption");
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let provider = SoftwareCryptoProvider::new();
        let key1 = vec![0x11u8; KEY_LEN];
        let key2 = vec![0x22u8; KEY_LEN];
        let nonce = provider.generate_nonce();

        let ciphertext = provider.encrypt(&key1, b"secret", &nonce).unwrap();
        let result = provider.decrypt(&key2, &ciphertext, &nonce);
        assert!(result.is_err(), "Wrong key must fail decryption");
    }

    #[test]
    fn test_decrypt_wrong_nonce() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0x33u8; KEY_LEN];
        let nonce1 = provider.generate_nonce();
        let nonce2 = provider.generate_nonce();

        let ciphertext = provider.encrypt(&key, b"secret", &nonce1).unwrap();
        let result = provider.decrypt(&key, &ciphertext, &nonce2);
        assert!(result.is_err(), "Wrong nonce must fail decryption");
    }

    #[test]
    fn test_invalid_key_length() {
        let provider = SoftwareCryptoProvider::new();
        let short_key = vec![0u8; 16]; // AES-128, not AES-256
        let nonce = provider.generate_nonce();

        assert!(provider.encrypt(&short_key, b"data", &nonce).is_err());
        assert!(provider.decrypt(&short_key, &[0u8; 32], &nonce).is_err());
    }

    #[test]
    fn test_invalid_nonce_length() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0u8; KEY_LEN];
        let short_nonce = vec![0u8; 8]; // Too short

        assert!(provider.encrypt(&key, b"data", &short_nonce).is_err());
    }

    #[test]
    fn test_ciphertext_too_short() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0u8; KEY_LEN];
        let nonce = provider.generate_nonce();

        let result = provider.decrypt(&key, &[0u8; 8], &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonce_uniqueness() {
        let provider = SoftwareCryptoProvider::new();
        let nonces: Vec<Vec<u8>> = (0..100).map(|_| provider.generate_nonce()).collect();
        for i in 0..nonces.len() {
            for j in (i + 1)..nonces.len() {
                assert_ne!(nonces[i], nonces[j], "Nonces must be unique");
            }
        }
    }

    #[test]
    fn test_hash() {
        let provider = SoftwareCryptoProvider::new();
        let hash = provider.hash(b"hello");
        assert_eq!(hash.len(), 32);
        // Known SHA-256 of "hello"
        assert_eq!(
            hex::encode(&hash),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_derive_shared_secret_invalid_lengths() {
        let provider = SoftwareCryptoProvider::new();
        let result = provider.derive_shared_secret(&[0u8; 16], &[0u8; 32]);
        assert!(result.is_err());
        let result = provider.derive_shared_secret(&[0u8; 32], &[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_payload_encryption() {
        let provider = SoftwareCryptoProvider::new();
        let key = vec![0x55u8; KEY_LEN];
        let nonce = provider.generate_nonce();
        // 1MB payload
        let plaintext = vec![0xAAu8; 1024 * 1024];

        let ciphertext = provider.encrypt(&key, &plaintext, &nonce).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_LEN);

        let decrypted = provider.decrypt(&key, &ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
