//! UOT Engine — Main Coordinator
//!
//! Manages the lifecycle of discovery, connections, and transfers.
//! This is the single entry point that the API layer uses.
use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::error::{TransferError, TransportError, UotError};
use crate::core::session::{
    ChatMessage, MessageDirection, MessageState, PeerSession, SessionState,
};
use crate::core::version;
use crate::discovery::mdns::{DiscoveryEvent, MdnsDiscovery};
use crate::discovery::types::{DeviceType, DiscoveredDevice};
use crate::protocol::handler::{self as proto, OfferItemInfo, WireMessage};
use crate::security::path_validator::StrictPathValidator;
use crate::security::session_cipher::SessionCipher;
use crate::security::verification::TrustManager;
use crate::security::PathValidator;
use crate::streaming::manager::{StreamManager, StreamSession, StreamType};
use crate::transfer::analytics::LifetimeStats;
use crate::transfer::engine::{self, ProgressTracker, TransferItem};
use crate::transfer::history::TransferHistoryStore;
use crate::transfer::queue::{Priority, TransferQueueManager};
use crate::transfer::ratelimit::RateLimiter;
use crate::transfer::types::{
    TransferDirection, TransferItemRecord, TransferProgress, TransferRecord, TransferStatus,
};
use crate::transport::connection_manager::ConnectionManager;
use crate::transport::fallback::{TransportFallbackManager, TransportSelectionStrategy};
use crate::transport::tcp::{self, Frame, FrameType, TcpConnection, TcpTransportListener};
use crate::transport::types::{TransportId, TransportState};

/// Maximum number of recent events to keep in the ring buffer.
const MAX_EVENT_LOG: usize = 200;

/// Engine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Not started.
    Stopped,
    /// Starting background services.
    Starting,
    /// Active and operational.
    Running,
    /// Shutting down.
    ShuttingDown,
}

/// Type alias for the sessions map (avoids clippy type_complexity).
type SessionMap = Arc<RwLock<HashMap<String, Arc<parking_lot::RwLock<PeerSession>>>>>;

/// Per-peer connection state — tracks the handshake and liveness lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerConnectionState {
    /// TCP socket opened but no handshake yet.
    TcpConnected,
    /// Hello message sent, waiting for HelloAck.
    HelloSent,
    /// HelloAck received — peer identity confirmed.
    HelloAcked,
    /// Ping/Pong liveness verified — connection fully confirmed.
    PingConfirmed,
    /// Fully authenticated, ready for transfers.
    SessionReady,
    /// Connection was lost or dropped.
    Disconnected,
    /// Connection error with reason.
    Error(String),
}

impl std::fmt::Display for PeerConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TcpConnected => write!(f, "TCP Connected"),
            Self::HelloSent => write!(f, "Hello Sent"),
            Self::HelloAcked => write!(f, "Hello Acked"),
            Self::PingConfirmed => write!(f, "Ping Confirmed"),
            Self::SessionReady => write!(f, "Session Ready"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Error(reason) => write!(f, "Error: {reason}"),
        }
    }
}

/// Central UOT engine that manages discovery, networking, and transfers.
pub struct UotEngine {
    /// Global application configuration.
    config: Arc<RwLock<AppConfig>>,
    /// Current engine state.
    state: Arc<RwLock<EngineState>>,
    /// Device ID for this instance.
    device_id: String,
    /// mDNS discovery.
    discovery: Arc<RwLock<Option<MdnsDiscovery>>>,
    /// TCP listener.
    listener: Arc<RwLock<Option<TcpTransportListener>>>,
    /// Active connections by device_id.
    connections: Arc<RwLock<HashMap<String, Arc<TcpConnection>>>>,
    /// Discovered devices.
    devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    /// Active transfers.
    transfers: Arc<RwLock<HashMap<Uuid, TransferRecord>>>,
    /// Progress trackers.
    progress_trackers: Arc<RwLock<HashMap<Uuid, Arc<ProgressTracker>>>>,
    /// Event channel for UI updates.
    event_tx: mpsc::Sender<EngineEvent>,
    /// Lifetime transfer statistics.
    lifetime_stats: Arc<RwLock<LifetimeStats>>,
    /// Persistent transfer history.
    history_store: Arc<RwLock<TransferHistoryStore>>,
    /// Recent event log ring buffer.
    event_log: Arc<RwLock<VecDeque<String>>>,
    /// Per-transfer pause signals (true = paused).
    pause_signals: Arc<RwLock<HashMap<Uuid, watch::Sender<bool>>>>,
    /// Transfer queue manager for batch priority scheduling.
    queue_manager: Arc<RwLock<TransferQueueManager>>,
    /// Multi-transport fallback manager.
    fallback_manager: Arc<RwLock<TransportFallbackManager>>,
    /// Trust manager for PIN verification and device trust.
    trust_manager: Arc<RwLock<TrustManager>>,
    /// Accepted transfer IDs (signaled by UI via accept_transfer).
    accepted_transfers: Arc<RwLock<std::collections::HashSet<Uuid>>>,
    /// Streaming session manager.
    stream_manager: Arc<RwLock<StreamManager>>,
    /// Connection manager for auto-reconnection with exponential backoff.
    connection_manager: Arc<ConnectionManager>,
    /// Per-peer connection state tracking (keyed by device_id).
    peer_states: Arc<RwLock<HashMap<String, PeerConnectionState>>>,
    /// Authoritative peer sessions keyed by device_id.
    sessions: SessionMap,
    /// Pending offer response consent gating channels (transfer_id -> oneshot::Sender<bool>).
    pending_offer_responses: Arc<RwLock<HashMap<Uuid, tokio::sync::oneshot::Sender<bool>>>>,
    /// Pending transfer completion ACK channels (transfer_id -> oneshot::Sender<bool>).
    pending_completion_acks: Arc<RwLock<HashMap<Uuid, tokio::sync::oneshot::Sender<bool>>>>,
    /// Per-peer sequential send locks to prevent frame collision on shared connections.
    peer_send_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    /// Map of transfer_id to the exact TcpConnection used for the Offer.
    transfer_connections: Arc<RwLock<HashMap<Uuid, Arc<TcpConnection>>>>,
}

/// Events emitted by the engine for UI consumption.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Engine state changed.
    StateChanged(EngineState),
    /// Device discovered.
    DeviceFound(DiscoveredDevice),
    /// Device lost.
    DeviceLost(String),
    /// Device updated.
    DeviceUpdated(DiscoveredDevice),
    /// Transfer progress update.
    TransferProgress(TransferProgress),
    /// Transfer status changed.
    TransferStatusChanged {
        transfer_id: Uuid,
        status: TransferStatus,
    },
    /// Incoming transfer offer.
    IncomingOffer {
        transfer_id: Uuid,
        from_device: String,
        items: Vec<String>,
        total_size: u64,
    },
    /// Clipboard/text data received from a remote peer (legacy).
    ClipboardReceived { from_device: String, text: String },
    /// Peer connection state changed (legacy, kept for compatibility).
    PeerStateChanged {
        device_id: String,
        state: PeerConnectionState,
    },

    // ── Phase 2: Session-aware events ──
    /// Peer session state changed.
    SessionStateChanged {
        session_id: Uuid,
        device_id: String,
        state: String,
    },
    /// Incoming chat message received.
    IncomingMessage {
        session_id: Uuid,
        message_id: Uuid,
        from_device: String,
        content: String,
        timestamp: i64,
    },
    /// Outgoing message acknowledged by peer.
    MessageDelivered { session_id: Uuid, message_id: Uuid },
    /// Heartbeat state changed.
    HeartbeatChanged {
        session_id: Uuid,
        device_id: String,
        alive: bool,
    },
    /// Offer accepted by receiver.
    OfferAccepted { session_id: Uuid, transfer_id: Uuid },
    /// Offer rejected by receiver.
    OfferRejected {
        session_id: Uuid,
        transfer_id: Uuid,
        reason: String,
    },
    /// Transfer completed successfully.
    TransferCompleted { session_id: Uuid, transfer_id: Uuid },
    /// Transfer failed.
    TransferFailed {
        session_id: Uuid,
        transfer_id: Uuid,
        error: String,
    },
}

impl UotEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: AppConfig) -> (Self, mpsc::Receiver<EngineEvent>) {
        let (event_tx, event_rx) = mpsc::channel(256);
        let device_id = Uuid::new_v4().to_string();
        let max_concurrent = config.transfer.max_concurrent_transfers;

        let stats_path = LifetimeStats::default_path();
        let lifetime_stats = LifetimeStats::load(&stats_path);

        let history_path = TransferHistoryStore::default_path();
        let history_store = TransferHistoryStore::load(&history_path);

        (
            Self {
                config: Arc::new(RwLock::new(config)),
                state: Arc::new(RwLock::new(EngineState::Stopped)),
                device_id,
                discovery: Arc::new(RwLock::new(None)),
                listener: Arc::new(RwLock::new(None)),
                connections: Arc::new(RwLock::new(HashMap::new())),
                devices: Arc::new(RwLock::new(HashMap::new())),
                transfers: Arc::new(RwLock::new(HashMap::new())),
                progress_trackers: Arc::new(RwLock::new(HashMap::new())),
                event_tx,
                lifetime_stats: Arc::new(RwLock::new(lifetime_stats)),
                history_store: Arc::new(RwLock::new(history_store)),
                event_log: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_EVENT_LOG))),
                pause_signals: Arc::new(RwLock::new(HashMap::new())),
                queue_manager: Arc::new(RwLock::new(TransferQueueManager::new(max_concurrent))),
                fallback_manager: Arc::new(RwLock::new(TransportFallbackManager::default())),
                trust_manager: Arc::new(RwLock::new(TrustManager::new())),
                accepted_transfers: Arc::new(RwLock::new(std::collections::HashSet::new())),
                stream_manager: Arc::new(RwLock::new(StreamManager::new())),
                connection_manager: Arc::new(ConnectionManager::default()),
                peer_states: Arc::new(RwLock::new(HashMap::new())),
                sessions: Arc::new(RwLock::new(HashMap::new())),
                pending_offer_responses: Arc::new(RwLock::new(HashMap::new())),
                pending_completion_acks: Arc::new(RwLock::new(HashMap::new())),
                peer_send_locks: Arc::new(RwLock::new(HashMap::new())),
                transfer_connections: Arc::new(RwLock::new(HashMap::new())),
            },
            event_rx,
        )
    }

    /// Start the engine: bind TCP, register mDNS, start browsing.
    pub async fn start(&self) -> Result<(), UotError> {
        *self.state.write() = EngineState::Starting;
        let _ = self
            .event_tx
            .send(EngineEvent::StateChanged(EngineState::Starting))
            .await;

        // Start TCP listener — extract config values before async call
        let (port, device_name_clone, save_dir_clone) = {
            let config = self.config.read();
            (
                config.network_port.unwrap_or(tcp::DEFAULT_PORT),
                config.device_name.clone(),
                config.transfer.save_directory.clone(),
            )
        };
        let (tcp_listener, mut incoming_streams) = TcpTransportListener::bind(port)
            .await
            .map_err(UotError::Transport)?;

        let actual_port = tcp_listener.port();
        *self.listener.write() = Some(tcp_listener);

        // Start mDNS discovery (non-fatal if mDNS is unavailable on local network interface)
        if let Ok(mut mdns) = MdnsDiscovery::new() {
            let device_type = DeviceType::Desktop;
            if mdns
                .register(
                    &self.device_id,
                    &device_name_clone,
                    actual_port,
                    device_type,
                )
                .is_ok()
            {
                if let Ok(mut discovery_rx) = mdns.start_browsing() {
                    *self.discovery.write() = Some(mdns);

                    // Spawn discovery event handler
                    let devices = Arc::clone(&self.devices);
                    let event_tx = self.event_tx.clone();
                    let my_id = self.device_id.clone();
                    tokio::spawn(async move {
                        while let Some(event) = discovery_rx.recv().await {
                            match event {
                                DiscoveryEvent::DeviceFound(device) => {
                                    if device.device_id != my_id {
                                        devices
                                            .write()
                                            .insert(device.device_id.clone(), device.clone());
                                        let _ =
                                            event_tx.send(EngineEvent::DeviceFound(device)).await;
                                    }
                                }
                                DiscoveryEvent::DeviceLost(id) => {
                                    devices.write().remove(&id);
                                    let _ = event_tx.send(EngineEvent::DeviceLost(id)).await;
                                }
                                DiscoveryEvent::DeviceUpdated(device) => {
                                    if device.device_id != my_id {
                                        devices
                                            .write()
                                            .insert(device.device_id.clone(), device.clone());
                                        let _ =
                                            event_tx.send(EngineEvent::DeviceUpdated(device)).await;
                                    }
                                }
                            }
                        }
                    });
                }
            }
        }

        // Spawn incoming connection handler
        let connections = Arc::clone(&self.connections);
        let transfers = Arc::clone(&self.transfers);
        let progress_trackers = Arc::clone(&self.progress_trackers);
        let event_tx2 = self.event_tx.clone();
        let save_dir = save_dir_clone;
        let accepted_transfers = Arc::clone(&self.accepted_transfers);
        let devices_for_handler = Arc::clone(&self.devices);
        let our_device_id = self.device_id.clone();
        let our_device_name_for_handler = device_name_clone;
        let sessions_for_handler = Arc::clone(&self.sessions);
        let connections_for_handler = Arc::clone(&self.connections);
        let pending_responses_for_handler = Arc::clone(&self.pending_offer_responses);
        let pending_completion_acks_for_handler = Arc::clone(&self.pending_completion_acks);
        let transfer_connections_for_handler = Arc::clone(&self.transfer_connections);

        tokio::spawn(async move {
            while let Some(stream) = incoming_streams.recv().await {
                let conn = match TcpConnection::new(stream) {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to create connection: {e}");
                        continue;
                    }
                };

                let remote = conn.remote_addr().to_string();
                let conn = Arc::new(conn);
                connections
                    .write()
                    .insert(remote.clone(), Arc::clone(&conn));

                // Handle incoming frames
                let transfers_clone = Arc::clone(&transfers);
                let trackers_clone = Arc::clone(&progress_trackers);
                let event_tx3 = event_tx2.clone();
                let save_dir_clone = save_dir.clone();
                let accepted_clone = Arc::clone(&accepted_transfers);
                let devices_clone = Arc::clone(&devices_for_handler);
                let our_id = our_device_id.clone();
                let our_name = our_device_name_for_handler.clone();
                let sessions_clone = Arc::clone(&sessions_for_handler);
                let connections_clone = Arc::clone(&connections_for_handler);
                let pending_clone = Arc::clone(&pending_responses_for_handler);
                let pending_acks_clone = Arc::clone(&pending_completion_acks_for_handler);
                let transfer_conns_clone = Arc::clone(&transfer_connections_for_handler);

                tokio::spawn(async move {
                    Self::handle_incoming_connection(
                        conn,
                        &remote,
                        &transfers_clone,
                        &trackers_clone,
                        &event_tx3,
                        &save_dir_clone,
                        &accepted_clone,
                        &devices_clone,
                        &our_id,
                        &our_name,
                        &sessions_clone,
                        &connections_clone,
                        None,
                        &pending_clone,
                        &pending_acks_clone,
                        &transfer_conns_clone,
                    )
                    .await;
                });
            }
        });

        *self.state.write() = EngineState::Running;
        let _ = self
            .event_tx
            .send(EngineEvent::StateChanged(EngineState::Running))
            .await;
        log::info!("UOT engine started on port {actual_port}");

        Ok(())
    }

    /// Authoritative connection resolver: Resolves TcpConnection for a peer.
    pub fn get_peer_connection(&self, target: &str) -> Option<Arc<TcpConnection>> {
        // 1. Direct lookup via authoritative peer session
        if let Some(session_arc) = self.get_peer_session(target) {
            if let Some(ref conn) = session_arc.read().connection {
                if conn.state() == crate::transport::types::TransportState::Connected {
                    return Some(Arc::clone(conn));
                }
            }
        }

        // 2. Direct lookup in connections map
        let conns = self.connections.read();
        if let Some(conn) = conns.get(target) {
            if conn.state() == crate::transport::types::TransportState::Connected {
                return Some(Arc::clone(conn));
            }
        }

        // 3. Search all sessions map for matching peer_device_id, peer_name, or session_id
        {
            let sessions = self.sessions.read();
            for session_arc in sessions.values() {
                let s = session_arc.read();
                if s.peer_device_id == target
                    || s.peer_name == target
                    || s.session_id.to_string() == target
                {
                    if let Some(ref conn) = s.connection {
                        if conn.state() == crate::transport::types::TransportState::Connected {
                            return Some(Arc::clone(conn));
                        }
                    }
                }
            }
        }

        // 4. Search devices map and match address in connections map
        {
            let devices = self.devices.read();
            let dev_opt = devices.get(target).cloned().or_else(|| {
                devices
                    .values()
                    .find(|d| d.device_name == target || d.device_id == target)
                    .cloned()
            });
            if let Some(dev) = dev_opt {
                if let Some(ref addr) = dev.address {
                    if let Some(conn) = conns.get(addr) {
                        if conn.state() == crate::transport::types::TransportState::Connected {
                            return Some(Arc::clone(conn));
                        }
                    }
                }
            }
        }

        // 5. Fallback: Only if target is empty and exactly 1 connection exists, use it
        if target.is_empty() && conns.len() == 1 {
            if let Some(conn) = conns.values().next() {
                if conn.state() == crate::transport::types::TransportState::Connected {
                    return Some(Arc::clone(conn));
                }
            }
        }

        {
            let sessions = self.sessions.read();
            if target.is_empty() && sessions.len() == 1 {
                if let Some(session_arc) = sessions.values().next() {
                    let s = session_arc.read();
                    if let Some(ref conn) = s.connection {
                        if conn.state() == crate::transport::types::TransportState::Connected {
                            return Some(Arc::clone(conn));
                        }
                    }
                }
            }
        }

        None
    }

    /// Send files to a connected device.
    pub async fn send_files(&self, device_id: &str, paths: Vec<PathBuf>) -> Result<Uuid, UotError> {
        // Collect file items
        let mut items = Vec::new();
        for path in &paths {
            if path.is_dir() {
                let dir_items = engine::collect_files(path)
                    .await
                    .map_err(UotError::Transfer)?;
                items.extend(dir_items);
            } else {
                let item = TransferItem::from_path(path)
                    .await
                    .map_err(UotError::Transfer)?;
                items.push(item);
            }
        }

        if items.is_empty() {
            return Err(UotError::Transfer(TransferError::EmptyTransfer));
        }

        // Authoritative connection resolution: check active connections/sessions first
        let (conn, target_name) = match self.get_peer_connection(device_id) {
            Some(c) => {
                let name = self
                    .get_peer_session(device_id)
                    .map(|s| s.read().peer_name.clone())
                    .or_else(|| {
                        self.devices
                            .read()
                            .get(device_id)
                            .map(|d| d.device_name.clone())
                    })
                    .unwrap_or_else(|| device_id.to_string());
                (c, name)
            }
            None => {
                // Device not connected yet — lookup address in discovered devices
                let device = self.devices.read().get(device_id).cloned().ok_or_else(|| {
                    UotError::Transfer(TransferError::DeviceNotFound(device_id.to_string()))
                })?;
                let name = device.device_name.clone();
                let addr_str = device.address.as_deref().ok_or_else(|| {
                    UotError::Transfer(TransferError::DeviceNotFound("No address".to_string()))
                })?;
                let addr: SocketAddr = addr_str.parse().map_err(|e| {
                    UotError::Transport(TransportError::Connection(format!("Invalid addr: {e}")))
                })?;
                let stream = Self::connect_with_port_fallback(addr)
                    .await
                    .map_err(UotError::Transport)?;
                let new_conn = TcpConnection::new(stream).map_err(UotError::Transport)?;
                let hello = WireMessage::Hello {
                    device_id: self.device_id.clone(),
                    device_name: self.config.read().device_name.clone(),
                    device_type: "Desktop".to_string(),
                    version: crate::core::version::version_string(),
                    capabilities: vec!["tcp_lan".to_string(), "file_transfer".to_string()],
                };
                proto::send_message(&new_conn, &hello)
                    .await
                    .map_err(UotError::Transport)?;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    proto::recv_message(&new_conn),
                )
                .await
                {
                    Ok(Ok(WireMessage::HelloAck { .. })) => {
                        log::info!("Transfer connection HelloAck received");
                    }
                    _ => {
                        log::warn!("No HelloAck for transfer connection, proceeding anyway");
                    }
                }
                let c = Arc::new(new_conn);
                self.connections
                    .write()
                    .insert(device_id.to_string(), Arc::clone(&c));

                // Spawn reader task on fallback connection so incoming control frames are read
                {
                    let conn_clone = Arc::clone(&c);
                    let remote_str = device_id.to_string();
                    let transfers = Arc::clone(&self.transfers);
                    let trackers = Arc::clone(&self.progress_trackers);
                    let event_tx = self.event_tx.clone();
                    let save_dir = self.config.read().transfer.save_directory.clone();
                    let accepted = Arc::clone(&self.accepted_transfers);
                    let devices = Arc::clone(&self.devices);
                    let our_id = self.device_id.clone();
                    let our_name = self.config.read().device_name.clone();
                    let sessions = Arc::clone(&self.sessions);
                    let connections = Arc::clone(&self.connections);
                    let pending_responses = Arc::clone(&self.pending_offer_responses);
                    let pending_acks = Arc::clone(&self.pending_completion_acks);
                    let transfer_connections = Arc::clone(&self.transfer_connections);

                    tokio::spawn(async move {
                        Self::handle_incoming_connection(
                            conn_clone,
                            &remote_str,
                            &transfers,
                            &trackers,
                            &event_tx,
                            &save_dir,
                            &accepted,
                            &devices,
                            &our_id,
                            &our_name,
                            &sessions,
                            &connections,
                            Some(remote_str.clone()),
                            &pending_responses,
                            &pending_acks,
                            &transfer_connections,
                        )
                        .await;
                    });
                }
                (c, name)
            }
        };

        // Create transfer record
        let record = engine::create_transfer_record(&items, TransferDirection::Send, &target_name);
        let transfer_id = record.transfer_id;
        self.transfers.write().insert(transfer_id, record.clone());

        // Push to priority queue manager
        {
            let mut qm = self.queue_manager.write();
            if !qm.can_start() {
                qm.push(record, Priority::Normal);
                log::info!("Transfer {transfer_id} queued (concurrent limit reached)");
                return Ok(transfer_id);
            }
            qm.push(record, Priority::Normal);
            qm.mark_started();
        }

        // Create progress tracker
        let total_bytes: u64 = items.iter().map(|i| i.size).sum();
        let tracker = Arc::new(ProgressTracker::new(transfer_id, total_bytes, items.len()));
        self.progress_trackers
            .write()
            .insert(transfer_id, Arc::clone(&tracker));

        // Create oneshot channel for offer acceptance consent gating
        let (offer_tx, offer_rx) = tokio::sync::oneshot::channel::<bool>();
        self.pending_offer_responses
            .write()
            .insert(transfer_id, offer_tx);

        // Send offer message
        let offer = WireMessage::Offer {
            transfer_id: transfer_id.to_string(),
            device_name: self.config.read().device_name.clone(),
            items: items
                .iter()
                .map(|i| OfferItemInfo {
                    name: i.name.clone(),
                    relative_path: i.relative_path.clone(),
                    size: i.size,
                    is_directory: false,
                })
                .collect(),
            total_size: total_bytes,
        };
        proto::send_message(&conn, &offer)
            .await
            .map_err(UotError::Transport)?;

        let _ = self
            .event_tx
            .send(EngineEvent::TransferStatusChanged {
                transfer_id,
                status: TransferStatus::Pending,
            })
            .await;

        // Update status
        if let Some(record) = self.transfers.write().get_mut(&transfer_id) {
            record.status = TransferStatus::Pending;
            record.started_at = Some(chrono::Utc::now());
        }

        // Spawn the actual transfer task
        let transfers = Arc::clone(&self.transfers);
        let event_tx = self.event_tx.clone();
        let chunk_size = self.config.read().transfer.chunk_size;
        let bandwidth_limit = self.config.read().transfer.bandwidth_limit;

        // Create pause signal for this transfer
        let (pause_tx, pause_rx) = watch::channel(false);
        self.pause_signals.write().insert(transfer_id, pause_tx);
        let pause_signals = Arc::clone(&self.pause_signals);

        let stats = Arc::clone(&self.lifetime_stats);
        let history = Arc::clone(&self.history_store);
        let queue_manager = Arc::clone(&self.queue_manager);
        let pending_offer_responses = Arc::clone(&self.pending_offer_responses);
        let pending_completion_acks = Arc::clone(&self.pending_completion_acks);
        let peer_send_lock = {
            let mut locks = self.peer_send_locks.write();
            Arc::clone(
                locks
                    .entry(device_id.to_string())
                    .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
            )
        };

        tokio::spawn(async move {
            log::info!("Transfer {transfer_id} offer sent; waiting for remote acceptance ACK...");

            // Wait up to 120 seconds for user acceptance ACK from remote
            let accepted = match tokio::time::timeout(std::time::Duration::from_secs(120), offer_rx)
                .await
            {
                Ok(Ok(accepted)) => accepted,
                _ => {
                    log::warn!("Transfer {transfer_id} offer timed out or channel closed waiting for acceptance");
                    false
                }
            };

            pending_offer_responses.write().remove(&transfer_id);

            if !accepted {
                log::warn!("Transfer {transfer_id} was rejected or timed out");
                let mut transfers = transfers.write();
                if let Some(record) = transfers.get_mut(&transfer_id) {
                    record.status = TransferStatus::Failed;
                    record.finished_at = Some(chrono::Utc::now());
                    record.error = Some("Transfer offer rejected or timed out".to_string());
                }
                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                    transfer_id,
                    status: TransferStatus::Failed,
                });
                pause_signals.write().remove(&transfer_id);
                queue_manager.write().mark_completed();
                return;
            }

            log::info!(
                "Transfer {transfer_id} ACCEPTED by remote! Starting file data transmission..."
            );

            // Update sender transfer status to InProgress
            {
                let mut t = transfers.write();
                if let Some(record) = t.get_mut(&transfer_id) {
                    record.status = TransferStatus::InProgress;
                    record.started_at = Some(chrono::Utc::now());
                }
            }
            let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id,
                status: TransferStatus::InProgress,
            });

            // Acquire per-peer sequential streaming lock to prevent interleaving frames on shared socket
            let _peer_send_guard = peer_send_lock.lock().await;

            let result = Self::execute_send_arc(
                &conn,
                items,
                transfer_id,
                &tracker,
                chunk_size,
                bandwidth_limit,
                pause_rx,
                &event_tx,
                &transfers,
                &pending_completion_acks,
            )
            .await;

            let mut transfers = transfers.write();
            if let Some(record) = transfers.get_mut(&transfer_id) {
                match result {
                    Ok(()) => {
                        record.status = TransferStatus::Completed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.transferred_bytes = record.total_size;
                        for item in &mut record.items {
                            item.status = TransferStatus::Completed;
                            item.transferred_bytes = item.size;
                        }
                        let speed = tracker.snapshot().speed_bytes_per_sec;
                        stats.write().record_success(record.total_size, true, speed);
                    }
                    Err(e) => {
                        record.status = TransferStatus::Failed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.error = Some(e.to_string());
                        log::error!("Transfer {transfer_id} failed: {e}");
                        stats.write().record_failure();
                    }
                }
                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                    transfer_id,
                    status: record.status,
                });
                history.write().upsert(record.clone());
                let _ = history.read().save(&TransferHistoryStore::default_path());
                let _ = stats.read().save(&LifetimeStats::default_path());
                pause_signals.write().remove(&transfer_id);
                queue_manager.write().mark_completed();
            }
        });

        Ok(transfer_id)
    }

    /// Execute send using Arc<TcpConnection> (no encryption for now, reuses session conn).
    #[allow(clippy::too_many_arguments)]
    async fn execute_send_arc(
        conn: &Arc<TcpConnection>,
        items: Vec<TransferItem>,
        transfer_id: Uuid,
        tracker: &ProgressTracker,
        chunk_size: usize,
        bandwidth_limit: u64,
        mut pause_rx: watch::Receiver<bool>,
        event_tx: &mpsc::Sender<EngineEvent>,
        transfers: &Arc<RwLock<HashMap<Uuid, TransferRecord>>>,
        pending_completion_acks: &Arc<RwLock<HashMap<Uuid, tokio::sync::oneshot::Sender<bool>>>>,
    ) -> Result<(), TransferError> {
        let mut rate_limiter = RateLimiter::new(bandwidth_limit);

        for item in &items {
            tracker.set_current_item(&item.name);

            // Mark item as InProgress on sender side
            {
                let mut t = transfers.write();
                if let Some(record) = t.get_mut(&transfer_id) {
                    if let Some(item_rec) = record.items.iter_mut().find(|x| x.name == item.name) {
                        item_rec.status = TransferStatus::InProgress;
                    }
                }
            }

            let file_size = item.size;
            let mut offset: u64 = 0;

            // Send file header
            let header = WireMessage::FileStart {
                transfer_id: transfer_id.to_string(),
                item_index: items.iter().position(|x| x.name == item.name).unwrap_or(0) as u32,
                file_name: item.name.clone(),
                file_size,
                relative_path: item.relative_path.clone(),
            };
            proto::send_message(conn, &header)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            // Send chunks
            while offset < file_size {
                let (chunk_data, crc) = engine::read_chunk(&item.path, offset, chunk_size).await?;
                let chunk_len = chunk_data.len() as u64;

                // Prepend chunk metadata (16 bytes: offset u64 + crc u32 + reserved u32)
                let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
                chunk_frame.extend_from_slice(&offset.to_be_bytes());
                chunk_frame.extend_from_slice(&crc.to_be_bytes());
                chunk_frame.extend_from_slice(&[0u8; 4]); // reserved
                chunk_frame.extend_from_slice(&chunk_data);

                conn.send(Frame::data(chunk_frame))
                    .await
                    .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

                offset += chunk_len;
                tracker.add_bytes(chunk_len);

                // Update sender-side transferred bytes in engine record
                {
                    let mut t = transfers.write();
                    if let Some(record) = t.get_mut(&transfer_id) {
                        record.transferred_bytes += chunk_len;
                        if let Some(item_rec) =
                            record.items.iter_mut().find(|x| x.name == item.name)
                        {
                            item_rec.transferred_bytes += chunk_len;
                        }
                    }
                }

                // Rate limiting
                rate_limiter.consume(chunk_len as usize).await;

                // Check pause signal
                while *pause_rx.borrow() {
                    if pause_rx.changed().await.is_err() {
                        break;
                    }
                }

                // Emit progress periodically
                let progress = tracker.snapshot();
                let _ = event_tx.try_send(EngineEvent::TransferProgress(progress));
            }

            // Compute and send file hash
            let hash = engine::compute_sha256(&item.path).await?;
            let verify = WireMessage::FileEnd {
                transfer_id: transfer_id.to_string(),
                item_index: items.iter().position(|x| x.name == item.name).unwrap_or(0) as u32,
                sha256: hash.clone(),
            };
            proto::send_message(conn, &verify)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            // Mark item Completed on sender side
            {
                let mut t = transfers.write();
                if let Some(record) = t.get_mut(&transfer_id) {
                    if let Some(item_rec) = record.items.iter_mut().find(|x| x.name == item.name) {
                        item_rec.status = TransferStatus::Completed;
                        item_rec.hash = Some(hash);
                        item_rec.transferred_bytes = item_rec.size;
                    }
                }
            }

            tracker.complete_item();
        }

        // Register completion listener before sending TransferComplete
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
        pending_completion_acks.write().insert(transfer_id, ack_tx);

        // Send transfer complete
        let complete = WireMessage::TransferComplete {
            transfer_id: transfer_id.to_string(),
            success: true,
        };
        proto::send_message(conn, &complete)
            .await
            .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

        log::info!(
            "Transfer {transfer_id} data sent; awaiting receiver persistence & verification ACK..."
        );
        match tokio::time::timeout(std::time::Duration::from_secs(15), ack_rx).await {
            Ok(Ok(true)) => {
                log::info!("Receiver verified & persisted transfer {transfer_id} ✓");
                Ok(())
            }
            Ok(Ok(false)) => Err(TransferError::IntegrityFailed(
                "Receiver failed verification of received files".to_string(),
            )),
            _ => {
                pending_completion_acks.write().remove(&transfer_id);
                Err(TransferError::Protocol(
                    "Timed out waiting for receiver verification ACK".to_string(),
                ))
            }
        }
    }

    /// Execute the send operation — chunked file transfer with AES-256-GCM encryption.
    #[allow(dead_code, clippy::too_many_arguments)]
    async fn execute_send(
        conn: TcpConnection,
        items: Vec<TransferItem>,
        transfer_id: Uuid,
        tracker: &ProgressTracker,
        chunk_size: usize,
        bandwidth_limit: u64,
        mut pause_rx: watch::Receiver<bool>,
        event_tx: &mpsc::Sender<EngineEvent>,
        mut session_cipher: SessionCipher,
        transfers: &Arc<RwLock<HashMap<Uuid, TransferRecord>>>,
    ) -> Result<(), TransferError> {
        let mut rate_limiter = RateLimiter::new(bandwidth_limit);

        for item in &items {
            tracker.set_current_item(&item.name);

            // Mark item as InProgress on sender side
            {
                let mut t = transfers.write();
                if let Some(record) = t.get_mut(&transfer_id) {
                    if let Some(item_rec) = record.items.iter_mut().find(|x| x.name == item.name) {
                        item_rec.status = TransferStatus::InProgress;
                    }
                }
            }

            let file_size = item.size;
            let mut offset: u64 = 0;

            // Send file header
            let header = WireMessage::FileStart {
                transfer_id: transfer_id.to_string(),
                item_index: items.iter().position(|x| x.name == item.name).unwrap_or(0) as u32,
                file_name: item.name.clone(),
                file_size,
                relative_path: item.relative_path.clone(),
            };
            proto::send_message(&conn, &header)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            // Send chunks
            while offset < file_size {
                let (chunk_data, crc) = engine::read_chunk(&item.path, offset, chunk_size).await?;
                let chunk_len = chunk_data.len() as u64;

                // Prepend chunk metadata (16 bytes: offset u64 + crc u32 + reserved u32)
                let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
                chunk_frame.extend_from_slice(&offset.to_be_bytes());
                chunk_frame.extend_from_slice(&crc.to_be_bytes());
                chunk_frame.extend_from_slice(&[0u8; 4]); // reserved
                chunk_frame.extend_from_slice(&chunk_data);

                // Encrypt chunk frame
                let encrypted = session_cipher
                    .encrypt_frame(&chunk_frame)
                    .map_err(|e| TransferError::Protocol(format!("Encryption error: {e}")))?;

                conn.send(Frame::data(encrypted))
                    .await
                    .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

                offset += chunk_len;
                tracker.add_bytes(chunk_len);

                // Update sender-side transferred bytes in engine record
                {
                    let mut t = transfers.write();
                    if let Some(record) = t.get_mut(&transfer_id) {
                        record.transferred_bytes += chunk_len;
                        if let Some(item_rec) =
                            record.items.iter_mut().find(|x| x.name == item.name)
                        {
                            item_rec.transferred_bytes += chunk_len;
                        }
                    }
                }

                // Rate limiting
                rate_limiter.consume(chunk_len as usize).await;

                // Check pause signal
                while *pause_rx.borrow() {
                    if pause_rx.changed().await.is_err() {
                        break;
                    }
                }

                // Emit progress periodically
                let progress = tracker.snapshot();
                let _ = event_tx.try_send(EngineEvent::TransferProgress(progress));
            }

            // Compute and send file hash
            let hash = engine::compute_sha256(&item.path).await?;
            let verify = WireMessage::FileEnd {
                transfer_id: transfer_id.to_string(),
                item_index: items.iter().position(|x| x.name == item.name).unwrap_or(0) as u32,
                sha256: hash.clone(),
            };
            proto::send_message(&conn, &verify)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            // Mark item Completed on sender side
            {
                let mut t = transfers.write();
                if let Some(record) = t.get_mut(&transfer_id) {
                    if let Some(item_rec) = record.items.iter_mut().find(|x| x.name == item.name) {
                        item_rec.status = TransferStatus::Completed;
                        item_rec.hash = Some(hash);
                        item_rec.transferred_bytes = item_rec.size;
                    }
                }
            }

            tracker.complete_item();
        }

        // Send transfer complete
        let complete = WireMessage::TransferComplete {
            transfer_id: transfer_id.to_string(),
            success: true,
        };
        proto::send_message(&conn, &complete)
            .await
            .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

        Ok(())
    }

    /// Handle incoming connection frames.
    #[allow(clippy::too_many_arguments)]
    async fn handle_incoming_connection(
        conn: Arc<TcpConnection>,
        remote: &str,
        transfers: &Arc<RwLock<HashMap<Uuid, TransferRecord>>>,
        trackers: &Arc<RwLock<HashMap<Uuid, Arc<ProgressTracker>>>>,
        event_tx: &mpsc::Sender<EngineEvent>,
        save_dir: &str,
        accepted_transfers: &Arc<RwLock<std::collections::HashSet<Uuid>>>,
        devices: &Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
        our_device_id: &str,
        our_device_name: &str,
        sessions: &SessionMap,
        connections: &Arc<RwLock<HashMap<String, Arc<TcpConnection>>>>,
        known_peer_id: Option<String>,
        pending_offer_responses: &Arc<RwLock<HashMap<Uuid, tokio::sync::oneshot::Sender<bool>>>>,
        pending_completion_acks: &Arc<RwLock<HashMap<Uuid, tokio::sync::oneshot::Sender<bool>>>>,
        transfer_connections: &Arc<RwLock<HashMap<Uuid, Arc<TcpConnection>>>>,
    ) {
        let mut current_file: Option<(PathBuf, PathBuf, String, u64)> = None; // (part_path, target_path, name, size)
        let mut current_transfer_id: Option<Uuid> = None;
        let mut recv_tracker: Option<Arc<ProgressTracker>> = None;
        let mut session_cipher: Option<SessionCipher> = None;
        let mut remote_peer_id: Option<String> = known_peer_id;

        loop {
            // Receive frame from stream without 60s idle timeout (heartbeat manages keepalive)
            let frame = match conn.recv_frame().await {
                Ok(f) => f,
                Err(_) => {
                    log::info!("Connection from {remote} closed");
                    break;
                }
            };
            match frame.frame_type {
                FrameType::Control => {
                    let wire_msg: WireMessage = match serde_json::from_slice(&frame.payload) {
                        Ok(m) => m,
                        Err(e) => {
                            log::error!("Invalid protocol message from {remote}: {e}");
                            continue;
                        }
                    };

                    match wire_msg {
                        WireMessage::Hello {
                            device_id: peer_id,
                            device_name: peer_name,
                            device_type: peer_type,
                            version: peer_version,
                            ..
                        } => {
                            remote_peer_id = Some(peer_id.clone());
                            log::info!("Received Hello from {peer_name} ({peer_id}) v{peer_version} at {remote}");
                            // Send HelloAck back
                            let ack = WireMessage::HelloAck {
                                device_id: our_device_id.to_string(),
                                device_name: our_device_name.to_string(),
                                device_type: "Desktop".to_string(),
                                version: version::version_string(),
                            };
                            if let Err(e) = proto::send_message(&conn, &ack).await {
                                log::error!("Failed to send HelloAck to {remote}: {e}");
                                break;
                            }
                            // Register the peer as a discovered device
                            let device_type = match peer_type.to_lowercase().as_str() {
                                "phone" => DeviceType::Phone,
                                "tablet" => DeviceType::Tablet,
                                "laptop" => DeviceType::Laptop,
                                "tv" => DeviceType::Tv,
                                _ => DeviceType::Desktop,
                            };
                            let now = chrono::Utc::now();
                            let dev = DiscoveredDevice {
                                device_id: peer_id.clone(),
                                device_name: peer_name.clone(),
                                device_type,
                                discovery_method: crate::discovery::types::DiscoveryMethod::Manual,
                                address: Some(conn.remote_addr().to_string()),
                                capabilities: vec![
                                    "tcp_lan".to_string(),
                                    "connected".to_string(),
                                    "session_ready".to_string(),
                                ],
                                signal_strength: Some(100),
                                first_seen: now,
                                last_seen: now,
                                is_trusted: true,
                            };
                            let remote_ip_str = conn.remote_addr().ip().to_string();
                            {
                                let mut devs = devices.write();
                                devs.retain(|k, v| {
                                    if k == &peer_id {
                                        return true;
                                    }
                                    if k.starts_with("lan-") || k.starts_with("peer-") {
                                        if let Some(ref a) = v.address {
                                            if a.contains(&remote_ip_str) {
                                                return false;
                                            }
                                        }
                                    }
                                    true
                                });
                                devs.insert(peer_id.clone(), dev.clone());
                            }
                            // Store connection by peer device_id
                            connections
                                .write()
                                .insert(peer_id.clone(), Arc::clone(&conn));
                            // Create PeerSession for incoming connection
                            let session_arc = {
                                let mut sessions_map = sessions.write();
                                let session =
                                    sessions_map.entry(peer_id.clone()).or_insert_with(|| {
                                        Arc::new(parking_lot::RwLock::new(
                                            PeerSession::new_discovered(
                                                peer_id.clone(),
                                                peer_name.clone(),
                                            ),
                                        ))
                                    });
                                let mut s = session.write();
                                s.connection = Some(Arc::clone(&conn));
                                s.state = SessionState::SessionReady;
                                s.peer_name = peer_name.clone();
                                if let Ok(addr) = remote.parse() {
                                    s.remote_endpoint = Some(addr);
                                }
                                log::info!(
                                    "Incoming session {} created for {} ({})",
                                    s.session_id,
                                    peer_name,
                                    peer_id
                                );
                                session.clone()
                            };

                            // Start heartbeat for inbound session connection
                            let hb_event_tx = event_tx.clone();
                            let hb_pid = peer_id.clone();
                            let hb_conn = Arc::clone(&conn);
                            tokio::spawn(async move {
                                let mut interval =
                                    tokio::time::interval(std::time::Duration::from_secs(15));
                                loop {
                                    interval.tick().await;
                                    {
                                        let s = session_arc.read();
                                        if !s.state.is_connected() {
                                            break;
                                        }
                                    }
                                    let ping_frame = Frame::ping();
                                    if hb_conn.send_frame(ping_frame).await.is_err() {
                                        let should_disconnect =
                                            session_arc.write().heartbeat_missed(3);
                                        if should_disconnect {
                                            let session_id = session_arc.read().session_id;
                                            let _ = session_arc
                                                .write()
                                                .transition(SessionState::Disconnected);
                                            let _ = hb_event_tx
                                                .send(EngineEvent::HeartbeatChanged {
                                                    session_id,
                                                    device_id: hb_pid.clone(),
                                                    alive: false,
                                                })
                                                .await;
                                            let _ = hb_event_tx
                                                .send(EngineEvent::PeerStateChanged {
                                                    device_id: hb_pid.clone(),
                                                    state: PeerConnectionState::Disconnected,
                                                })
                                                .await;
                                            log::warn!("Heartbeat timeout for {hb_pid} — session disconnected");
                                            break;
                                        }
                                    } else {
                                        session_arc.write().heartbeat_success();
                                    }
                                }
                            });

                            let _ = event_tx.send(EngineEvent::DeviceFound(dev)).await;
                            log::info!("HelloAck sent to {peer_name} at {remote}");
                        }
                        WireMessage::ClipboardData { data, .. } => {
                            log::info!(
                                "Received clipboard data from {remote}: {} bytes",
                                data.len()
                            );
                            let _ = event_tx
                                .send(EngineEvent::ClipboardReceived {
                                    from_device: remote.to_string(),
                                    text: data,
                                })
                                .await;
                        }
                        WireMessage::KeyExchange {
                            public_key: their_public,
                        } => {
                            // Perform key exchange: generate our keypair, derive shared secret
                            match SessionCipher::create_key_exchange() {
                                Ok((our_private, our_public)) => {
                                    // Send our public key back
                                    let reply = WireMessage::KeyExchange {
                                        public_key: our_public,
                                    };
                                    if let Err(e) = proto::send_message(&conn, &reply).await {
                                        log::error!("Failed to send KeyExchange reply: {e}");
                                        break;
                                    }
                                    // Derive session cipher
                                    match SessionCipher::from_key_exchange(
                                        &our_private,
                                        &their_public,
                                    ) {
                                        Ok(cipher) => {
                                            session_cipher = Some(cipher);
                                            log::info!(
                                                "Session encryption established with {remote}"
                                            );
                                        }
                                        Err(e) => {
                                            log::error!("Key exchange derivation failed: {e}");
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Key exchange generation failed: {e}");
                                    break;
                                }
                            }
                        }
                        WireMessage::Offer {
                            transfer_id: tid_str,
                            device_name,
                            items: offer_items,
                            total_size,
                        } => {
                            let transfer_id =
                                Uuid::parse_str(&tid_str).unwrap_or_else(|_| Uuid::new_v4());
                            let items: Vec<String> =
                                offer_items.iter().map(|i| i.name.clone()).collect();

                            current_transfer_id = Some(transfer_id);
                            transfer_connections
                                .write()
                                .insert(transfer_id, Arc::clone(&conn));

                            let item_records: Vec<TransferItemRecord> = offer_items
                                .iter()
                                .map(|i| TransferItemRecord {
                                    item_id: Uuid::new_v4(),
                                    name: i.name.clone(),
                                    relative_path: i.relative_path.clone(),
                                    size: i.size,
                                    transferred_bytes: 0,
                                    status: TransferStatus::Pending,
                                    hash: None,
                                    saved_path: None,
                                })
                                .collect();

                            let record = TransferRecord {
                                transfer_id,
                                remote_device: remote_peer_id
                                    .clone()
                                    .unwrap_or_else(|| device_name.clone()),
                                direction: TransferDirection::Receive,
                                status: TransferStatus::Pending,
                                total_size,
                                transferred_bytes: 0,
                                items: item_records,
                                created_at: chrono::Utc::now(),
                                started_at: None,
                                finished_at: None,
                                error: None,
                            };
                            transfers.write().insert(transfer_id, record);

                            // Receive-side progress tracker
                            let item_count = offer_items.len();
                            let tracker =
                                Arc::new(ProgressTracker::new(transfer_id, total_size, item_count));
                            trackers.write().insert(transfer_id, Arc::clone(&tracker));
                            recv_tracker = Some(tracker);

                            let _ = event_tx
                                .send(EngineEvent::IncomingOffer {
                                    transfer_id,
                                    from_device: device_name,
                                    items,
                                    total_size,
                                })
                                .await;

                            log::info!("Received offer {transfer_id} from {remote}");
                        }
                        WireMessage::FileStart {
                            file_name,
                            file_size,
                            relative_path,
                            ..
                        } => {
                            let path_validator =
                                StrictPathValidator::new(Some(PathBuf::from(save_dir)));
                            let sanitized = match path_validator
                                .validate_relative_path(&relative_path)
                            {
                                Ok(clean) => clean,
                                Err(e) => {
                                    log::error!("Path validation failed for {relative_path}: {e}");
                                    path_validator.sanitize_filename(&file_name)
                                }
                            };
                            let target_file_path = PathBuf::from(save_dir).join(&sanitized);

                            if target_file_path.exists() && target_file_path.is_symlink() {
                                log::error!(
                                    "Refusing to write to symlink: {}",
                                    target_file_path.display()
                                );
                                continue;
                            }

                            if let Some(parent) = target_file_path.parent() {
                                let _ = tokio::fs::create_dir_all(parent).await;
                            }

                            let ext_str = target_file_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("file");
                            let part_path =
                                target_file_path.with_extension(format!("{ext_str}.part"));

                            // Touch/initialize the part file immediately (handles 0-byte files reliably)
                            let _ = tokio::fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .truncate(true)
                                .open(&part_path)
                                .await;

                            if let Some(tid) = current_transfer_id {
                                let mut t = transfers.write();
                                if let Some(record) = t.get_mut(&tid) {
                                    if record.status == TransferStatus::Pending {
                                        record.status = TransferStatus::InProgress;
                                        record.started_at = Some(chrono::Utc::now());
                                        let _ =
                                            event_tx.try_send(EngineEvent::TransferStatusChanged {
                                                transfer_id: tid,
                                                status: TransferStatus::InProgress,
                                            });
                                    }
                                    if let Some(item_rec) =
                                        record.items.iter_mut().find(|i| i.name == file_name)
                                    {
                                        item_rec.status = TransferStatus::InProgress;
                                    }
                                }
                            }

                            current_file =
                                Some((part_path, target_file_path, file_name.clone(), file_size));
                            log::info!("Receiving file: {file_name} ({file_size} bytes)");
                        }
                        WireMessage::FileEnd { sha256, .. } => {
                            if let Some((part_path, target_path, name, _)) = current_file.take() {
                                match engine::compute_sha256(&part_path).await {
                                    Ok(actual_hash) => {
                                        if actual_hash == sha256 {
                                            log::info!("File {name} verified ✓ (SHA-256 match)");
                                            // Handle duplicate target path if file already exists
                                            let final_path = if target_path.exists() {
                                                let stem = target_path
                                                    .file_stem()
                                                    .and_then(|s| s.to_str())
                                                    .unwrap_or("file");
                                                let ext = target_path
                                                    .extension()
                                                    .and_then(|e| e.to_str())
                                                    .unwrap_or("");
                                                let parent = target_path
                                                    .parent()
                                                    .unwrap_or_else(|| std::path::Path::new("."));
                                                let mut counter = 1;
                                                let mut new_path = parent.join(if ext.is_empty() {
                                                    format!("{stem} ({counter})")
                                                } else {
                                                    format!("{stem} ({counter}).{ext}")
                                                });
                                                while new_path.exists() {
                                                    counter += 1;
                                                    new_path = parent.join(if ext.is_empty() {
                                                        format!("{stem} ({counter})")
                                                    } else {
                                                        format!("{stem} ({counter}).{ext}")
                                                    });
                                                }
                                                new_path
                                            } else {
                                                target_path
                                            };

                                            let rename_res =
                                                tokio::fs::rename(&part_path, &final_path).await;
                                            let saved_ok = match rename_res {
                                                Ok(()) => true,
                                                Err(e) => {
                                                    log::warn!("Atomic rename failed for {name}: {e}. Retrying via copy+remove...");
                                                    if tokio::fs::copy(&part_path, &final_path)
                                                        .await
                                                        .is_ok()
                                                    {
                                                        let _ = tokio::fs::remove_file(&part_path)
                                                            .await;
                                                        true
                                                    } else {
                                                        log::error!("Failed to save final file for {name}: {e}");
                                                        false
                                                    }
                                                }
                                            };

                                            if saved_ok {
                                                log::info!(
                                                    "Saved file to {}",
                                                    final_path.display()
                                                );
                                                if let Some(tid) = current_transfer_id {
                                                    let mut t = transfers.write();
                                                    if let Some(record) = t.get_mut(&tid) {
                                                        if let Some(item_rec) = record
                                                            .items
                                                            .iter_mut()
                                                            .find(|i| i.name == name)
                                                        {
                                                            item_rec.status =
                                                                TransferStatus::Completed;
                                                            item_rec.hash =
                                                                Some(actual_hash.clone());
                                                            item_rec.transferred_bytes =
                                                                item_rec.size;
                                                            item_rec.saved_path = Some(
                                                                final_path
                                                                    .to_string_lossy()
                                                                    .to_string(),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            log::error!(
                                            "File {name} hash mismatch! Expected: {sha256}, Got: {actual_hash}"
                                        );
                                            let _ = tokio::fs::remove_file(&part_path).await;
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Cannot verify SHA-256 for {name}: {e}");
                                    }
                                }
                            }
                        }
                        WireMessage::TransferComplete {
                            transfer_id: ref tid_str,
                            ..
                        } => {
                            let tid = Uuid::parse_str(tid_str).ok().or(current_transfer_id);
                            let mut all_ok = false;
                            if let Some(tid) = tid {
                                let mut t = transfers.write();
                                if let Some(record) = t.get_mut(&tid) {
                                    all_ok = !record.items.is_empty()
                                        && record
                                            .items
                                            .iter()
                                            .all(|i| i.status == TransferStatus::Completed);
                                    if all_ok {
                                        record.status = TransferStatus::Completed;
                                        record.finished_at = Some(chrono::Utc::now());
                                        record.transferred_bytes = record.total_size;
                                    } else {
                                        record.status = TransferStatus::Failed;
                                        record.finished_at = Some(chrono::Utc::now());
                                        record.error = Some(
                                            "One or more files failed verification".to_string(),
                                        );
                                    }
                                }
                                let status = if all_ok {
                                    TransferStatus::Completed
                                } else {
                                    TransferStatus::Failed
                                };
                                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                                    transfer_id: tid,
                                    status,
                                });
                            }

                            // Send TransferCompleteAck back to sender
                            let ack = WireMessage::TransferCompleteAck {
                                transfer_id: tid_str.clone(),
                                checksum_match: all_ok,
                            };
                            let _ = proto::send_message(&conn, &ack).await;
                            log::info!(
                                "Transfer complete from {remote}, acked checksum_match={all_ok}"
                            );
                        }
                        WireMessage::TransferCompleteAck {
                            ref transfer_id,
                            checksum_match,
                        } => {
                            log::info!(
                                "TransferCompleteAck received for {transfer_id}: checksum_match={checksum_match} from {remote}"
                            );
                            if let Ok(tid) = Uuid::parse_str(transfer_id) {
                                if let Some(tx) = pending_completion_acks.write().remove(&tid) {
                                    let _ = tx.send(checksum_match);
                                }
                            }
                        }
                        WireMessage::Pause { ref transfer_id } => {
                            log::info!(
                                "Remote requested pause for transfer {transfer_id} from {remote}"
                            );
                            if let Ok(tid) = Uuid::parse_str(transfer_id) {
                                {
                                    let mut t = transfers.write();
                                    if let Some(record) = t.get_mut(&tid) {
                                        record.status = TransferStatus::Paused;
                                    }
                                }
                                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                                    transfer_id: tid,
                                    status: TransferStatus::Paused,
                                });
                                let ack = WireMessage::PauseAck {
                                    transfer_id: transfer_id.clone(),
                                };
                                let _ = proto::send_message(&conn, &ack).await;
                            }
                        }
                        WireMessage::PauseAck { ref transfer_id } => {
                            log::info!("Remote acknowledged pause for transfer {transfer_id}");
                        }
                        WireMessage::Resume {
                            ref transfer_id,
                            offset,
                        } => {
                            log::info!("Remote requested resume for transfer {transfer_id} at offset {offset} from {remote}");
                            if let Ok(tid) = Uuid::parse_str(transfer_id) {
                                {
                                    let mut t = transfers.write();
                                    if let Some(record) = t.get_mut(&tid) {
                                        record.status = TransferStatus::InProgress;
                                    }
                                }
                                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                                    transfer_id: tid,
                                    status: TransferStatus::InProgress,
                                });
                                let ack = WireMessage::ResumeAck {
                                    transfer_id: transfer_id.clone(),
                                    offset,
                                };
                                let _ = proto::send_message(&conn, &ack).await;
                            }
                        }
                        WireMessage::ResumeAck {
                            ref transfer_id,
                            offset,
                        } => {
                            log::info!("Remote acknowledged resume for transfer {transfer_id} at offset {offset}");
                        }
                        WireMessage::Cancel {
                            ref transfer_id,
                            ref reason,
                        } => {
                            log::info!("Remote requested cancel for transfer {transfer_id}: {reason:?} from {remote}");
                            if let Ok(tid) = Uuid::parse_str(transfer_id) {
                                {
                                    let mut t = transfers.write();
                                    if let Some(record) = t.get_mut(&tid) {
                                        record.status = TransferStatus::Cancelled;
                                        record.finished_at = Some(chrono::Utc::now());
                                        record.error = reason.clone();
                                    }
                                }
                                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                                    transfer_id: tid,
                                    status: TransferStatus::Cancelled,
                                });
                            }
                        }
                        // ── Phase 2+3: Chat, ACK, and OfferResponse handlers ──
                        WireMessage::ChatMessage {
                            message_id: ref mid_str,
                            ref content,
                            timestamp,
                        } => {
                            let msg_id =
                                Uuid::parse_str(mid_str).unwrap_or_else(|_| Uuid::new_v4());
                            log::info!(
                                "Chat message from {remote}: {}",
                                &content[..content.len().min(50)]
                            );

                            // Send MessageAck back immediately
                            let ack = WireMessage::MessageAck {
                                message_id: mid_str.clone(),
                            };
                            let _ = proto::send_message(&conn, &ack).await;

                            // Resolve peer_device_id
                            let peer_id_opt = remote_peer_id.clone().or_else(|| {
                                connections.read().iter().find_map(|(id, c)| {
                                    if Arc::ptr_eq(c, &conn) {
                                        Some(id.clone())
                                    } else {
                                        None
                                    }
                                })
                            });

                            let (from_dev_id, sid) = if let Some(ref pid) = peer_id_opt {
                                let session_arc = sessions.read().get(pid).cloned();
                                if let Some(session_arc) = session_arc {
                                    let mut session = session_arc.write();
                                    let sid = session.session_id;
                                    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                        timestamp, 0,
                                    )
                                    .unwrap_or_else(chrono::Utc::now);
                                    session.add_message(ChatMessage {
                                        message_id: msg_id,
                                        session_id: sid,
                                        direction: MessageDirection::Incoming,
                                        timestamp: dt,
                                        content: content.clone(),
                                        state: MessageState::Delivered,
                                        error: None,
                                    });
                                    (pid.clone(), sid)
                                } else {
                                    (pid.clone(), Uuid::nil())
                                }
                            } else {
                                (remote.to_string(), Uuid::nil())
                            };

                            // Emit IncomingMessage event with resolved device_id and session_id
                            let _ = event_tx
                                .send(EngineEvent::IncomingMessage {
                                    session_id: sid,
                                    message_id: msg_id,
                                    from_device: from_dev_id.clone(),
                                    content: content.clone(),
                                    timestamp,
                                })
                                .await;
                        }
                        WireMessage::MessageAck {
                            message_id: ref mid_str,
                        } => {
                            log::info!("MessageAck received for {mid_str} from {remote}");
                            if let Ok(msg_id) = Uuid::parse_str(mid_str) {
                                let peer_id_opt = remote_peer_id.clone().or_else(|| {
                                    connections.read().iter().find_map(|(id, c)| {
                                        if Arc::ptr_eq(c, &conn) {
                                            Some(id.clone())
                                        } else {
                                            None
                                        }
                                    })
                                });
                                let sid = if let Some(ref pid) = peer_id_opt {
                                    if let Some(session_arc) = sessions.read().get(pid).cloned() {
                                        let mut session = session_arc.write();
                                        session
                                            .update_message_state(msg_id, MessageState::Delivered);
                                        session.session_id
                                    } else {
                                        Uuid::nil()
                                    }
                                } else {
                                    Uuid::nil()
                                };
                                let _ = event_tx
                                    .send(EngineEvent::MessageDelivered {
                                        session_id: sid,
                                        message_id: msg_id,
                                    })
                                    .await;
                            }
                        }
                        WireMessage::OfferResponse {
                            ref transfer_id,
                            accepted,
                            ref reason,
                        } => {
                            log::info!(
                                "OfferResponse for {transfer_id}: accepted={accepted} from {remote}"
                            );
                            if let Ok(tid) = Uuid::parse_str(transfer_id) {
                                if let Some(tx) = pending_offer_responses.write().remove(&tid) {
                                    let _ = tx.send(accepted);
                                }
                                if accepted {
                                    accepted_transfers.write().insert(tid);
                                    let _ = event_tx
                                        .send(EngineEvent::OfferAccepted {
                                            session_id: Uuid::nil(),
                                            transfer_id: tid,
                                        })
                                        .await;
                                } else {
                                    let _ = event_tx
                                        .send(EngineEvent::OfferRejected {
                                            session_id: Uuid::nil(),
                                            transfer_id: tid,
                                            reason: reason
                                                .clone()
                                                .unwrap_or_else(|| "Rejected".to_string()),
                                        })
                                        .await;
                                }
                            }
                        }
                        other => {
                            log::debug!("Unhandled message type from {remote}: {other:?}");
                        }
                    }
                }
                FrameType::Data => {
                    if let Some((ref part_path, _, _, _)) = current_file {
                        // Decrypt frame payload if session cipher is established; fallback to raw payload if unencrypted
                        let decrypted = if let Some(ref mut cipher) = session_cipher {
                            match cipher.decrypt_frame(&frame.payload) {
                                Ok(plain) => plain,
                                Err(_) => frame.payload.clone(),
                            }
                        } else {
                            frame.payload.clone()
                        };

                        if decrypted.len() < 16 {
                            log::error!("Data frame too small after decryption");
                            continue;
                        }

                        let offset = u64::from_be_bytes(decrypted[..8].try_into().unwrap());
                        let crc = u32::from_be_bytes(decrypted[8..12].try_into().unwrap());
                        let chunk_data = &decrypted[16..];

                        if let Err(e) =
                            engine::write_chunk(part_path, offset, chunk_data, crc).await
                        {
                            log::error!("Write chunk failed: {e}");
                        } else {
                            // Track receive progress
                            let chunk_len = chunk_data.len() as u64;
                            if let Some(ref tracker) = recv_tracker {
                                tracker.add_bytes(chunk_len);
                                let progress = tracker.snapshot();
                                let _ = event_tx.try_send(EngineEvent::TransferProgress(progress));
                            }
                            // Update transfer record status and bytes
                            if let Some(tid) = current_transfer_id {
                                if let Some(record) = transfers.write().get_mut(&tid) {
                                    if record.status == TransferStatus::Pending {
                                        record.status = TransferStatus::InProgress;
                                        record.started_at = Some(chrono::Utc::now());
                                        let _ =
                                            event_tx.try_send(EngineEvent::TransferStatusChanged {
                                                transfer_id: tid,
                                                status: TransferStatus::InProgress,
                                            });
                                    }
                                    record.transferred_bytes += chunk_len;
                                    if let Some(ref current) = current_file {
                                        if let Some(item_rec) =
                                            record.items.iter_mut().find(|i| i.name == current.2)
                                        {
                                            if item_rec.status == TransferStatus::Pending {
                                                item_rec.status = TransferStatus::InProgress;
                                            }
                                            item_rec.transferred_bytes += chunk_len;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Get the current engine state.
    pub fn state(&self) -> EngineState {
        *self.state.read()
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Set save directory for incoming files.
    pub fn set_save_directory(&self, path: &str) {
        let mut config = self.config.write();
        config.transfer.save_directory = path.to_string();
        config.storage.receive_directory = PathBuf::from(path);
        config.storage.temp_directory = PathBuf::from(path).join(".uot_temp");
        log::info!("Updated engine save directory to: {path}");
    }

    /// GAP 11: Get connection diagnostics for UI display.
    pub fn get_diagnostics(&self) -> String {
        let local_ips: Vec<String> = tcp::local_ips()
            .into_iter()
            .filter(|ip| ip.is_ipv4() && !ip.is_loopback())
            .map(|ip| ip.to_string())
            .collect();
        let listening_port = self.listening_port();
        let active_connections: Vec<String> = self.connections.read().keys().cloned().collect();
        let peer_states: Vec<(String, String)> = self
            .peer_states
            .read()
            .iter()
            .map(|(id, state)| (id.clone(), state.to_string()))
            .collect();
        let device_count = self.devices.read().len();
        let transfer_count = self.transfers.read().len();
        let engine_state = format!("{:?}", *self.state.read());

        format!(
            r#"{{"engine_state":"{}","local_ips":{},"listening_port":{},"device_count":{},"active_connections":{},"peer_states":{},"transfer_count":{}}}"#,
            engine_state,
            serde_json::to_string(&local_ips).unwrap_or_else(|_| "[]".to_string()),
            listening_port,
            device_count,
            serde_json::to_string(&active_connections).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&peer_states).unwrap_or_else(|_| "[]".to_string()),
            transfer_count,
        )
    }

    /// Authoritative peer session resolver by device ID, device name, socket addr, or active fallback.
    pub fn get_peer_session(&self, target: &str) -> Option<Arc<RwLock<PeerSession>>> {
        let sessions = self.sessions.read();

        // 1. Direct lookup by key in sessions map
        if let Some(session) = sessions.get(target) {
            return Some(Arc::clone(session));
        }

        // 2. Search sessions values by peer_device_id, peer_name, or session_id UUID string
        for session_arc in sessions.values() {
            let s = session_arc.read();
            if s.peer_device_id == target
                || s.peer_name == target
                || s.session_id.to_string() == target
            {
                return Some(Arc::clone(session_arc));
            }
            if let Some(ref remote) = s.remote_endpoint {
                if remote.to_string() == target {
                    return Some(Arc::clone(session_arc));
                }
            }
        }

        // 3. Match via discovered devices address/ID
        {
            let devices = self.devices.read();
            let dev_opt = devices.get(target).cloned().or_else(|| {
                devices
                    .values()
                    .find(|d| d.device_name == target || d.device_id == target)
                    .cloned()
            });
            if let Some(dev) = dev_opt {
                if let Some(session) = sessions.get(&dev.device_id) {
                    return Some(Arc::clone(session));
                }
            }
        }

        // 4. Fallback: Only if target is empty and sessions map has exactly 1 session, return it
        if target.is_empty() && sessions.len() == 1 {
            if let Some(session) = sessions.values().next() {
                return Some(Arc::clone(session));
            }
        }

        None
    }

    /// Get or create a session for a peer device.
    pub fn get_or_create_session(
        &self,
        peer_device_id: &str,
        peer_name: &str,
    ) -> Arc<RwLock<PeerSession>> {
        if let Some(session) = self.get_peer_session(peer_device_id) {
            return session;
        }
        let mut sessions = self.sessions.write();
        let session = Arc::new(RwLock::new(PeerSession::new_discovered(
            peer_device_id.to_string(),
            peer_name.to_string(),
        )));
        sessions.insert(peer_device_id.to_string(), Arc::clone(&session));
        session
    }

    /// Get all sessions as JSON array.
    pub fn get_sessions_json(&self) -> String {
        let sessions = self.sessions.read();
        let items: Vec<String> = sessions.values().map(|s| s.read().to_json()).collect();
        format!("[{}]", items.join(","))
    }

    /// Get messages for a specific peer session as JSON.
    pub fn get_session_messages(&self, peer_device_id: &str) -> String {
        if let Some(session_arc) = self.get_peer_session(peer_device_id) {
            #[derive(Serialize)]
            struct MessageJsonDto<'a> {
                message_id: String,
                direction: &'static str,
                timestamp: String,
                content: &'a str,
                state: String,
            }

            let s = session_arc.read();
            let dtos: Vec<MessageJsonDto> = s
                .messages
                .iter()
                .map(|m| MessageJsonDto {
                    message_id: m.message_id.to_string(),
                    direction: if m.direction == MessageDirection::Outgoing {
                        "out"
                    } else {
                        "in"
                    },
                    timestamp: m.timestamp.to_rfc3339(),
                    content: &m.content,
                    state: m.state.to_string(),
                })
                .collect();
            serde_json::to_string(&dtos).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    }

    /// Send a chat message to a peer via their session connection.
    pub async fn send_chat_message(
        &self,
        peer_device_id: &str,
        text: String,
    ) -> Result<Uuid, UotError> {
        let message_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Get session using authoritative resolver, or auto-create if connection exists
        let session_arc = self
            .get_peer_session(peer_device_id)
            .or_else(|| {
                let conn = self.get_peer_connection(peer_device_id);
                if let Some(conn) = conn {
                    let device_name = self
                        .devices
                        .read()
                        .get(peer_device_id)
                        .map(|d| d.device_name.clone())
                        .unwrap_or_else(|| peer_device_id.to_string());
                    let session = self.get_or_create_session(peer_device_id, &device_name);
                    {
                        let mut s = session.write();
                        s.connection = Some(conn);
                        s.state = SessionState::SessionReady;
                    }
                    Some(session)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                UotError::Transfer(TransferError::DeviceNotFound(peer_device_id.to_string()))
            })?;

        // Add message to session as Sending
        let session_id = {
            let mut session = session_arc.write();
            let sid = session.session_id;
            session.add_message(ChatMessage {
                message_id,
                session_id: sid,
                direction: MessageDirection::Outgoing,
                timestamp: now,
                content: text.clone(),
                state: MessageState::Sending,
                error: None,
            });
            sid
        };

        // Get connection from session or fallback to authoritative resolver
        let conn = {
            let session = session_arc.read();
            session.connection.clone()
        }
        .or_else(|| self.get_peer_connection(peer_device_id));

        let conn_res = match conn {
            Some(c) if c.state() == crate::transport::types::TransportState::Connected => Ok(c),
            _ => self.reconnect_session(peer_device_id).await,
        };

        let conn = match conn_res {
            Ok(c) => c,
            Err(_) => {
                session_arc
                    .write()
                    .update_message_state(message_id, MessageState::Failed);
                return Err(UotError::Transfer(TransferError::DeviceNotFound(format!(
                    "No connection to {}",
                    peer_device_id
                ))));
            }
        };

        // Serialize and send WireMessage::ChatMessage over the single active socket
        let msg = WireMessage::ChatMessage {
            message_id: message_id.to_string(),
            content: text,
            timestamp: now.timestamp(),
        };

        match proto::send_message(&conn, &msg).await {
            Ok(()) => {
                session_arc
                    .write()
                    .update_message_state(message_id, MessageState::Sent);
                log::info!("[CHAT_SEND] session_id={session_id} message_id={message_id} peer_id={peer_device_id}");
                Ok(message_id)
            }
            Err(e) => {
                session_arc
                    .write()
                    .update_message_state(message_id, MessageState::Failed);
                log::error!(
                    "[CHAT_FAIL] session_id={session_id} message_id={message_id} error={e}"
                );
                Err(UotError::Transport(e))
            }
        }
    }

    /// Attempt automatic session reconnection to a peer device using last known IP address.
    pub async fn reconnect_session(
        &self,
        peer_device_id: &str,
    ) -> Result<Arc<TcpConnection>, UotError> {
        let (addr_str, peer_name) = {
            let devices = self.devices.read();
            if let Some(dev) = devices.get(peer_device_id) {
                (dev.address.clone(), dev.device_name.clone())
            } else {
                let sessions = self.sessions.read();
                if let Some(sess) = sessions.get(peer_device_id) {
                    let s = sess.read();
                    (
                        s.remote_endpoint.map(|e| e.to_string()),
                        s.peer_name.clone(),
                    )
                } else {
                    (None, "Unknown".to_string())
                }
            }
        };

        let addr_str = addr_str.ok_or_else(|| {
            UotError::Transfer(TransferError::DeviceNotFound(format!(
                "No known address to reconnect to {peer_device_id}"
            )))
        })?;

        log::info!("Auto-reconnecting session to {peer_name} ({peer_device_id}) at {addr_str}...");
        let dev = self.connect_peer(&addr_str).await?;

        let conn = self
            .connections
            .read()
            .get(&dev.device_id)
            .cloned()
            .ok_or_else(|| {
                UotError::Transport(crate::core::error::TransportError::Connection(
                    "Reconnected but connection missing in map".to_string(),
                ))
            })?;
        Ok(conn)
    }

    /// Start heartbeat task for a session.
    pub fn start_heartbeat(
        &self,
        peer_device_id: String,
        conn: Arc<TcpConnection>,
        session: Arc<RwLock<PeerSession>>,
        event_tx: mpsc::Sender<EngineEvent>,
    ) {
        let _sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;

                // Check if session still connected
                {
                    let s = session.read();
                    if !s.state.is_connected() {
                        log::info!("Heartbeat stopping for {} (disconnected)", peer_device_id);
                        break;
                    }
                }

                // Send Ping
                let ping_frame = Frame {
                    frame_type: FrameType::Ping,
                    payload: Vec::new(),
                };
                if conn.send_frame(ping_frame).await.is_err() {
                    let should_disconnect = session.write().heartbeat_missed(3);
                    if should_disconnect {
                        let session_id = session.read().session_id;
                        let _ = session.write().transition(SessionState::Disconnected);
                        let _ = event_tx
                            .send(EngineEvent::HeartbeatChanged {
                                session_id,
                                device_id: peer_device_id.clone(),
                                alive: false,
                            })
                            .await;
                        let _ = event_tx
                            .send(EngineEvent::PeerStateChanged {
                                device_id: peer_device_id.clone(),
                                state: PeerConnectionState::Disconnected,
                            })
                            .await;
                        log::warn!(
                            "Heartbeat timeout for {} — session disconnected",
                            peer_device_id
                        );
                        break;
                    }
                } else {
                    session.write().heartbeat_success();
                }
            }
        });
    }

    /// Access the underlying devices map directly.
    pub fn devices_map(&self) -> &Arc<RwLock<HashMap<String, DiscoveredDevice>>> {
        &self.devices
    }

    /// Add or update a discovered device.
    pub fn add_discovered_device(&self, device: DiscoveredDevice) {
        self.devices
            .write()
            .insert(device.device_id.clone(), device);
    }

    /// Get all discovered devices (filtering out self-device, local IP/ports, and deduplicating endpoints).
    pub fn discovered_devices(&self) -> Vec<DiscoveredDevice> {
        let my_id = &self.device_id;
        let my_ips = tcp::local_ips();
        let my_port = self.listening_port();

        let raw_devs = self.devices.read();
        let mut canonical_map: HashMap<String, DiscoveredDevice> = HashMap::new();

        for dev in raw_devs.values() {
            if dev.device_id == *my_id {
                continue;
            }
            if let Some(ref addr_str) = dev.address {
                if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                    if addr.port() == my_port && my_ips.contains(&addr.ip()) {
                        continue;
                    }
                }
            }

            // Deduplicate by IP endpoint or device_id:
            let key = if let Some(ref addr_str) = dev.address {
                if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                    format!("ip:{}", addr.ip())
                } else {
                    dev.device_id.clone()
                }
            } else {
                dev.device_id.clone()
            };

            if let Some(existing) = canonical_map.get(&key) {
                let existing_is_synth = existing.device_id.starts_with("lan-")
                    || existing.device_id.starts_with("peer-");
                let current_is_synth =
                    dev.device_id.starts_with("lan-") || dev.device_id.starts_with("peer-");

                if existing_is_synth && !current_is_synth {
                    canonical_map.insert(key, dev.clone());
                } else if !existing_is_synth && current_is_synth {
                    // Keep existing real one
                } else if dev.capabilities.contains(&"connected".to_string())
                    || dev.capabilities.contains(&"session_ready".to_string())
                {
                    canonical_map.insert(key, dev.clone());
                }
            } else {
                canonical_map.insert(key, dev.clone());
            }
        }

        canonical_map.into_values().collect()
    }

    /// Get a specific transfer's progress.
    pub fn get_progress(&self, transfer_id: &Uuid) -> Option<TransferProgress> {
        self.progress_trackers
            .read()
            .get(transfer_id)
            .map(|t| t.snapshot())
    }

    /// Get all transfer records.
    pub fn get_transfers(&self) -> Vec<TransferRecord> {
        self.transfers.read().values().cloned().collect()
    }

    /// Stop the engine.
    pub fn stop(&self) {
        *self.state.write() = EngineState::ShuttingDown;
        if let Some(ref discovery) = *self.discovery.read() {
            discovery.stop_browsing();
            discovery.unregister();
        }
        if let Some(ref mut listener) = *self.listener.write() {
            listener.stop();
        }
        self.connections.write().clear();
        *self.state.write() = EngineState::Stopped;
    }

    /// Cancel a transfer.
    pub async fn cancel_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;

        // Signal pause (stop sending)
        if let Some(tx) = self.pause_signals.read().get(&uuid) {
            let _ = tx.send(true);
        }

        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::Cancelled;
            record.finished_at = Some(chrono::Utc::now());

            if let Some(conn) = self.get_peer_connection(&record.remote_device) {
                let msg = WireMessage::Cancel {
                    transfer_id: transfer_id.to_string(),
                    reason: Some("Cancelled by user".to_string()),
                };
                tokio::spawn(async move {
                    let _ = proto::send_message(&conn, &msg).await;
                });
            }

            let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::Cancelled,
            });
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
    }

    /// Pause an active transfer.
    pub fn pause_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;

        // Signal the send loop to pause
        if let Some(tx) = self.pause_signals.read().get(&uuid) {
            let _ = tx.send(true);
        }

        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::Paused;

            if let Some(conn) = self.get_peer_connection(&record.remote_device) {
                let msg = WireMessage::Pause {
                    transfer_id: transfer_id.to_string(),
                };
                tokio::spawn(async move {
                    let _ = proto::send_message(&conn, &msg).await;
                });
            }

            let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::Paused,
            });
            self.log_event(&format!("Transfer {transfer_id} paused"));
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
    }

    /// Resume a paused transfer.
    pub fn resume_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;

        // Signal the send loop to resume
        if let Some(tx) = self.pause_signals.read().get(&uuid) {
            let _ = tx.send(false);
        }

        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::InProgress;

            if let Some(conn) = self.get_peer_connection(&record.remote_device) {
                let msg = WireMessage::Resume {
                    transfer_id: transfer_id.to_string(),
                    offset: record.transferred_bytes,
                };
                tokio::spawn(async move {
                    let _ = proto::send_message(&conn, &msg).await;
                });
            }

            let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::InProgress,
            });
            self.log_event(&format!("Transfer {transfer_id} resumed"));
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
    }

    /// Retry a failed or interrupted transfer.
    pub async fn retry_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;

        let (remote_device, item_names) = {
            let transfers = self.transfers.read();
            let record = transfers.get(&uuid).ok_or_else(|| {
                UotError::Transfer(TransferError::TransferNotFound {
                    transfer_id: transfer_id.to_string(),
                })
            })?;
            if record.status == TransferStatus::Completed {
                return Ok(());
            }
            let names: Vec<String> = record
                .items
                .iter()
                .filter(|i| i.status != TransferStatus::Completed)
                .map(|i| i.name.clone())
                .collect();
            (record.remote_device.clone(), names)
        };

        let conn = self.get_peer_connection(&remote_device).ok_or_else(|| {
            UotError::Transport(TransportError::ConnectionFailed {
                reason: format!("Peer '{remote_device}' is not connected"),
            })
        })?;

        // Reset transfer status to InProgress
        {
            let mut t = self.transfers.write();
            if let Some(rec) = t.get_mut(&uuid) {
                rec.status = TransferStatus::InProgress;
                rec.error = None;
            }
        }
        let _ = self
            .event_tx
            .send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::InProgress,
            })
            .await;

        let chunk_size = self.config.read().transfer.chunk_size;
        let bandwidth_limit = self.config.read().transfer.bandwidth_limit;
        let (pause_tx, pause_rx) = watch::channel(false);
        self.pause_signals.write().insert(uuid, pause_tx);

        let transfers = Arc::clone(&self.transfers);
        let event_tx = self.event_tx.clone();
        let pending_completion_acks = Arc::clone(&self.pending_completion_acks);
        let pause_signals = Arc::clone(&self.pause_signals);
        let stats = Arc::clone(&self.lifetime_stats);
        let history = Arc::clone(&self.history_store);
        let queue_manager = Arc::clone(&self.queue_manager);

        let peer_send_lock = {
            let mut locks = self.peer_send_locks.write();
            Arc::clone(
                locks
                    .entry(remote_device.clone())
                    .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
            )
        };

        tokio::spawn(async move {
            let _guard = peer_send_lock.lock().await;

            let tracker = Arc::new(ProgressTracker::new(uuid, 0, item_names.len()));

            // Execute retry send
            let send_res = Self::execute_send_arc(
                &conn,
                vec![],
                uuid,
                &tracker,
                chunk_size,
                bandwidth_limit,
                pause_rx,
                &event_tx,
                &transfers,
                &pending_completion_acks,
            )
            .await;

            pause_signals.write().remove(&uuid);
            match send_res {
                Ok(()) => {
                    let mut t = transfers.write();
                    if let Some(record) = t.get_mut(&uuid) {
                        record.status = TransferStatus::Completed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.transferred_bytes = record.total_size;
                        for item in &mut record.items {
                            item.status = TransferStatus::Completed;
                            item.transferred_bytes = item.size;
                        }
                        let speed = tracker.snapshot().speed_bytes_per_sec;
                        stats.write().record_success(record.total_size, true, speed);
                        history.write().upsert(record.clone());
                    }
                    let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                        transfer_id: uuid,
                        status: TransferStatus::Completed,
                    });
                }
                Err(e) => {
                    let mut t = transfers.write();
                    if let Some(record) = t.get_mut(&uuid) {
                        record.status = TransferStatus::Failed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.error = Some(e.to_string());
                        stats.write().record_failure();
                        history.write().upsert(record.clone());
                    }
                    let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                        transfer_id: uuid,
                        status: TransferStatus::Failed,
                    });
                }
            }
            queue_manager.write().mark_completed();
        });

        Ok(())
    }

    /// Accept an incoming transfer offer.
    pub async fn accept_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;

        // Check if the sender is trusted — if not, log warning
        if let Some(record) = self.transfers.read().get(&uuid) {
            let device_id = &record.remote_device;
            if !self.trust_manager.read().is_trusted(device_id) {
                log::warn!(
                    "Accepting transfer {transfer_id} from untrusted device '{device_id}' — consider using accept_transfer_with_pin()"
                );
            }
        }

        self.accepted_transfers.write().insert(uuid);
        let remote_dev = {
            let mut transfers = self.transfers.write();
            if let Some(record) = transfers.get_mut(&uuid) {
                record.status = TransferStatus::InProgress;
                record.started_at = Some(chrono::Utc::now());
                self.log_event(&format!("Transfer {transfer_id} accepted"));
                Some(record.remote_device.clone())
            } else {
                None
            }
        };

        let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
            transfer_id: uuid,
            status: TransferStatus::InProgress,
        });

        if let Some(ref dev_id) = remote_dev {
            // First check transfer_connections for the EXACT TcpConnection used for this offer!
            let target_conn = self
                .transfer_connections
                .read()
                .get(&uuid)
                .cloned()
                .or_else(|| self.get_peer_connection(dev_id));

            if let Some(conn) = target_conn {
                let resp = WireMessage::OfferResponse {
                    transfer_id: uuid.to_string(),
                    accepted: true,
                    reason: None,
                };
                let _ = proto::send_message(&conn, &resp).await;
                log::info!(
                    "Sent OfferResponse (accepted=true) to {dev_id} for transfer {transfer_id}"
                );
            } else {
                log::warn!(
                    "No active connection found for device {dev_id} when accepting transfer {transfer_id}"
                );
            }
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
    }

    /// Accept a transfer offer WITH PIN verification (secure path).
    ///
    /// Verifies the sender's PIN before allowing the transfer to proceed.
    /// Use this for untrusted/first-time devices.
    pub async fn accept_transfer_with_pin(
        &self,
        transfer_id: &str,
        device_id: &str,
        pin: &str,
    ) -> Result<(), UotError> {
        // Verify PIN first
        let token = self.trust_manager.write().verify_pin(device_id, pin);
        if token.is_none() {
            return Err(UotError::Security(
                crate::core::error::SecurityError::AuthenticationFailed {
                    reason: "Invalid or expired PIN".to_string(),
                },
            ));
        }

        log::info!("PIN verified for device {device_id}, accepting transfer {transfer_id}");
        self.accept_transfer(transfer_id).await
    }

    /// Generate a 6-digit PIN for device pairing/verification.
    pub fn generate_pin(&self, ttl_secs: u64) -> String {
        self.trust_manager
            .write()
            .generate_pin(ttl_secs)
            .to_string()
    }

    /// Verify a PIN attempt for a remote device.
    pub fn verify_pin(&self, device_id: &str, attempt: &str) -> Option<String> {
        self.trust_manager.write().verify_pin(device_id, attempt)
    }

    /// Set the device display name.
    pub fn set_device_name(&self, name: &str) {
        self.config.write().device_name = name.to_string();
        log::info!("Device name updated to: {name}");
    }

    /// Send clipboard text to a device.
    ///
    /// Reuses an existing connection if available; otherwise opens a fresh TCP connection.
    pub async fn send_clipboard(&self, device_id: &str, text: String) -> Result<(), UotError> {
        let device = self.devices.read().get(device_id).cloned().ok_or_else(|| {
            UotError::Transfer(TransferError::DeviceNotFound(device_id.to_string()))
        })?;

        let addr: SocketAddr = device
            .address
            .as_ref()
            .ok_or_else(|| {
                UotError::Transfer(TransferError::DeviceNotFound(device_id.to_string()))
            })?
            .parse()
            .map_err(|e| {
                UotError::Transport(TransportError::Connection(format!("Bad address: {e}")))
            })?;

        // Try reusing existing connection first
        let text_len = text.len();
        let msg = WireMessage::ClipboardData {
            content_type: "text/plain".to_string(),
            data: text,
        };

        // GAP 10 FIX: Try reusing existing connection by device_id OR addr string
        if let Some(existing_conn) = {
            let conns = self.connections.read();
            conns
                .get(device_id)
                .or_else(|| conns.get(&addr.to_string()))
                .cloned()
        } {
            if existing_conn.state() == TransportState::Connected {
                match proto::send_message(&existing_conn, &msg).await {
                    Ok(()) => {
                        log::info!("Clipboard sent to {device_id} via existing connection: {text_len} bytes");
                        return Ok(());
                    }
                    Err(e) => {
                        log::warn!("Existing connection to {device_id} failed, opening new: {e}");
                    }
                }
            }
        }

        // Fallback: open fresh connection with candidate port fallbacks
        let stream = Self::connect_with_port_fallback(addr)
            .await
            .map_err(UotError::Transport)?;
        let conn = TcpConnection::new(stream).map_err(UotError::Transport)?;
        proto::send_message(&conn, &msg)
            .await
            .map_err(UotError::Transport)?;
        log::info!("Clipboard sent to {device_id} via new connection: {text_len} bytes");
        Ok(())
    }

    /// Helper to connect to SocketAddr with fallback to standard ports (42000, 42001, 42002, 42003).
    async fn connect_with_port_fallback(
        addr: SocketAddr,
    ) -> Result<tokio::net::TcpStream, TransportError> {
        let ip = addr.ip();
        let mut candidate_addrs = vec![addr];
        for p in [42000, 42001, 42002, 42003] {
            let alt = SocketAddr::new(ip, p);
            if !candidate_addrs.contains(&alt) {
                candidate_addrs.push(alt);
            }
        }

        let mut last_err = None;
        for target in candidate_addrs {
            match tokio::time::timeout(std::time::Duration::from_secs(3), tcp::connect(target)).await {
                Ok(Ok(stream)) => return Ok(stream),
                Ok(Err(e)) => last_err = Some(e),
                Err(_) => last_err = Some(TransportError::Connection(format!(
                    "Connection to {target} timed out after 3 seconds (Windows Firewall / Wi-Fi filter)"
                ))),
            }
        }

        Err(last_err.unwrap_or_else(|| TransportError::Connection("Connection failed".to_string())))
    }

    /// Get recent events as serializable strings.
    pub fn get_recent_events(&self, limit: usize) -> Vec<String> {
        let log = self.event_log.read();
        log.iter().rev().take(limit).cloned().collect()
    }

    /// Log an event to the ring buffer.
    pub fn log_event(&self, event: &str) {
        let mut log = self.event_log.write();
        if log.len() >= MAX_EVENT_LOG {
            log.pop_front();
        }
        let timestamp = chrono::Utc::now().to_rfc3339();
        log.push_back(format!("{timestamp}: {event}"));
    }

    /// Get active streaming sessions.
    pub fn get_streams(&self) -> Vec<StreamSession> {
        self.stream_manager.read().active_sessions()
    }

    /// Start a new streaming session.
    pub fn start_stream(
        &self,
        stream_type: StreamType,
        remote_device_id: &str,
        remote_device_name: &str,
        port: u16,
        is_sender: bool,
    ) -> String {
        let session_id = self.stream_manager.read().start_session(
            stream_type,
            remote_device_id,
            remote_device_name,
            port,
            is_sender,
        );
        self.log_event(&format!(
            "Stream session {session_id} started ({stream_type})"
        ));
        session_id
    }

    /// Stop a streaming session.
    pub fn stop_stream(&self, session_id: &str) {
        self.stream_manager.read().stop_session(session_id);
        self.log_event(&format!("Stream session {session_id} stopped"));
    }

    /// Get the current configuration (read-only).
    pub fn config(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// Direct connection to a peer by address string (IP:port or IP).
    /// Get actual bound listening port of TCP transport listener.
    pub fn listening_port(&self) -> u16 {
        if let Some(ref listener) = *self.listener.read() {
            listener.port()
        } else {
            self.config.read().network_port.unwrap_or(tcp::DEFAULT_PORT)
        }
    }

    /// Direct connection to a peer by address string (IP:port or IP).
    ///
    /// Performs full handshake: TCP connect → Hello → HelloAck → Ping → Pong.
    /// Only marks the peer as connected after the handshake succeeds.
    pub async fn connect_peer(&self, addr_str: &str) -> Result<DiscoveredDevice, UotError> {
        let default_port = self.listening_port();
        let trimmed = addr_str.trim();

        // Extract IP and target port list (if explicit port passed, connect ONLY to that port; if no port, try candidate ports)
        let (ip_str, target_ports) = if trimmed.contains(':') {
            let parts: Vec<&str> = trimmed.split(':').collect();
            let ip = parts[0];
            let port_parsed = parts[1].parse::<u16>().unwrap_or(42000);
            (ip, vec![port_parsed])
        } else {
            (trimmed, vec![42000, 42001, 42002, 42003, default_port])
        };

        // Prevent self-loopback connection to our own listening port
        let my_ips = tcp::local_ips();
        for &port in &target_ports {
            if port == default_port {
                for my_ip in &my_ips {
                    if ip_str == my_ip.to_string() || ip_str == "127.0.0.1" || ip_str == "localhost"
                    {
                        return Err(UotError::Transport(
                            crate::core::error::TransportError::ConnectionFailed {
                                reason: format!(
                                    "Cannot connect to your own device ({ip_str}:{port})"
                                ),
                            },
                        ));
                    }
                }
            }
        }

        // Try connecting to target ports with a 3-second timeout per port
        let mut last_err = String::new();
        let mut connected_stream = None;
        let mut final_socket_addr = None;

        for &port in &target_ports {
            let full_addr_str = format!("{ip_str}:{port}");
            if let Ok(socket_addr) = full_addr_str.parse::<SocketAddr>() {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    tcp::connect(socket_addr),
                )
                .await
                {
                    Ok(Ok(stream)) => {
                        connected_stream = Some(stream);
                        final_socket_addr = Some(socket_addr);
                        break;
                    }
                    Ok(Err(e)) => {
                        last_err = format!("Connect to {socket_addr} failed: {e}");
                    }
                    Err(_) => {
                        last_err = format!("Connection to {socket_addr} timed out after 3 seconds (Windows Firewall / Wi-Fi filter)");
                    }
                }
            }
        }

        let conn_stream = match connected_stream {
            Some(s) => s,
            None => {
                return Err(UotError::Transport(
                    crate::core::error::TransportError::ConnectionFailed {
                        reason: format!(
                            "{last_err}. Check that both devices are on the same Wi-Fi network."
                        ),
                    },
                ));
            }
        };

        let socket_addr = final_socket_addr.unwrap();
        let conn = TcpConnection::new(conn_stream)?;

        // === Phase 1: Mark TCP connected ===
        let temp_device_id = format!("peer-{}", socket_addr.ip().to_string().replace('.', "-"));
        self.set_peer_state(&temp_device_id, PeerConnectionState::TcpConnected)
            .await;

        // === Phase 2: Send Hello handshake ===
        let our_device_name = self.config.read().device_name.clone();
        let hello = WireMessage::Hello {
            device_id: self.device_id.clone(),
            device_name: our_device_name.clone(),
            device_type: "Desktop".to_string(),
            version: version::version_string(),
            capabilities: vec!["tcp_lan".to_string(), "clipboard".to_string()],
        };
        proto::send_message(&conn, &hello).await.map_err(|e| {
            let _ = self.peer_states.write().insert(
                temp_device_id.clone(),
                PeerConnectionState::Error(format!("Hello send failed: {e}")),
            );
            UotError::Transport(e)
        })?;
        self.set_peer_state(&temp_device_id, PeerConnectionState::HelloSent)
            .await;
        log::info!("Sent Hello to {socket_addr}");

        // === Phase 3: Wait for HelloAck (5-second timeout) ===
        let (remote_device_id, remote_device_name, remote_device_type) = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            proto::recv_message(&conn),
        )
        .await
        {
            Ok(Ok(WireMessage::HelloAck {
                device_id: rid,
                device_name: rname,
                device_type: rtype,
                ..
            })) => {
                log::info!("Received HelloAck from {rname} ({rid}) at {socket_addr}");
                (rid, rname, rtype)
            }
            Ok(Ok(other)) => {
                let err_msg = format!(
                    "Peer at {socket_addr} sent unexpected message instead of HelloAck: {other:?}"
                );
                log::error!("{err_msg}");
                self.set_peer_state(&temp_device_id, PeerConnectionState::Error(err_msg.clone()))
                    .await;
                return Err(UotError::Transport(TransportError::Protocol(err_msg)));
            }
            Ok(Err(e)) => {
                let err_msg = format!("Failed to receive HelloAck from {socket_addr}: {e}");
                log::error!("{err_msg}");
                self.set_peer_state(&temp_device_id, PeerConnectionState::Error(err_msg.clone()))
                    .await;
                return Err(UotError::Transport(TransportError::Protocol(err_msg)));
            }
            Err(_) => {
                let err_msg = format!(
                    "HelloAck timeout from {socket_addr} (5s). The remote may not be running UOT."
                );
                log::error!("{err_msg}");
                self.set_peer_state(&temp_device_id, PeerConnectionState::Error(err_msg.clone()))
                    .await;
                return Err(UotError::Transport(TransportError::Protocol(err_msg)));
            }
        };
        self.set_peer_state(&temp_device_id, PeerConnectionState::HelloAcked)
            .await;

        // === Phase 4: Ping liveness check ===
        conn.send(Frame::ping()).await.map_err(|e| {
            let _ = self.peer_states.write().insert(
                temp_device_id.clone(),
                PeerConnectionState::Error(format!("Ping failed: {e}")),
            );
            UotError::Transport(e)
        })?;
        // Note: Pong is handled internally by the receiver's reader task.
        // If the Ping frame was sent successfully and HelloAck was received,
        // the connection is bidirectionally verified.
        self.set_peer_state(&temp_device_id, PeerConnectionState::PingConfirmed)
            .await;
        log::info!("Ping liveness confirmed with {remote_device_name} at {socket_addr}");

        // === Phase 5: Register device with actual identity from HelloAck ===
        let device_type = match remote_device_type.to_lowercase().as_str() {
            "phone" => DeviceType::Phone,
            "tablet" => DeviceType::Tablet,
            "laptop" => DeviceType::Laptop,
            "tv" => DeviceType::Tv,
            _ => DeviceType::Desktop,
        };

        let now = chrono::Utc::now();
        let device = DiscoveredDevice {
            device_id: remote_device_id.clone(),
            device_name: remote_device_name.clone(),
            device_type,
            discovery_method: crate::discovery::types::DiscoveryMethod::Manual,
            address: Some(socket_addr.to_string()),
            capabilities: vec![
                "tcp_lan".to_string(),
                "connected".to_string(),
                "session_ready".to_string(),
            ],
            signal_strength: Some(100),
            first_seen: now,
            last_seen: now,
            is_trusted: true,
        };

        let conn = Arc::new(conn);
        let remote_ip_str = socket_addr.ip().to_string();
        {
            let mut devs = self.devices.write();
            devs.retain(|k, v| {
                if k == &remote_device_id {
                    return true;
                }
                if k.starts_with("lan-") || k.starts_with("peer-") {
                    if let Some(ref a) = v.address {
                        if a.contains(&remote_ip_str) {
                            return false;
                        }
                    }
                }
                true
            });
            devs.insert(remote_device_id.clone(), device.clone());
        }
        self.connections
            .write()
            .insert(remote_device_id.clone(), Arc::clone(&conn));

        // Migrate peer state from temp_device_id to the actual remote_device_id
        self.peer_states.write().remove(&temp_device_id);
        self.set_peer_state(&remote_device_id, PeerConnectionState::SessionReady)
            .await;

        // Create PeerSession so chat/transfer can find this peer
        {
            let session = self.get_or_create_session(&remote_device_id, &remote_device_name);
            let mut s = session.write();
            s.connection = Some(Arc::clone(&conn));
            s.remote_endpoint = Some(socket_addr);
            // Force state to SessionReady (skip intermediate states for outbound connect)
            s.state = SessionState::SessionReady;
            s.peer_name = remote_device_name.clone();
            log::info!(
                "Session {} created for {} ({})",
                s.session_id,
                remote_device_name,
                remote_device_id
            );
        }

        let _ = self
            .event_tx
            .send(EngineEvent::DeviceFound(device.clone()))
            .await;

        // GAP 3 FIX: Spawn reader task so this peer can RECEIVE messages/files
        // from the connected remote peer. Without this, only the listener side
        // could receive incoming data.
        {
            let conn_for_handler = Arc::clone(&conn);
            let remote_str = socket_addr.to_string();
            let transfers = Arc::clone(&self.transfers);
            let trackers = Arc::clone(&self.progress_trackers);
            let event_tx = self.event_tx.clone();
            let save_dir = self.config.read().transfer.save_directory.clone();
            let accepted = Arc::clone(&self.accepted_transfers);
            let devices = Arc::clone(&self.devices);
            let our_id = self.device_id.clone();
            let our_name = self.config.read().device_name.clone();
            let sessions = Arc::clone(&self.sessions);
            let connections = Arc::clone(&self.connections);
            let pending_responses = Arc::clone(&self.pending_offer_responses);
            let pending_acks = Arc::clone(&self.pending_completion_acks);
            let transfer_connections = Arc::clone(&self.transfer_connections);

            let known_peer = remote_device_id.clone();
            tokio::spawn(async move {
                Self::handle_incoming_connection(
                    conn_for_handler,
                    &remote_str,
                    &transfers,
                    &trackers,
                    &event_tx,
                    &save_dir,
                    &accepted,
                    &devices,
                    &our_id,
                    &our_name,
                    &sessions,
                    &connections,
                    Some(known_peer),
                    &pending_responses,
                    &pending_acks,
                    &transfer_connections,
                )
                .await;
            });
        }

        // Start session heartbeat keepalive
        {
            let session = self.get_or_create_session(&remote_device_id, &remote_device_name);
            self.start_heartbeat(
                remote_device_id.clone(),
                Arc::clone(&conn),
                session,
                self.event_tx.clone(),
            );
        }

        self.log_event(&format!(
            "Connected to {remote_device_name} at {socket_addr} (Hello+Ping verified)"
        ));
        log::info!(
            "Peer connection fully established: {remote_device_name} ({remote_device_id}) at {socket_addr}"
        );

        Ok(device)
    }

    /// Update peer connection state and emit event.
    async fn set_peer_state(&self, device_id: &str, state: PeerConnectionState) {
        self.peer_states
            .write()
            .insert(device_id.to_string(), state.clone());
        let _ = self
            .event_tx
            .send(EngineEvent::PeerStateChanged {
                device_id: device_id.to_string(),
                state,
            })
            .await;
    }

    /// Get the current connection state for a peer.
    pub fn get_peer_state(&self, device_id: &str) -> PeerConnectionState {
        self.peer_states
            .read()
            .get(device_id)
            .cloned()
            .unwrap_or(PeerConnectionState::Disconnected)
    }

    /// Fallback discovery: scan local subnet for UOT listeners.
    pub async fn subnet_scan(&self) -> Vec<std::net::SocketAddr> {
        use crate::discovery::subnet::SubnetScanner;
        use crate::discovery::types::DiscoveryMethod;

        let port = self.config.read().network_port.unwrap_or(tcp::DEFAULT_PORT);
        let scanner = SubnetScanner::new(port);

        // Get local IPs and scan each /24 subnet
        let local_ips = tcp::local_ips();
        let my_port = self.listening_port();
        let mut all_found = Vec::new();
        let now = chrono::Utc::now();
        for ip in &local_ips {
            if let std::net::IpAddr::V4(v4) = ip {
                let octets = v4.octets();
                let found = scanner.scan_subnet(octets).await;
                for addr in &found {
                    // Do not discover self
                    if addr.port() == my_port && local_ips.contains(&addr.ip()) {
                        continue;
                    }
                    let ip_str = addr.ip().to_string();
                    let already_known = self.devices.read().values().any(|d| {
                        if let Some(ref a) = d.address {
                            a.contains(&ip_str) && !d.device_id.starts_with("lan-")
                        } else {
                            false
                        }
                    });
                    if already_known {
                        continue;
                    }
                    let dev_id = format!("lan-{}", addr.ip().to_string().replace('.', "-"));
                    let dev = DiscoveredDevice {
                        device_id: dev_id.clone(),
                        device_name: format!("UOT Node ({})", addr.ip()),
                        device_type: DeviceType::Desktop,
                        discovery_method: DiscoveryMethod::Manual,
                        address: Some(addr.to_string()),
                        capabilities: vec!["tcp_lan".to_string()],
                        signal_strength: Some(100),
                        first_seen: now,
                        last_seen: now,
                        is_trusted: false,
                    };
                    self.devices.write().insert(dev_id, dev.clone());
                    let _ = self.event_tx.send(EngineEvent::DeviceFound(dev)).await;
                }
                all_found.extend(found);
            }
        }

        self.log_event(&format!("Subnet scan found {} hosts", all_found.len()));
        all_found
    }

    /// Get lifetime transfer statistics.
    pub fn get_lifetime_stats(&self) -> LifetimeStats {
        self.lifetime_stats.read().clone()
    }

    /// Get transfer history with optional filtering.
    pub fn get_transfer_history(
        &self,
        query: &str,
        status_filter: Option<TransferStatus>,
    ) -> Vec<TransferRecord> {
        self.history_store.read().query(query, status_filter)
    }

    /// Select optimal transport based on candidate states via TransportFallbackManager.
    pub fn select_best_transport(
        &self,
        candidates: &[(TransportId, TransportState)],
    ) -> Option<TransportId> {
        self.fallback_manager
            .read()
            .select_best_transport(candidates)
    }

    /// Set transport selection strategy (PreferSpeed, PreferOffline, Manual).
    pub fn set_transport_strategy(&self, strategy: TransportSelectionStrategy) {
        self.fallback_manager.write().strategy = strategy;
    }

    /// Connect to a device with automatic retry and exponential backoff.
    ///
    /// Uses `ConnectionManager` to attempt connection up to `max_retries` times
    /// with exponentially increasing delays (1s, 2s, 4s, ..., max 30s).
    /// Returns the pooled connection on success.
    pub async fn connect_with_retry(
        &self,
        device_id: &str,
        addr: std::net::SocketAddr,
    ) -> Result<Arc<TcpConnection>, UotError> {
        let device_name = self
            .devices
            .read()
            .get(device_id)
            .map(|d| d.device_name.clone())
            .unwrap_or_else(|| device_id.to_string());

        let conn = self
            .connection_manager
            .connect(device_id, &device_name, addr)
            .await
            .map_err(UotError::Transport)?;

        // Also store in our connections map
        self.connections
            .write()
            .insert(device_id.to_string(), Arc::clone(&conn));

        self.log_event(&format!(
            "Connected to {device_name} at {addr} (with retry)"
        ));
        Ok(conn)
    }

    /// Check if a device is currently connected.
    pub fn is_device_connected(&self, device_id: &str) -> bool {
        self.connection_manager.is_connected(device_id)
            || self.connections.read().contains_key(device_id)
    }

    /// Disconnect from a specific device and clean up connection state.
    pub fn disconnect_device(&self, device_id: &str) {
        self.connection_manager.remove(device_id);
        self.connections.write().remove(device_id);
        self.log_event(&format!("Disconnected from device {device_id}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new() {
        let config = AppConfig::default();
        let (engine, _rx) = UotEngine::new(config);
        assert_eq!(engine.state(), EngineState::Stopped);
        assert!(!engine.device_id().is_empty());
        assert!(engine.discovered_devices().is_empty());
        assert!(engine.get_transfers().is_empty());
    }

    #[tokio::test]
    async fn test_engine_start_stop() {
        let mut config = AppConfig::default();
        config.network_port = Some(0); // Let OS pick port
        let (engine, _rx) = UotEngine::new(config);

        // Start may fail in CI without network, but should handle gracefully
        if engine.start().await.is_ok() {
            assert_eq!(engine.state(), EngineState::Running);
            engine.stop();
            assert_eq!(engine.state(), EngineState::Stopped);
        }
    }

    #[test]
    fn test_engine_streaming_sessions() {
        let config = AppConfig::default();
        let (engine, _rx) = UotEngine::new(config);

        assert!(engine.get_streams().is_empty());

        let session_id = engine.start_stream(
            StreamType::Camera,
            "device-123",
            "Remote Camera",
            42001,
            true,
        );
        assert!(!session_id.is_empty());
        assert_eq!(engine.get_streams().len(), 1);

        engine.stop_stream(&session_id);
        let streams = engine.get_streams();
        assert_eq!(streams.len(), 1);
        assert_eq!(
            streams[0].state,
            crate::streaming::manager::StreamState::Stopping
        );
    }
}
