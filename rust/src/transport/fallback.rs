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
                // Priority: TcpLan -> WifiDirect -> Hotspot -> BluetoothLe -> QrCode -> Relay -> Others
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::TcpLan) {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::WifiDirect) {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::Hotspot) {
                    return Some(t.0);
                }
                if let Some(t) = active
                    .iter()
                    .find(|(id, _)| *id == TransportId::BluetoothLe)
                {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::QrCode) {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::Relay) {
                    return Some(t.0);
                }
                Some(active[0].0)
            }
            TransportSelectionStrategy::PreferOffline => {
                // Prefer direct peer-to-peer / offline transports
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::WifiDirect) {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::Hotspot) {
                    return Some(t.0);
                }
                if let Some(t) = active
                    .iter()
                    .find(|(id, _)| *id == TransportId::BluetoothLe)
                {
                    return Some(t.0);
                }
                if let Some(t) = active.iter().find(|(id, _)| *id == TransportId::TcpLan) {
                    return Some(t.0);
                }
                Some(active[0].0)
            }
            TransportSelectionStrategy::Manual => Some(active[0].0),
        }
    }

    /// Classify network topology between local addresses and remote peer address truthfully.
    pub fn classify_network_topology(
        local_ips: &[std::net::IpAddr],
        remote_ip: std::net::IpAddr,
    ) -> &'static str {
        if remote_ip.is_loopback() {
            return "Local Loopback";
        }

        // Check for Android Wi-Fi Direct subnet (standard 192.168.49.0/24)
        if let std::net::IpAddr::V4(ipv4) = remote_ip {
            let octets = ipv4.octets();
            if octets[0] == 192 && octets[1] == 168 && octets[2] == 49 {
                return "Wi-Fi Direct";
            }
            // Standard mobile hotspot subnets (192.168.43.0/24, 192.168.137.0/24 Windows ICS)
            if (octets[0] == 192 && octets[1] == 168 && octets[2] == 43)
                || (octets[0] == 192 && octets[1] == 168 && octets[2] == 137)
            {
                return "Hotspot";
            }
        }

        // Check if any local IP shares the /24 subnet with remote IP
        for local in local_ips {
            if let (std::net::IpAddr::V4(loc), std::net::IpAddr::V4(rem)) = (local, remote_ip) {
                let loc_o = loc.octets();
                let rem_o = rem.octets();
                if loc_o[0] == rem_o[0] && loc_o[1] == rem_o[1] && loc_o[2] == rem_o[2] {
                    return "Same network";
                }
            }
        }

        // Remote network or different subnet
        if remote_ip.is_unspecified() {
            "Unknown"
        } else {
            "Remote network"
        }
    }
}

impl Default for TransportFallbackManager {
    fn default() -> Self {
        Self::new(TransportSelectionStrategy::PreferSpeed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_manager_prefer_speed() {
        let manager = TransportFallbackManager::default();
        let candidates = vec![
            (TransportId::BluetoothLe, TransportState::Connected),
            (TransportId::TcpLan, TransportState::Connected),
        ];
        let best = manager.select_best_transport(&candidates);
        assert_eq!(best, Some(TransportId::TcpLan));
    }

    #[test]
    fn test_fallback_manager_empty_or_inactive() {
        let manager = TransportFallbackManager::new(TransportSelectionStrategy::PreferOffline);
        let candidates = vec![(TransportId::TcpLan, TransportState::Disconnected)];
        assert_eq!(manager.select_best_transport(&candidates), None);
    }
}
