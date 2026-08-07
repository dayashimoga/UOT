//! UOT Core Error Types
//!
//! Provides a comprehensive, typed error hierarchy for all UOT operations.
//! Uses `thiserror` for ergonomic error derivation and clear error messages.
use thiserror::Error;

/// Top-level error type for all UOT operations.
#[derive(Error, Debug)]
pub enum UotError {
    /// Transport-layer errors (connection, send, receive).
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Protocol-layer errors (invalid state, malformed messages).
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Security errors (authentication, encryption, authorization).
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    /// Discovery errors (mDNS, BLE, scan failures).
    #[error("Discovery error: {0}")]
    Discovery(#[from] DiscoveryError),

    /// Transfer engine errors (file I/O, chunking, integrity).
    #[error("Transfer error: {0}")]
    Transfer(#[from] TransferError),

    /// Streaming errors (media, codec, buffer).
    #[error("Streaming error: {0}")]
    Streaming(#[from] StreamingError),

    /// Configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O errors from the standard library.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Transport-specific errors.
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("Connection lost: {reason}")]
    ConnectionLost { reason: String },

    #[error("Send failed: {reason}")]
    SendFailed { reason: String },

    #[error("Receive failed: {reason}")]
    ReceiveFailed { reason: String },

    #[error("Transport not available: {transport}")]
    NotAvailable { transport: String },

    #[error("Connection timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Address already in use: {address}")]
    AddressInUse { address: String },
}

/// Protocol-specific errors.
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Malformed message: {reason}")]
    MalformedMessage { reason: String },

    #[error("Unsupported protocol version: {version}")]
    UnsupportedVersion { version: u32 },

    #[error("Session expired: {session_id}")]
    SessionExpired { session_id: String },

    #[error("Message too large: {size} bytes (max: {max_size})")]
    MessageTooLarge { size: u64, max_size: u64 },

    #[error("Unexpected message type: {message_type}")]
    UnexpectedMessage { message_type: String },
}

/// Security-specific errors.
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },

    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },

    #[error("Invalid certificate: {reason}")]
    InvalidCertificate { reason: String },

    #[error("Key generation failed: {reason}")]
    KeyGenerationFailed { reason: String },

    #[error("Session key expired")]
    SessionKeyExpired,

    #[error("Replay attack detected: nonce={nonce}")]
    ReplayDetected { nonce: String },

    #[error("Path traversal attempt: {path}")]
    PathTraversal { path: String },
}

/// Discovery-specific errors.
#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Scan failed: {reason}")]
    ScanFailed { reason: String },

    #[error("Service registration failed: {reason}")]
    RegistrationFailed { reason: String },

    #[error("Device not found: {device_id}")]
    DeviceNotFound { device_id: String },

    #[error("Discovery timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

/// Transfer engine errors.
#[derive(Error, Debug)]
pub enum TransferError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Integrity check failed: expected={expected}, got={actual}")]
    IntegrityFailed { expected: String, actual: String },

    #[error("Transfer cancelled: {transfer_id}")]
    Cancelled { transfer_id: String },

    #[error("Insufficient disk space: need={needed} bytes, available={available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("Chunk out of order: expected={expected}, got={actual}")]
    ChunkOutOfOrder { expected: u64, actual: u64 },

    #[error("Transfer not found: {transfer_id}")]
    TransferNotFound { transfer_id: String },

    #[error("Resume not possible: {reason}")]
    ResumeNotPossible { reason: String },
}

/// Streaming-specific errors.
#[derive(Error, Debug)]
pub enum StreamingError {
    #[error("Stream not supported: {capability}")]
    NotSupported { capability: String },

    #[error("Codec error: {reason}")]
    CodecError { reason: String },

    #[error("Buffer overflow: {reason}")]
    BufferOverflow { reason: String },

    #[error("Stream ended unexpectedly")]
    UnexpectedEnd,

    #[error("Permission denied for {resource}")]
    PermissionDenied { resource: String },
}

/// A specialized Result type for UOT operations.
pub type UotResult<T> = Result<T, UotError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_transport() {
        let err = TransportError::ConnectionFailed {
            reason: "host unreachable".to_string(),
        };
        assert_eq!(err.to_string(), "Connection failed: host unreachable");
    }

    #[test]
    fn test_error_display_protocol() {
        let err = ProtocolError::InvalidStateTransition {
            from: "IDLE".to_string(),
            to: "TRANSFER".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid state transition: IDLE -> TRANSFER"
        );
    }

    #[test]
    fn test_error_display_security() {
        let err = SecurityError::PathTraversal {
            path: "../../../etc/passwd".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Path traversal attempt: ../../../etc/passwd"
        );
    }

    #[test]
    fn test_error_display_transfer() {
        let err = TransferError::IntegrityFailed {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Integrity check failed: expected=abc123, got=def456"
        );
    }

    #[test]
    fn test_error_display_streaming() {
        let err = StreamingError::NotSupported {
            capability: "screen_capture".to_string(),
        };
        assert_eq!(err.to_string(), "Stream not supported: screen_capture");
    }

    #[test]
    fn test_error_display_discovery() {
        let err = DiscoveryError::Timeout { timeout_ms: 5000 };
        assert_eq!(err.to_string(), "Discovery timeout after 5000ms");
    }

    #[test]
    fn test_uot_error_from_transport() {
        let transport_err = TransportError::Timeout { timeout_ms: 3000 };
        let uot_err: UotError = transport_err.into();
        assert!(matches!(uot_err, UotError::Transport(_)));
        assert_eq!(
            uot_err.to_string(),
            "Transport error: Connection timeout after 3000ms"
        );
    }

    #[test]
    fn test_uot_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let uot_err: UotError = io_err.into();
        assert!(matches!(uot_err, UotError::Io(_)));
    }

    #[test]
    fn test_uot_result_type() {
        let ok_result: UotResult<i32> = Ok(42);
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: UotResult<i32> = Err(UotError::Config("bad config".to_string()));
        assert!(err_result.is_err());
    }
}
