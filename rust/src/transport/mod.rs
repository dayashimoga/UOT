//! Transport Abstraction Layer
//!
//! Defines the trait interfaces for all transport implementations.
//! Each transport (Wi-Fi/LAN, Bluetooth, QR, USB, etc.) implements
//! these traits, allowing the Connection Orchestrator to select,
//! switch, and fall back between transports transparently.
pub mod tcp;
pub mod types;

use crate::core::error::TransportError;
use async_trait::async_trait;
use types::{TransportCapabilities, TransportId, TransportState, TransportStats};

/// A transport connection represents an active data channel to a peer.
#[async_trait]
pub trait TransportConnection: Send + Sync {
    /// Send raw bytes over this connection.
    async fn send(&self, data: &[u8]) -> Result<usize, TransportError>;

    /// Receive raw bytes from this connection.
    /// Returns the number of bytes read into the provided buffer.
    async fn receive(&self, buffer: &mut [u8]) -> Result<usize, TransportError>;

    /// Close this connection gracefully.
    async fn close(&self) -> Result<(), TransportError>;

    /// Check if the connection is still alive.
    fn is_connected(&self) -> bool;

    /// Get the current transport statistics.
    fn stats(&self) -> TransportStats;

    /// Get the transport identifier.
    fn transport_id(&self) -> TransportId;
}

/// A transport provider can create connections using a specific technology.
#[async_trait]
pub trait TransportProvider: Send + Sync {
    /// Unique identifier for this transport type.
    fn id(&self) -> TransportId;

    /// Human-readable name (e.g., "Wi-Fi Direct", "Bluetooth LE").
    fn name(&self) -> &str;

    /// Query the capabilities of this transport on the current platform.
    fn capabilities(&self) -> TransportCapabilities;

    /// Check if this transport is currently available.
    async fn is_available(&self) -> bool;

    /// Get the current state of this transport.
    fn state(&self) -> TransportState;

    /// Start listening for incoming connections.
    async fn listen(&self, port: u16) -> Result<(), TransportError>;

    /// Stop listening for incoming connections.
    async fn stop_listening(&self) -> Result<(), TransportError>;

    /// Connect to a peer at the given address.
    async fn connect(&self, address: &str) -> Result<Box<dyn TransportConnection>, TransportError>;

    /// Accept an incoming connection (blocks until one arrives or timeout).
    async fn accept(&self) -> Result<Box<dyn TransportConnection>, TransportError>;
}

/// Re-export async_trait for transport implementors.
pub use async_trait::async_trait as transport_async_trait;
