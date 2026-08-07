//! UOT Version and Build Information
//!
//! Provides compile-time version constants and a runtime-queryable
//! version info struct for the core engine.
use serde::{Deserialize, Serialize};

/// Semantic version of the UOT core engine.
pub const VERSION_MAJOR: u32 = 0;
pub const VERSION_MINOR: u32 = 1;
pub const VERSION_PATCH: u32 = 0;

/// Pre-release label (empty for stable releases).
pub const VERSION_PRE: &str = "alpha";

/// Protocol version — incremented on breaking protocol changes.
pub const PROTOCOL_VERSION: u32 = 1;

/// Full version string (e.g., "0.1.0-alpha").
pub fn version_string() -> String {
    if VERSION_PRE.is_empty() {
        format!("{VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}")
    } else {
        format!("{VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}-{VERSION_PRE}")
    }
}

/// Runtime-queryable build information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildInfo {
    /// Semantic version string.
    pub version: String,
    /// Protocol version number.
    pub protocol_version: u32,
    /// Rust compiler version used for build.
    pub rust_version: String,
    /// Target triple (e.g., "x86_64-pc-windows-msvc").
    pub target: String,
    /// Build profile ("debug" or "release").
    pub profile: String,
}

impl BuildInfo {
    /// Create a new `BuildInfo` with compile-time information.
    pub fn current() -> Self {
        Self {
            version: version_string(),
            protocol_version: PROTOCOL_VERSION,
            rust_version: env!("CARGO_PKG_RUST_VERSION", "unknown").to_string(),
            target: std::env::consts::ARCH.to_string(),
            profile: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            },
        }
    }
}

impl std::fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UOT v{} (protocol v{}, {}, {})",
            self.version, self.protocol_version, self.target, self.profile
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string_with_pre() {
        // Since VERSION_PRE is "alpha", we expect the pre-release label
        let vs = version_string();
        assert!(vs.contains("alpha"), "Expected pre-release label in '{vs}'");
        assert!(
            vs.starts_with("0.1.0"),
            "Expected version to start with '0.1.0', got '{vs}'"
        );
    }

    #[test]
    fn test_version_constants() {
        assert_eq!(VERSION_MAJOR, 0);
        assert_eq!(VERSION_MINOR, 1);
        assert_eq!(VERSION_PATCH, 0);
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    #[test]
    fn test_build_info_current() {
        let info = BuildInfo::current();
        assert!(!info.version.is_empty());
        assert_eq!(info.protocol_version, PROTOCOL_VERSION);
        assert_eq!(info.profile, "debug"); // tests run in debug mode
        assert!(!info.target.is_empty());
    }

    #[test]
    fn test_build_info_display() {
        let info = BuildInfo::current();
        let display = info.to_string();
        assert!(display.contains("UOT v"));
        assert!(display.contains("protocol v1"));
    }

    #[test]
    fn test_build_info_serialization() {
        let info = BuildInfo::current();
        let json = serde_json::to_string(&info).expect("Failed to serialize BuildInfo");
        let deserialized: BuildInfo =
            serde_json::from_str(&json).expect("Failed to deserialize BuildInfo");
        assert_eq!(info, deserialized);
    }
}
