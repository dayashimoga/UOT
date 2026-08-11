//! E2E Two-Peer Automated Workflow Integration Test
//!
//! Spawns two real `UotEngine` instances (Peer A and Peer B) on dynamic local ports.
//! Tests the full user workflow:
//! 1. Startup & TCP Listener binding
//! 2. Direct IP connection & Hello/HelloAck handshake
//! 3. Ping/Pong & SessionReady state confirmation
//! 4. Encrypted Instant Message delivery via ClipboardData
//! 5. Encrypted Chunked File Transfer (50 KB test payload)
//! 6. Receiver offer acceptance & disk persistence
//! 7. SHA-256 Hash Verification (Byte-for-byte exact match)

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::tempdir;

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::{PeerConnectionState, UotEngine};
use rust_lib_uot_app::transfer::types::TransferStatus;

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
async fn test_e2e_two_peer_full_transfer_workflow_with_sha256_verification() {
    // 1. Initialize Peer A
    let dir_a = tempdir().expect("Failed to create tempdir A");
    let mut config_a = AppConfig::default();
    config_a.transfer.save_directory = dir_a.path().to_string_lossy().to_string();
    config_a.device_name = "Automated_Peer_A".to_string();
    config_a.network_port = Some(0);

    let (engine_a, _rx_a) = UotEngine::new(config_a);
    engine_a.start().await.expect("Peer A failed to start");
    let port_a = engine_a.listening_port();
    assert!(port_a > 0, "Peer A listening port must be non-zero");

    // 2. Initialize Peer B
    let dir_b = tempdir().expect("Failed to create tempdir B");
    let mut config_b = AppConfig::default();
    config_b.transfer.save_directory = dir_b.path().to_string_lossy().to_string();
    config_b.device_name = "Automated_Peer_B".to_string();
    config_b.network_port = Some(0);

    let (engine_b, _rx_b) = UotEngine::new(config_b);
    engine_b.start().await.expect("Peer B failed to start");
    let port_b = engine_b.listening_port();
    assert!(port_b > 0, "Peer B listening port must be non-zero");

    // 3. Perform Direct IP Connect & Hello Handshake (A -> B)
    let addr_b_str = format!("127.0.0.1:{port_b}");
    let dev_b_info = engine_a
        .connect_peer(&addr_b_str)
        .await
        .expect("Peer A failed to connect to Peer B");

    assert_eq!(dev_b_info.device_name, "Automated_Peer_B");
    assert!(
        dev_b_info.capabilities.contains(&"connected".to_string()),
        "Peer B must have connected capability"
    );

    // Verify SessionReady state
    let state_a_to_b = engine_a.get_peer_state(&dev_b_info.device_id);
    assert_eq!(
        state_a_to_b,
        PeerConnectionState::SessionReady,
        "Connection state must be SessionReady"
    );

    // 4. Test Instant Message Delivery (A -> B)
    let test_msg = "MESSAGE:E2E Integration Test Payload 12345".to_string();
    let msg_res = engine_a
        .send_clipboard(&dev_b_info.device_id, test_msg.clone())
        .await;
    assert!(msg_res.is_ok(), "Instant message send must succeed");

    // 5. Prepare Test File Fixture on Peer A
    let source_file_path = dir_a.path().join("test_file_50k.bin");
    let mut dummy_data = Vec::with_capacity(50_000);
    for i in 0..50_000 {
        dummy_data.push((i % 256) as u8);
    }
    {
        let mut file_a = File::create(&source_file_path).expect("Failed to create test file");
        file_a
            .write_all(&dummy_data)
            .expect("Failed to write test data");
    }

    let source_sha256 = compute_sha256(&source_file_path);
    assert!(!source_sha256.is_empty());

    // 6. Initiate File Transfer (Peer A -> Peer B)
    let transfer_id = engine_a
        .send_files(&dev_b_info.device_id, vec![source_file_path.clone()])
        .await
        .expect("send_files must return transfer ID");

    // 7. Auto-accept on Peer B — wait for FileStart offer to arrive on Peer B
    let mut offer_arrived = false;
    for _ in 0..30 {
        if engine_b
            .get_transfers()
            .iter()
            .any(|t| t.transfer_id == transfer_id)
        {
            offer_arrived = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(offer_arrived, "File transfer offer must arrive on Peer B");

    let accept_res = engine_b.accept_transfer(&transfer_id.to_string()).await;
    assert!(accept_res.is_ok(), "Peer B accept_transfer must return ok");

    // 8. Wait for Transfer Completion on BOTH sender (A) and receiver (B)
    let mut completed_a = false;
    let mut completed_b = false;
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if !completed_a {
            if let Some(record) = engine_a
                .get_transfers()
                .iter()
                .find(|t| t.transfer_id == transfer_id)
            {
                if record.status == TransferStatus::Completed {
                    completed_a = true;
                }
            }
        }
        if !completed_b {
            if let Some(record) = engine_b
                .get_transfers()
                .iter()
                .find(|t| t.transfer_id == transfer_id)
            {
                if record.status == TransferStatus::Completed {
                    completed_b = true;
                }
            }
        }
        if completed_a && completed_b {
            break;
        }
    }
    assert!(completed_a, "Sender A transfer status must be Completed");
    assert!(completed_b, "Receiver B transfer status must be Completed");

    // 9. Verify Disk Persistence & SHA-256 Hash Equality on Peer B
    let dest_file_path = dir_b.path().join("test_file_50k.bin");
    assert!(
        dest_file_path.exists(),
        "Transferred file must exist in Peer B save directory"
    );

    let dest_sha256 = compute_sha256(&dest_file_path);
    assert_eq!(
        source_sha256, dest_sha256,
        "Source and Destination SHA-256 hashes MUST match byte-for-byte!"
    );

    // Cleanup
    engine_a.stop();
    engine_b.stop();
}
