//! UOT API Module
//!
//! Public API exposed to Flutter/Dart via flutter_rust_bridge.
//! This layer is intentionally thin — it delegates to internal modules.
pub mod engine_api;
pub mod init;
pub mod simple;
pub mod types;
