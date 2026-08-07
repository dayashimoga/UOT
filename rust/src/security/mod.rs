//! Security Module
//!
//! Defines trait interfaces for encryption, authentication,
//! key management, and security validation.
pub mod types;

use crate::core::error::SecurityError;

/// Trait for cryptographic operations.
pub trait CryptoProvider: Send + Sync {
    /// Generate a new ephemeral key pair for session establishment.
    fn generate_key_pair(&self) -> Result<KeyPair, SecurityError>;

    /// Derive a shared secret from a key exchange.
    fn derive_shared_secret(
        &self,
        our_private: &[u8],
        their_public: &[u8],
    ) -> Result<Vec<u8>, SecurityError>;

    /// Encrypt data with the given session key.
    fn encrypt(&self, key: &[u8], plaintext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, SecurityError>;

    /// Decrypt data with the given session key.
    fn decrypt(
        &self,
        key: &[u8],
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> Result<Vec<u8>, SecurityError>;

    /// Generate a cryptographic nonce.
    fn generate_nonce(&self) -> Vec<u8>;

    /// Compute a hash of the given data (SHA-256).
    fn hash(&self, data: &[u8]) -> Vec<u8>;
}

/// A cryptographic key pair.
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Public key bytes.
    pub public_key: Vec<u8>,
    /// Private key bytes (never log or serialize this).
    pub private_key: Vec<u8>,
}

/// Trait for validating file paths against security threats.
pub trait PathValidator: Send + Sync {
    /// Validate that a filename is safe (no traversal, no invalid chars).
    fn validate_filename(&self, filename: &str) -> Result<String, SecurityError>;

    /// Validate that a relative path is safe (no traversal, no symlinks).
    fn validate_relative_path(&self, path: &str) -> Result<String, SecurityError>;

    /// Sanitize a filename, removing or replacing dangerous characters.
    fn sanitize_filename(&self, filename: &str) -> String;
}
