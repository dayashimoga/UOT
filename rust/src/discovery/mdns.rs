//! mDNS Discovery Implementation
//!
//! Uses mDNS/DNS-SD to discover UOT peers on the local network.
//! Registers this device as a service and browses for others.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::discovery::types::{DeviceType, DiscoveredDevice, DiscoveryMethod};

/// mDNS service type for UOT.
const SERVICE_TYPE: &str = "_uot._tcp.local.";

/// Property keys.
const PROP_DEVICE_ID: &str = "id";
const PROP_DEVICE_TYPE: &str = "type";
const PROP_VERSION: &str = "ver";
const PROP_CAPABILITIES: &str = "caps";

/// Events emitted by the discovery service.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new device was discovered.
    DeviceFound(DiscoveredDevice),
    /// A device was lost.
    DeviceLost(String),
    /// A device's info was updated.
    DeviceUpdated(DiscoveredDevice),
}

/// mDNS discovery service.
pub struct MdnsDiscovery {
    /// The mDNS daemon.
    daemon: ServiceDaemon,
    /// Our registered service name.
    service_name: String,
    /// Currently discovered devices.
    devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    /// Whether we're scanning.
    scanning: Arc<RwLock<bool>>,
}

impl MdnsDiscovery {
    /// Create a new mDNS discovery service.
    pub fn new() -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {e}"))?;

        Ok(Self {
            daemon,
            service_name: String::new(),
            devices: Arc::new(RwLock::new(HashMap::new())),
            scanning: Arc::new(RwLock::new(false)),
        })
    }

    /// Register this device on the network.
    pub fn register(
        &mut self,
        device_id: &str,
        device_name: &str,
        port: u16,
        device_type: DeviceType,
    ) -> Result<(), String> {
        let instance_name = format!("{device_name} ({device_id})");

        let mut properties = HashMap::new();
        properties.insert(PROP_DEVICE_ID.to_string(), device_id.to_string());
        properties.insert(PROP_DEVICE_TYPE.to_string(), format!("{device_type}"));
        properties.insert(
            PROP_VERSION.to_string(),
            crate::core::version::version_string(),
        );
        properties.insert(
            PROP_CAPABILITIES.to_string(),
            "file,folder,clipboard".to_string(),
        );

        let host = format!("{device_id}.local.");

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &host,
            "",
            port,
            properties,
        )
        .map_err(|e| format!("Failed to create service info: {e}"))?;

        self.daemon
            .register(service_info)
            .map_err(|e| format!("Failed to register service: {e}"))?;

        self.service_name = instance_name;
        log::info!("Registered mDNS service: {}", self.service_name);

        Ok(())
    }

    /// Start browsing for other UOT devices.
    /// Returns a receiver for discovery events.
    pub fn start_browsing(&self) -> Result<mpsc::Receiver<DiscoveryEvent>, String> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("Failed to browse: {e}"))?;

        *self.scanning.write() = true;

        let (tx, rx) = mpsc::channel(64);
        let devices = Arc::clone(&self.devices);
        let scanning = Arc::clone(&self.scanning);
        let own_name = self.service_name.clone();

        std::thread::spawn(move || {
            while *scanning.read() {
                match receiver.recv_timeout(Duration::from_millis(500)) {
                    Ok(event) => {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                let full_name = info.get_fullname().to_string();
                                // Skip our own service
                                if full_name.contains(&own_name) {
                                    continue;
                                }

                                let device = service_info_to_device(&info);

                                let is_new = {
                                    let mut devs = devices.write();
                                    let existed = devs.contains_key(&device.device_id);
                                    devs.insert(device.device_id.clone(), device.clone());
                                    !existed
                                };

                                let event = if is_new {
                                    DiscoveryEvent::DeviceFound(device)
                                } else {
                                    DiscoveryEvent::DeviceUpdated(device)
                                };

                                if tx.blocking_send(event).is_err() {
                                    break;
                                }
                            }
                            ServiceEvent::ServiceRemoved(_, full_name) => {
                                let device_id = extract_device_id(&full_name);
                                if let Some(id) = device_id {
                                    devices.write().remove(&id);
                                    if tx
                                        .blocking_send(DiscoveryEvent::DeviceLost(id))
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(flume::RecvTimeoutError::Timeout) => continue,
                    Err(flume::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(rx)
    }

    /// Get all currently discovered devices.
    pub fn discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.devices.read().values().cloned().collect()
    }

    /// Get a specific device by ID.
    pub fn get_device(&self, device_id: &str) -> Option<DiscoveredDevice> {
        self.devices.read().get(device_id).cloned()
    }

    /// Stop scanning.
    pub fn stop_browsing(&self) {
        *self.scanning.write() = false;
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
    }

    /// Whether we're currently scanning.
    pub fn is_scanning(&self) -> bool {
        *self.scanning.read()
    }

    /// Unregister our service.
    pub fn unregister(&self) {
        if !self.service_name.is_empty() {
            let full_name = format!("{}.{}", self.service_name, SERVICE_TYPE);
            let _ = self.daemon.unregister(&full_name);
        }
    }
}

impl Drop for MdnsDiscovery {
    fn drop(&mut self) {
        self.stop_browsing();
        self.unregister();
        let _ = self.daemon.shutdown();
    }
}

/// Convert mDNS ServiceInfo to a DiscoveredDevice.
fn service_info_to_device(info: &ServiceInfo) -> DiscoveredDevice {
    let properties = info.get_properties();

    let device_id = properties
        .get_property_val_str(PROP_DEVICE_ID)
        .unwrap_or("unknown")
        .to_string();

    let device_type_str = properties
        .get_property_val_str(PROP_DEVICE_TYPE)
        .unwrap_or("Unknown");

    let device_type = match device_type_str {
        "Phone" => DeviceType::Phone,
        "Tablet" => DeviceType::Tablet,
        "Laptop" => DeviceType::Laptop,
        "Desktop" => DeviceType::Desktop,
        "TV" => DeviceType::Tv,
        _ => DeviceType::Unknown,
    };

    let capabilities: Vec<String> = properties
        .get_property_val_str(PROP_CAPABILITIES)
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // Get the first address and port
    let addresses: Vec<_> = info.get_addresses().iter().collect();
    let port = info.get_port();

    let address = addresses
        .first()
        .map(|addr| format!("{addr}:{port}"));

    // Extract device name from the instance name
    let device_name = info
        .get_fullname()
        .split('.')
        .next()
        .unwrap_or("Unknown")
        .to_string();

    let now = chrono::Utc::now();

    DiscoveredDevice {
        device_id,
        device_name,
        device_type,
        discovery_method: DiscoveryMethod::Mdns,
        address,
        capabilities,
        signal_strength: None,
        first_seen: now,
        last_seen: now,
        is_trusted: false,
    }
}

/// Extract device ID from a full mDNS name.
fn extract_device_id(full_name: &str) -> Option<String> {
    // Full name looks like "DeviceName (device-id)._uot._tcp.local."
    let paren_start = full_name.find('(')?;
    let paren_end = full_name.find(')')?;
    if paren_start < paren_end {
        Some(full_name[paren_start + 1..paren_end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_device_id() {
        assert_eq!(
            extract_device_id("My Phone (abc-123)._uot._tcp.local."),
            Some("abc-123".to_string())
        );
        assert_eq!(extract_device_id("no-parens"), None);
    }

    #[test]
    fn test_mdns_discovery_new() {
        // May fail in CI without network, but should not panic
        let result = MdnsDiscovery::new();
        if let Ok(discovery) = result {
            assert!(!discovery.is_scanning());
            assert!(discovery.discovered_devices().is_empty());
        }
    }
}
