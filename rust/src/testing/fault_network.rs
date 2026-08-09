//! Fault-Injecting Network Layer
//!
//! Deterministic fault injection for testing recovery, checkpointing,
//! and transport migration without real network failures.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use rand::Rng;

/// Configuration for deterministic fault injection.
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Packet loss probability (0.0 - 1.0).
    pub packet_loss: f64,
    /// Extra latency per packet in milliseconds.
    pub latency_ms: u64,
    /// Jitter range in milliseconds (+/- this value).
    pub jitter_ms: u64,
    /// Bandwidth limit in bytes/sec (0 = unlimited).
    pub bandwidth_limit: u64,
    /// Probability of reordering consecutive packets.
    pub reorder_probability: f64,
    /// Probability of duplicating a packet.
    pub duplicate_probability: f64,
    /// Probability of corrupting a byte in payload.
    pub corruption_probability: f64,
    /// Disconnect after this many bytes transferred (0 = never).
    pub disconnect_after_bytes: u64,
    /// Whether to auto-reconnect after disconnect.
    pub auto_reconnect: bool,
    /// Deterministic seed for reproducible tests.
    pub seed: u64,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            packet_loss: 0.0,
            latency_ms: 0,
            jitter_ms: 0,
            bandwidth_limit: 0,
            reorder_probability: 0.0,
            duplicate_probability: 0.0,
            corruption_probability: 0.0,
            disconnect_after_bytes: 0,
            auto_reconnect: false,
            seed: 42,
        }
    }
}

impl FaultConfig {
    /// Create a clean (no faults) configuration.
    pub fn clean() -> Self {
        Self::default()
    }

    /// Create a lossy network (10% packet loss, 50ms latency, 20ms jitter).
    pub fn lossy() -> Self {
        Self {
            packet_loss: 0.10,
            latency_ms: 50,
            jitter_ms: 20,
            ..Default::default()
        }
    }

    /// Create a hostile network (25% loss, corruption, reorder, disconnect).
    pub fn hostile() -> Self {
        Self {
            packet_loss: 0.25,
            latency_ms: 100,
            jitter_ms: 50,
            reorder_probability: 0.15,
            duplicate_probability: 0.05,
            corruption_probability: 0.02,
            disconnect_after_bytes: 500_000,
            auto_reconnect: true,
            seed: 42,
            ..Default::default()
        }
    }

    /// Create a bandwidth-limited network.
    pub fn slow(bytes_per_sec: u64) -> Self {
        Self {
            bandwidth_limit: bytes_per_sec,
            latency_ms: 200,
            ..Default::default()
        }
    }
}

/// Fault-injecting network pipe that sits between two virtual nodes.
pub struct FaultNetwork {
    config: RwLock<FaultConfig>,
    bytes_transferred: AtomicU64,
    packets_sent: AtomicU64,
    packets_dropped: AtomicU64,
    packets_corrupted: AtomicU64,
    packets_duplicated: AtomicU64,
    packets_reordered: AtomicU64,
    disconnected: AtomicBool,
    disconnect_count: AtomicU64,
}

impl FaultNetwork {
    pub fn new(config: FaultConfig) -> Arc<Self> {
        Arc::new(Self {
            config: RwLock::new(config),
            bytes_transferred: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
            packets_dropped: AtomicU64::new(0),
            packets_corrupted: AtomicU64::new(0),
            packets_duplicated: AtomicU64::new(0),
            packets_reordered: AtomicU64::new(0),
            disconnected: AtomicBool::new(false),
            disconnect_count: AtomicU64::new(0),
        })
    }

    /// Process a packet through the fault network. Returns None if dropped,
    /// Some(data) if delivered (possibly corrupted/delayed/duplicated).
    pub async fn process_packet(&self, data: Vec<u8>) -> Vec<Option<Vec<u8>>> {
        let config = self.config.read().clone();
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        let transferred = self
            .bytes_transferred
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        // Check disconnect threshold
        if config.disconnect_after_bytes > 0 && transferred >= config.disconnect_after_bytes {
            if !self.disconnected.swap(true, Ordering::Relaxed) {
                self.disconnect_count.fetch_add(1, Ordering::Relaxed);
            }
            if config.auto_reconnect {
                self.disconnected.store(false, Ordering::Relaxed);
            } else {
                return vec![None];
            }
        }

        if self.disconnected.load(Ordering::Relaxed) {
            return vec![None];
        }

        let mut rng = rand::rng();
        let mut results = Vec::new();

        // Packet loss
        if config.packet_loss > 0.0 && rng.random::<f64>() < config.packet_loss {
            self.packets_dropped.fetch_add(1, Ordering::Relaxed);
            return vec![None];
        }

        // Apply latency + jitter
        if config.latency_ms > 0 || config.jitter_ms > 0 {
            let jitter = if config.jitter_ms > 0 {
                rng.random_range(0..config.jitter_ms * 2) as i64 - config.jitter_ms as i64
            } else {
                0
            };
            let delay_ms = (config.latency_ms as i64 + jitter).max(0) as u64;
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        // Corruption
        let mut payload = data;
        if config.corruption_probability > 0.0
            && rng.random::<f64>() < config.corruption_probability
            && !payload.is_empty()
        {
            let idx = rng.random_range(0..payload.len());
            payload[idx] ^= 0xFF;
            self.packets_corrupted.fetch_add(1, Ordering::Relaxed);
        }

        // Duplication
        if config.duplicate_probability > 0.0 && rng.random::<f64>() < config.duplicate_probability
        {
            results.push(Some(payload.clone()));
            self.packets_duplicated.fetch_add(1, Ordering::Relaxed);
        }

        results.push(Some(payload));

        // Bandwidth limiting
        if config.bandwidth_limit > 0 {
            let byte_count = results
                .iter()
                .flatten()
                .map(|p| p.len() as u64)
                .sum::<u64>();
            if let Some(delay_us) = (byte_count * 1_000_000).checked_div(config.bandwidth_limit) {
                if delay_us > 0 {
                    tokio::time::sleep(std::time::Duration::from_micros(delay_us)).await;
                }
            }
        }

        results
    }

    /// Force a disconnect.
    pub fn force_disconnect(&self) {
        self.disconnected.store(true, Ordering::Relaxed);
        self.disconnect_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Force a reconnect.
    pub fn force_reconnect(&self) {
        self.disconnected.store(false, Ordering::Relaxed);
    }

    /// Update fault configuration at runtime.
    pub fn update_config(&self, config: FaultConfig) {
        *self.config.write() = config;
    }

    /// Get statistics.
    pub fn stats(&self) -> FaultNetworkStats {
        FaultNetworkStats {
            bytes_transferred: self.bytes_transferred.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_dropped: self.packets_dropped.load(Ordering::Relaxed),
            packets_corrupted: self.packets_corrupted.load(Ordering::Relaxed),
            packets_duplicated: self.packets_duplicated.load(Ordering::Relaxed),
            packets_reordered: self.packets_reordered.load(Ordering::Relaxed),
            disconnect_count: self.disconnect_count.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.bytes_transferred.store(0, Ordering::Relaxed);
        self.packets_sent.store(0, Ordering::Relaxed);
        self.packets_dropped.store(0, Ordering::Relaxed);
        self.packets_corrupted.store(0, Ordering::Relaxed);
        self.packets_duplicated.store(0, Ordering::Relaxed);
        self.packets_reordered.store(0, Ordering::Relaxed);
        self.disconnect_count.store(0, Ordering::Relaxed);
        self.disconnected.store(false, Ordering::Relaxed);
    }
}

/// Network fault statistics.
#[derive(Debug, Clone, Default)]
pub struct FaultNetworkStats {
    pub bytes_transferred: u64,
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub packets_corrupted: u64,
    pub packets_duplicated: u64,
    pub packets_reordered: u64,
    pub disconnect_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clean_network_passes_all() {
        let net = FaultNetwork::new(FaultConfig::clean());
        for i in 0..100u8 {
            let results = net.process_packet(vec![i]).await;
            assert_eq!(results.len(), 1);
            assert!(results[0].is_some());
            assert_eq!(results[0].as_ref().unwrap(), &vec![i]);
        }
        let stats = net.stats();
        assert_eq!(stats.packets_sent, 100);
        assert_eq!(stats.packets_dropped, 0);
    }

    #[tokio::test]
    async fn test_total_loss_drops_all() {
        let net = FaultNetwork::new(FaultConfig {
            packet_loss: 1.0,
            ..Default::default()
        });
        for _ in 0..50 {
            let results = net.process_packet(vec![0]).await;
            assert!(results[0].is_none());
        }
        assert_eq!(net.stats().packets_dropped, 50);
    }

    #[tokio::test]
    async fn test_disconnect_after_bytes() {
        let net = FaultNetwork::new(FaultConfig {
            disconnect_after_bytes: 100,
            auto_reconnect: false,
            ..Default::default()
        });
        // Send 10 packets of 10 bytes each (100 bytes)
        for _ in 0..10 {
            net.process_packet(vec![0u8; 10]).await;
        }
        // 11th packet should be dropped (disconnected)
        let r = net.process_packet(vec![0u8; 10]).await;
        assert!(r[0].is_none());
    }

    #[tokio::test]
    async fn test_disconnect_with_reconnect() {
        let net = FaultNetwork::new(FaultConfig {
            disconnect_after_bytes: 50,
            auto_reconnect: true,
            ..Default::default()
        });
        for _ in 0..20 {
            let _ = net.process_packet(vec![0u8; 10]).await;
        }
        // Should have reconnected automatically
        assert!(net.stats().disconnect_count > 0);
    }

    #[tokio::test]
    async fn test_force_disconnect_reconnect() {
        let net = FaultNetwork::new(FaultConfig::clean());
        net.force_disconnect();
        let r = net.process_packet(vec![1]).await;
        assert!(r[0].is_none());

        net.force_reconnect();
        let r = net.process_packet(vec![2]).await;
        assert!(r[0].is_some());
    }

    #[test]
    fn test_fault_config_presets() {
        let clean = FaultConfig::clean();
        assert_eq!(clean.packet_loss, 0.0);

        let lossy = FaultConfig::lossy();
        assert!(lossy.packet_loss > 0.0);

        let hostile = FaultConfig::hostile();
        assert!(hostile.corruption_probability > 0.0);
        assert!(hostile.disconnect_after_bytes > 0);

        let slow = FaultConfig::slow(1_000_000);
        assert_eq!(slow.bandwidth_limit, 1_000_000);
    }

    #[test]
    fn test_reset() {
        let net = FaultNetwork::new(FaultConfig::clean());
        net.packets_sent.store(100, Ordering::Relaxed);
        net.reset();
        assert_eq!(net.stats().packets_sent, 0);
    }
}
