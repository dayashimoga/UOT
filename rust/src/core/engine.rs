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
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::error::{TransferError, TransportError, UotError};
use crate::discovery::mdns::{DiscoveryEvent, MdnsDiscovery};
use crate::discovery::types::{DeviceType, DiscoveredDevice};
use crate::security::path_validator::StrictPathValidator;
use crate::security::PathValidator;
use crate::transfer::analytics::LifetimeStats;
use crate::transfer::engine::{self, ProgressTracker, TransferItem};
use crate::transfer::history::TransferHistoryStore;
use crate::transfer::types::{TransferDirection, TransferProgress, TransferRecord, TransferStatus};
use crate::transport::tcp::{self, Frame, FrameType, TcpConnection, TcpTransportListener};

/// Maximum number of recent events to keep in the ring buffer.
const MAX_EVENT_LOG: usize = 200;

/// Engine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Not started.
    Stopped,
    /// Starting up.
    Starting,
    /// Running and ready.
    Running,
    /// Shutting down.
    ShuttingDown,
}

/// Main UOT engine coordinator.
pub struct UotEngine {
    /// Configuration.
    config: Arc<RwLock<AppConfig>>,
    /// Current state.
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

                tokio::spawn(async move {
                    Self::handle_incoming_connection(
                        conn,
                        &remote,
                        &transfers_clone,
                        &trackers_clone,
                        &event_tx3,
                        &save_dir_clone,
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
        self.transfers.write().insert(transfer_id, record);

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
        let offer = serde_json::json!({
            "type": "offer",
            "transfer_id": transfer_id.to_string(),
            "device_name": self.config.read().device_name,
            "items": items.iter().map(|i| serde_json::json!({
                "name": i.name,
                "relative_path": i.relative_path,
                "size": i.size,
            })).collect::<Vec<_>>(),
            "total_size": total_bytes,
        });
        let offer_bytes = serde_json::to_vec(&offer).map_err(|e| {
            UotError::Transfer(TransferError::Protocol(format!("Serialize error: {e}")))
        })?;

        conn.send(Frame::control(&offer_bytes))
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

        let stats = Arc::clone(&self.lifetime_stats);
        let history = Arc::clone(&self.history_store);

        tokio::spawn(async move {
            let result =
                Self::execute_send(conn, items, transfer_id, &tracker, chunk_size, &event_tx).await;

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
            }
        });

        Ok(transfer_id)
    }

    /// Execute the send operation — chunked file transfer.
    async fn execute_send(
        conn: TcpConnection,
        items: Vec<TransferItem>,
        transfer_id: Uuid,
        tracker: &ProgressTracker,
        chunk_size: usize,
        event_tx: &mpsc::Sender<EngineEvent>,
    ) -> Result<(), TransferError> {
        for item in &items {
            tracker.set_current_item(&item.name);

            let file_size = item.size;
            let mut offset: u64 = 0;

            // Send file header
            let header = serde_json::json!({
                "type": "file_start",
                "transfer_id": transfer_id.to_string(),
                "name": item.name,
                "relative_path": item.relative_path,
                "size": file_size,
            });
            let header_bytes = serde_json::to_vec(&header)
                .map_err(|e| TransferError::Protocol(format!("Serialize error: {e}")))?;
            conn.send(Frame::control(&header_bytes))
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

                // Emit progress periodically
                let progress = tracker.snapshot();
                let _ = event_tx.try_send(EngineEvent::TransferProgress(progress));
            }

            // Compute and send file hash
            let hash = engine::compute_sha256(&item.path).await?;
            let verify = serde_json::json!({
                "type": "file_end",
                "transfer_id": transfer_id.to_string(),
                "name": item.name,
                "sha256": hash,
            });
            let verify_bytes = serde_json::to_vec(&verify)
                .map_err(|e| TransferError::Protocol(format!("Serialize error: {e}")))?;
            conn.send(Frame::control(&verify_bytes))
                .await
                .map_err(|e| TransferError::Protocol(format!("Send error: {e}")))?;

            tracker.complete_item();
        }

        // Send transfer complete
        let complete = serde_json::json!({
            "type": "transfer_complete",
            "transfer_id": transfer_id.to_string(),
        });
        let complete_bytes = serde_json::to_vec(&complete)
            .map_err(|e| TransferError::Protocol(format!("Serialize error: {e}")))?;
        conn.send(Frame::control(&complete_bytes))
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
    ) {
        let mut current_file: Option<(PathBuf, String, u64)> = None; // (path, name, size)
        let mut current_transfer_id: Option<Uuid> = None;

        loop {
            let frame = match conn.recv_frame().await {
                Ok(f) => f,
                Err(_) => break, // Connection closed
            };
            match frame.frame_type {
                FrameType::Control => {
                    let msg: serde_json::Value = match serde_json::from_slice(&frame.payload) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Invalid JSON from {remote}: {e}");
                            continue;
                        }
                    };

                    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    match msg_type {
                        "offer" => {
                            let transfer_id_str = msg
                                .get("transfer_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let transfer_id =
                                Uuid::parse_str(transfer_id_str).unwrap_or_else(|_| Uuid::new_v4());
                            let from_device = msg
                                .get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            let total_size =
                                msg.get("total_size").and_then(|v| v.as_u64()).unwrap_or(0);
                            let items: Vec<String> = msg
                                .get("items")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|i| {
                                            i.get("name")
                                                .and_then(|n| n.as_str())
                                                .map(|s| s.to_string())
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            current_transfer_id = Some(transfer_id);

                            // Auto-accept for now (will add consent UI later)
                            let _ = event_tx
                                .send(EngineEvent::IncomingOffer {
                                    transfer_id,
                                    from_device,
                                    items,
                                    total_size,
                                })
                                .await;

                            log::info!("Accepted transfer {transfer_id} from {remote}");
                        }
                        "file_start" => {
                            let name = msg
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let relative_path = msg
                                .get("relative_path")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&name);
                            let size = msg.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

                            // Sanitize path (security: strict path validation)
                            let path_validator =
                                StrictPathValidator::new(Some(PathBuf::from(save_dir)));
                            let sanitized = match path_validator
                                .validate_relative_path(relative_path)
                            {
                                Ok(clean) => clean,
                                Err(e) => {
                                    log::error!("Path validation failed for {relative_path}: {e}");
                                    path_validator.sanitize_filename(&name)
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

                            current_file = Some((file_path, name.clone(), size));
                            log::info!("Receiving file: {name} ({size} bytes)");
                        }
                        "file_end" => {
                            if let Some((ref path, ref name, _)) = current_file {
                                let expected_hash =
                                    msg.get("sha256").and_then(|v| v.as_str()).unwrap_or("");

                                match engine::compute_sha256(path).await {
                                    Ok(actual_hash) => {
                                        if actual_hash == expected_hash {
                                            log::info!("File {name} verified ✓");
                                        } else {
                                            log::error!("File {name} hash mismatch! Expected: {expected_hash}, Got: {actual_hash}");
                                        }
                                    }
                                    Err(e) => log::error!("Cannot verify {name}: {e}"),
                                }
                            }
                            current_file = None;
                        }
                        "transfer_complete" => {
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
                        _ => {
                            log::debug!("Unknown message type: {msg_type}");
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

    /// Pause a transfer.
    pub async fn pause_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;
        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::Paused;
            let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::Paused,
            });
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
    }

    /// Resume a transfer.
    pub async fn resume_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;
        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::InProgress;
            let _ = self.event_tx.try_send(EngineEvent::TransferStatusChanged {
                transfer_id: uuid,
                status: TransferStatus::InProgress,
            });
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
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

    /// Accept an incoming transfer offer.
    pub async fn accept_transfer(&self, transfer_id: &str) -> Result<(), UotError> {
        let uuid = Uuid::parse_str(transfer_id).map_err(|_e| {
            UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })
        })?;
        let mut transfers = self.transfers.write();
        if let Some(record) = transfers.get_mut(&uuid) {
            record.status = TransferStatus::InProgress;
            Ok(())
        } else {
            Err(UotError::Transfer(TransferError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            }))
        }
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

        let msg = serde_json::json!({
            "type": "clipboard",
            "content_type": "text/plain",
            "data": text,
        });
        let payload = serde_json::to_vec(&msg).map_err(|e| {
            UotError::Transfer(TransferError::Protocol(format!("Serialize error: {e}")))
        })?;

        conn.send(Frame::control(&payload))
            .await
            .map_err(UotError::Transport)?;
        log::info!("Clipboard sent to {device_id}: {} bytes", text.len());
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
