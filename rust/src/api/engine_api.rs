//! UOT Engine API
//!
//! High-level API for the UOT engine, exposed to Dart via FRB.
//! Manages the engine singleton and provides async operations.
use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::RwLock;

use crate::core::config::AppConfig;
use crate::core::engine::UotEngine;
use crate::security::CryptoProvider;

/// Global engine singleton.
static ENGINE: OnceLock<RwLock<Option<EngineHandle>>> = OnceLock::new();

/// Buffered events from the Rust engine, drained by Flutter via polling.
static EVENT_BUFFER: OnceLock<RwLock<Vec<String>>> = OnceLock::new();

struct EngineHandle {
    engine: UotEngine,
    runtime: tokio::runtime::Runtime,
}

/// Initialize the UOT engine. Call once at app startup.
pub fn engine_init() -> String {
    let cell = ENGINE.get_or_init(|| RwLock::new(None));
    let mut lock = cell.write();

    if lock.is_some() {
        return "already_initialized".to_string();
    }

    // Initialize event buffer
    EVENT_BUFFER.get_or_init(|| RwLock::new(Vec::new()));

    let config = AppConfig::default();
    let (engine, event_rx) = UotEngine::new(config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Start the engine
    let device_id = engine.device_id().to_string();

    // NOTE: Non-elevated netsh removed (GAP 8). Windows firewall is handled
    // by engine_fix_windows_firewall() which uses proper UAC elevation via
    // Start-Process -Verb RunAs. Auto-called from main.dart on Windows startup.

    let start_result = runtime.block_on(async { engine.start().await });

    // Spawn event forwarding task: drains engine events into EVENT_BUFFER
    // so Flutter can poll them via engine_poll_events()
    runtime.spawn(event_forwarder(event_rx));

    match start_result {
        Ok(()) => {
            *lock = Some(EngineHandle { engine, runtime });
            format!("ok:{device_id}")
        }
        Err(e) => {
            // Still store engine even if start partially failed
            *lock = Some(EngineHandle { engine, runtime });
            format!("partial:{device_id}:{e}")
        }
    }
}

/// Background task that forwards engine events into the poll buffer.
async fn event_forwarder(
    mut event_rx: tokio::sync::mpsc::Receiver<crate::core::engine::EngineEvent>,
) {
    use crate::core::engine::EngineEvent;
    while let Some(event) = event_rx.recv().await {
        let json = match &event {
            EngineEvent::IncomingOffer {
                transfer_id,
                from_device,
                items,
                total_size,
            } => {
                format!(
                    r#"{{"type":"IncomingOffer","transfer_id":"{}","from_device":"{}","items":{},"total_size":{}}}"#,
                    transfer_id,
                    from_device,
                    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string()),
                    total_size
                )
            }
            EngineEvent::TransferProgress(progress) => {
                format!(
                    r#"{{"type":"TransferProgress","data":{}}}"#,
                    serde_json::to_string(progress).unwrap_or_else(|_| "{}".to_string())
                )
            }
            EngineEvent::TransferStatusChanged {
                transfer_id,
                status,
            } => {
                format!(
                    r#"{{"type":"TransferStatusChanged","transfer_id":"{}","status":"{:?}"}}"#,
                    transfer_id, status
                )
            }
            EngineEvent::ClipboardReceived { from_device, text } => {
                let escaped_text = text
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n");
                format!(
                    r#"{{"type":"ClipboardReceived","from_device":"{}","text":"{}"}}"#,
                    from_device, escaped_text
                )
            }
            EngineEvent::DeviceFound(device) => {
                format!(
                    r#"{{"type":"DeviceFound","data":{}}}"#,
                    serde_json::to_string(device).unwrap_or_else(|_| "{}".to_string())
                )
            }
            EngineEvent::DeviceLost(id) => {
                format!(r#"{{"type":"DeviceLost","device_id":"{}"}}"#, id)
            }
            EngineEvent::DeviceUpdated(device) => {
                format!(
                    r#"{{"type":"DeviceUpdated","data":{}}}"#,
                    serde_json::to_string(device).unwrap_or_else(|_| "{}".to_string())
                )
            }
            EngineEvent::PeerStateChanged { device_id, state } => {
                format!(
                    r#"{{"type":"PeerStateChanged","device_id":"{}","state":"{}"}}"#,
                    device_id, state
                )
            }
            EngineEvent::StateChanged(state) => {
                format!(r#"{{"type":"StateChanged","state":"{:?}"}}"#, state)
            }
            // Phase 2: Session-aware events
            EngineEvent::SessionStateChanged {
                session_id,
                device_id,
                state,
            } => {
                format!(
                    r#"{{"type":"SessionStateChanged","session_id":"{}","device_id":"{}","state":"{}"}}"#,
                    session_id, device_id, state
                )
            }
            EngineEvent::IncomingMessage {
                session_id,
                message_id,
                from_device,
                content,
                timestamp,
            } => {
                let escaped = content
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n");
                format!(
                    r#"{{"type":"IncomingMessage","session_id":"{}","message_id":"{}","from_device":"{}","content":"{}","timestamp":{}}}"#,
                    session_id, message_id, from_device, escaped, timestamp
                )
            }
            EngineEvent::MessageDelivered {
                session_id,
                message_id,
            } => {
                format!(
                    r#"{{"type":"MessageDelivered","session_id":"{}","message_id":"{}"}}"#,
                    session_id, message_id
                )
            }
            EngineEvent::HeartbeatChanged {
                session_id,
                device_id,
                alive,
            } => {
                format!(
                    r#"{{"type":"HeartbeatChanged","session_id":"{}","device_id":"{}","alive":{}}}"#,
                    session_id, device_id, alive
                )
            }
            EngineEvent::OfferAccepted {
                session_id,
                transfer_id,
            } => {
                format!(
                    r#"{{"type":"OfferAccepted","session_id":"{}","transfer_id":"{}"}}"#,
                    session_id, transfer_id
                )
            }
            EngineEvent::OfferRejected {
                session_id,
                transfer_id,
                reason,
            } => {
                format!(
                    r#"{{"type":"OfferRejected","session_id":"{}","transfer_id":"{}","reason":"{}"}}"#,
                    session_id, transfer_id, reason
                )
            }
            EngineEvent::TransferCompleted {
                session_id,
                transfer_id,
            } => {
                format!(
                    r#"{{"type":"TransferCompleted","session_id":"{}","transfer_id":"{}"}}"#,
                    session_id, transfer_id
                )
            }
            EngineEvent::TransferFailed {
                session_id,
                transfer_id,
                error,
            } => {
                format!(
                    r#"{{"type":"TransferFailed","session_id":"{}","transfer_id":"{}","error":"{}"}}"#,
                    session_id, transfer_id, error
                )
            }
        };
        if let Some(buf) = EVENT_BUFFER.get() {
            let mut buffer = buf.write();
            // Cap buffer at 500 events to prevent unbounded memory growth
            if buffer.len() < 500 {
                buffer.push(json);
            }
        }
    }
}

/// Get the current engine state.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_state() -> String {
    with_engine(|engine| {
        let state = engine.state();
        format!("{state:?}")
    })
    .unwrap_or_else(|| "Stopped".to_string())
}

/// Get the device ID.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_device_id() -> String {
    with_engine(|engine| engine.device_id().to_string()).unwrap_or_default()
}

/// Get all discovered devices as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_devices() -> String {
    with_engine(|engine| {
        let devices = engine.discovered_devices();
        serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Get all transfers as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_transfers() -> String {
    with_engine(|engine| {
        let transfers = engine.get_transfers();
        let value_list: Vec<serde_json::Value> = transfers
            .into_iter()
            .map(|t| {
                let first_file = t
                    .items
                    .first()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| "File transfer".to_string());
                let total_size = t.total_size;
                let transferred_bytes = t.transferred_bytes;
                let progress = if total_size > 0 {
                    (transferred_bytes as f64) / (total_size as f64)
                } else {
                    0.0
                };

                let mut val = match serde_json::to_value(&t) {
                    Ok(v) => v,
                    Err(_) => return serde_json::json!({}),
                };

                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::json!(t.transfer_id.to_string()),
                    );
                    obj.insert("file_name".to_string(), serde_json::json!(first_file));
                    obj.insert(
                        "remote_name".to_string(),
                        serde_json::json!(t.remote_device),
                    );
                    obj.insert("total_bytes".to_string(), serde_json::json!(total_size));
                    obj.insert("progress".to_string(), serde_json::json!(progress));
                }

                val
            })
            .collect();

        serde_json::to_string(&value_list).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Send files to a device. Returns transfer ID or error.
pub fn engine_send_files(device_id: String, file_paths: Vec<String>) -> String {
    let paths: Vec<PathBuf> = file_paths.into_iter().map(PathBuf::from).collect();

    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.send_files(&device_id, paths).await }) {
            Ok(transfer_id) => format!("ok:{transfer_id}"),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Stop the engine.
pub fn engine_stop() {
    let cell = ENGINE.get_or_init(|| RwLock::new(None));
    let lock = cell.read();
    if let Some(handle) = lock.as_ref() {
        handle.engine.stop();
    }
}

/// Pause a transfer.
pub fn engine_pause_transfer(transfer_id: String) -> String {
    with_engine_runtime(
        |engine, _runtime| match engine.pause_transfer(&transfer_id) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        },
    )
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Resume a transfer.
pub fn engine_resume_transfer(transfer_id: String) -> String {
    with_engine_runtime(
        |engine, _runtime| match engine.resume_transfer(&transfer_id) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        },
    )
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Cancel a transfer.
pub fn engine_cancel_transfer(transfer_id: String) -> String {
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.cancel_transfer(&transfer_id).await }) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Accept an incoming transfer.
pub fn engine_accept_transfer(transfer_id: String) -> String {
    with_engine_runtime(|engine, runtime| {
        tokio::task::block_in_place(|| {
            match runtime.block_on(async { engine.accept_transfer(&transfer_id).await }) {
                Ok(()) => "ok".to_string(),
                Err(e) => format!("error:{e}"),
            }
        })
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Get transfer progress as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_progress(transfer_id: String) -> String {
    with_engine(|engine| {
        if let Ok(uuid) = uuid::Uuid::parse_str(&transfer_id) {
            if let Some(progress) = engine.get_progress(&uuid) {
                return serde_json::to_string(&progress).unwrap_or_else(|_| "null".to_string());
            }
        }
        "null".to_string()
    })
    .unwrap_or_else(|| "null".to_string())
}

/// Set device name.
pub fn engine_set_device_name(name: String) -> String {
    with_engine(|engine| {
        engine.set_device_name(&name);
        "ok".to_string()
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Send clipboard text to a device.
pub fn engine_send_clipboard(device_id: String, text: String) -> String {
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.send_clipboard(&device_id, text).await }) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Poll buffered engine events as JSON array. Drains the buffer.
/// Call this periodically from Flutter (e.g., every 1-2 seconds) to receive
/// IncomingOffer, TransferProgress, ClipboardReceived, PeerStateChanged etc.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_poll_events() -> String {
    if let Some(buf) = EVENT_BUFFER.get() {
        let mut buffer = buf.write();
        if buffer.is_empty() {
            return "[]".to_string();
        }
        let events: Vec<String> = buffer.drain(..).collect();
        format!("[{}]", events.join(","))
    } else {
        "[]".to_string()
    }
}

/// Get event log (latest N events as JSON).
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_events(limit: u32) -> String {
    with_engine(|engine| {
        let events = engine.get_recent_events(limit as usize);
        serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Get active streaming sessions as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_streams() -> String {
    with_engine(|engine| {
        let streams = engine.get_streams();
        serde_json::to_string(&streams).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Start a new media streaming session.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_start_stream(
    stream_type: String,
    remote_device_id: String,
    remote_device_name: String,
    port: u16,
    is_sender: bool,
) -> String {
    let st = match stream_type.to_lowercase().as_str() {
        "camera" => crate::streaming::manager::StreamType::Camera,
        "screen" => crate::streaming::manager::StreamType::Screen,
        "video" => crate::streaming::manager::StreamType::Video,
        "audio" => crate::streaming::manager::StreamType::Audio,
        _ => crate::streaming::manager::StreamType::Camera,
    };
    with_engine(|engine| {
        engine.start_stream(st, &remote_device_id, &remote_device_name, port, is_sender)
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Stop an active media streaming session.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_stop_stream(session_id: String) -> String {
    with_engine(|engine| {
        engine.stop_stream(&session_id);
        "ok".to_string()
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Load user settings as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_load_settings() -> String {
    let path = crate::core::settings::UserSettings::default_path();
    let settings = crate::core::settings::UserSettings::load(&path);
    serde_json::to_string(&settings).unwrap_or_else(|_| "{}".to_string())
}

/// Save user settings from JSON.
pub fn engine_save_settings(json: String) -> String {
    let path = crate::core::settings::UserSettings::default_path();
    match serde_json::from_str::<crate::core::settings::UserSettings>(&json) {
        Ok(settings) => match settings.save(&path) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        },
        Err(e) => format!("error:parse:{e}"),
    }
}

/// Generate QR invitation JSON string for device pairing.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_generate_qr_invitation(pin: String) -> String {
    with_engine(|engine| {
        let crypto = crate::security::crypto::SoftwareCryptoProvider::new();
        let key_pair = match crypto.generate_key_pair() {
            Ok(kp) => kp,
            Err(e) => {
                return format!("error:keygen:{e}");
            }
        };
        let public_key_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &key_pair.public_key,
        );

        let config = engine.config();
        // Use actual local IP (not 127.0.0.1) so remote devices can connect
        let local_ip = crate::transport::tcp::local_ips()
            .into_iter()
            .find(|ip| ip.is_ipv4() && !ip.is_loopback())
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let actual_port = engine.listening_port();
        let inv = crate::security::qr::QrInvitation::new(
            config.device_name.clone(),
            engine.device_id().to_string(),
            public_key_b64,
            format!("{}:{}", local_ip, actual_port),
            pin,
            300,
        );
        inv.to_json().unwrap_or_else(|_| "{}".to_string())
    })
    .unwrap_or_else(|| "{}".to_string())
}

/// Parse and validate QR invitation JSON string.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_parse_qr_invitation(json: String) -> String {
    match crate::security::qr::QrInvitation::from_json(&json) {
        Ok(inv) => {
            if inv.is_expired() {
                "error:expired".to_string()
            } else {
                serde_json::to_string(&inv).unwrap_or_else(|_| "error:serialize".to_string())
            }
        }
        Err(e) => format!("error:parse:{e}"),
    }
}

/// Search persistent transfer history with query string.
/// Search transfer history via engine's in-memory store.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_search_history(query: String) -> String {
    with_engine(|engine| {
        let results = engine.get_transfer_history(&query, None);
        serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Get cumulative lifetime transfer statistics from engine.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_stats() -> String {
    with_engine(|engine| {
        let stats = engine.get_lifetime_stats();
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    })
    .unwrap_or_else(|| "{}".to_string())
}

/// Fallback subnet scan for device discovery.
pub fn engine_subnet_scan() -> String {
    with_engine_runtime(|engine, runtime| {
        let addrs = runtime.block_on(engine.subnet_scan());
        let strs: Vec<String> = addrs.iter().map(|a| a.to_string()).collect();
        serde_json::to_string(&strs).unwrap_or_else(|_| "[]".to_string())
    })
    .unwrap_or_else(|| "[]".to_string())
}

/// Direct connection to a peer by address (IP:port or IP).
pub fn engine_connect_peer(address: String) -> String {
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.connect_peer(&address).await }) {
            Ok(dev) => serde_json::to_string(&dev).unwrap_or_else(|_| "ok".to_string()),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Get connection diagnostics as JSON (GAP 11).
/// Returns: engine_state, local_ips, listening_port, device_count, active_connections, peer_states, transfer_count.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_diagnostics() -> String {
    with_engine(|engine| engine.get_diagnostics())
        .unwrap_or_else(|| r#"{"engine_state":"Stopped"}"#.to_string())
}

/// Get all peer sessions as JSON array.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_sessions() -> String {
    with_engine(|engine| engine.get_sessions_json()).unwrap_or_else(|| "[]".to_string())
}

/// Get chat messages for a peer session as JSON array.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_messages(peer_device_id: String) -> String {
    with_engine(|engine| engine.get_session_messages(&peer_device_id))
        .unwrap_or_else(|| "[]".to_string())
}

/// Send a chat message to a peer. Returns message_id or error.
pub fn engine_send_message(peer_device_id: String, text: String) -> String {
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.send_chat_message(&peer_device_id, text).await }) {
            Ok(msg_id) => format!("ok:{msg_id}"),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Get local IPv4 addresses as JSON list.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_local_ips() -> String {
    let ips: Vec<String> = crate::transport::tcp::local_ips()
        .into_iter()
        .filter_map(|ip| {
            if let std::net::IpAddr::V4(v4) = ip {
                if !v4.is_loopback() {
                    return Some(v4.to_string());
                }
            }
            None
        })
        .collect();
    serde_json::to_string(&ips).unwrap_or_else(|_| "[]".to_string())
}

/// Get the actual bound listening port of the local TCP transport engine.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_listening_port() -> u16 {
    with_engine(|engine| engine.listening_port()).unwrap_or(42000)
}

/// Get the connection state of a specific peer by device ID.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_peer_state(device_id: String) -> String {
    with_engine(|engine| {
        let state = engine.get_peer_state(&device_id);
        serde_json::to_string(&state).unwrap_or_else(|_| "\"Disconnected\"".to_string())
    })
    .unwrap_or_else(|| "\"Disconnected\"".to_string())
}

/// Generate a 6-digit verification PIN with specified TTL in seconds.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_generate_pin(ttl_secs: u64) -> String {
    with_engine(|engine| engine.generate_pin(ttl_secs))
        .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Verify a PIN attempt for a device ID, returning session token on success.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_verify_pin(device_id: String, attempt: String) -> String {
    with_engine(|engine| {
        engine
            .verify_pin(&device_id, &attempt)
            .unwrap_or_else(|| "invalid".to_string())
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Trigger Windows UAC prompt to create firewall rule allowing inbound TCP ports 42000-42010 & UDP.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_fix_windows_firewall() -> String {
    #[cfg(target_os = "windows")]
    {
        let cmd = "/c netsh advfirewall firewall delete rule name=\"UOT File Transfer\" & \
                   netsh advfirewall firewall add rule name=\"UOT File Transfer\" dir=in action=allow protocol=TCP localport=42000-42010 profile=any & \
                   netsh advfirewall firewall delete rule name=\"UOT Multicast Discovery\" & \
                   netsh advfirewall firewall add rule name=\"UOT Multicast Discovery\" dir=in action=allow protocol=UDP localport=42000,5353 profile=any";

        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process cmd.exe -Verb RunAs -ArgumentList '{cmd}' -WindowStyle Hidden"
                ),
            ])
            .status();

        match status {
            Ok(s) if s.success() => "ok:firewall_rule_added".to_string(),
            Ok(s) => format!("error:exit_code_{}", s.code().unwrap_or(-1)),
            Err(e) => format!("error:{e}"),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        "ok:not_windows".to_string()
    }
}

/// Encode data payload into animated fountain packets (for zero-network Optical QR transfer).
#[flutter_rust_bridge::frb(sync)]
pub fn engine_fountain_encode(data_base64: String, block_size: u32) -> String {
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&data_base64) {
        Ok(b) => b,
        Err(e) => return format!("error:base64:{e}"),
    };

    let bs = if block_size == 0 {
        128
    } else {
        block_size as usize
    };
    let mut encoder = crate::protocol::fountain::FountainEncoder::new(&bytes, bs);

    let mut packets = Vec::new();
    for _ in 0..60 {
        packets.push(encoder.next_packet());
    }

    serde_json::to_string(&packets).unwrap_or_else(|_| "[]".to_string())
}

/// Helper: access the engine.
fn with_engine<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&UotEngine) -> R,
{
    let cell = ENGINE.get()?;
    let lock = cell.read();
    let handle = lock.as_ref()?;
    Some(f(&handle.engine))
}

/// Helper: access the engine and runtime.
fn with_engine_runtime<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&UotEngine, &tokio::runtime::Runtime) -> R,
{
    let cell = ENGINE.get()?;
    let lock = cell.read();
    let handle = lock.as_ref()?;
    Some(f(&handle.engine, &handle.runtime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_api_full_inline_suite() {
        let before_state = engine_state();
        assert!(!before_state.is_empty());

        let init_res = engine_init();
        assert!(
            init_res.starts_with("ok:")
                || init_res.starts_with("partial:")
                || init_res == "already_initialized"
        );

        let state = engine_state();
        assert!(!state.is_empty());

        let dev_id = engine_device_id();
        assert!(!dev_id.is_empty());

        let devices = engine_get_devices();
        assert!(devices.starts_with('['));

        let transfers = engine_get_transfers();
        assert!(transfers.starts_with('['));

        let send_res = engine_send_files(
            "nonexistent-dev".to_string(),
            vec!["/tmp/fake.txt".to_string()],
        );
        assert!(send_res.starts_with("error:"));

        let pause_res = engine_pause_transfer("invalid-uuid".to_string());
        assert!(pause_res.starts_with("error:"));

        let resume_res = engine_resume_transfer("invalid-uuid".to_string());
        assert!(resume_res.starts_with("error:"));

        let cancel_res = engine_cancel_transfer("invalid-uuid".to_string());
        assert!(cancel_res.starts_with("error:"));

        let accept_res = engine_accept_transfer("invalid-uuid".to_string());
        assert!(accept_res.starts_with("error:"));

        let prog_res = engine_get_progress("invalid-uuid".to_string());
        assert_eq!(prog_res, "null");

        let name_res = engine_set_device_name("CustomAPIName".to_string());
        assert_eq!(name_res, "ok");

        let clip_res = engine_send_clipboard(
            "nonexistent-dev".to_string(),
            "clipboard content".to_string(),
        );
        assert!(clip_res.starts_with("error:"));

        let events = engine_get_events(10);
        assert!(events.starts_with('['));

        let streams = engine_get_streams();
        assert!(streams.starts_with('['));

        let stream_id = engine_start_stream(
            "Camera".to_string(),
            "dev-1".to_string(),
            "Device 1".to_string(),
            42000,
            true,
        );
        assert!(!stream_id.is_empty());

        let stop_stream_res = engine_stop_stream(stream_id);
        assert_eq!(stop_stream_res, "ok");

        let settings = engine_load_settings();
        assert!(settings.contains("device_name"));

        let save_res = engine_save_settings(settings);
        assert_eq!(save_res, "ok");

        let save_err_res = engine_save_settings("invalid-json".to_string());
        assert!(save_err_res.starts_with("error:"));

        let pin = engine_generate_pin(300);
        assert_eq!(pin.len(), 6);

        let qr_json = engine_generate_qr_invitation(pin.clone());
        assert!(qr_json.contains("pin"));

        let parse_qr = engine_parse_qr_invitation(qr_json.clone());
        assert!(parse_qr.contains("pin"));

        let parse_qr_err = engine_parse_qr_invitation("invalid-qr-json".to_string());
        assert!(parse_qr_err.starts_with("error:"));

        let history = engine_search_history("".to_string());
        assert!(history.starts_with('['));

        let stats = engine_get_stats();
        assert!(stats.contains("total_transfers"));

        let scan = engine_subnet_scan();
        assert!(scan.starts_with('['));

        let local_ips = engine_get_local_ips();
        assert!(local_ips.starts_with('['));

        let conn_peer_err = engine_connect_peer("127.0.0.1:59997".to_string());
        assert!(conn_peer_err.starts_with("error:"));

        engine_stop();
    }
}
