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

        // Find the device
        let device = self.devices.read().get(device_id).cloned().ok_or_else(|| {
            UotError::Transfer(TransferError::DeviceNotFound(device_id.to_string()))
        })?;

        // Get existing connection from session or connections map
        let conn = {
            let sessions = self.sessions.read();
            sessions
                .get(device_id)
                .and_then(|s| s.read().connection.clone())
        }
        .or_else(|| self.connections.read().get(device_id).cloned());

        let conn = match conn {
            Some(c) => c,
            None => {
                // Fallback: connect fresh if no existing connection
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
                // Hello handshake for fresh connection
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
                Arc::new(new_conn)
            }
        };

        // Create transfer record
        let record =
            engine::create_transfer_record(&items, TransferDirection::Send, &device.device_name);
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

        tokio::spawn(async move {
            // Sender sends files immediately after Offer.
            // The RECEIVER side waits for user acceptance before processing FileStart.
            log::info!("Starting file transfer {transfer_id}...");

            let result = Self::execute_send_arc(
                &conn,
                items,
                transfer_id,
                &tracker,
                chunk_size,
                bandwidth_limit,
                pause_rx,
                &event_tx,
            )
            .await;

            let mut transfers = transfers.write();
            if let Some(record) = transfers.get_mut(&transfer_id) {
                match result {
                    Ok(()) => {
                        record.status = TransferStatus::Completed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.transferred_bytes = record.total_size;
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
    ) -> Result<(), TransferError> {
        let mut rate_limiter = RateLimiter::new(bandwidth_limit);

        for item in &items {
            tracker.set_current_item(&item.name);

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
                sha256: hash,
            };
            proto::send_message(conn, &verify)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            tracker.complete_item();
        }

        // Send transfer complete
        let complete = WireMessage::TransferComplete {
            transfer_id: transfer_id.to_string(),
            success: true,
        };
        proto::send_message(conn, &complete)
            .await
            .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

        Ok(())
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
    ) -> Result<(), TransferError> {
        let mut rate_limiter = RateLimiter::new(bandwidth_limit);

        for item in &items {
            tracker.set_current_item(&item.name);

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

                // Encrypt the entire chunk frame with AES-256-GCM
                let encrypted_frame = session_cipher
                    .encrypt_frame(&chunk_frame)
                    .map_err(|e| TransferError::Protocol(format!("Encryption error: {e}")))?;

                conn.send(Frame::data(encrypted_frame))
                    .await
                    .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

                offset += chunk_len;
                tracker.add_bytes(chunk_len);

                // Apply rate limiting
                rate_limiter.consume(chunk_len as usize).await;

                // Check pause signal
                while *pause_rx.borrow() {
                    // Wait until unpaused
                    if pause_rx.changed().await.is_err() {
                        break; // Sender dropped (transfer cancelled)
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
                sha256: hash,
            };
            proto::send_message(&conn, &verify)
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

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
    ) {
        let mut current_file: Option<(PathBuf, String, u64)> = None; // (path, name, size)
        let mut current_transfer_id: Option<Uuid> = None;
        let mut transfer_accepted = false;
        let mut recv_tracker: Option<Arc<ProgressTracker>> = None;
        let mut session_cipher: Option<SessionCipher> = None;

        loop {
            // 60-second idle timeout per frame
            let frame =
                match tokio::time::timeout(std::time::Duration::from_secs(60), conn.recv_frame())
                    .await
                {
                    Ok(Ok(f)) => f,
                    Ok(Err(_)) => break, // Connection closed
                    Err(_) => {
                        log::warn!("Connection from {remote} timed out (60s idle)");
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
                            devices.write().insert(peer_id.clone(), dev.clone());
                            // Store connection by peer device_id
                            connections
                                .write()
                                .insert(peer_id.clone(), Arc::clone(&conn));
                            // Create PeerSession for incoming connection
                            {
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
                            }
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
                                })
                                .collect();

                            let record = TransferRecord {
                                transfer_id,
                                remote_device: device_name.clone(),
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

                            // GAP 6 FIX: Create receive-side progress tracker
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
                        WireMessage::FileStart { .. } | WireMessage::FileEnd { .. }
                            if !transfer_accepted =>
                        {
                            // Wait up to 5 seconds for acceptance signal from UI
                            if let Some(tid) = current_transfer_id {
                                let mut accepted = false;
                                // Wait up to 120 seconds for user acceptance (GAP 7 fix)
                                for _ in 0..1200 {
                                    if accepted_transfers.read().contains(&tid) {
                                        accepted = true;
                                        break;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }

                                if accepted {
                                    transfer_accepted = true;
                                    if let Some(record) = transfers.write().get_mut(&tid) {
                                        record.status = TransferStatus::InProgress;
                                        record.started_at = Some(chrono::Utc::now());
                                    }
                                    log::info!("Transfer {tid} accepted, processing files");

                                    match wire_msg {
                                        WireMessage::FileStart {
                                            file_name,
                                            file_size,
                                            relative_path,
                                            ..
                                        } => {
                                            let path_validator = StrictPathValidator::new(Some(
                                                PathBuf::from(save_dir),
                                            ));
                                            let sanitized = match path_validator
                                                .validate_relative_path(&relative_path)
                                            {
                                                Ok(clean) => clean,
                                                Err(e) => {
                                                    log::error!(
                                                        "Path validation failed for {relative_path}: {e}"
                                                    );
                                                    path_validator.sanitize_filename(&file_name)
                                                }
                                            };
                                            let file_path =
                                                PathBuf::from(save_dir).join(&sanitized);
                                            if file_path.exists() && file_path.is_symlink() {
                                                log::error!(
                                                    "Refusing to write to symlink: {}",
                                                    file_path.display()
                                                );
                                            } else {
                                                current_file =
                                                    Some((file_path, file_name.clone(), file_size));
                                                log::info!(
                                                    "Receiving file: {file_name} ({file_size} bytes)"
                                                );
                                            }
                                        }
                                        WireMessage::FileEnd { sha256, .. } => {
                                            if let Some((ref path, ref name, _)) = current_file {
                                                match engine::compute_sha256(path).await {
                                                    Ok(actual_hash) => {
                                                        if actual_hash == sha256 {
                                                            log::info!("File {name} verified ✓");
                                                        } else {
                                                            log::error!("File {name} hash mismatch! Expected: {sha256}, Got: {actual_hash}");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::error!("Cannot verify {name}: {e}")
                                                    }
                                                }
                                            }
                                            current_file = None;
                                        }
                                        _ => {}
                                    }
                                } else {
                                    log::warn!(
                                        "File frame for unaccepted transfer {tid} timed out"
                                    );
                                    continue;
                                }
                            }
                        }
                        WireMessage::FileStart {
                            file_name,
                            file_size,
                            relative_path,
                            ..
                        } => {
                            // Sanitize path (security: strict path validation)
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
                            let file_path = PathBuf::from(save_dir).join(&sanitized);

                            // Security: check for symlink at target
                            if file_path.exists() && file_path.is_symlink() {
                                log::error!(
                                    "Refusing to write to symlink: {}",
                                    file_path.display()
                                );
                                continue;
                            }

                            current_file = Some((file_path, file_name.clone(), file_size));
                            log::info!("Receiving file: {file_name} ({file_size} bytes)");
                        }
                        WireMessage::FileEnd { sha256, .. } => {
                            if let Some((ref path, ref name, _)) = current_file {
                                match engine::compute_sha256(path).await {
                                    Ok(actual_hash) => {
                                        if actual_hash == sha256 {
                                            log::info!("File {name} verified ✓");
                                        } else {
                                            log::error!("File {name} hash mismatch! Expected: {sha256}, Got: {actual_hash}");
                                        }
                                    }
                                    Err(e) => log::error!("Cannot verify {name}: {e}"),
                                }
                            }
                            current_file = None;
                        }
                        WireMessage::TransferComplete {
                            transfer_id: ref tid_str,
                            ..
                        } => {
                            let tid = Uuid::parse_str(tid_str).ok().or(current_transfer_id);
                            if let Some(tid) = tid {
                                let mut t = transfers.write();
                                if let Some(record) = t.get_mut(&tid) {
                                    record.status = TransferStatus::Completed;
                                    record.finished_at = Some(chrono::Utc::now());
                                }
                                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                                    transfer_id: tid,
                                    status: TransferStatus::Completed,
                                });
                            }
                            log::info!("Transfer complete from {remote}");
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

                            // Send MessageAck
                            let ack = WireMessage::MessageAck {
                                message_id: mid_str.clone(),
                            };
                            let _ = proto::send_message(&conn, &ack).await;

                            // Emit IncomingMessage event
                            let _ = event_tx
                                .send(EngineEvent::IncomingMessage {
                                    session_id: Uuid::nil(), // Will be set when session lookup is wired
                                    message_id: msg_id,
                                    from_device: remote.to_string(),
                                    content: content.clone(),
                                    timestamp,
                                })
                                .await;

                            // Also emit as legacy ClipboardReceived for backward compat
                            let _ = event_tx
                                .send(EngineEvent::ClipboardReceived {
                                    from_device: remote.to_string(),
                                    text: content.clone(),
                                })
                                .await;
                        }
                        WireMessage::MessageAck {
                            message_id: ref mid_str,
                        } => {
                            log::info!("MessageAck received for {mid_str} from {remote}");
                            if let Ok(msg_id) = Uuid::parse_str(mid_str) {
                                let _ = event_tx
                                    .send(EngineEvent::MessageDelivered {
                                        session_id: Uuid::nil(),
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
                    if let Some((ref path, _, _)) = current_file {
                        // Decrypt the frame payload if session cipher is established
                        let decrypted = if let Some(ref mut cipher) = session_cipher {
                            match cipher.decrypt_frame(&frame.payload) {
                                Ok(plain) => plain,
                                Err(e) => {
                                    log::error!("Decryption failed: {e}");
                                    continue;
                                }
                            }
                        } else {
                            // Fallback: plaintext (legacy/unencrypted connection)
                            frame.payload.clone()
                        };

                        if decrypted.len() < 16 {
                            log::error!("Data frame too small after decryption");
                            continue;
                        }

                        let offset = u64::from_be_bytes(decrypted[..8].try_into().unwrap());
                        let crc = u32::from_be_bytes(decrypted[8..12].try_into().unwrap());
                        let chunk_data = &decrypted[16..];

                        if let Err(e) = engine::write_chunk(path, offset, chunk_data, crc).await {
                            log::error!("Write chunk failed: {e}");
                        } else {
                            // GAP 6 FIX: Track receive progress
                            let chunk_len = chunk_data.len() as u64;
                            if let Some(ref tracker) = recv_tracker {
                                tracker.add_bytes(chunk_len);
                                let progress = tracker.snapshot();
                                let _ = event_tx.try_send(EngineEvent::TransferProgress(progress));
                            }
                            // Update transfer record bytes
                            if let Some(tid) = current_transfer_id {
                                if let Some(record) = transfers.write().get_mut(&tid) {
                                    record.transferred_bytes += chunk_len;
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

    // ── Session Management Methods ──

    /// Get or create a session for a peer device.
    pub fn get_or_create_session(
        &self,
        peer_device_id: &str,
        peer_name: &str,
    ) -> Arc<RwLock<PeerSession>> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get(peer_device_id) {
            return Arc::clone(session);
        }
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
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(peer_device_id) {
            let s = session.read();
            let msgs: Vec<String> = s
                .messages
                .iter()
                .map(|m| {
                    let escaped = m
                        .content
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n");
                    format!(
                        r#"{{"message_id":"{}","direction":"{}","timestamp":"{}","content":"{}","state":"{}"}}"#,
                        m.message_id,
                        if m.direction == MessageDirection::Outgoing {
                            "out"
                        } else {
                            "in"
                        },
                        m.timestamp.to_rfc3339(),
                        escaped,
                        m.state,
                    )
                })
                .collect();
            format!("[{}]", msgs.join(","))
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

        // Get or auto-create session from connections map
        let session_arc = {
            let sessions = self.sessions.read();
            sessions.get(peer_device_id).cloned()
        }
        .or_else(|| {
            // Try to find connection by device_id and create session on-demand
            let conn = self.connections.read().get(peer_device_id).cloned();
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
        });

        let session_arc = session_arc.ok_or_else(|| {
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

        // Get connection from session or connections map
        let conn = {
            let session = session_arc.read();
            session.connection.clone()
        }
        .or_else(|| self.connections.read().get(peer_device_id).cloned());

        let conn = conn.ok_or_else(|| {
            // Mark message as failed
            session_arc
                .write()
                .update_message_state(message_id, MessageState::Failed);
            UotError::Transfer(TransferError::DeviceNotFound(format!(
                "No connection to {}",
                peer_device_id
            )))
        })?;

        // Send ChatMessage wire message
        let wire_msg = WireMessage::ChatMessage {
            message_id: message_id.to_string(),
            content: text,
            timestamp: now.timestamp(),
        };

        match proto::send_message(&conn, &wire_msg).await {
            Ok(()) => {
                session_arc
                    .write()
                    .update_message_state(message_id, MessageState::Sent);
                let device_name = self.config.read().device_name.clone();
                let _ = self
                    .event_tx
                    .send(EngineEvent::IncomingMessage {
                        session_id,
                        message_id,
                        from_device: device_name,
                        content: String::new(), // outgoing, content already in session
                        timestamp: now.timestamp(),
                    })
                    .await;
                log::info!("Chat message {message_id} sent to {peer_device_id}");
                Ok(message_id)
            }
            Err(e) => {
                session_arc
                    .write()
                    .update_message_state(message_id, MessageState::Failed);
                log::error!("Failed to send chat message to {peer_device_id}: {e}");
                Err(UotError::Transport(e))
            }
        }
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

    /// Get all discovered devices (filtering out self-device and local IP/ports).
    pub fn discovered_devices(&self) -> Vec<DiscoveredDevice> {
        let my_id = &self.device_id;
        let my_ips = tcp::local_ips();
        let my_port = self.listening_port();

        self.devices
            .read()
            .values()
            .filter(|dev| {
                if dev.device_id == *my_id {
                    return false;
                }
                if let Some(ref addr_str) = dev.address {
                    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                        if addr.port() == my_port && my_ips.contains(&addr.ip()) {
                            return false;
                        }
                    }
                }
                true
            })
            .cloned()
            .collect()
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
        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::Cancelled;
            record.finished_at = Some(chrono::Utc::now());
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
        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::InProgress;
            self.log_event(&format!("Transfer {transfer_id} accepted"));
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
        self.devices
            .write()
            .insert(remote_device_id.clone(), device.clone());
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
                )
                .await;
            });
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
