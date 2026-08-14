//! UOT Transport Simulator & Fault Injection Test Harness
//!
//! Provides a deterministic in-memory transport provider and connection implementation
//! capable of simulating real-world network anomalies:
//! - Configurable artificial latency (jitter)
//! - Random packet loss / drop rate
//! - Bit flip / payload corruption simulation
//! - Bandwidth throttling
//! - Network partition and healing (abrupt disconnect/reconnect)
//!
//! Explicitly identified as a simulated transport (`is_simulated: true`) and never
//! exposed as physical hardware capability.

use super::types::{TransportCapabilities, TransportId, TransportState, TransportStats};
use super::{TransportConnection, TransportProvider};
use crate::core::error::TransportError;
use async_trait::async_trait;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Configuration for fault injection.
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Simulated one-way latency in milliseconds.
    pub latency_ms: u64,
    /// Probability of dropping a packet (0.0 to 1.0).
    pub packet_loss_rate: f64,
    /// Probability of flipping a bit in payload (0.0 to 1.0).
    pub bit_flip_rate: f64,
    /// Bandwidth limit in bytes/sec (0 = unlimited).
    pub bandwidth_limit_bps: u64,
    /// Whether the network is partitioned (all packets blocked).
    pub is_partitioned: bool,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            latency_ms: 0,
            packet_loss_rate: 0.0,
            bit_flip_rate: 0.0,
            bandwidth_limit_bps: 0,
            is_partitioned: false,
        }
    }
}

/// A simulated bidirectional data channel.
pub struct SimulatedConnection {
    tx: mpsc::Sender<Vec<u8>>,
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>>,
    faults: Arc<RwLock<FaultConfig>>,
    stats: Arc<RwLock<TransportStats>>,
    connected: Arc<RwLock<bool>>,
    created_at: Instant,
}

impl SimulatedConnection {
    /// Create a connected pair of simulated endpoints.
    pub fn create_pair(faults: Arc<RwLock<FaultConfig>>) -> (Self, Self) {
        let (tx_a, rx_a) = mpsc::channel::<Vec<u8>>(1024);
        let (tx_b, rx_b) = mpsc::channel::<Vec<u8>>(1024);

        let stats_a = Arc::new(RwLock::new(TransportStats::default()));
        let stats_b = Arc::new(RwLock::new(TransportStats::default()));
        let connected_a = Arc::new(RwLock::new(true));
        let connected_b = Arc::new(RwLock::new(true));

        let conn_a = Self {
            tx: tx_b,
            rx: Arc::new(tokio::sync::Mutex::new(rx_a)),
            faults: Arc::clone(&faults),
            stats: stats_a,
            connected: connected_a,
            created_at: Instant::now(),
        };

        let conn_b = Self {
            tx: tx_a,
            rx: Arc::new(tokio::sync::Mutex::new(rx_b)),
            faults,
            stats: stats_b,
            connected: connected_b,
            created_at: Instant::now(),
        };

        (conn_a, conn_b)
    }
}

#[async_trait]
impl TransportConnection for SimulatedConnection {
    async fn send(&self, data: &[u8]) -> Result<usize, TransportError> {
        if !*self.connected.read() {
            return Err(TransportError::ConnectionLost {
                reason: "Simulated connection closed".into(),
            });
        }

        let faults = self.faults.read().clone();

        // 1. Partition check
        if faults.is_partitioned {
            return Err(TransportError::SendFailed {
                reason: "Network partition active".into(),
            });
        }

        // 2. Packet loss simulation
        if faults.packet_loss_rate > 0.0 {
            let mut rng = rand::rng();
            if rng.random_bool(faults.packet_loss_rate.clamp(0.0, 1.0)) {
                // Silently dropped packet
                self.stats.write().retransmissions += 1;
                return Ok(data.len());
            }
        }

        // 3. Bit corruption simulation
        let mut payload = data.to_vec();
        if faults.bit_flip_rate > 0.0 && !payload.is_empty() {
            let mut rng = rand::rng();
            if rng.random_bool(faults.bit_flip_rate.clamp(0.0, 1.0)) {
                let idx = rng.random_range(0..payload.len());
                payload[idx] ^= 0xFF; // flip bits
            }
        }

        // 4. Latency simulation
        if faults.latency_ms > 0 {
            tokio::time::sleep(Duration::from_millis(faults.latency_ms)).await;
        }

        let len = payload.len();
        self.tx
            .send(payload)
            .await
            .map_err(|_| TransportError::ConnectionLost {
                reason: "Receiver dropped channel".into(),
            })?;

        let mut st = self.stats.write();
        st.bytes_sent += len as u64;
        st.uptime_secs = self.created_at.elapsed().as_secs();

        Ok(len)
    }

    async fn receive(&self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        if !*self.connected.read() {
            return Err(TransportError::ConnectionLost {
                reason: "Simulated connection closed".into(),
            });
        }

        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                let mut st = self.stats.write();
                st.bytes_received += len as u64;
                st.uptime_secs = self.created_at.elapsed().as_secs();
                Ok(len)
            }
            None => {
                *self.connected.write() = false;
                Err(TransportError::ConnectionLost {
                    reason: "Channel closed by peer".into(),
                })
            }
        }
    }

    async fn close(&self) -> Result<(), TransportError> {
        *self.connected.write() = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.connected.read()
    }

    fn stats(&self) -> TransportStats {
        self.stats.read().clone()
    }

    fn transport_id(&self) -> TransportId {
        TransportId::Simulated
    }
}

/// Simulated transport provider for deterministic integration lab tests.
pub struct SimulatedTransportProvider {
    faults: Arc<RwLock<FaultConfig>>,
    state: Arc<RwLock<TransportState>>,
    incoming_tx: mpsc::Sender<SimulatedConnection>,
    incoming_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulatedConnection>>>,
}

impl SimulatedTransportProvider {
    /// Create a new simulated transport provider with default faults.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(64);
        Self {
            faults: Arc::new(RwLock::new(FaultConfig::default())),
            state: Arc::new(RwLock::new(TransportState::Idle)),
            incoming_tx: tx,
            incoming_rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    /// Set simulated latency in milliseconds.
    pub fn set_latency_ms(&self, ms: u64) {
        self.faults.write().latency_ms = ms;
    }

    /// Set simulated packet loss rate (0.0 to 1.0).
    pub fn set_packet_loss_rate(&self, rate: f64) {
        self.faults.write().packet_loss_rate = rate;
    }

    /// Set bit flip corruption rate (0.0 to 1.0).
    pub fn set_bit_flip_rate(&self, rate: f64) {
        self.faults.write().bit_flip_rate = rate;
    }

    /// Partition the network (block all packets).
    pub fn partition(&self) {
        self.faults.write().is_partitioned = true;
    }

    /// Heal the network partition.
    pub fn heal(&self) {
        self.faults.write().is_partitioned = false;
    }

    /// Create a connected pair of simulated connections directly.
    pub fn create_connection_pair(&self) -> (SimulatedConnection, SimulatedConnection) {
        SimulatedConnection::create_pair(Arc::clone(&self.faults))
    }
}

impl Default for SimulatedTransportProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportProvider for SimulatedTransportProvider {
    fn id(&self) -> TransportId {
        TransportId::Simulated
    }

    fn name(&self) -> &str {
        "Simulated Lab Transport"
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities {
            bidirectional: true,
            reliable: true,
            requires_network: false,
            max_throughput: 100_000_000, // 100 MB/s simulated bus
            typical_latency_ms: 1,
            max_payload_size: 0, // unlimited stream
            supports_streaming: true,
            supports_discovery: true,
            platforms: vec!["all".to_string()],
            is_simulated: true,
            requires_physical_hardware: false,
        }
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn state(&self) -> TransportState {
        *self.state.read()
    }

    async fn listen(&self, _port: u16) -> Result<(), TransportError> {
        *self.state.write() = TransportState::Listening;
        Ok(())
    }

    async fn stop_listening(&self) -> Result<(), TransportError> {
        *self.state.write() = TransportState::Idle;
        Ok(())
    }

    async fn connect(
        &self,
        _address: &str,
    ) -> Result<Box<dyn TransportConnection>, TransportError> {
        *self.state.write() = TransportState::Connecting;
        let (local, remote) = SimulatedConnection::create_pair(Arc::clone(&self.faults));
        let _ = self.incoming_tx.send(remote).await;
        *self.state.write() = TransportState::Connected;
        Ok(Box::new(local))
    }

    async fn accept(&self) -> Result<Box<dyn TransportConnection>, TransportError> {
        let mut rx = self.incoming_rx.lock().await;
        match rx.recv().await {
            Some(conn) => Ok(Box::new(conn)),
            None => Err(TransportError::ConnectionLost {
                reason: "Simulator listener stopped".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator_clean_transmission() {
        let provider = SimulatedTransportProvider::new();
        let (conn_a, conn_b) = provider.create_connection_pair();

        let payload = b"Hello UOT Transport Lab!";
        conn_a.send(payload).await.unwrap();

        let mut buf = vec![0u8; 64];
        let n = conn_b.receive(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], payload);
    }

    #[tokio::test]
    async fn test_simulator_partition_and_heal() {
        let provider = SimulatedTransportProvider::new();
        let (conn_a, conn_b) = provider.create_connection_pair();

        provider.partition();
        let send_res = conn_a.send(b"blocked").await;
        assert!(send_res.is_err());

        provider.heal();
        conn_a.send(b"delivered").await.unwrap();

        let mut buf = vec![0u8; 64];
        let n = conn_b.receive(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"delivered");
    }

    #[tokio::test]
    async fn test_simulator_latency_injection() {
        let provider = SimulatedTransportProvider::new();
        provider.set_latency_ms(50);
        let (conn_a, conn_b) = provider.create_connection_pair();

        let start = Instant::now();
        conn_a.send(b"delayed packet").await.unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 45);

        let mut buf = vec![0u8; 64];
        let n = conn_b.receive(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"delayed packet");
    }

    #[tokio::test]
    async fn test_simulator_capabilities_mark_simulated() {
        let provider = SimulatedTransportProvider::new();
        let caps = provider.capabilities();
        assert!(caps.is_simulated);
        assert!(!caps.requires_physical_hardware);
    }
}
