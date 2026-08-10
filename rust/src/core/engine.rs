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
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::error::{TransferError, TransportError, UotError};
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
                    tokio::spawn(async move {
                        while let Some(event) = discovery_rx.recv().await {
                            match event {
                                DiscoveryEvent::DeviceFound(device) => {
                                    devices
                                        .write()
                                        .insert(device.device_id.clone(), device.clone());
                                    let _ = event_tx.send(EngineEvent::DeviceFound(device)).await;
                                }
                                DiscoveryEvent::DeviceLost(id) => {
                                    devices.write().remove(&id);
                                    let _ = event_tx.send(EngineEvent::DeviceLost(id)).await;
                                }
                                DiscoveryEvent::DeviceUpdated(device) => {
                                    devices
                                        .write()
                                        .insert(device.device_id.clone(), device.clone());
                                    let _ = event_tx.send(EngineEvent::DeviceUpdated(device)).await;
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

                tokio::spawn(async move {
                    Self::handle_incoming_connection(
                        conn,
                        &remote,
                        &transfers_clone,
                        &trackers_clone,
                        &event_tx3,
                        &save_dir_clone,
                        &accepted_clone,
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

        // Find the device address
        let device = self.devices.read().get(device_id).cloned().ok_or_else(|| {
            UotError::Transfer(TransferError::DeviceNotFound(device_id.to_string()))
        })?;

        let addr_str = device.address.ok_or_else(|| {
            UotError::Transfer(TransferError::DeviceNotFound("No address".to_string()))
        })?;

        let addr: SocketAddr = addr_str.parse().map_err(|e| {
            UotError::Transport(TransportError::Connection(format!("Invalid addr: {e}")))
        })?;

        // Create transfer record
        let record =
            engine::create_transfer_record(&items, TransferDirection::Send, &device.device_name);
        let transfer_id = record.transfer_id;
        self.transfers.write().insert(transfer_id, record.clone());

        // Push to priority queue manager for batch scheduling
        {
            let mut qm = self.queue_manager.write();
            if !qm.can_start() {
                // Queue the transfer for later execution
                qm.push(record, Priority::Normal);
                log::info!("Transfer {transfer_id} queued (concurrent limit reached)");
                return Ok(transfer_id);
            }
            qm.push(record, Priority::Normal);
            qm.mark_started();
        }

        // Connect to the device
        let stream = tcp::connect(addr).await.map_err(UotError::Transport)?;
        let conn = TcpConnection::new(stream).map_err(UotError::Transport)?;

        // Perform X25519 key exchange for session encryption
        let (our_private, our_public) =
            SessionCipher::create_key_exchange().map_err(UotError::Security)?;

        // Send our public key to the receiver
        let key_msg = WireMessage::KeyExchange {
            public_key: our_public,
        };
        proto::send_message(&conn, &key_msg)
            .await
            .map_err(UotError::Transport)?;

        // Wait for their public key
        let their_key_msg = proto::recv_message(&conn)
            .await
            .map_err(UotError::Transport)?;
        let their_public = match their_key_msg {
            WireMessage::KeyExchange { public_key } => public_key,
            _ => {
                return Err(UotError::Security(
                    crate::core::error::SecurityError::KeyExchangeFailed {
                        reason: "Expected KeyExchange message from receiver".to_string(),
                    },
                ));
            }
        };

        // Derive session cipher
        let session_cipher = SessionCipher::from_key_exchange(&our_private, &their_public)
            .map_err(UotError::Security)?;

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
            let result = Self::execute_send(
                conn,
                items,
                transfer_id,
                &tracker,
                chunk_size,
                bandwidth_limit,
                pause_rx,
                &event_tx,
                session_cipher,
            )
            .await;

            let mut transfers = transfers.write();
            if let Some(record) = transfers.get_mut(&transfer_id) {
                match result {
                    Ok(()) => {
                        record.status = TransferStatus::Completed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.transferred_bytes = record.total_size;
                        // Record success in analytics
                        let speed = tracker.snapshot().speed_bytes_per_sec;
                        stats.write().record_success(record.total_size, true, speed);
                    }
                    Err(e) => {
                        record.status = TransferStatus::Failed;
                        record.finished_at = Some(chrono::Utc::now());
                        record.error = Some(e.to_string());
                        log::error!("Transfer {transfer_id} failed: {e}");
                        // Record failure in analytics
                        stats.write().record_failure();
                    }
                }
                let _ = event_tx.try_send(EngineEvent::TransferStatusChanged {
                    transfer_id,
                    status: record.status,
                });
                // Persist history
                history.write().upsert(record.clone());
                let _ = history.read().save(&TransferHistoryStore::default_path());
                let _ = stats.read().save(&LifetimeStats::default_path());
                // Clean up pause signal
                pause_signals.write().remove(&transfer_id);
                // Mark transfer completed in queue manager
                queue_manager.write().mark_completed();
            }
        });

        Ok(transfer_id)
    }

    /// Execute the send operation — chunked file transfer with AES-256-GCM encryption.
    #[allow(clippy::too_many_arguments)]
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
    async fn handle_incoming_connection(
        conn: Arc<TcpConnection>,
        remote: &str,
        transfers: &Arc<RwLock<HashMap<Uuid, TransferRecord>>>,
        _trackers: &Arc<RwLock<HashMap<Uuid, Arc<ProgressTracker>>>>,
        event_tx: &mpsc::Sender<EngineEvent>,
        save_dir: &str,
        accepted_transfers: &Arc<RwLock<std::collections::HashSet<Uuid>>>,
    ) {
        let mut current_file: Option<(PathBuf, String, u64)> = None; // (path, name, size)
        let mut current_transfer_id: Option<Uuid> = None;
        let mut transfer_accepted = false;
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
                            // Check if the transfer has been accepted by the UI
                            if let Some(tid) = current_transfer_id {
                                if accepted_transfers.read().contains(&tid) {
                                    transfer_accepted = true;
                                    if let Some(record) = transfers.write().get_mut(&tid) {
                                        record.status = TransferStatus::InProgress;
                                        record.started_at = Some(chrono::Utc::now());
                                    }
                                    log::info!("Transfer {tid} accepted, processing files");
                                    // IMPORTANT: Do NOT continue — fall through to let
                                    // the wire_msg be re-matched by the FileStart/FileEnd
                                    // arms below (transfer_accepted is now true so this
                                    // guard arm won't re-match; Rust will try the next arms).
                                    //
                                    // However, Rust match already consumed this arm.
                                    // We must manually dispatch the frame here.
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
                                        _ => {} // unreachable due to guard
                                    }
                                } else {
                                    // Not yet accepted — skip file frames until accepted
                                    log::debug!(
                                        "Skipping file frame for unaccepted transfer {tid}"
                                    );
                                    continue;
                                }
                            } else {
                                log::warn!("File frame without prior offer from {remote}");
                                continue;
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
                        WireMessage::TransferComplete { .. } => {
                            if let Some(tid) = current_transfer_id {
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

    /// Get all discovered devices.
    pub fn discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.devices.read().values().cloned().collect()
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

        let stream = tcp::connect(addr).await.map_err(UotError::Transport)?;
        let conn = TcpConnection::new(stream).map_err(UotError::Transport)?;

        let text_len = text.len();
        let msg = WireMessage::ClipboardData {
            content_type: "text/plain".to_string(),
            data: text,
        };
        proto::send_message(&conn, &msg)
            .await
            .map_err(UotError::Transport)?;
        log::info!("Clipboard sent to {device_id}: {text_len} bytes");
        Ok(())
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
    pub async fn connect_peer(&self, addr_str: &str) -> Result<DiscoveredDevice, UotError> {
        let default_port = self.config.read().network_port.unwrap_or(tcp::DEFAULT_PORT);
        let trimmed = addr_str.trim();

        // Extract IP and target port list (only fallback ports if no explicit port supplied)
        let (ip_str, target_ports) = if trimmed.contains(':') {
            let parts: Vec<&str> = trimmed.split(':').collect();
            let ip = parts[0];
            let port_parsed = parts[1].parse::<u16>().unwrap_or(default_port);
            (ip, vec![port_parsed])
        } else {
            (trimmed, vec![default_port, 42000, 42001, 8080, 50000])
        };

        // Try connecting to target ports with a 4-second timeout per port
        let mut last_err = String::new();
        let mut connected_stream = None;
        let mut final_socket_addr = None;

        for &port in &target_ports {
            let full_addr_str = format!("{ip_str}:{port}");
            if let Ok(socket_addr) = full_addr_str.parse::<SocketAddr>() {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(4),
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
                        last_err = format!("Connection to {socket_addr} timed out after 4 seconds (Windows Firewall / Wi-Fi filter)");
                    }
                }
            }
        }

        let conn_stream = match connected_stream {
            Some(s) => s,
            None => {
                return Err(UotError::Transport(
                    crate::core::error::TransportError::ConnectionFailed {
                        reason: format!("{last_err}. Check that both devices are on the same Wi-Fi and port 42000 is allowed in Firewall."),
                    },
                ));
            }
        };

        let socket_addr = final_socket_addr.unwrap();
        let conn = TcpConnection::new(conn_stream)?;

        let remote_ip = socket_addr.ip().to_string();
        let device_id = format!("peer-{}", remote_ip.replace('.', "-"));
        let device_name = format!("Device ({remote_ip})");
        let now = chrono::Utc::now();

        let device = DiscoveredDevice {
            device_id: device_id.clone(),
            device_name: device_name.clone(),
            device_type: DeviceType::Desktop,
            discovery_method: crate::discovery::types::DiscoveryMethod::Manual,
            address: Some(socket_addr.to_string()),
            capabilities: vec!["tcp_lan".to_string()],
            signal_strength: Some(100),
            first_seen: now,
            last_seen: now,
            is_trusted: false,
        };

        self.devices
            .write()
            .insert(device_id.clone(), device.clone());
        self.connections
            .write()
            .insert(device_id.clone(), Arc::new(conn));
        let _ = self
            .event_tx
            .send(EngineEvent::DeviceFound(device.clone()))
            .await;

        Ok(device)
    }

    /// Fallback discovery: scan local subnet for UOT listeners.
    pub async fn subnet_scan(&self) -> Vec<std::net::SocketAddr> {
        use crate::discovery::subnet::SubnetScanner;
        use crate::discovery::types::DiscoveryMethod;

        let port = self.config.read().network_port.unwrap_or(tcp::DEFAULT_PORT);
        let scanner = SubnetScanner::new(port);

        // Get local IPs and scan each /24 subnet
        let local_ips = tcp::local_ips();
        let mut all_found = Vec::new();
        let now = chrono::Utc::now();
        for ip in local_ips {
            if let std::net::IpAddr::V4(v4) = ip {
                let octets = v4.octets();
                let found = scanner.scan_subnet(octets).await;
                for addr in &found {
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
