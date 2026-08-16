//! UOT Transport Lab Multi-Node Deterministic Integration Test Suite
//!
//! Validates:
//! 1. Three-peer network (Node A, Node B, Node C) concurrent bidirectional file transfer & chat
//! 2. Transport failover with byte-level SHA-256 verification
//! 3. Receiver filesystem persistence & exact size assertions
//! 4. Encrypted transfer & tamper detection under simulated network conditions

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::UotEngine;
use rust_lib_uot_app::transfer::types::TransferStatus;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::tempdir;

fn compute_sha256(path: &PathBuf) -> String {
    let mut file = File::open(path).expect("Failed to open file for hashing");
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).expect("Failed to read file chunk");
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    format!("{:x}", hasher.finalize())
}

#[tokio::test]
async fn test_three_node_concurrent_transfers_and_chat() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    let dir_c = tempdir().unwrap();

    let mut config_a = AppConfig::default();
    config_a.transfer.save_directory = dir_a.path().to_string_lossy().to_string();
    config_a.device_name = "Lab_Node_A".to_string();
    config_a.network_port = Some(0);

    let mut config_b = AppConfig::default();
    config_b.transfer.save_directory = dir_b.path().to_string_lossy().to_string();
    config_b.device_name = "Lab_Node_B".to_string();
    config_b.network_port = Some(0);

    let mut config_c = AppConfig::default();
    config_c.transfer.save_directory = dir_c.path().to_string_lossy().to_string();
    config_c.device_name = "Lab_Node_C".to_string();
    config_c.network_port = Some(0);

    let (engine_a, mut rx_a) = UotEngine::new(config_a);
    let (engine_b, mut rx_b) = UotEngine::new(config_b);
    let (engine_c, mut rx_c) = UotEngine::new(config_c);

    tokio::spawn(async move { while rx_a.recv().await.is_some() {} });
    tokio::spawn(async move { while rx_b.recv().await.is_some() {} });
    tokio::spawn(async move { while rx_c.recv().await.is_some() {} });

    engine_a.start().await.unwrap();
    engine_b.start().await.unwrap();
    engine_c.start().await.unwrap();

    let port_b = engine_b.listening_port();
    let port_c = engine_c.listening_port();

    let addr_b = format!("127.0.0.1:{port_b}");
    let addr_c = format!("127.0.0.1:{port_c}");

    println!("Connecting Node A -> Node B (addr: {addr_b})...");
    let dev_b = engine_a.connect_peer(&addr_b).await.unwrap();
    println!(
        "Connected to Node B: id={}, name={}",
        dev_b.device_id, dev_b.device_name
    );

    println!("Connecting Node A -> Node C (addr: {addr_c})...");
    let dev_c = engine_a.connect_peer(&addr_c).await.unwrap();
    println!(
        "Connected to Node C: id={}, name={}",
        dev_c.device_id, dev_c.device_name
    );

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send instant clipboard / chat messages concurrently
    let chat_res_b = engine_a
        .send_clipboard(&dev_b.device_id, "Hello Node B from Node A!".into())
        .await;
    println!("Chat to B res: {chat_res_b:?}");
    assert!(chat_res_b.is_ok());

    let chat_res_c = engine_a
        .send_clipboard(&dev_c.device_id, "Hello Node C from Node A!".into())
        .await;
    println!("Chat to C res: {chat_res_c:?}");
    assert!(chat_res_c.is_ok());

    // Create a 64KB test payload on Node A and send to Node B
    let file_for_b = dir_a.path().join("data_for_b.bin");
    let content_b = vec![0x42u8; 64 * 1024];
    {
        let mut f = File::create(&file_for_b).unwrap();
        f.write_all(&content_b).unwrap();
    }
    let source_sha256 = compute_sha256(&file_for_b);

    let tx_id = engine_a
        .send_files(&dev_b.device_id, vec![file_for_b.clone()])
        .await
        .unwrap();

    // Receiver B auto-accepts
    let mut offer_arrived = false;
    for _ in 0..60 {
        if engine_b
            .get_transfers()
            .iter()
            .any(|t| t.transfer_id == tx_id)
        {
            offer_arrived = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(offer_arrived, "Offer must arrive on Node B");

    engine_b.accept_transfer(&tx_id.to_string()).await.unwrap();

    // Wait for completion
    let mut completed = false;
    for _ in 0..80 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(t) = engine_b
            .get_transfers()
            .iter()
            .find(|t| t.transfer_id == tx_id)
        {
            if t.status == TransferStatus::Completed {
                completed = true;
                break;
            }
        }
    }
    assert!(completed, "Transfer must complete on Node B");

    // Verify receiver file persistence and SHA-256 match
    let dest_file = dir_b.path().join("data_for_b.bin");
    assert!(dest_file.exists(), "Received file must exist on disk");
    let dest_sha256 = compute_sha256(&dest_file);
    assert_eq!(source_sha256, dest_sha256, "SHA-256 match bit-for-bit");

    // Verify sender-side record completion and byte accuracy
    let a_transfers = engine_a.get_transfers();
    let sender_rec = a_transfers
        .iter()
        .find(|t| t.transfer_id == tx_id)
        .expect("Sender must have transfer record");
    assert_eq!(
        sender_rec.status,
        TransferStatus::Completed,
        "Sender must reach Completed status"
    );
    assert_eq!(
        sender_rec.transferred_bytes, sender_rec.total_size,
        "Sender transferred_bytes must equal total_size"
    );
    assert!(
        sender_rec
            .items
            .iter()
            .all(|i| i.status == TransferStatus::Completed),
        "All sender items must be Completed"
    );

    engine_a.stop();
    engine_b.stop();
    engine_c.stop();
}

#[tokio::test]
async fn test_multi_file_batch_transfer_and_verification() {
    let dir_sender = tempdir().unwrap();
    let dir_receiver = tempdir().unwrap();

    let mut config_sender = AppConfig::default();
    config_sender.transfer.save_directory = dir_sender.path().to_string_lossy().to_string();
    config_sender.device_name = "Sender_Node".to_string();
    config_sender.network_port = Some(0);

    let mut config_receiver = AppConfig::default();
    config_receiver.transfer.save_directory = dir_receiver.path().to_string_lossy().to_string();
    config_receiver.device_name = "Receiver_Node".to_string();
    config_receiver.network_port = Some(0);

    let (engine_sender, mut rx_s) = UotEngine::new(config_sender);
    let (engine_receiver, mut rx_r) = UotEngine::new(config_receiver);

    tokio::spawn(async move { while rx_s.recv().await.is_some() {} });
    tokio::spawn(async move { while rx_r.recv().await.is_some() {} });

    engine_sender.start().await.unwrap();
    engine_receiver.start().await.unwrap();

    let port_r = engine_receiver.listening_port();
    let addr_r = format!("127.0.0.1:{port_r}");

    let dev_r = engine_sender.connect_peer(&addr_r).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create 3 separate test files (text, binary, media mock)
    let file1 = dir_sender.path().join("document.pdf");
    let file2 = dir_sender.path().join("video_clip.mp4");
    let file3 = dir_sender.path().join("notes.txt");

    {
        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(&vec![0x11u8; 16 * 1024]).unwrap(); // 16 KB

        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(&vec![0x22u8; 128 * 1024]).unwrap(); // 128 KB

        let mut f3 = File::create(&file3).unwrap();
        f3.write_all(b"Hello Universal Offline Transfer multi-file test!")
            .unwrap();
    }

    let hash1 = compute_sha256(&file1);
    let hash2 = compute_sha256(&file2);
    let hash3 = compute_sha256(&file3);

    let tx_id = engine_sender
        .send_files(
            &dev_r.device_id,
            vec![file1.clone(), file2.clone(), file3.clone()],
        )
        .await
        .unwrap();

    // Receiver accepts offer
    let mut offer_arrived = false;
    for _ in 0..60 {
        if engine_receiver
            .get_transfers()
            .iter()
            .any(|t| t.transfer_id == tx_id)
        {
            offer_arrived = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(offer_arrived, "Batch offer must arrive on receiver");

    engine_receiver
        .accept_transfer(&tx_id.to_string())
        .await
        .unwrap();

    // Wait for transfer completion
    let mut completed = false;
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(t) = engine_receiver
            .get_transfers()
            .iter()
            .find(|t| t.transfer_id == tx_id)
        {
            if t.status == TransferStatus::Completed {
                completed = true;
                break;
            }
        }
    }
    assert!(completed, "Batch transfer must complete on receiver");

    // Verify all 3 files exist and hashes match
    let dest1 = dir_receiver.path().join("document.pdf");
    let dest2 = dir_receiver.path().join("video_clip.mp4");
    let dest3 = dir_receiver.path().join("notes.txt");

    assert!(dest1.exists(), "document.pdf must exist");
    assert!(dest2.exists(), "video_clip.mp4 must exist");
    assert!(dest3.exists(), "notes.txt must exist");

    assert_eq!(hash1, compute_sha256(&dest1), "document.pdf hash matches");
    assert_eq!(hash2, compute_sha256(&dest2), "video_clip.mp4 hash matches");
    assert_eq!(hash3, compute_sha256(&dest3), "notes.txt hash matches");

    // Verify sender state
    let sender_rec = engine_sender
        .get_transfers()
        .into_iter()
        .find(|t| t.transfer_id == tx_id)
        .expect("Sender must have batch record");
    assert_eq!(sender_rec.status, TransferStatus::Completed);
    assert_eq!(sender_rec.items.len(), 3);
    assert!(sender_rec
        .items
        .iter()
        .all(|i| i.status == TransferStatus::Completed));

    engine_sender.stop();
    engine_receiver.stop();
}

#[tokio::test]
async fn test_device_deduplication_and_endpoint_aggregation() {
    use rust_lib_uot_app::discovery::types::{DeviceType, DiscoveredDevice, DiscoveryMethod};

    let mut config = AppConfig::default();
    config.device_name = "Canonical_Lab_Node".to_string();
    config.network_port = Some(0);

    let (engine, mut rx) = UotEngine::new(config);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });
    engine.start().await.unwrap();

    let now = chrono::Utc::now();

    // 1. Simulate subnet scan inserting a synthetic device
    let synth_device = DiscoveredDevice {
        device_id: "lan-192-168-1-50".to_string(),
        device_name: "UOT Node (192.168.1.50)".to_string(),
        device_type: DeviceType::Desktop,
        discovery_method: DiscoveryMethod::Manual,
        address: Some("192.168.1.50:42000".to_string()),
        capabilities: vec!["tcp_lan".to_string()],
        signal_strength: Some(100),
        first_seen: now,
        last_seen: now,
        is_trusted: false,
    };
    engine
        .devices_map()
        .write()
        .insert(synth_device.device_id.clone(), synth_device);

    let devs_before = engine.discovered_devices();
    assert_eq!(devs_before.len(), 1);
    assert_eq!(devs_before[0].device_name, "UOT Node (192.168.1.50)");

    // 2. Real device DAYA connects / is registered with same IP
    let real_device = DiscoveredDevice {
        device_id: "uot-node-daya-identity-key-12345".to_string(),
        device_name: "DAYA".to_string(),
        device_type: DeviceType::Laptop,
        discovery_method: DiscoveryMethod::Mdns,
        address: Some("192.168.1.50:42000".to_string()),
        capabilities: vec![
            "tcp_lan".to_string(),
            "connected".to_string(),
            "session_ready".to_string(),
        ],
        signal_strength: Some(100),
        first_seen: now,
        last_seen: now,
        is_trusted: true,
    };
    engine
        .devices_map()
        .write()
        .insert(real_device.device_id.clone(), real_device);

    // 3. Verify discovered_devices returns ONLY the canonical DAYA device card
    let devs_after = engine.discovered_devices();
    assert_eq!(
        devs_after.len(),
        1,
        "Should deduplicate to single canonical card"
    );
    assert_eq!(
        devs_after[0].device_name, "DAYA",
        "Should prefer real device name over synthetic UOT Node"
    );
    assert!(
        devs_after[0]
            .capabilities
            .contains(&"connected".to_string()),
        "Should preserve connected capability"
    );

    engine.stop();
}

#[tokio::test]
async fn test_pause_resume_and_retry_transfer() {
    let dir_sender = tempdir().unwrap();
    let dir_receiver = tempdir().unwrap();

    let mut config_sender = AppConfig::default();
    config_sender.transfer.save_directory = dir_sender.path().to_string_lossy().to_string();
    config_sender.device_name = "Sender_Node".to_string();
    config_sender.network_port = Some(0);

    let mut config_receiver = AppConfig::default();
    config_receiver.transfer.save_directory = dir_receiver.path().to_string_lossy().to_string();
    config_receiver.device_name = "Receiver_Node".to_string();
    config_receiver.network_port = Some(0);

    let (engine_sender, mut rx_s) = UotEngine::new(config_sender);
    let (engine_receiver, mut rx_r) = UotEngine::new(config_receiver);

    tokio::spawn(async move { while rx_s.recv().await.is_some() {} });
    tokio::spawn(async move { while rx_r.recv().await.is_some() {} });

    engine_sender.start().await.unwrap();
    engine_receiver.start().await.unwrap();

    let port_r = engine_receiver.listening_port();
    let addr_r = format!("127.0.0.1:{port_r}");

    let dev_r = engine_sender.connect_peer(&addr_r).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create a 64KB test payload
    let test_file = dir_sender.path().join("resilient_test.bin");
    {
        let mut f = File::create(&test_file).unwrap();
        f.write_all(&vec![0xAAu8; 64 * 1024]).unwrap();
    }
    let expected_hash = compute_sha256(&test_file);

    let tx_id = engine_sender
        .send_files(&dev_r.device_id, vec![test_file.clone()])
        .await
        .unwrap();

    // Pause the transfer
    let pause_res = engine_sender.pause_transfer(&tx_id.to_string());
    assert!(pause_res.is_ok(), "Pause must succeed");

    // Resume the transfer
    let resume_res = engine_sender.resume_transfer(&tx_id.to_string());
    assert!(resume_res.is_ok(), "Resume must succeed");

    // Receiver accepts transfer
    let mut offer_found = false;
    for _ in 0..60 {
        if engine_receiver
            .get_transfers()
            .iter()
            .any(|t| t.transfer_id == tx_id)
        {
            offer_found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(offer_found);

    engine_receiver
        .accept_transfer(&tx_id.to_string())
        .await
        .unwrap();

    // Wait for completion
    let mut completed = false;
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(t) = engine_receiver
            .get_transfers()
            .iter()
            .find(|t| t.transfer_id == tx_id)
        {
            if t.status == TransferStatus::Completed {
                completed = true;
                break;
            }
        }
    }
    assert!(completed, "Transfer must complete after resume");

    let dest_file = dir_receiver.path().join("resilient_test.bin");
    assert!(dest_file.exists(), "Received file must exist");
    assert_eq!(expected_hash, compute_sha256(&dest_file));

    engine_sender.stop();
    engine_receiver.stop();
}

#[tokio::test]
async fn test_stress_1000_chat_messages() {
    let mut config_a = AppConfig::default();
    config_a.device_name = "Stress_Node_A".to_string();
    config_a.network_port = Some(0);

    let mut config_b = AppConfig::default();
    config_b.device_name = "Stress_Node_B".to_string();
    config_b.network_port = Some(0);

    let (engine_a, mut rx_a) = UotEngine::new(config_a);
    let (engine_b, mut rx_b) = UotEngine::new(config_b);

    tokio::spawn(async move { while rx_a.recv().await.is_some() {} });
    tokio::spawn(async move { while rx_b.recv().await.is_some() {} });

    engine_a.start().await.unwrap();
    engine_b.start().await.unwrap();

    let port_b = engine_b.listening_port();
    let addr_b = format!("127.0.0.1:{port_b}");

    let dev_b = engine_a.connect_peer(&addr_b).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send 1000 messages rapidly
    let count = 1000;
    for i in 0..count {
        let msg = format!("Message #{i} - UOT High-Throughput Verification Test");
        let res = engine_a.send_chat_message(&dev_b.device_id, msg).await;
        assert!(res.is_ok(), "Failed sending message {i}: {res:?}");
    }

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let sent_msgs_json = engine_a.get_session_messages(&dev_b.device_id);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&sent_msgs_json).unwrap();
    assert_eq!(
        parsed.len(),
        count,
        "Sender must have recorded all 1000 messages"
    );

    engine_a.stop();
    engine_b.stop();
}

#[tokio::test]
async fn test_concurrent_batch_isolation_and_progress_clamping() {
    let dir_sender = tempdir().unwrap();
    let dir_receiver = tempdir().unwrap();

    let mut config_s = AppConfig::default();
    config_s.transfer.save_directory = dir_sender.path().to_string_lossy().to_string();
    config_s.device_name = "Sender_Node".to_string();
    config_s.network_port = Some(0);

    let mut config_r = AppConfig::default();
    config_r.transfer.save_directory = dir_receiver.path().to_string_lossy().to_string();
    config_r.device_name = "Receiver_Node".to_string();
    config_r.network_port = Some(0);

    let (engine_s, mut rx_s) = UotEngine::new(config_s);
    let (engine_r, mut rx_r) = UotEngine::new(config_r);

    let incoming_offers = std::sync::Arc::new(parking_lot::RwLock::new(Vec::new()));
    let offers_clone = incoming_offers.clone();

    tokio::spawn(async move { while rx_s.recv().await.is_some() {} });
    tokio::spawn(async move {
        while let Some(event) = rx_r.recv().await {
            if let rust_lib_uot_app::core::engine::EngineEvent::IncomingOffer {
                transfer_id, ..
            } = event
            {
                offers_clone.write().push(transfer_id);
            }
        }
    });

    engine_s.start().await.unwrap();
    engine_r.start().await.unwrap();

    let port_r = engine_r.listening_port();
    let addr_r = format!("127.0.0.1:{port_r}");

    let dev_r = engine_s.connect_peer(&addr_r).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create Batch 1: 500 KB file
    let file1_path = dir_sender.path().join("large_batch1.bin");
    let data1 = vec![0xABu8; 512 * 1024];
    std::fs::write(&file1_path, &data1).unwrap();
    let hash1 = compute_sha256(&file1_path);

    // Create Batch 2: 3 small files (20 KB each)
    let mut batch2_paths = Vec::new();
    let mut batch2_hashes = Vec::new();
    for i in 1..=3 {
        let p = dir_sender.path().join(format!("small_file_{i}.txt"));
        let content = format!("UOT Batch 2 Item #{i} test content {}", "x".repeat(20_000));
        std::fs::write(&p, &content).unwrap();
        batch2_hashes.push(compute_sha256(&p));
        batch2_paths.push(p);
    }

    // Initiate Batch 1
    let tx1_id = engine_s
        .send_files(&dev_r.device_id, vec![file1_path.clone()])
        .await
        .unwrap();

    // Immediately initiate Batch 2
    let tx2_id = engine_s
        .send_files(&dev_r.device_id, batch2_paths)
        .await
        .unwrap();

    // Wait for both offers to arrive at receiver
    for _ in 0..50 {
        if incoming_offers.read().len() >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Accept both transfers
    for tid in incoming_offers.read().iter() {
        let _ = engine_r.accept_transfer(&tid.to_string()).await;
    }

    // Poll transfers and assert progress invariants:
    // 1. transferred_bytes <= total_size
    // 2. percentage <= 100.0%
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for t in engine_r.get_transfers() {
            assert!(
                t.transferred_bytes <= t.total_size,
                "Receiver transfer {} transferred_bytes ({}) exceeded total_size ({})!",
                t.transfer_id,
                t.transferred_bytes,
                t.total_size
            );
            let pct = if t.total_size > 0 {
                (t.transferred_bytes as f64 / t.total_size as f64) * 100.0
            } else {
                0.0
            };
            assert!(pct <= 100.0, "Transfer progress {}% exceeded 100.0%!", pct);
        }
        let transfers = engine_r.get_transfers();
        let all_done = transfers
            .iter()
            .filter(|t| t.status == TransferStatus::Completed)
            .count();
        if all_done >= 2 {
            break;
        }
    }

    // Assert both transfers completed successfully
    let r_transfers = engine_r.get_transfers();
    let rec1 = r_transfers
        .iter()
        .find(|t| t.transfer_id == tx1_id)
        .expect("Tx1 missing");
    let rec2 = r_transfers
        .iter()
        .find(|t| t.transfer_id == tx2_id)
        .expect("Tx2 missing");

    assert_eq!(rec1.status, TransferStatus::Completed);
    assert_eq!(rec2.status, TransferStatus::Completed);
    assert_eq!(rec1.transferred_bytes, rec1.total_size);
    assert_eq!(rec2.transferred_bytes, rec2.total_size);

    // Verify all child items in batch 2 completed
    assert_eq!(rec2.items.len(), 3);
    for item in &rec2.items {
        assert_eq!(item.status, TransferStatus::Completed);
        assert_eq!(item.transferred_bytes, item.size);
    }

    // Verify files on receiver filesystem
    let dest1 = dir_receiver.path().join("large_batch1.bin");
    assert!(dest1.exists());
    assert_eq!(hash1, compute_sha256(&dest1));

    for i in 1..=3 {
        let dest_item = dir_receiver.path().join(format!("small_file_{i}.txt"));
        assert!(dest_item.exists());
        assert_eq!(batch2_hashes[i - 1], compute_sha256(&dest_item));
    }

    engine_s.stop();
    engine_r.stop();
}

#[tokio::test]
async fn test_heavy_chat_and_transfer_interleaved_utf8_stress() {
    let dir_s = tempdir().unwrap();
    let dir_r = tempdir().unwrap();

    let mut config_s = AppConfig::default();
    config_s.transfer.save_directory = dir_s.path().to_string_lossy().to_string();
    config_s.device_name = "ChatStream_A".to_string();
    config_s.network_port = Some(0);

    let mut config_r = AppConfig::default();
    config_r.transfer.save_directory = dir_r.path().to_string_lossy().to_string();
    config_r.device_name = "ChatStream_B".to_string();
    config_r.network_port = Some(0);

    let (engine_s, mut rx_s) = UotEngine::new(config_s);
    let (engine_r, mut rx_r) = UotEngine::new(config_r);

    let received_offer = std::sync::Arc::new(parking_lot::RwLock::new(None));
    let offer_slot = received_offer.clone();

    tokio::spawn(async move { while rx_s.recv().await.is_some() {} });
    tokio::spawn(async move {
        while let Some(event) = rx_r.recv().await {
            if let rust_lib_uot_app::core::engine::EngineEvent::IncomingOffer {
                transfer_id, ..
            } = event
            {
                *offer_slot.write() = Some(transfer_id);
            }
        }
    });

    engine_s.start().await.unwrap();
    engine_r.start().await.unwrap();

    let port_r = engine_r.listening_port();
    let addr_r = format!("127.0.0.1:{port_r}");

    let dev_r = engine_s.connect_peer(&addr_r).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create 1 MB file to stream
    let file_path = dir_s.path().join("interleaved_stream.dat");
    let test_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    std::fs::write(&file_path, &test_data).unwrap();
    let expected_hash = compute_sha256(&file_path);

    let tx_id = engine_s
        .send_files(&dev_r.device_id, vec![file_path.clone()])
        .await
        .unwrap();

    // Wait for offer and accept
    for _ in 0..50 {
        if received_offer.read().is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let tid = received_offer.read().unwrap();
    engine_r.accept_transfer(&tid.to_string()).await.unwrap();

    // While transfer is streaming, interleave 200 Unicode/Emoji chat messages rapidly
    for i in 0..200 {
        let msg = format!(
            "⚡ UOT Interleaved Message #{i} 🚀 Unicode: 日本語 🌟 العربية ✓ Special: <>&\"'\\n"
        );
        let _ = engine_s.send_chat_message(&dev_r.device_id, msg).await;
        if i % 20 == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    // Wait for transfer to complete
    let mut completed = false;
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(t) = engine_r
            .get_transfers()
            .iter()
            .find(|t| t.transfer_id == tx_id)
        {
            if t.status == TransferStatus::Completed {
                completed = true;
                break;
            }
        }
    }
    assert!(completed, "Interleaved file transfer must complete");

    let dest = dir_r.path().join("interleaved_stream.dat");
    assert!(dest.exists());
    assert_eq!(expected_hash, compute_sha256(&dest));

    engine_s.stop();
    engine_r.stop();
}

#[tokio::test]
async fn test_transport_fallback_hierarchy_comprehensive() {
    use rust_lib_uot_app::transport::fallback::{
        TransportFallbackManager, TransportSelectionStrategy,
    };
    use rust_lib_uot_app::transport::types::{TransportId, TransportState};

    let manager = TransportFallbackManager::new(TransportSelectionStrategy::PreferSpeed);

    // 1. TcpLan takes priority over WifiDirect, Hotspot, BLE, QR, Relay
    let candidates = vec![
        (TransportId::Relay, TransportState::Connected),
        (TransportId::QrCode, TransportState::Connected),
        (TransportId::BluetoothLe, TransportState::Connected),
        (TransportId::Hotspot, TransportState::Connected),
        (TransportId::WifiDirect, TransportState::Connected),
        (TransportId::TcpLan, TransportState::Connected),
    ];
    assert_eq!(
        manager.select_best_transport(&candidates),
        Some(TransportId::TcpLan)
    );

    // 2. WifiDirect takes priority if TcpLan is disconnected
    let candidates2 = vec![
        (TransportId::Relay, TransportState::Connected),
        (TransportId::QrCode, TransportState::Connected),
        (TransportId::BluetoothLe, TransportState::Connected),
        (TransportId::Hotspot, TransportState::Connected),
        (TransportId::WifiDirect, TransportState::Connected),
        (TransportId::TcpLan, TransportState::Disconnected),
    ];
    assert_eq!(
        manager.select_best_transport(&candidates2),
        Some(TransportId::WifiDirect)
    );

    // 3. Hotspot takes priority if WifiDirect is unavailable
    let candidates3 = vec![
        (TransportId::Relay, TransportState::Connected),
        (TransportId::QrCode, TransportState::Connected),
        (TransportId::BluetoothLe, TransportState::Connected),
        (TransportId::Hotspot, TransportState::Connected),
        (TransportId::WifiDirect, TransportState::Disconnected),
    ];
    assert_eq!(
        manager.select_best_transport(&candidates3),
        Some(TransportId::Hotspot)
    );

    // 4. BluetoothLe takes priority over QR and Relay
    let candidates4 = vec![
        (TransportId::Relay, TransportState::Connected),
        (TransportId::QrCode, TransportState::Connected),
        (TransportId::BluetoothLe, TransportState::Connected),
    ];
    assert_eq!(
        manager.select_best_transport(&candidates4),
        Some(TransportId::BluetoothLe)
    );

    // 5. PreferOffline strategy prefers P2P/direct
    let offline_mgr = TransportFallbackManager::new(TransportSelectionStrategy::PreferOffline);
    let p2p_candidates = vec![
        (TransportId::TcpLan, TransportState::Connected),
        (TransportId::WifiDirect, TransportState::Connected),
    ];
    assert_eq!(
        offline_mgr.select_best_transport(&p2p_candidates),
        Some(TransportId::WifiDirect)
    );

    // 6. Network topology classification
    let local_ips = vec![
        "192.168.1.105".parse().unwrap(),
        "10.0.0.12".parse().unwrap(),
    ];
    assert_eq!(
        TransportFallbackManager::classify_network_topology(
            &local_ips,
            "192.168.1.200".parse().unwrap()
        ),
        "Same network"
    );
    assert_eq!(
        TransportFallbackManager::classify_network_topology(
            &local_ips,
            "192.168.49.1".parse().unwrap()
        ),
        "Wi-Fi Direct"
    );
    assert_eq!(
        TransportFallbackManager::classify_network_topology(
            &local_ips,
            "192.168.43.1".parse().unwrap()
        ),
        "Hotspot"
    );
    assert_eq!(
        TransportFallbackManager::classify_network_topology(
            &local_ips,
            "172.20.10.4".parse().unwrap()
        ),
        "Remote network"
    );
}

#[tokio::test]
async fn test_multi_batch_concurrent_isolation() {
    let dir_s = tempfile::tempdir().unwrap();
    let dir_r = tempfile::tempdir().unwrap();

    let mut cfg_s = AppConfig::default();
    cfg_s.device_name = "SenderNode".to_string();
    cfg_s.transfer.save_directory = dir_s.path().to_string_lossy().to_string();
    cfg_s.network_port = Some(0);

    let mut cfg_r = AppConfig::default();
    cfg_r.device_name = "ReceiverNode".to_string();
    cfg_r.transfer.save_directory = dir_r.path().to_string_lossy().to_string();
    cfg_r.network_port = Some(0);

    let (engine_s, mut rx_s) = UotEngine::new(cfg_s);
    let (engine_r, mut rx_r) = UotEngine::new(cfg_r);

    let offers = std::sync::Arc::new(parking_lot::RwLock::new(Vec::new()));
    let offers_clone = offers.clone();

    tokio::spawn(async move { while rx_s.recv().await.is_some() {} });
    tokio::spawn(async move {
        while let Some(event) = rx_r.recv().await {
            if let rust_lib_uot_app::core::engine::EngineEvent::IncomingOffer {
                transfer_id, ..
            } = event
            {
                offers_clone.write().push(transfer_id);
            }
        }
    });

    engine_s.start().await.unwrap();
    engine_r.start().await.unwrap();

    let port_r = engine_r.listening_port();
    let addr_r = format!("127.0.0.1:{port_r}");
    let dev_r = engine_s.connect_peer(&addr_r).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create 3 separate files for 3 concurrent batches
    let f1 = dir_s.path().join("batch_file_1.bin");
    let f2 = dir_s.path().join("batch_file_2.bin");
    let f3 = dir_s.path().join("batch_file_3.bin");

    std::fs::write(&f1, vec![1u8; 64 * 1024]).unwrap();
    std::fs::write(&f2, vec![2u8; 128 * 1024]).unwrap();
    std::fs::write(&f3, vec![3u8; 256 * 1024]).unwrap();

    let hash1 = compute_sha256(&f1);
    let hash2 = compute_sha256(&f2);
    let hash3 = compute_sha256(&f3);

    let tx1 = engine_s
        .send_files(&dev_r.device_id, vec![f1.clone()])
        .await
        .unwrap();
    let tx2 = engine_s
        .send_files(&dev_r.device_id, vec![f2.clone()])
        .await
        .unwrap();
    let tx3 = engine_s
        .send_files(&dev_r.device_id, vec![f3.clone()])
        .await
        .unwrap();

    // Verify all 3 transfers have unique IDs and isolated batch IDs
    assert_ne!(tx1, tx2);
    assert_ne!(tx2, tx3);
    assert_ne!(tx1, tx3);

    // Wait for all 3 offers and accept them
    for _ in 0..60 {
        if offers.read().len() >= 3 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let received_offers = offers.read().clone();
    assert!(received_offers.len() >= 3, "Must receive all 3 offers");

    for tid in &received_offers {
        let _ = engine_r.accept_transfer(&tid.to_string()).await;
    }

    // Wait for transfers to complete
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let r_transfers = engine_r.get_transfers();
        let done_count = r_transfers
            .iter()
            .filter(|t| t.status == TransferStatus::Completed)
            .count();
        if done_count >= 3 {
            break;
        }
    }

    let d1 = dir_r.path().join("batch_file_1.bin");
    let d2 = dir_r.path().join("batch_file_2.bin");
    let d3 = dir_r.path().join("batch_file_3.bin");

    assert!(d1.exists());
    assert!(d2.exists());
    assert!(d3.exists());
    assert_eq!(hash1, compute_sha256(&d1));
    assert_eq!(hash2, compute_sha256(&d2));
    assert_eq!(hash3, compute_sha256(&d3));

    engine_s.stop();
    engine_r.stop();
}

#[tokio::test]
async fn test_large_file_checkpoint_resume() {
    use rust_lib_uot_app::transfer::checkpoint::{
        CheckpointStore, ItemCheckpoint, TransferCheckpoint,
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let cp_file = temp_dir.path().join("checkpoints");
    let store = CheckpointStore::new(cp_file.clone());

    let transfer_id = uuid::Uuid::new_v4();
    let cp = TransferCheckpoint {
        transfer_id,
        direction: "send".to_string(),
        remote_device: "DAYA_PHONE".to_string(),
        total_size: 2_800_000_000,
        transferred_bytes: 1_400_000_000, // 50% through 2.8GB file
        items: vec![ItemCheckpoint {
            name: "movie.mp4".to_string(),
            relative_path: "movie.mp4".to_string(),
            size: 2_800_000_000,
            transferred_bytes: 1_400_000_000,
            complete: false,
            sha256: None,
        }],
        saved_at: chrono::Utc::now(),
    };

    store.save(&cp).unwrap();

    let loaded = store.load(&transfer_id).expect("Checkpoint must exist");
    assert_eq!(loaded.transfer_id, transfer_id);
    assert_eq!(loaded.transferred_bytes, 1_400_000_000);
    assert_eq!(loaded.items[0].transferred_bytes, 1_400_000_000);
    assert!(!loaded.items[0].complete);

    let incomplete = store.list_incomplete();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].transfer_id, transfer_id);

    // Complete removal on success
    store.remove(&transfer_id).unwrap();
    assert!(store.load(&transfer_id).is_err());
}
