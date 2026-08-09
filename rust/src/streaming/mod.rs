//! Media Streaming Module
//!
//! Defines trait interfaces and types for local media streaming.
//! Supports video, audio, camera, and screen streaming where
//! platform capabilities allow.
pub mod capture;
pub mod manager;
pub mod pipeline;
pub mod types;

use types::{StreamCapability, StreamStatus};

/// Trait for streaming capability detection.
pub trait StreamCapabilityDetector: Send + Sync {
    /// Get available streaming capabilities on the current platform.
    fn available_capabilities(&self) -> Vec<StreamCapability>;

    /// Check if a specific capability is available.
    fn is_capable(&self, capability: &StreamCapability) -> bool;

    /// Get the reason a capability is unavailable (if applicable).
    fn unavailable_reason(&self, capability: &StreamCapability) -> Option<String>;
}

/// Trait for managing media streams.
pub trait StreamManager: Send + Sync {
    /// Get the current stream status.
    fn status(&self) -> StreamStatus;

    /// Check if a stream is currently active.
    fn is_active(&self) -> bool;
}
