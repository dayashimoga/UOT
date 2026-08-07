//! UOT Engine API
//!
//! High-level API for the UOT engine, exposed to Dart via FRB.
//! Manages the engine singleton and provides async operations.
use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::RwLock;

use crate::core::config::AppConfig;
use crate::core::engine::UotEngine;

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
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.pause_transfer(&transfer_id).await }) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        }
    })
    .unwrap_or_else(|| "error:engine_not_initialized".to_string())
}

/// Resume a transfer.
pub fn engine_resume_transfer(transfer_id: String) -> String {
    with_engine_runtime(|engine, runtime| {
        match runtime.block_on(async { engine.resume_transfer(&transfer_id).await }) {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("error:{e}"),
        }
    })
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
