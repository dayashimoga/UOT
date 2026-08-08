//! Transfer Engine Module
//!
//! Defines trait interfaces and types for the file transfer engine.
//! Handles chunking, resume, integrity verification, queue management,
//! and transfer lifecycle.
pub mod analytics;
pub mod clipboard;
pub mod engine;
pub mod history;
pub mod queue;
pub mod types;

use types::{TransferProgress, TransferRecord};

/// Trait for the transfer engine.
pub trait TransferEngine: Send + Sync {
    /// Get the current transfer queue.
    fn queue(&self) -> Vec<TransferRecord>;

    /// Get transfer history.
    fn history(&self, limit: usize) -> Vec<TransferRecord>;

    /// Get a specific transfer by ID.
    fn get_transfer(&self, transfer_id: &str) -> Option<TransferRecord>;

    /// Get the progress of a specific transfer.
    fn get_progress(&self, transfer_id: &str) -> Option<TransferProgress>;
}
