//! PeerSession — Authoritative session model for peer connections.
//!
//! One PeerSession per discovered/connected peer. Contains the connection,
//! chat history, transfer state, and session lifecycle.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::transport::tcp::TcpConnection;

/// Session state machine.
/// Discovered → Connecting → TcpConnected → HelloVerified → Authenticated
/// → PingVerified → SessionReady → Disconnected/Failed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Peer discovered via mDNS/subnet scan but not connected.
    Discovered,
    /// TCP connection attempt in progress.
    Connecting,
    /// TCP socket established.
    TcpConnected,
    /// Hello/HelloAck exchanged — peer identity verified.
    HelloVerified,
    /// Key exchange completed — encrypted channel established.
    Authenticated,
    /// Ping/Pong round-trip confirmed — bidirectional communication verified.
    PingVerified,
    /// Fully ready for chat and file transfers.
    SessionReady,
    /// Connection lost.
    Disconnected,
    /// Connection failed with reason.
    Failed(String),
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovered => write!(f, "Discovered"),
            Self::Connecting => write!(f, "Connecting"),
            Self::TcpConnected => write!(f, "TCP Connected"),
            Self::HelloVerified => write!(f, "Hello Verified"),
            Self::Authenticated => write!(f, "Authenticated"),
            Self::PingVerified => write!(f, "Ping Verified"),
            Self::SessionReady => write!(f, "Session Ready"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Failed(r) => write!(f, "Failed: {r}"),
        }
    }
}

impl SessionState {
    /// Whether this state represents an active/usable session.
    pub fn is_connected(&self) -> bool {
        matches!(
            self,
            Self::TcpConnected
                | Self::HelloVerified
                | Self::Authenticated
                | Self::PingVerified
                | Self::SessionReady
        )
    }

    /// Whether this state is fully ready for data exchange.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::SessionReady)
    }
}

/// Authentication state for the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthState {
    /// No authentication attempted.
    None,
    /// Key exchange in progress.
    KeyExchanging,
    /// Authenticated with session cipher.
    Authenticated,
    /// Authentication failed.
    Failed(String),
}

impl std::fmt::Display for AuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::KeyExchanging => write!(f, "Key Exchanging"),
            Self::Authenticated => write!(f, "Authenticated"),
            Self::Failed(r) => write!(f, "Failed: {r}"),
        }
    }
}

/// Direction of a message or transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageDirection {
    Outgoing,
    Incoming,
}

/// State of a chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageState {
    /// Being sent to peer.
    Sending,
    /// Sent (socket write succeeded).
    Sent,
    /// Delivery confirmed by peer (MessageAck received).
    Delivered,
    /// Send failed.
    Failed,
}

impl std::fmt::Display for MessageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sending => write!(f, "Sending"),
            Self::Sent => write!(f, "Sent"),
            Self::Delivered => write!(f, "Delivered"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// A chat message within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID.
    pub message_id: Uuid,
    /// Session this message belongs to.
    pub session_id: Uuid,
    /// Incoming or outgoing.
    pub direction: MessageDirection,
    /// When the message was created/received.
    pub timestamp: DateTime<Utc>,
    /// Message text content.
    pub content: String,
    /// Delivery state.
    pub state: MessageState,
    /// Error description if failed.
    pub error: Option<String>,
}

/// Transport type used for the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    TcpLan,
    Ble,
    WifiDirect,
    Usb,
    Quic,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TcpLan => write!(f, "TCP/LAN"),
            Self::Ble => write!(f, "BLE"),
            Self::WifiDirect => write!(f, "Wi-Fi Direct"),
            Self::Usb => write!(f, "USB"),
            Self::Quic => write!(f, "QUIC"),
        }
    }
}

/// The authoritative peer session model.
///
/// One PeerSession per peer device. Contains the connection, state machine,
/// chat history, and active transfer IDs.
pub struct PeerSession {
    /// Unique session identifier.
    pub session_id: Uuid,
    /// Remote peer's device ID (canonical key).
    pub peer_device_id: String,
    /// Remote peer's display name.
    pub peer_name: String,
    /// Local endpoint address.
    pub local_endpoint: Option<SocketAddr>,
    /// Remote endpoint address.
    pub remote_endpoint: Option<SocketAddr>,
    /// Transport type.
    pub transport: TransportType,
    /// Current session state.
    pub state: SessionState,
    /// Authentication state.
    pub auth_state: AuthState,
    /// Last successful heartbeat time.
    pub last_heartbeat: Option<Instant>,
    /// Missed heartbeat count.
    pub missed_heartbeats: u32,
    /// Peer capabilities.
    pub capabilities: Vec<String>,
    /// Chat message history (chronological).
    pub messages: Vec<ChatMessage>,
    /// Active transfer IDs for this session.
    pub active_transfers: Vec<Uuid>,
    /// When this session was created.
    pub created_at: DateTime<Utc>,
    /// The TCP connection (if connected).
    pub connection: Option<Arc<TcpConnection>>,
}

impl PeerSession {
    /// Create a new session for a discovered peer.
    pub fn new_discovered(peer_device_id: String, peer_name: String) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            peer_device_id,
            peer_name,
            local_endpoint: None,
            remote_endpoint: None,
            transport: TransportType::TcpLan,
            state: SessionState::Discovered,
            auth_state: AuthState::None,
            last_heartbeat: None,
            missed_heartbeats: 0,
            capabilities: Vec::new(),
            messages: Vec::new(),
            active_transfers: Vec::new(),
            created_at: Utc::now(),
            connection: None,
        }
    }

    /// Create a new session for an incoming connection (Hello received).
    pub fn new_incoming(
        peer_device_id: String,
        peer_name: String,
        remote_endpoint: SocketAddr,
        connection: Arc<TcpConnection>,
    ) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            peer_device_id,
            peer_name,
            local_endpoint: Some(connection.local_addr()),
            remote_endpoint: Some(remote_endpoint),
            transport: TransportType::TcpLan,
            state: SessionState::HelloVerified,
            auth_state: AuthState::None,
            last_heartbeat: None,
            missed_heartbeats: 0,
            capabilities: Vec::new(),
            messages: Vec::new(),
            active_transfers: Vec::new(),
            created_at: Utc::now(),
            connection: Some(connection),
        }
    }

    /// Transition session state. Enforces valid transitions.
    pub fn transition(&mut self, new_state: SessionState) -> Result<(), String> {
        let valid = match (&self.state, &new_state) {
            // Forward progression
            (SessionState::Discovered, SessionState::Connecting) => true,
            (SessionState::Connecting, SessionState::TcpConnected) => true,
            (SessionState::TcpConnected, SessionState::HelloVerified) => true,
            (SessionState::HelloVerified, SessionState::Authenticated) => true,
            (SessionState::Authenticated, SessionState::PingVerified) => true,
            (SessionState::PingVerified, SessionState::SessionReady) => true,
            // Skip auth for now (direct Hello → PingVerified path)
            (SessionState::HelloVerified, SessionState::PingVerified) => true,
            // Any state can disconnect/fail
            (_, SessionState::Disconnected) => true,
            (_, SessionState::Failed(_)) => true,
            // Reconnection from disconnected
            (SessionState::Disconnected, SessionState::Connecting) => true,
            (SessionState::Failed(_), SessionState::Connecting) => true,
            // Discovery can come from disconnected
            (SessionState::Disconnected, SessionState::Discovered) => true,
            _ => false,
        };

        if valid {
            log::info!(
                "Session {} ({}) state: {} → {}",
                self.session_id,
                self.peer_name,
                self.state,
                new_state
            );
            self.state = new_state;
            Ok(())
        } else {
            let msg = format!(
                "Invalid session transition: {} → {} for {}",
                self.state, new_state, self.peer_name
            );
            log::warn!("{}", msg);
            Err(msg)
        }
    }

    /// Add a chat message to the session history.
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Update a message's delivery state by message_id.
    pub fn update_message_state(&mut self, message_id: Uuid, new_state: MessageState) {
        if let Some(msg) = self
            .messages
            .iter_mut()
            .find(|m| m.message_id == message_id)
        {
            msg.state = new_state;
        }
    }

    /// Record a successful heartbeat.
    pub fn heartbeat_success(&mut self) {
        self.last_heartbeat = Some(Instant::now());
        self.missed_heartbeats = 0;
    }

    /// Record a missed heartbeat. Returns true if session should be disconnected.
    pub fn heartbeat_missed(&mut self, max_misses: u32) -> bool {
        self.missed_heartbeats += 1;
        self.missed_heartbeats >= max_misses
    }

    /// Serialize session info to JSON (without connection).
    pub fn to_json(&self) -> String {
        let msgs_json: Vec<String> = self
            .messages
            .iter()
            .rev()
            .take(50)
            .rev()
            .map(|m| {
                let escaped = m.content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                format!(
                    r#"{{"message_id":"{}","direction":"{}","timestamp":"{}","content":"{}","state":"{}"}}"#,
                    m.message_id,
                    if m.direction == MessageDirection::Outgoing { "out" } else { "in" },
                    m.timestamp.to_rfc3339(),
                    escaped,
                    m.state,
                )
            })
            .collect();

        let transfers_json: Vec<String> = self
            .active_transfers
            .iter()
            .map(|t| format!("\"{}\"", t))
            .collect();

        format!(
            r#"{{"session_id":"{}","peer_device_id":"{}","peer_name":"{}","local_endpoint":"{}","remote_endpoint":"{}","transport":"{}","state":"{}","auth_state":"{}","last_heartbeat_ago_ms":{},"missed_heartbeats":{},"capabilities":{:?},"messages":[{}],"active_transfers":[{}],"created_at":"{}"}}"#,
            self.session_id,
            self.peer_device_id,
            self.peer_name,
            self.local_endpoint
                .map(|e| e.to_string())
                .unwrap_or_default(),
            self.remote_endpoint
                .map(|e| e.to_string())
                .unwrap_or_default(),
            self.transport,
            self.state,
            self.auth_state,
            self.last_heartbeat
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0),
            self.missed_heartbeats,
            self.capabilities,
            msgs_json.join(","),
            transfers_json.join(","),
            self.created_at.to_rfc3339(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_display_and_helpers() {
        assert_eq!(SessionState::Discovered.to_string(), "Discovered");
        assert_eq!(SessionState::Connecting.to_string(), "Connecting");
        assert_eq!(SessionState::TcpConnected.to_string(), "TCP Connected");
        assert_eq!(SessionState::HelloVerified.to_string(), "Hello Verified");
        assert_eq!(SessionState::Authenticated.to_string(), "Authenticated");
        assert_eq!(SessionState::PingVerified.to_string(), "Ping Verified");
        assert_eq!(SessionState::SessionReady.to_string(), "Session Ready");
        assert_eq!(SessionState::Disconnected.to_string(), "Disconnected");
        assert_eq!(
            SessionState::Failed("err".into()).to_string(),
            "Failed: err"
        );

        assert!(!SessionState::Discovered.is_connected());
        assert!(!SessionState::Connecting.is_connected());
        assert!(SessionState::TcpConnected.is_connected());
        assert!(SessionState::HelloVerified.is_connected());
        assert!(SessionState::Authenticated.is_connected());
        assert!(SessionState::PingVerified.is_connected());
        assert!(SessionState::SessionReady.is_connected());
        assert!(!SessionState::Disconnected.is_connected());
        assert!(!SessionState::Failed("e".into()).is_connected());

        assert!(SessionState::SessionReady.is_ready());
        assert!(!SessionState::PingVerified.is_ready());
    }

    #[test]
    fn test_auth_and_message_state_display() {
        assert_eq!(AuthState::None.to_string(), "None");
        assert_eq!(AuthState::KeyExchanging.to_string(), "Key Exchanging");
        assert_eq!(AuthState::Authenticated.to_string(), "Authenticated");
        assert_eq!(AuthState::Failed("bad".into()).to_string(), "Failed: bad");

        assert_eq!(MessageState::Sending.to_string(), "Sending");
        assert_eq!(MessageState::Sent.to_string(), "Sent");
        assert_eq!(MessageState::Delivered.to_string(), "Delivered");
        assert_eq!(MessageState::Failed.to_string(), "Failed");

        assert_eq!(TransportType::TcpLan.to_string(), "TCP/LAN");
        assert_eq!(TransportType::Ble.to_string(), "BLE");
        assert_eq!(TransportType::WifiDirect.to_string(), "Wi-Fi Direct");
        assert_eq!(TransportType::Usb.to_string(), "USB");
        assert_eq!(TransportType::Quic.to_string(), "QUIC");
    }

    #[test]
    fn test_session_state_transitions_all_branches() {
        let mut session = PeerSession::new_discovered("dev1".into(), "Test".into());
        assert_eq!(session.state, SessionState::Discovered);

        assert!(session.transition(SessionState::Connecting).is_ok());
        assert!(session.transition(SessionState::TcpConnected).is_ok());
        assert!(session.transition(SessionState::HelloVerified).is_ok());
        assert!(session.transition(SessionState::Authenticated).is_ok());
        assert!(session.transition(SessionState::PingVerified).is_ok());
        assert!(session.transition(SessionState::SessionReady).is_ok());
        assert!(session
            .transition(SessionState::Failed("timeout".into()))
            .is_ok());
        assert!(session.transition(SessionState::Connecting).is_ok());
        assert!(session.transition(SessionState::Disconnected).is_ok());
        assert!(session.transition(SessionState::Discovered).is_ok());
    }

    #[test]
    fn test_invalid_transition() {
        let mut session = PeerSession::new_discovered("dev1".into(), "Test".into());
        assert!(session.transition(SessionState::SessionReady).is_err());
    }

    #[test]
    fn test_chat_message_lifecycle() {
        let mut session = PeerSession::new_discovered("dev1".into(), "Test".into());
        let msg_id = Uuid::new_v4();
        session.add_message(ChatMessage {
            message_id: msg_id,
            session_id: session.session_id,
            direction: MessageDirection::Outgoing,
            timestamp: Utc::now(),
            content: "Hello".into(),
            state: MessageState::Sending,
            error: None,
        });
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].state, MessageState::Sending);

        session.update_message_state(msg_id, MessageState::Delivered);
        assert_eq!(session.messages[0].state, MessageState::Delivered);

        // Update non-existent message state should be safe no-op
        session.update_message_state(Uuid::new_v4(), MessageState::Failed);
        assert_eq!(session.messages[0].state, MessageState::Delivered);
    }

    #[test]
    fn test_heartbeat_miss_detection() {
        let mut session = PeerSession::new_discovered("dev1".into(), "Test".into());
        assert!(!session.heartbeat_missed(3));
        assert!(!session.heartbeat_missed(3));
        assert!(session.heartbeat_missed(3)); // 3rd miss → should disconnect

        session.heartbeat_success();
        assert_eq!(session.missed_heartbeats, 0);
        assert!(session.last_heartbeat.is_some());
    }

    #[test]
    fn test_session_to_json_full() {
        let mut session = PeerSession::new_discovered("dev1".into(), "Test Device".into());
        session.local_endpoint = Some("127.0.0.1:42000".parse().unwrap());
        session.remote_endpoint = Some("127.0.0.1:42001".parse().unwrap());
        session.heartbeat_success();

        let transfer_id = Uuid::new_v4();
        session.active_transfers.push(transfer_id);

        let msg_id1 = Uuid::new_v4();
        session.add_message(ChatMessage {
            message_id: msg_id1,
            session_id: session.session_id,
            direction: MessageDirection::Incoming,
            timestamp: Utc::now(),
            content: "Line 1\nLine \"2\" \\ path".into(),
            state: MessageState::Delivered,
            error: None,
        });

        let json = session.to_json();
        assert!(json.contains("\"peer_device_id\":\"dev1\""));
        assert!(json.contains("\"state\":\"Discovered\""));
        assert!(json.contains("\"local_endpoint\":\"127.0.0.1:42000\""));
        assert!(json.contains("\"remote_endpoint\":\"127.0.0.1:42001\""));
        assert!(json.contains("Line \\\"2\\\""));
        assert!(json.contains(&transfer_id.to_string()));
    }
}
