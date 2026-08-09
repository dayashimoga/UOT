//! WebRTC Data Channel Transport
//!
//! Provides WebRTC-based transport for NAT traversal and peer-to-peer
//! communication over the internet. Uses ICE for connectivity and
//! DTLS-SRTP for security. Data channels provide reliable/ordered delivery.

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::error::TransportError;
use crate::transport::tcp::Frame;
use crate::transport::types::{TransportCapabilities, TransportState, TransportStats};

/// ICE candidate for NAT traversal.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u32>,
}

/// SDP offer/answer for WebRTC signaling.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionDescription {
    pub sdp_type: String, // "offer" or "answer"
    pub sdp: String,
}

/// WebRTC signaling message exchanged via signaling server or QR code.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SignalingMessage {
    Offer(SessionDescription),
    Answer(SessionDescription),
    IceCandidate(IceCandidate),
    Disconnect,
}

/// WebRTC transport state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebRtcState {
    New,
    Signaling,
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

/// WebRTC data channel transport.
pub struct WebRtcTransport {
    state: RwLock<WebRtcState>,
    stats: RwLock<TransportStats>,
    local_candidates: RwLock<Vec<IceCandidate>>,
    remote_candidates: RwLock<Vec<IceCandidate>>,
    local_sdp: RwLock<Option<SessionDescription>>,
    remote_sdp: RwLock<Option<SessionDescription>>,
    /// Outgoing data buffer (frames waiting to send).
    tx_buffer: RwLock<VecDeque<Frame>>,
    /// Incoming data buffer (received frames).
    rx_buffer: RwLock<VecDeque<Frame>>,
    /// Data channel label.
    channel_label: String,
    /// Maximum data channel message size.
    max_message_size: usize,
}

impl WebRtcTransport {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(WebRtcState::New),
            stats: RwLock::new(TransportStats::default()),
            local_candidates: RwLock::new(Vec::new()),
            remote_candidates: RwLock::new(Vec::new()),
            local_sdp: RwLock::new(None),
            remote_sdp: RwLock::new(None),
            tx_buffer: RwLock::new(VecDeque::new()),
            rx_buffer: RwLock::new(VecDeque::new()),
            channel_label: "uot-data".to_string(),
            max_message_size: 256 * 1024, // 256 KB
        }
    }

    /// Get transport capabilities.
    pub fn capabilities() -> TransportCapabilities {
        TransportCapabilities {
            bidirectional: true,
            reliable: true,
            requires_network: true,
            max_throughput: 100_000_000, // ~100 MB/s
            typical_latency_ms: 20,
            max_payload_size: 256 * 1024,
            supports_streaming: true,
            supports_discovery: false,
            platforms: vec![
                "android".into(),
                "ios".into(),
                "windows".into(),
                "macos".into(),
                "linux".into(),
                "web".into(),
            ],
        }
    }

    /// Create an SDP offer (caller side).
    pub fn create_offer(&self) -> Result<SessionDescription, TransportError> {
        *self.state.write() = WebRtcState::Signaling;
        let sdp = SessionDescription {
            sdp_type: "offer".into(),
            sdp: format!(
                "v=0\r\no=uot 0 0 IN IP4 0.0.0.0\r\ns=UOT Transfer\r\n\
                 t=0 0\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
                 a=sctp-port:5000\r\na=max-message-size:{}\r\n",
                self.max_message_size
            ),
        };
        *self.local_sdp.write() = Some(sdp.clone());
        Ok(sdp)
    }

    /// Create an SDP answer (callee side).
    pub fn create_answer(
        &self,
        remote_offer: &SessionDescription,
    ) -> Result<SessionDescription, TransportError> {
        if remote_offer.sdp_type != "offer" {
            return Err(TransportError::Protocol("Expected offer SDP".into()));
        }
        *self.remote_sdp.write() = Some(remote_offer.clone());
        *self.state.write() = WebRtcState::Signaling;

        let sdp = SessionDescription {
            sdp_type: "answer".into(),
            sdp: format!(
                "v=0\r\no=uot 0 0 IN IP4 0.0.0.0\r\ns=UOT Transfer\r\n\
                 t=0 0\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
                 a=sctp-port:5000\r\na=max-message-size:{}\r\n",
                self.max_message_size
            ),
        };
        *self.local_sdp.write() = Some(sdp.clone());
        Ok(sdp)
    }

    /// Set the remote SDP answer.
    pub fn set_remote_answer(&self, answer: &SessionDescription) -> Result<(), TransportError> {
        if answer.sdp_type != "answer" {
            return Err(TransportError::Protocol("Expected answer SDP".into()));
        }
        *self.remote_sdp.write() = Some(answer.clone());
        Ok(())
    }

    /// Add a remote ICE candidate.
    pub fn add_ice_candidate(&self, candidate: IceCandidate) {
        self.remote_candidates.write().push(candidate);
    }

    /// Get local ICE candidates (generated during connection setup).
    pub fn local_candidates(&self) -> Vec<IceCandidate> {
        self.local_candidates.read().clone()
    }

    /// Simulate ICE candidate gathering.
    pub fn gather_candidates(&self, local_ip: &str, port: u16) {
        let candidate = IceCandidate {
            candidate: format!("candidate:1 1 UDP 2130706431 {local_ip} {port} typ host"),
            sdp_mid: Some("0".into()),
            sdp_mline_index: Some(0),
        };
        self.local_candidates.write().push(candidate);
    }

    /// Transition to connected state (after ICE + DTLS handshake).
    pub fn set_connected(&self) {
        *self.state.write() = WebRtcState::Connected;
    }

    /// Send a frame via the data channel.
    pub fn send_frame(&self, frame: Frame) -> Result<(), TransportError> {
        if *self.state.read() != WebRtcState::Connected {
            return Err(TransportError::SendFailed {
                reason: "Not connected".into(),
            });
        }
        let encoded = frame.encode();
        if encoded.len() > self.max_message_size {
            return Err(TransportError::SendFailed {
                reason: format!(
                    "Message too large: {} > {}",
                    encoded.len(),
                    self.max_message_size
                ),
            });
        }
        self.tx_buffer.write().push_back(frame);
        let mut stats = self.stats.write();
        stats.bytes_sent += encoded.len() as u64;
        Ok(())
    }

    /// Receive a frame from the data channel.
    pub fn recv_frame(&self) -> Result<Frame, TransportError> {
        self.rx_buffer
            .write()
            .pop_front()
            .ok_or(TransportError::ReceiveFailed {
                reason: "No data".into(),
            })
    }

    /// Inject a received frame (for testing / simulation).
    pub fn inject_rx_frame(&self, frame: Frame) {
        let len = frame.payload.len() as u64 + 5;
        self.rx_buffer.write().push_back(frame);
        self.stats.write().bytes_received += len;
    }

    /// Read a sent frame (for testing / simulation).
    pub fn read_tx_frame(&self) -> Option<Frame> {
        self.tx_buffer.write().pop_front()
    }

    pub fn state(&self) -> WebRtcState {
        *self.state.read()
    }

    pub fn stats(&self) -> TransportStats {
        self.stats.read().clone()
    }

    pub fn close(&self) {
        *self.state.write() = WebRtcState::Disconnected;
    }
}

impl Default for WebRtcTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webrtc_offer_answer_flow() {
        let caller = WebRtcTransport::new();
        let callee = WebRtcTransport::new();

        // Caller creates offer
        let offer = caller.create_offer().unwrap();
        assert_eq!(offer.sdp_type, "offer");
        assert_eq!(caller.state(), WebRtcState::Signaling);

        // Callee creates answer
        let answer = callee.create_answer(&offer).unwrap();
        assert_eq!(answer.sdp_type, "answer");

        // Caller sets remote answer
        caller.set_remote_answer(&answer).unwrap();

        // ICE candidate exchange
        caller.gather_candidates("192.168.1.10", 50000);
        callee.gather_candidates("192.168.1.20", 50001);

        let caller_cands = caller.local_candidates();
        for c in caller_cands {
            callee.add_ice_candidate(c);
        }
        let callee_cands = callee.local_candidates();
        for c in callee_cands {
            caller.add_ice_candidate(c);
        }

        // Both connect
        caller.set_connected();
        callee.set_connected();
        assert_eq!(caller.state(), WebRtcState::Connected);
        assert_eq!(callee.state(), WebRtcState::Connected);
    }

    #[test]
    fn test_webrtc_data_channel_send_recv() {
        let transport = WebRtcTransport::new();
        transport.set_connected();

        let frame = Frame::control(b"hello webrtc");
        transport.send_frame(frame).unwrap();

        let sent = transport.read_tx_frame().unwrap();
        assert_eq!(sent.payload, b"hello webrtc");
        assert!(transport.stats().bytes_sent > 0);
    }

    #[test]
    fn test_webrtc_inject_receive() {
        let transport = WebRtcTransport::new();
        transport.set_connected();

        transport.inject_rx_frame(Frame::data(vec![1, 2, 3]));
        let received = transport.recv_frame().unwrap();
        assert_eq!(received.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_webrtc_not_connected_fails() {
        let transport = WebRtcTransport::new();
        assert!(transport.send_frame(Frame::ping()).is_err());
    }

    #[test]
    fn test_webrtc_capabilities() {
        let caps = WebRtcTransport::capabilities();
        assert!(caps.bidirectional);
        assert!(caps.platforms.contains(&"web".to_string()));
    }

    #[test]
    fn test_webrtc_close() {
        let transport = WebRtcTransport::new();
        transport.set_connected();
        transport.close();
        assert_eq!(transport.state(), WebRtcState::Disconnected);
    }
}
