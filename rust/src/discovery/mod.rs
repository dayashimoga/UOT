//! Device Discovery Module
//!
//! Defines trait interfaces for discovering nearby devices
//! using various mechanisms (mDNS, BLE, QR, etc.).
pub mod types;

use types::{DiscoveredDevice, DiscoveryMethod};

/// Trait for device discovery providers.
pub trait DiscoveryProvider: Send + Sync {
    /// The discovery method this provider uses.
    fn method(&self) -> DiscoveryMethod;

    /// Human-readable name of this discovery provider.
    fn name(&self) -> &str;

    /// Whether this discovery method is available on the current platform.
    fn is_available(&self) -> bool;
}

/// Trait for the discovery service that aggregates multiple providers.
pub trait DiscoveryService: Send + Sync {
    /// Get the list of currently discovered devices.
    fn discovered_devices(&self) -> Vec<DiscoveredDevice>;

    /// Get a specific discovered device by ID.
    fn get_device(&self, device_id: &str) -> Option<DiscoveredDevice>;

    /// Check if discovery is currently active.
    fn is_scanning(&self) -> bool;
}
