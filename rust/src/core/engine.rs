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
use crate::security::verification::TrustManager;
use crate::security::PathValidator;
use crate::transfer::analytics::LifetimeStats;
use crate::transfer::engine::{self, ProgressTracker, TransferItem};
use crate::transfer::history::TransferHistoryStore;
use crate::transfer::queue::{Priority, TransferQueueManager};
use crate::transfer::ratelimit::RateLimiter;
use crate::transfer::types::{
    TransferDirection, TransferItemRecord, TransferProgress, TransferRecord, TransferStatus,
};
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

        // Start mDNS discovery
        let mut mdns = MdnsDiscovery::new().map_err(|e| {
            UotError::Discovery(crate::core::error::DiscoveryError::ServiceError(e))
        })?;

        let device_type = DeviceType::Desktop; // TODO: detect platform
        mdns.register(
            &self.device_id,
            &device_name_clone,
            actual_port,
            device_type,
        )
        .map_err(|e| UotError::Discovery(crate::core::error::DiscoveryError::ServiceError(e)))?;

        // Start browsing
        let mut discovery_rx = mdns.start_browsing().map_err(|e| {
            UotError::Discovery(crate::core::error::DiscoveryError::ServiceError(e))
        })?;

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
        self.queue_manager.write().push(record, Priority::Normal);

        // Connect to the device
        let stream = tcp::connect(addr).await.map_err(UotError::Transport)?;
        let conn = TcpConnection::new(stream).map_err(UotError::Transport)?;

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
            }
        });

        Ok(transfer_id)
    }

    /// Execute the send operation — chunked file transfer.
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

                conn.send(Frame::data(chunk_frame))
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

        loop {
            // 60-second idle timeout per frame
            let frame = match tokio::time::timeout(
                std::time::Duration::from_secs(60),
                conn.recv_frame(),
            )
            .await
            {
                Ok(Ok(f)) => f,
                Ok(Err(_)) => break,  // Connection closed
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
                        WireMessage::FileStart { .. }
                        | WireMessage::FileEnd { .. }
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
                                    // Re-process this frame by continuing the match
                                    // We need to re-dispatch — fall through to the next match iteration
                                } else {
                                    // Not yet accepted — skip file frames until accepted
                                    log::debug!("Skipping file frame for unaccepted transfer {tid}");
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
                        if frame.payload.len() < 16 {
                            log::error!("Data frame too small");
                            continue;
                        }

                        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
                        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
                        let chunk_data = &frame.payload[16..];

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

    /// Generate a 6-digit PIN for device pairing/verification.
    pub fn generate_pin(&self, ttl_secs: u64) -> String {
        self.trust_manager.write().generate_pin(ttl_secs).to_string()
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
    pub fn get_streams(&self) -> Vec<serde_json::Value> {
        // Streaming sessions will be managed by StreamManager
        // For now, return empty list
        Vec::new()
    }

    /// Get the current configuration (read-only).
    pub fn config(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// Fallback discovery: scan local subnet for UOT listeners.
    pub async fn subnet_scan(&self) -> Vec<std::net::SocketAddr> {
        use crate::discovery::subnet::SubnetScanner;

        let port = self.config.read().network_port.unwrap_or(tcp::DEFAULT_PORT);
        let scanner = SubnetScanner::new(port);

        // Get local IPs and scan each /24 subnet
        let local_ips = tcp::local_ips();
        let mut all_found = Vec::new();
        for ip in local_ips {
            if let std::net::IpAddr::V4(v4) = ip {
                let octets = v4.octets();
                let found = scanner.scan_subnet(octets).await;
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
        self.fallback_manager.read().select_best_transport(candidates)
    }

    /// Set transport selection strategy (PreferSpeed, PreferOffline, Manual).
    pub fn set_transport_strategy(&self, strategy: TransportSelectionStrategy) {
        self.fallback_manager.write().strategy = strategy;
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
}
