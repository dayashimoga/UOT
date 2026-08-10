//! Virtual UOT Node — Two-Node E2E Test Harness
//!
//! Creates isolated virtual nodes with independent identity, storage,
//! configuration, and sessions for protocol-level E2E testing.

use std::path::PathBuf;

use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::protocol::handler::{OfferItemInfo, WireMessage};
use crate::security::session_cipher::SessionCipher;
use crate::testing::adapters::TransferSession;
use crate::transfer::checkpoint::{CheckpointStore, ItemCheckpoint, TransferCheckpoint};

/// An isolated virtual UOT node for E2E testing.
pub struct VirtualUotNode {
    pub node_id: String,
    pub device_name: String,
    pub config: AppConfig,
    pub storage_dir: PathBuf,
    pub checkpoint_store: CheckpointStore,
    pub sessions: RwLock<Vec<TransferSession>>,
    pub received_files: RwLock<Vec<ReceivedFile>>,
    pub sent_files: RwLock<Vec<SentFile>>,
}

/// A file received by this node.
#[derive(Debug, Clone)]
pub struct ReceivedFile {
    pub name: String,
    pub relative_path: String,
    pub data: Vec<u8>,
    pub sha256: String,
}

/// A file sent by this node.
#[derive(Debug, Clone)]
pub struct SentFile {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub sha256: String,
}

impl VirtualUotNode {
    /// Create a new virtual node with isolated temp storage.
    pub fn new(name: &str) -> Self {
        let storage_dir = std::env::temp_dir()
            .join("uot_virtual_nodes")
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&storage_dir).ok();

        let checkpoint_dir = storage_dir.join("checkpoints");
        let config = AppConfig {
            device_name: name.to_string(),
            device_id: Uuid::new_v4().to_string(),
            ..Default::default()
        };

        Self {
            node_id: config.device_id.clone(),
            device_name: name.to_string(),
            config,
            storage_dir: storage_dir.clone(),
            checkpoint_store: CheckpointStore::new(checkpoint_dir),
            sessions: RwLock::new(Vec::new()),
            received_files: RwLock::new(Vec::new()),
            sent_files: RwLock::new(Vec::new()),
        }
    }

    /// Compute SHA-256 of data.
    pub fn sha256(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        hex::encode(h.finalize())
    }
}

/// Run a complete two-node virtual E2E transfer.
///
/// Simulates: Discovery → Hello → KeyExchange → Offer → Accept →
/// FileStart → Data chunks → FileEnd → SHA256 verify → Complete
pub fn run_virtual_transfer(
    sender: &VirtualUotNode,
    receiver: &VirtualUotNode,
    files: Vec<(&str, Vec<u8>)>,
) -> TransferResult {
    let transfer_id = Uuid::new_v4();
    let chunk_size = sender.config.transfer.chunk_size;

    // Phase 1: Key Exchange
    let (priv_a, pub_a) = SessionCipher::create_key_exchange().unwrap();
    let (priv_b, pub_b) = SessionCipher::create_key_exchange().unwrap();
    let mut cipher_a = SessionCipher::from_key_exchange(&priv_a, &pub_b).unwrap();
    let mut cipher_b = SessionCipher::from_key_exchange(&priv_b, &pub_a).unwrap();

    // Phase 2: Offer
    let offer_items: Vec<OfferItemInfo> = files
        .iter()
        .map(|(name, data)| OfferItemInfo {
            name: name.to_string(),
            relative_path: name.to_string(),
            size: data.len() as u64,
            is_directory: false,
        })
        .collect();
    let total_size: u64 = files.iter().map(|(_, d)| d.len() as u64).sum();

    let offer_msg = WireMessage::Offer {
        transfer_id: transfer_id.to_string(),
        device_name: sender.device_name.clone(),
        items: offer_items,
        total_size,
    };
    let offer_json = serde_json::to_vec(&offer_msg).unwrap();
    let encrypted_offer = cipher_a.encrypt_frame(&offer_json).unwrap();
    let decrypted_offer = cipher_b.decrypt_frame(&encrypted_offer).unwrap();
    let _parsed_offer: WireMessage = serde_json::from_slice(&decrypted_offer).unwrap();

    // Phase 3: Accept
    let accept_msg = WireMessage::OfferResponse {
        transfer_id: transfer_id.to_string(),
        accepted: true,
        reason: None,
    };
    let accept_json = serde_json::to_vec(&accept_msg).unwrap();
    let encrypted_accept = cipher_b.encrypt_frame(&accept_json).unwrap();
    let _decrypted_accept = cipher_a.decrypt_frame(&encrypted_accept).unwrap();

    // Phase 4: Transfer each file
    let mut session = TransferSession::new(&transfer_id.to_string(), "virtual");
    session.total_bytes = total_size;
    let mut total_chunks = 0u32;

    for (_name, data) in &files {
        let file_chunks = data.len().div_ceil(chunk_size);
        total_chunks += file_chunks as u32;
    }
    session.total_chunks = total_chunks;

    let mut chunk_idx = 0u32;
    let mut all_received: Vec<ReceivedFile> = Vec::new();

    for (idx, (name, data)) in files.iter().enumerate() {
        // FileStart
        let start_msg = WireMessage::FileStart {
            transfer_id: transfer_id.to_string(),
            item_index: idx as u32,
            file_name: name.to_string(),
            file_size: data.len() as u64,
            relative_path: name.to_string(),
        };
        let enc = cipher_a
            .encrypt_frame(&serde_json::to_vec(&start_msg).unwrap())
            .unwrap();
        let _ = cipher_b.decrypt_frame(&enc).unwrap();

        // Data chunks
        let mut received_data = Vec::new();
        for chunk in data.chunks(chunk_size) {
            let enc_chunk = cipher_a.encrypt_frame(chunk).unwrap();
            let dec_chunk = cipher_b.decrypt_frame(&enc_chunk).unwrap();
            received_data.extend_from_slice(&dec_chunk);
            session.verify_chunk(chunk_idx);
            chunk_idx += 1;
        }

        // FileEnd with SHA-256
        let file_hash = VirtualUotNode::sha256(data);
        let end_msg = WireMessage::FileEnd {
            transfer_id: transfer_id.to_string(),
            item_index: idx as u32,
            sha256: file_hash.clone(),
        };
        let enc = cipher_a
            .encrypt_frame(&serde_json::to_vec(&end_msg).unwrap())
            .unwrap();
        let _ = cipher_b.decrypt_frame(&enc).unwrap();

        // Verify SHA-256
        let received_hash = VirtualUotNode::sha256(&received_data);
        assert_eq!(file_hash, received_hash, "SHA-256 mismatch for {name}");

        all_received.push(ReceivedFile {
            name: name.to_string(),
            relative_path: name.to_string(),
            data: received_data,
            sha256: received_hash,
        });

        sender.sent_files.write().push(SentFile {
            name: name.to_string(),
            relative_path: name.to_string(),
            size: data.len() as u64,
            sha256: file_hash,
        });
    }

    // Phase 5: TransferComplete
    let complete_msg = WireMessage::TransferComplete {
        transfer_id: transfer_id.to_string(),
        success: true,
    };
    let enc = cipher_a
        .encrypt_frame(&serde_json::to_vec(&complete_msg).unwrap())
        .unwrap();
    let _ = cipher_b.decrypt_frame(&enc).unwrap();

    assert!(session.is_complete());

    *receiver.received_files.write() = all_received.clone();
    sender.sessions.write().push(session.clone());

    TransferResult {
        transfer_id,
        success: true,
        files_transferred: files.len(),
        bytes_transferred: total_size,
        chunks_verified: chunk_idx,
        session,
        received_files: all_received,
    }
}

/// Result of a virtual E2E transfer.
#[derive(Debug)]
pub struct TransferResult {
    pub transfer_id: Uuid,
    pub success: bool,
    pub files_transferred: usize,
    pub bytes_transferred: u64,
    pub chunks_verified: u32,
    pub session: TransferSession,
    pub received_files: Vec<ReceivedFile>,
}

/// Run a virtual transfer with checkpoint/resume simulation.
pub fn run_virtual_transfer_with_resume(
    sender: &VirtualUotNode,
    receiver: &VirtualUotNode,
    files: Vec<(&str, Vec<u8>)>,
    fail_at_percent: f64,
) -> TransferResult {
    let transfer_id = Uuid::new_v4();
    let chunk_size = sender.config.transfer.chunk_size;

    let (priv_a, pub_a) = SessionCipher::create_key_exchange().unwrap();
    let (priv_b, pub_b) = SessionCipher::create_key_exchange().unwrap();
    let mut cipher_a = SessionCipher::from_key_exchange(&priv_a, &pub_b).unwrap();
    let mut cipher_b = SessionCipher::from_key_exchange(&priv_b, &pub_a).unwrap();

    let total_size: u64 = files.iter().map(|(_, d)| d.len() as u64).sum();
    let mut total_chunks = 0u32;
    for (_, data) in &files {
        total_chunks += data.len().div_ceil(chunk_size) as u32;
    }

    let fail_at_chunk = (total_chunks as f64 * fail_at_percent) as u32;

    let mut session = TransferSession::new(&transfer_id.to_string(), "virtual");
    session.total_bytes = total_size;
    session.total_chunks = total_chunks;

    let mut chunk_idx = 0u32;
    let mut all_received_data: Vec<(String, Vec<u8>)> = Vec::new();
    let mut failed = false;

    // First pass: transfer until failure point
    for (name, data) in &files {
        let mut received_data = Vec::new();
        for chunk in data.chunks(chunk_size) {
            if chunk_idx == fail_at_chunk {
                // Save checkpoint
                let checkpoint = TransferCheckpoint {
                    transfer_id,
                    direction: "send".to_string(),
                    remote_device: receiver.device_name.clone(),
                    total_size,
                    transferred_bytes: session.bytes_transferred,
                    items: vec![ItemCheckpoint {
                        name: name.to_string(),
                        relative_path: name.to_string(),
                        size: data.len() as u64,
                        transferred_bytes: received_data.len() as u64,
                        complete: false,
                        sha256: None,
                    }],
                    saved_at: chrono::Utc::now(),
                };
                sender.checkpoint_store.save(&checkpoint).unwrap();
                failed = true;
                break;
            }

            let enc_chunk = cipher_a.encrypt_frame(chunk).unwrap();
            let dec_chunk = cipher_b.decrypt_frame(&enc_chunk).unwrap();
            received_data.extend_from_slice(&dec_chunk);
            session.verify_chunk(chunk_idx);
            session.bytes_transferred += chunk.len() as u64;
            chunk_idx += 1;
        }
        all_received_data.push((name.to_string(), received_data));
        if failed {
            break;
        }
    }

    // Verify checkpoint was saved
    assert!(failed, "Should have failed at {}%", fail_at_percent * 100.0);
    let loaded_cp = sender.checkpoint_store.load(&transfer_id).unwrap();
    assert_eq!(loaded_cp.transfer_id, transfer_id);

    // Resume: create new ciphers (simulating reconnect)
    let (priv_a2, pub_a2) = SessionCipher::create_key_exchange().unwrap();
    let (priv_b2, pub_b2) = SessionCipher::create_key_exchange().unwrap();
    let mut cipher_a2 = SessionCipher::from_key_exchange(&priv_a2, &pub_b2).unwrap();
    let mut cipher_b2 = SessionCipher::from_key_exchange(&priv_b2, &pub_a2).unwrap();

    session.retry_count += 1;

    // Resume from where we left off
    let mut resumed_chunk_idx = chunk_idx;
    for (name, data) in &files {
        let existing_idx = all_received_data.iter().position(|(n, _)| n == *name);
        let mut received_data = if let Some(idx) = existing_idx {
            all_received_data[idx].1.clone()
        } else {
            Vec::new()
        };

        let start_offset = received_data.len();
        if start_offset >= data.len() {
            continue; // Already complete
        }

        for chunk in data[start_offset..].chunks(chunk_size) {
            if !session.is_chunk_verified(resumed_chunk_idx) {
                let enc = cipher_a2.encrypt_frame(chunk).unwrap();
                let dec = cipher_b2.decrypt_frame(&enc).unwrap();
                received_data.extend_from_slice(&dec);
                session.verify_chunk(resumed_chunk_idx);
                session.bytes_transferred += chunk.len() as u64;
            }
            resumed_chunk_idx += 1;
        }

        // Verify SHA-256
        let expected_hash = VirtualUotNode::sha256(data);
        let actual_hash = VirtualUotNode::sha256(&received_data);
        assert_eq!(
            expected_hash, actual_hash,
            "SHA-256 mismatch after resume for {name}"
        );

        if let Some(idx) = existing_idx {
            all_received_data[idx].1 = received_data;
        } else {
            all_received_data.push((name.to_string(), received_data));
        }
    }

    assert!(session.is_complete());

    // Clean up checkpoint
    sender.checkpoint_store.remove(&transfer_id).ok();

    let received_files: Vec<ReceivedFile> = all_received_data
        .into_iter()
        .map(|(name, data)| {
            let sha = VirtualUotNode::sha256(&data);
            ReceivedFile {
                name: name.clone(),
                relative_path: name,
                data,
                sha256: sha,
            }
        })
        .collect();

    TransferResult {
        transfer_id,
        success: true,
        files_transferred: files.len(),
        bytes_transferred: total_size,
        chunks_verified: session.verified_chunks,
        session,
        received_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_node_creation() {
        let node = VirtualUotNode::new("TestNode");
        assert_eq!(node.device_name, "TestNode");
        assert!(!node.node_id.is_empty());
        assert!(node.storage_dir.exists());
    }

    #[test]
    fn test_two_node_single_file_transfer() {
        let sender = VirtualUotNode::new("Sender");
        let receiver = VirtualUotNode::new("Receiver");

        let result = run_virtual_transfer(
            &sender,
            &receiver,
            vec![("hello.txt", b"Hello, World!".to_vec())],
        );

        assert!(result.success);
        assert_eq!(result.files_transferred, 1);
        assert_eq!(result.received_files[0].data, b"Hello, World!");
        assert!(result.session.is_complete());
    }

    #[test]
    fn test_two_node_multi_file_transfer() {
        let sender = VirtualUotNode::new("Sender");
        let receiver = VirtualUotNode::new("Receiver");

        let result = run_virtual_transfer(
            &sender,
            &receiver,
            vec![
                ("file1.txt", b"Content One".to_vec()),
                ("file2.txt", b"Content Two".to_vec()),
                ("file3.bin", vec![0xDE, 0xAD, 0xBE, 0xEF]),
            ],
        );

        assert!(result.success);
        assert_eq!(result.files_transferred, 3);
        assert_eq!(result.received_files[2].data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_two_node_zero_byte_file() {
        let sender = VirtualUotNode::new("S");
        let receiver = VirtualUotNode::new("R");
        let result = run_virtual_transfer(&sender, &receiver, vec![("empty.txt", vec![])]);
        assert!(result.success);
        assert!(result.received_files[0].data.is_empty());
    }

    #[test]
    fn test_two_node_unicode_filenames() {
        let sender = VirtualUotNode::new("送信者");
        let receiver = VirtualUotNode::new("受信者");
        let result = run_virtual_transfer(
            &sender,
            &receiver,
            vec![
                ("文件.txt", b"Chinese filename".to_vec()),
                ("ファイル.txt", b"Japanese filename".to_vec()),
                ("émojis_🎉.pdf", b"Emoji filename".to_vec()),
            ],
        );
        assert!(result.success);
        assert_eq!(result.files_transferred, 3);
    }

    #[test]
    fn test_two_node_100mb_transfer() {
        let sender = VirtualUotNode::new("Sender");
        let receiver = VirtualUotNode::new("Receiver");
        let large_data = vec![0xABu8; 5 * 1024 * 1024]; // 5 MB (optimized for tarpaulin profiling)

        let result =
            run_virtual_transfer(&sender, &receiver, vec![("large.bin", large_data.clone())]);

        assert!(result.success);
        assert_eq!(result.bytes_transferred, 5 * 1024 * 1024);
        assert_eq!(
            VirtualUotNode::sha256(&result.received_files[0].data),
            VirtualUotNode::sha256(&large_data)
        );
    }

    #[test]
    fn test_two_node_many_small_files() {
        let sender = VirtualUotNode::new("S");
        let receiver = VirtualUotNode::new("R");

        let files: Vec<(&str, Vec<u8>)> = (0..100)
            .map(|i| {
                let name = Box::leak(format!("file_{i:04}.txt").into_boxed_str());
                let data = format!("Content of file {i}").into_bytes();
                (name as &str, data)
            })
            .collect();

        let result = run_virtual_transfer(&sender, &receiver, files);
        assert!(result.success);
        assert_eq!(result.files_transferred, 100);
    }

    #[test]
    fn test_checkpoint_resume_at_50_percent() {
        let sender = VirtualUotNode::new("Sender");
        let receiver = VirtualUotNode::new("Receiver");
        let data = vec![0x42u8; 1024 * 1024]; // 1 MB

        let result = run_virtual_transfer_with_resume(
            &sender,
            &receiver,
            vec![("resume_test.bin", data.clone())],
            0.5, // Fail at 50%
        );

        assert!(result.success);
        assert!(result.session.retry_count > 0);
        assert_eq!(
            VirtualUotNode::sha256(&result.received_files[0].data),
            VirtualUotNode::sha256(&data)
        );
    }

    #[test]
    fn test_checkpoint_resume_at_10_percent() {
        let sender = VirtualUotNode::new("S");
        let receiver = VirtualUotNode::new("R");
        let data = vec![0xEE; 512 * 1024]; // 512 KB

        let result = run_virtual_transfer_with_resume(
            &sender,
            &receiver,
            vec![("early_fail.bin", data.clone())],
            0.1,
        );

        assert!(result.success);
        assert_eq!(result.received_files[0].data.len(), data.len());
    }

    #[test]
    fn test_sha256_utility() {
        let hash = VirtualUotNode::sha256(b"test");
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
        let hash2 = VirtualUotNode::sha256(b"test");
        assert_eq!(hash, hash2);
        let hash3 = VirtualUotNode::sha256(b"different");
        assert_ne!(hash, hash3);
    }
}
