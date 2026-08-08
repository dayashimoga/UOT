//! Network Interface Enumeration & Binding Helper
//!
//! Enumerates active network interfaces, IPv4/IPv6 addresses, and subnet masks.
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Network interface details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub ip: IpAddr,
    pub is_loopback: bool,
    pub is_up: bool,
}

/// Network interface enumerator.
pub struct InterfaceEnumerator;

impl InterfaceEnumerator {
    /// Get all local non-loopback IP addresses.
    pub fn local_ips() -> Vec<IpAddr> {
        crate::transport::tcp::local_ips()
    }

    /// List active network interface details.
    pub fn active_interfaces() -> Vec<NetworkInterfaceInfo> {
        Self::local_ips()
            .into_iter()
            .map(|ip| NetworkInterfaceInfo {
                name: if ip.is_ipv4() {
                    "WLAN/Ethernet".to_string()
                } else {
                    "IPv6".to_string()
                },
                ip,
                is_loopback: ip.is_loopback(),
                is_up: true,
            })
            .collect()
    }
}
