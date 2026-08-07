//! UOT API Module
//!
//! Public API exposed to Flutter/Dart via flutter_rust_bridge.
//! This layer is intentionally thin — it delegates to internal modules.
pub mod simple;
pub mod init;
pub mod types;
pub mod engine_api;
