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

    let config = AppConfig::default();
    let (engine, _event_rx) = UotEngine::new(config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Start the engine
    let device_id = engine.device_id().to_string();

    let start_result = runtime.block_on(async { engine.start().await });

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
        serde_json::to_string(&transfers).unwrap_or_else(|_| "[]".to_string())
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
        match runtime.block_on(async { engine.accept_transfer(&transfer_id).await }) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        }
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
        let inv = crate::security::qr::QrInvitation::new(
            config.device_name.clone(),
            engine.device_id().to_string(),
            public_key_b64,
            format!("127.0.0.1:{}", config.network_port.unwrap_or(42000)),
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
#[flutter_rust_bridge::frb(sync)]
pub fn engine_search_history(query: String) -> String {
    let path = crate::transfer::history::TransferHistoryStore::default_path();
    let store = crate::transfer::history::TransferHistoryStore::load(&path);
    let results = store.query(&query, None);
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

/// Get cumulative lifetime transfer statistics as JSON.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_get_stats() -> String {
    let path = crate::transfer::analytics::LifetimeStats::default_path();
    let stats = crate::transfer::analytics::LifetimeStats::load(&path);
    serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
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
    fn test_engine_state_before_init() {
        let state = engine_state();
        // Before init, should return "Stopped" since engine doesn't exist
        assert!(state == "Stopped" || state == "Running");
    }
}
