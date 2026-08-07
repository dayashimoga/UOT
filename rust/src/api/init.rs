//! UOT Initialization API
//!
//! Functions for initializing the core engine, querying version info,
//! and performing health checks. Exposed to Dart via FRB.
use crate::core::config::AppConfig;
use crate::core::version::{self, BuildInfo};

/// Get the current version string of the UOT core engine.
#[flutter_rust_bridge::frb(sync)]
pub fn get_version() -> String {
    version::version_string()
}

/// Get the current protocol version.
#[flutter_rust_bridge::frb(sync)]
pub fn get_protocol_version() -> u32 {
    version::PROTOCOL_VERSION
}

/// Get detailed build information.
#[flutter_rust_bridge::frb(sync)]
pub fn get_build_info() -> String {
    let info = BuildInfo::current();
    serde_json::to_string(&info).unwrap_or_else(|_| info.to_string())
}

/// Perform a health check on the core engine.
/// Returns a JSON string with status information.
#[flutter_rust_bridge::frb(sync)]
pub fn health_check() -> String {
    let info = BuildInfo::current();
    let config = AppConfig::default();
    let health = serde_json::json!({
        "status": "healthy",
        "version": info.version,
        "protocol_version": info.protocol_version,
        "target": info.target,
        "profile": info.profile,
        "device_name": config.device_name,
        "chunk_size": config.transfer.chunk_size,
    });
    serde_json::to_string(&health).unwrap_or_else(|_| r#"{"status":"error"}"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    #[test]
    fn test_get_protocol_version() {
        let pv = get_protocol_version();
        assert_eq!(pv, 1);
    }

    #[test]
    fn test_get_build_info() {
        let info = get_build_info();
        assert!(info.contains("version"));
        assert!(info.contains("protocol_version"));
    }

    #[test]
    fn test_health_check() {
        let health = health_check();
        assert!(health.contains("healthy"));
        assert!(health.contains("version"));
    }
}
