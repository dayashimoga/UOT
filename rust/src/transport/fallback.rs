//! Transport Auto-Switching & Fallback Orchestrator
//!
//! Evaluates network conditions and automatically switches/falls back
//! across Wi-Fi LAN, Wi-Fi Direct, Bluetooth LE, and Optical QR transports.
use crate::transport::types::{TransportId, TransportState};

/// Active transport selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportSelectionStrategy {
    /// Always prefer fastest local Wi-Fi / TCP connection.
    PreferSpeed,
    /// Prefer offline BLE / P2P when external network is unavailable.
    PreferOffline,
    /// Manual selection override.
    Manual,
}

/// Fallback manager for evaluating active transports.
pub struct TransportFallbackManager {
    pub strategy: TransportSelectionStrategy,
}

impl TransportFallbackManager {
    pub fn new(strategy: TransportSelectionStrategy) -> Self {
        Self { strategy }
    }

    /// Select optimal transport based on available candidate states.
    pub fn select_best_transport(
        &self,
        candidates: &[(TransportId, TransportState)],
    ) -> Option<TransportId> {
        let active: Vec<&(TransportId, TransportState)> = candidates
            .iter()
            .filter(|(_, state)| {
                *state == TransportState::Connected || *state == TransportState::Listening
            })
            .collect();

        if active.is_empty() {
            return None;
        }

        match self.strategy {
            TransportSelectionStrategy::PreferSpeed => {
                // Priority: TcpLan -> WifiDirect -> BluetoothLe -> QrCode
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::TcpLan) {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::WifiDirect) {
                    return Some(t.0);
                }
                if let Some(t) = active
                    .iter()
                    .find(|(id, _)| *id == TransportId::BluetoothLe)
                {
                    return Some(t.0);
                }
                Some(active[0].0)
            }
            TransportSelectionStrategy::PreferOffline | TransportSelectionStrategy::Manual => {
                Some(active[0].0)
            }
        }
    }
}

impl Default for TransportFallbackManager {
    fn default() -> Self {
        Self::new(TransportSelectionStrategy::PreferSpeed)
    }
}
