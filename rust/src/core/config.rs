//! UOT Configuration Management
//!
//! Provides application-level configuration with sensible defaults,
//! validation, and serialization support.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Device display name (shown to other devices).
    pub device_name: String,

    /// Unique device identifier (generated on first run).
    pub device_id: String,

    /// Transfer configuration.
    pub transfer: TransferConfig,

    /// Discovery configuration.
    pub discovery: DiscoveryConfig,

    /// Security configuration.
    pub security: SecurityConfig,

    /// Storage configuration.
    pub storage: StorageConfig,

    /// Network port override (None = use default 42000).
    pub network_port: Option<u16>,
}

/// Transfer engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferConfig {
    /// Chunk size in bytes for file transfers (default: 256 KiB).
    pub chunk_size: usize,

    /// Maximum concurrent transfers (default: 4).
    pub max_concurrent_transfers: usize,

    /// Transfer timeout in seconds (default: 300).
    pub transfer_timeout_secs: u64,

    /// Whether to automatically accept transfers from trusted devices.
    pub auto_accept_trusted: bool,

    /// Maximum file size in bytes (0 = unlimited).
    pub max_file_size: u64,

    /// Directory to save received files.
    pub save_directory: String,
}

/// Discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Whether device is discoverable by default.
    pub discoverable: bool,

    /// mDNS service name.
    pub service_name: String,

    /// Discovery scan interval in seconds.
    pub scan_interval_secs: u64,

    /// Discovery scan timeout in seconds.
    pub scan_timeout_secs: u64,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether to require PIN for new device pairing.
    pub require_pin: bool,

    /// Session timeout in seconds (default: 3600).
    pub session_timeout_secs: u64,

    /// QR invitation expiry in seconds (default: 300).
    pub qr_expiry_secs: u64,

    /// Whether to allow receiving from unknown devices.
    pub allow_unknown_devices: bool,
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory for received files.
    pub receive_directory: PathBuf,

    /// Directory for temporary/in-progress files.
    pub temp_directory: PathBuf,

    /// Maximum transfer history entries to keep.
    pub max_history_entries: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            device_name: hostname_or_default(),
            device_id: uuid::Uuid::new_v4().to_string(),
            transfer: TransferConfig::default(),
            discovery: DiscoveryConfig::default(),
            security: SecurityConfig::default(),
            storage: StorageConfig::default(),
            network_port: None,
        }
    }
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: 256 * 1024, // 256 KiB
            max_concurrent_transfers: 4,
            transfer_timeout_secs: 300,
            auto_accept_trusted: false,
            max_file_size: 0, // unlimited
            save_directory: dirs_fallback().to_string_lossy().to_string(),
        }
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discoverable: true,
            service_name: "_uot._tcp".to_string(),
            scan_interval_secs: 5,
            scan_timeout_secs: 30,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_pin: false,
            session_timeout_secs: 3600,
            qr_expiry_secs: 300,
            allow_unknown_devices: true,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        let downloads = dirs_fallback();
        Self {
            receive_directory: downloads.clone(),
            temp_directory: downloads.join(".uot_temp"),
            max_history_entries: 1000,
        }
    }
}

impl AppConfig {
    /// Validate the configuration, returning errors for invalid values.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.device_name.trim().is_empty() {
            errors.push("Device name cannot be empty".to_string());
        }
        if self.device_name.len() > 64 {
            errors.push("Device name must be 64 characters or fewer".to_string());
        }
        if self.transfer.chunk_size == 0 {
            errors.push("Chunk size must be greater than 0".to_string());
        }
        if self.transfer.chunk_size > 16 * 1024 * 1024 {
            errors.push("Chunk size must be 16 MiB or smaller".to_string());
        }
        if self.transfer.max_concurrent_transfers == 0 {
            errors.push("Max concurrent transfers must be at least 1".to_string());
        }
        if self.discovery.scan_interval_secs == 0 {
            errors.push("Scan interval must be greater than 0".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Get the system hostname, or a sensible default.
fn hostname_or_default() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "UOT Device".to_string())
}

/// Get a reasonable default downloads directory, falling back to current dir.
fn dirs_fallback() -> PathBuf {
    // Try common environment variables for downloads
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let downloads = PathBuf::from(&home).join("Downloads");
        if downloads.exists() {
            return downloads;
        }
        return PathBuf::from(home);
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(!config.device_name.is_empty());
        assert!(!config.device_id.is_empty());
        assert_eq!(config.transfer.chunk_size, 256 * 1024);
        assert_eq!(config.transfer.max_concurrent_transfers, 4);
        assert!(config.discovery.discoverable);
        assert_eq!(config.security.session_timeout_secs, 3600);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_name() {
        let mut config = AppConfig::default();
        config.device_name = "".to_string();
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn test_config_validation_long_name() {
        let mut config = AppConfig::default();
        config.device_name = "a".repeat(65);
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_zero_chunk_size() {
        let mut config = AppConfig::default();
        config.transfer.chunk_size = 0;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_huge_chunk_size() {
        let mut config = AppConfig::default();
        config.transfer.chunk_size = 32 * 1024 * 1024;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        let deserialized: AppConfig =
            serde_json::from_str(&json).expect("Failed to deserialize config");
        assert_eq!(config.device_name, deserialized.device_name);
        assert_eq!(config.transfer.chunk_size, deserialized.transfer.chunk_size);
    }

    #[test]
    fn test_transfer_config_defaults() {
        let tc = TransferConfig::default();
        assert_eq!(tc.chunk_size, 256 * 1024);
        assert_eq!(tc.max_concurrent_transfers, 4);
        assert_eq!(tc.transfer_timeout_secs, 300);
        assert!(!tc.auto_accept_trusted);
        assert_eq!(tc.max_file_size, 0);
    }

    #[test]
    fn test_discovery_config_defaults() {
        let dc = DiscoveryConfig::default();
        assert!(dc.discoverable);
        assert_eq!(dc.service_name, "_uot._tcp");
    }

    #[test]
    fn test_security_config_defaults() {
        let sc = SecurityConfig::default();
        assert!(!sc.require_pin);
        assert_eq!(sc.session_timeout_secs, 3600);
        assert_eq!(sc.qr_expiry_secs, 300);
    }

    #[test]
    fn test_hostname_or_default() {
        let hostname = hostname_or_default();
        assert!(!hostname.is_empty());
    }
}
