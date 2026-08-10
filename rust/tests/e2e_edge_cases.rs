//! E2E Edge Cases, Recovery & Chaos Hardening Tests
//!
//! Covers: long filenames, large batch transfers, restart recovery via checkpoints,
//! corrupted checkpoint recovery, timeout/disconnect at every protocol phase.

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use rust_lib_uot_app::protocol::handler::{OfferItemInfo, WireMessage};
use rust_lib_uot_app::transfer::checkpoint::{CheckpointStore, ItemCheckpoint, TransferCheckpoint};
use rust_lib_uot_app::transfer::engine as transfer_engine;
use rust_lib_uot_app::transport::tcp::{
    connect, Frame, FrameType, TcpConnection, TcpTransportListener,
};
use tempfile::tempdir;

/// Helper: compute SHA-256 of a file as hex string.
async fn sha256_file(path: &std::path::Path) -> String {
    transfer_engine::compute_sha256(path).await.unwrap()
}

/// Helper: create a test file with specified content.
fn create_test_file(dir: &std::path::Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content).unwrap();
    path
}

// ═══════════════════════════════════════════════════════════════════
// E2E EDGE CASE: Long filename (255 chars)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_long_filename_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // 251-char name + ".txt" = 255 chars (max typical filesystem)
    let long_name = format!("{}.txt", "a".repeat(251));
    let content = b"long filename test payload";
    let file_path = create_test_file(sender_dir.path(), &long_name, content);
    let sender_hash = sha256_file(&file_path).await;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let expected_hash = sender_hash.clone();
    let fname = long_name.clone();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::FileStart {
                file_name,
                file_size,
                ..
            } => {
                assert_eq!(file_name, fname);
                assert_eq!(file_size, content.len() as u64);
            }
            _ => panic!("Expected FileStart"),
        }

        let frame = conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Data);
        let chunk = &frame.payload[16..];
        let out = recv_dir.join(&fname);
        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
        transfer_engine::write_chunk(&out, offset, chunk, crc)
            .await
            .unwrap();

        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::FileEnd { sha256, .. } => {
                let actual = sha256_file(&out).await;
                assert_eq!(actual, sha256);
                assert_eq!(sha256, expected_hash);
            }
            _ => panic!("Expected FileEnd"),
        }
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let fs = WireMessage::FileStart {
        transfer_id: "long-name-001".to_string(),
        item_index: 0,
        file_name: long_name.clone(),
        file_size: content.len() as u64,
        relative_path: long_name.clone(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
        .await
        .unwrap();

    let (chunk_data, crc) = transfer_engine::read_chunk(&file_path, 0, 1024 * 1024)
        .await
        .unwrap();
    let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
    chunk_frame.extend_from_slice(&0u64.to_be_bytes());
    chunk_frame.extend_from_slice(&crc.to_be_bytes());
    chunk_frame.extend_from_slice(&[0u8; 4]);
    chunk_frame.extend_from_slice(&chunk_data);
    conn.send(Frame::data(chunk_frame)).await.unwrap();

    let fe = WireMessage::FileEnd {
        transfer_id: "long-name-001".to_string(),
        item_index: 0,
        sha256: sender_hash,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fe).unwrap()))
        .await
        .unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// E2E EDGE CASE: Large batch (10 files) offer message
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_large_batch_offer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::Offer {
                items, total_size, ..
            } => {
                assert_eq!(items.len(), 10);
                assert_eq!(total_size, 10 * 1024);
                for (i, item) in items.iter().enumerate() {
                    assert_eq!(item.name, format!("file_{i:03}.bin"));
                    assert_eq!(item.size, 1024);
                }
            }
            _ => panic!("Expected Offer"),
        }
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let items: Vec<OfferItemInfo> = (0..10)
        .map(|i| OfferItemInfo {
            name: format!("file_{i:03}.bin"),
            relative_path: format!("batch/file_{i:03}.bin"),
            size: 1024,
            is_directory: false,
        })
        .collect();

    let offer = WireMessage::Offer {
        transfer_id: "batch-001".to_string(),
        device_name: "BatchSender".to_string(),
        items,
        total_size: 10 * 1024,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&offer).unwrap()))
        .await
        .unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// RECOVERY: Checkpoint save → simulate restart → resume from checkpoint
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_checkpoint_restart_recovery() {
    let dir = tempdir().unwrap();
    let store = CheckpointStore::new(dir.path());

    let tid = uuid::Uuid::new_v4();

    // Simulate partial transfer: 60% done
    let checkpoint = TransferCheckpoint {
        transfer_id: tid,
        direction: "receive".to_string(),
        remote_device: "SenderPhone".to_string(),
        total_size: 10_000_000,
        transferred_bytes: 6_000_000,
        items: vec![
            ItemCheckpoint {
                name: "video.mp4".to_string(),
                relative_path: "media/video.mp4".to_string(),
                size: 8_000_000,
                transferred_bytes: 6_000_000,
                complete: false,
                sha256: None,
            },
            ItemCheckpoint {
                name: "thumb.jpg".to_string(),
                relative_path: "media/thumb.jpg".to_string(),
                size: 2_000_000,
                transferred_bytes: 0,
                complete: false,
                sha256: None,
            },
        ],
        saved_at: chrono::Utc::now(),
    };
    store.save(&checkpoint).unwrap();

    // --- Simulate app restart: new CheckpointStore instance ---
    let store2 = CheckpointStore::new(dir.path());
    let incomplete = store2.list_incomplete();
    assert_eq!(incomplete.len(), 1, "Should find 1 incomplete transfer");

    let resumed = &incomplete[0];
    assert_eq!(resumed.transfer_id, tid);
    assert_eq!(resumed.transferred_bytes, 6_000_000);
    assert_eq!(resumed.items.len(), 2);
    assert_eq!(resumed.items[0].transferred_bytes, 6_000_000);
    assert!(!resumed.items[0].complete);
    assert_eq!(resumed.items[1].transferred_bytes, 0);

    // Simulate completing the transfer
    let mut completed = resumed.clone();
    completed.transferred_bytes = 10_000_000;
    completed.items[0].transferred_bytes = 8_000_000;
    completed.items[0].complete = true;
    completed.items[0].sha256 = Some("abc123def456".to_string());
    completed.items[1].transferred_bytes = 2_000_000;
    completed.items[1].complete = true;
    completed.items[1].sha256 = Some("789xyz".to_string());
    store2.save(&completed).unwrap();

    // Should no longer be incomplete
    let incomplete2 = store2.list_incomplete();
    assert_eq!(
        incomplete2.len(),
        0,
        "Completed transfer should not be listed"
    );

    // Cleanup
    store2.remove(&tid).unwrap();
    assert!(store2.load(&tid).is_err());
}

// ═══════════════════════════════════════════════════════════════════
// RECOVERY: Corrupted checkpoint file → graceful handling
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_corrupted_checkpoint_recovery() {
    let dir = tempdir().unwrap();
    let store = CheckpointStore::new(dir.path());

    // Write a valid checkpoint
    let tid = uuid::Uuid::new_v4();
    let cp = TransferCheckpoint {
        transfer_id: tid,
        direction: "send".to_string(),
        remote_device: "Dev".to_string(),
        total_size: 5000,
        transferred_bytes: 2500,
        items: vec![],
        saved_at: chrono::Utc::now(),
    };
    store.save(&cp).unwrap();

    // Corrupt the checkpoint file
    let filename = format!("{tid}.checkpoint.json");
    let path = dir.path().join(&filename);
    std::fs::write(&path, "{{{{not valid json!!!!").unwrap();

    // Load should fail gracefully (not panic)
    let result = store.load(&tid);
    assert!(result.is_err(), "Corrupted checkpoint should return error");

    // list_incomplete should skip corrupted files (not panic)
    let incomplete = store.list_incomplete();
    assert_eq!(
        incomplete.len(),
        0,
        "Corrupted checkpoint should be skipped"
    );

    // Write a second corrupted file with wrong extension — should be ignored
    std::fs::write(dir.path().join("random.txt"), "not a checkpoint").unwrap();
    let incomplete2 = store.list_incomplete();
    assert_eq!(incomplete2.len(), 0);

    // Write a truncated JSON
    let tid2 = uuid::Uuid::new_v4();
    let path2 = dir.path().join(format!("{tid2}.checkpoint.json"));
    std::fs::write(&path2, r#"{"transfer_id":""#).unwrap();
    let result2 = store.load(&tid2);
    assert!(result2.is_err());
}

// ═══════════════════════════════════════════════════════════════════
// CHAOS: Disconnect during key exchange (sender drops mid-handshake)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_disconnect_during_key_exchange() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Try to receive — sender will drop connection immediately
        let result = conn.recv_frame().await;
        // Should get an error (connection reset) or empty frame
        assert!(result.is_err(), "Should get error when sender disconnects");
    });

    // Sender: connect then immediately drop
    let client_stream = connect(addr).await.unwrap();
    drop(client_stream); // Simulate crash

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// CHAOS: Disconnect after offer (sender drops before sending data)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_disconnect_after_offer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive Offer
        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::Offer { items, .. } => {
                assert_eq!(items.len(), 1);
            }
            _ => panic!("Expected Offer"),
        }

        // Try to receive next message — sender crashes
        let result = conn.recv_frame().await;
        assert!(
            result.is_err(),
            "Should get error when sender crashes after offer"
        );
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let offer = WireMessage::Offer {
        transfer_id: "crash-001".to_string(),
        device_name: "CrashingSender".to_string(),
        items: vec![OfferItemInfo {
            name: "data.bin".to_string(),
            relative_path: "data.bin".to_string(),
            size: 1024,
            is_directory: false,
        }],
        total_size: 1024,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&offer).unwrap()))
        .await
        .unwrap();

    // Drop connection — simulate crash after sending offer
    drop(conn);

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// CHAOS: Disconnect mid-transfer (sender drops after partial data)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_disconnect_mid_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let content = vec![0xCDu8; 100_000]; // 100KB
    let file_path = create_test_file(sender_dir.path(), "partial.bin", &content);

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive FileStart
        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::FileStart { file_size, .. } => {
                assert_eq!(file_size, 100_000);
            }
            _ => panic!("Expected FileStart"),
        }

        // Receive first data chunk
        let frame = conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Data);

        // Try to receive more — sender crashes
        let result = conn.recv_frame().await;
        assert!(
            result.is_err(),
            "Should get error after sender drops mid-transfer"
        );
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let fs = WireMessage::FileStart {
        transfer_id: "mid-crash-001".to_string(),
        item_index: 0,
        file_name: "partial.bin".to_string(),
        file_size: 100_000,
        relative_path: "partial.bin".to_string(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
        .await
        .unwrap();

    // Send only first chunk (50KB of 100KB), then crash
    let (chunk_data, crc) = transfer_engine::read_chunk(&file_path, 0, 50_000)
        .await
        .unwrap();
    let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
    chunk_frame.extend_from_slice(&0u64.to_be_bytes());
    chunk_frame.extend_from_slice(&crc.to_be_bytes());
    chunk_frame.extend_from_slice(&[0u8; 4]);
    chunk_frame.extend_from_slice(&chunk_data);
    conn.send(Frame::data(chunk_frame)).await.unwrap();

    // Drop connection — simulate crash mid-transfer
    drop(conn);

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// CHAOS: Timeout simulation (receiver doesn't respond to offer)
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sender_timeout_on_unresponsive_receiver() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Receiver: accept but never respond
    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let _conn = TcpConnection::new(stream).unwrap();
        // Just hold the connection open, never send anything
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    // Send offer
    let offer = WireMessage::Offer {
        transfer_id: "timeout-001".to_string(),
        device_name: "TimeoutSender".to_string(),
        items: vec![OfferItemInfo {
            name: "file.bin".to_string(),
            relative_path: "file.bin".to_string(),
            size: 1024,
            is_directory: false,
        }],
        total_size: 1024,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&offer).unwrap()))
        .await
        .unwrap();

    // Try to read response with a 1-second timeout
    let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), conn.recv_frame()).await;

    assert!(
        result.is_err(),
        "Should timeout waiting for unresponsive receiver"
    );

    receiver_handle.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASE: Multiple files with subdirectory structure
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_nested_directory_file_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // Create nested files
    let file1 = create_test_file(sender_dir.path(), "docs/readme.md", b"# README\nHello");
    let file2 = create_test_file(sender_dir.path(), "docs/sub/nested.txt", b"nested content");
    let hash1 = sha256_file(&file1).await;
    let hash2 = sha256_file(&file2).await;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let exp_hash1 = hash1.clone();
    let exp_hash2 = hash2.clone();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive offer with 2 items
        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::Offer { items, .. } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].relative_path, "docs/readme.md");
                assert_eq!(items[1].relative_path, "docs/sub/nested.txt");
            }
            _ => panic!("Expected Offer"),
        }

        // Receive file 1
        let frame = conn.recv_frame().await.unwrap();
        match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::FileStart { file_name, .. } => assert_eq!(file_name, "readme.md"),
            _ => panic!("Expected FileStart for file 1"),
        }

        let frame = conn.recv_frame().await.unwrap();
        let out1 = recv_dir.join("docs/readme.md");
        std::fs::create_dir_all(out1.parent().unwrap()).ok();
        let chunk = &frame.payload[16..];
        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
        transfer_engine::write_chunk(&out1, offset, chunk, crc)
            .await
            .unwrap();

        let frame = conn.recv_frame().await.unwrap();
        match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::FileEnd { sha256, .. } => assert_eq!(sha256, exp_hash1),
            _ => panic!("Expected FileEnd for file 1"),
        }

        // Receive file 2
        let frame = conn.recv_frame().await.unwrap();
        match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::FileStart { file_name, .. } => assert_eq!(file_name, "nested.txt"),
            _ => panic!("Expected FileStart for file 2"),
        }

        let frame = conn.recv_frame().await.unwrap();
        let out2 = recv_dir.join("docs/sub/nested.txt");
        std::fs::create_dir_all(out2.parent().unwrap()).ok();
        let chunk = &frame.payload[16..];
        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
        transfer_engine::write_chunk(&out2, offset, chunk, crc)
            .await
            .unwrap();

        let frame = conn.recv_frame().await.unwrap();
        match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::FileEnd { sha256, .. } => assert_eq!(sha256, exp_hash2),
            _ => panic!("Expected FileEnd for file 2"),
        }

        // Verify
        assert_eq!(sha256_file(&out1).await, exp_hash1);
        assert_eq!(sha256_file(&out2).await, exp_hash2);
    });

    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    // Offer
    let offer = WireMessage::Offer {
        transfer_id: "nested-001".to_string(),
        device_name: "NestedSender".to_string(),
        items: vec![
            OfferItemInfo {
                name: "readme.md".to_string(),
                relative_path: "docs/readme.md".to_string(),
                size: 14,
                is_directory: false,
            },
            OfferItemInfo {
                name: "nested.txt".to_string(),
                relative_path: "docs/sub/nested.txt".to_string(),
                size: 14,
                is_directory: false,
            },
        ],
        total_size: 28,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&offer).unwrap()))
        .await
        .unwrap();

    // Send file 1
    let fs1 = WireMessage::FileStart {
        transfer_id: "nested-001".to_string(),
        item_index: 0,
        file_name: "readme.md".to_string(),
        file_size: 14,
        relative_path: "docs/readme.md".to_string(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs1).unwrap()))
        .await
        .unwrap();

    let (chunk, crc) = transfer_engine::read_chunk(&file1, 0, 1024 * 1024)
        .await
        .unwrap();
    let mut frame_data = Vec::with_capacity(16 + chunk.len());
    frame_data.extend_from_slice(&0u64.to_be_bytes());
    frame_data.extend_from_slice(&crc.to_be_bytes());
    frame_data.extend_from_slice(&[0u8; 4]);
    frame_data.extend_from_slice(&chunk);
    conn.send(Frame::data(frame_data)).await.unwrap();

    let fe1 = WireMessage::FileEnd {
        transfer_id: "nested-001".to_string(),
        item_index: 0,
        sha256: hash1,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fe1).unwrap()))
        .await
        .unwrap();

    // Send file 2
    let fs2 = WireMessage::FileStart {
        transfer_id: "nested-001".to_string(),
        item_index: 1,
        file_name: "nested.txt".to_string(),
        file_size: 14,
        relative_path: "docs/sub/nested.txt".to_string(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs2).unwrap()))
        .await
        .unwrap();

    let (chunk, crc) = transfer_engine::read_chunk(&file2, 0, 1024 * 1024)
        .await
        .unwrap();
    let mut frame_data = Vec::with_capacity(16 + chunk.len());
    frame_data.extend_from_slice(&0u64.to_be_bytes());
    frame_data.extend_from_slice(&crc.to_be_bytes());
    frame_data.extend_from_slice(&[0u8; 4]);
    frame_data.extend_from_slice(&chunk);
    conn.send(Frame::data(frame_data)).await.unwrap();

    let fe2 = WireMessage::FileEnd {
        transfer_id: "nested-001".to_string(),
        item_index: 1,
        sha256: hash2,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fe2).unwrap()))
        .await
        .unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}
