//! Connection Manager — Auto-reconnection & Connection Pooling
//!
//! Handles automatic reconnection to previously-connected devices
//! and maintains a pool of active connections.
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::time::sleep;

use crate::core::error::TransportError;
use crate::transport::tcp::{self, TcpConnection};

/// Reconnection strategy.
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries (exponential backoff).
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Connection state tracking.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub device_id: String,
    pub device_name: String,
    pub address: SocketAddr,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub retry_count: u32,
}

/// Manages connections with auto-reconnection.
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, Arc<TcpConnection>>>>,
    connection_info: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    policy: ReconnectPolicy,
}

impl ConnectionManager {
    pub fn new(policy: ReconnectPolicy) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_info: Arc::new(RwLock::new(HashMap::new())),
            policy,
        }
    }

    /// Connect to a device, with retry logic.
    pub async fn connect(
        &self,
        device_id: &str,
        device_name: &str,
        addr: SocketAddr,
    ) -> Result<Arc<TcpConnection>, TransportError> {
        let mut attempt = 0;
        loop {
            match tcp::connect(addr).await {
                Ok(stream) => {
                    let conn = Arc::new(TcpConnection::new(stream)?);
                    self.connections
                        .write()
                        .insert(device_id.to_string(), Arc::clone(&conn));
                    self.connection_info.write().insert(
                        device_id.to_string(),
                        ConnectionInfo {
                            device_id: device_id.to_string(),
                            device_name: device_name.to_string(),
                            address: addr,
                            connected_at: Instant::now(),
                            last_activity: Instant::now(),
                            retry_count: 0,
                        },
                    );
                    return Ok(conn);
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.policy.max_retries {
                        return Err(e);
                    }
                    let delay = std::cmp::min(
                        self.policy.base_delay * 2u32.saturating_pow(attempt - 1),
                        self.policy.max_delay,
                    );
                    log::warn!(
                        "Connection to {} failed (attempt {}/{}), retrying in {:?}",
                        device_id,
                        attempt,
                        self.policy.max_retries,
                        delay
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    /// Get an existing connection.
    pub fn get(&self, device_id: &str) -> Option<Arc<TcpConnection>> {
        self.connections.read().get(device_id).cloned()
    }

    /// Remove a connection.
    pub fn remove(&self, device_id: &str) {
        self.connections.write().remove(device_id);
        self.connection_info.write().remove(device_id);
    }

    /// Get all connection infos.
    pub fn active_connections(&self) -> Vec<ConnectionInfo> {
        self.connection_info.read().values().cloned().collect()
    }

    /// Check if connected to a device.
    pub fn is_connected(&self, device_id: &str) -> bool {
        self.connections.read().contains_key(device_id)
    }

    /// Clear all connections.
    pub fn clear(&self) {
        self.connections.write().clear();
        self.connection_info.write().clear();
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new(ReconnectPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnect_policy_default() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_connection_manager_state() {
        let manager = ConnectionManager::default();
        assert!(!manager.is_connected("dev1"));
        assert!(manager.get("dev1").is_none());
        assert!(manager.active_connections().is_empty());
        manager.clear();
    }

    #[tokio::test]
    async fn test_connection_manager_connect_retry_and_methods() {
        let policy = ReconnectPolicy {
            max_retries: 1,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(50),
        };
        let manager = ConnectionManager::new(policy);
        let dummy_addr: std::net::SocketAddr = "127.0.0.1:59998".parse().unwrap();
        let res = manager.connect("dev_fail", "Fail Device", dummy_addr).await;
        assert!(res.is_err());

        manager.remove("dev_fail");
        assert!(!manager.is_connected("dev_fail"));
    }

    #[tokio::test]
    async fn test_connection_manager_connect_success() {
        let (mut listener, _incoming) = crate::transport::tcp::TcpTransportListener::bind(0)
            .await
            .unwrap();
        let port = listener.port();
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

        let manager = ConnectionManager::default();
        let conn = manager.connect("dev-ok", "OK Device", addr).await;
        assert!(conn.is_ok());
        assert!(manager.is_connected("dev-ok"));
        assert!(manager.get("dev-ok").is_some());
        assert_eq!(manager.active_connections().len(), 1);

        manager.remove("dev-ok");
        assert!(!manager.is_connected("dev-ok"));
        assert!(manager.active_connections().is_empty());
    }
}
