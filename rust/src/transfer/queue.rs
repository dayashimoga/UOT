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
    active_count: usize,
}

impl TransferQueueManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Vec::new(),
            max_concurrent,
            active_count: 0,
        }
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Check if a new transfer can start (under concurrent limit).
    pub fn can_start(&self) -> bool {
        self.active_count < self.max_concurrent
    }

    /// Mark a transfer as actively running (increments active count).
    pub fn mark_started(&mut self) {
        self.active_count += 1;
    }

    /// Mark a transfer as completed (decrements active count).
    pub fn mark_completed(&mut self) {
        self.active_count = self.active_count.saturating_sub(1);
    }

    /// Get the current number of active transfers.
    pub fn active_count(&self) -> usize {
        self.active_count
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::types::{TransferDirection, TransferStatus};

    fn make_record(name: &str) -> TransferRecord {
        TransferRecord {
            transfer_id: Uuid::new_v4(),
            batch_id: Some("batch_queue_test".to_string()),
            remote_device: name.to_string(),
            direction: TransferDirection::Send,
            status: TransferStatus::Pending,
            total_size: 1000,
            transferred_bytes: 0,
            verified_bytes: 0,
            transport: Some("Wi-Fi".to_string()),
            retry_count: 0,
            resume_offset: 0,
            items: vec![],
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    #[test]
    fn test_queue_concurrency_enforcement() {
        let mut qm = TransferQueueManager::new(2);
        assert!(qm.can_start());

        qm.mark_started();
        assert!(qm.can_start()); // 1 < 2

        qm.mark_started();
        assert!(!qm.can_start()); // 2 == 2

        qm.mark_completed();
        assert!(qm.can_start()); // 1 < 2
    }

    #[test]
    fn test_queue_priority_ordering() {
        let mut qm = TransferQueueManager::new(4);

        qm.push(make_record("low"), Priority::Low);
        qm.push(make_record("urgent"), Priority::Urgent);
        qm.push(make_record("normal"), Priority::Normal);

        let first = qm.pop_next().unwrap();
        assert_eq!(first.priority, Priority::Urgent);

        let second = qm.pop_next().unwrap();
        assert_eq!(second.priority, Priority::Normal);

        let third = qm.pop_next().unwrap();
        assert_eq!(third.priority, Priority::Low);
    }

    #[test]
    fn test_queue_manager_methods_coverage() {
        let mut qm = TransferQueueManager::new(3);
        assert_eq!(qm.max_concurrent(), 3);

        qm.set_max_concurrent(5);
        assert_eq!(qm.max_concurrent(), 5);

        let rec = make_record("remove_test");
        let id = rec.transfer_id;
        qm.push(rec, Priority::High);
        assert_eq!(qm.items().len(), 1);
        assert_eq!(qm.active_count(), 0);

        let removed = qm.remove(&id);
        assert!(removed.is_some());
        assert!(qm.remove(&id).is_none());

        assert!(qm.pop_next().is_none());
    }
}
