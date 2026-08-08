//! Transfer Queue Priority & Scheduling Manager
//!
//! Handles transfer item priority ordering, concurrent transfer limits, and pause/resume scheduling.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::transfer::types::TransferRecord;

/// Priority level for queued transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

/// Queued transfer entry with priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTransfer {
    pub record: TransferRecord,
    pub priority: Priority,
    pub queued_at: chrono::DateTime<chrono::Utc>,
}

/// Transfer queue manager.
#[derive(Debug, Default)]
pub struct TransferQueueManager {
    queue: Vec<QueuedTransfer>,
    max_concurrent: usize,
}

impl TransferQueueManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Vec::new(),
            max_concurrent,
        }
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Enqueue a new transfer.
    pub fn push(&mut self, record: TransferRecord, priority: Priority) {
        let queued = QueuedTransfer {
            record,
            priority,
            queued_at: chrono::Utc::now(),
        };
        self.queue.push(queued);
        self.sort();
    }

    /// Sort queue by priority (highest first) and queued timestamp.
    pub fn sort(&mut self) {
        self.queue.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.queued_at.cmp(&b.queued_at))
        });
    }

    /// Remove a transfer by ID.
    pub fn remove(&mut self, transfer_id: &Uuid) -> Option<QueuedTransfer> {
        if let Some(idx) = self
            .queue
            .iter()
            .position(|q| &q.record.transfer_id == transfer_id)
        {
            Some(self.queue.remove(idx))
        } else {
            None
        }
    }

    /// Set max concurrent transfers limit.
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = max;
    }

    /// Pop the next highest priority queued transfer.
    pub fn pop_next(&mut self) -> Option<QueuedTransfer> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    /// Get current queued items.
    pub fn items(&self) -> &[QueuedTransfer] {
        &self.queue
    }
}
