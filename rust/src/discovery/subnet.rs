//! LAN Subnet Active Scanner (Fallback Discovery)
//!
//! Scans local IP subnets on port 42000 when multicast mDNS discovery is disabled or restricted.
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Subnet scanner configuration.
pub struct SubnetScanner {
    pub port: u16,
    pub timeout_ms: u64,
}

impl SubnetScanner {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            timeout_ms: 300,
        }
    }

    /// Scan a /24 IPv4 subnet range for active UOT listeners.
    pub async fn scan_subnet(&self, base_ip: [u8; 4]) -> Vec<SocketAddr> {
        let mut tasks = Vec::new();
        let port = self.port;
        let timeout_ms = self.timeout_ms;

        for i in 1..=254 {
            let ip = IpAddr::V4(std::net::Ipv4Addr::new(
                base_ip[0], base_ip[1], base_ip[2], i,
            ));
            let addr = SocketAddr::new(ip, port);

            tasks.push(tokio::spawn(async move {
                match timeout(Duration::from_millis(timeout_ms), TcpStream::connect(addr)).await {
                    Ok(Ok(_)) => Some(addr),
                    _ => None,
                }
            }));
        }

        let mut active = Vec::new();
        for task in tasks {
            if let Ok(Some(addr)) = task.await {
                active.push(addr);
            }
        }
        active
    }
}

impl Default for SubnetScanner {
    fn default() -> Self {
        Self::new(42000)
    }
}
