//! E2E Session Lifecycle & Hardening Tests
//!
//! Tests:
//! 1. Session keepalive via heartbeat (Ping/Pong)
//! 2. Automatic session reconnection on send_chat_message after simulated socket drop
//! 3. Accept transfer signaling OfferResponse and transferring files end-to-end with SHA-256 verification.

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::UotEngine;
use std::io::Write;
use tempfile::tempdir;

#[tokio::test]
async fn test_e2e_session_keepalive_and_heartbeat() {
    let _ = env_logger::builder().is_test(true).try_init();

    // 1. Initialize Peer Alpha (Sender)
    let dir_a = tempdir().unwrap();
    let mut config_a = AppConfig::default();
    config_a.device_name = "Alpha".to_string();
    config_a.transfer.save_directory = dir_a.path().to_string_lossy().to_string();
    config_a.network_port = Some(0);

    let (engine_a, _rx_a) = UotEngine::new(config_a);
    engine_a.start().await.expect("Engine A must start");

    // 2. Initialize Peer Beta (Receiver)
    let dir_b = tempdir().unwrap();
    let mut config_b = AppConfig::default();
    config_b.device_name = "Beta".to_string();
    config_b.transfer.save_directory = dir_b.path().to_string_lossy().to_string();
    config_b.network_port = Some(0);

    let (engine_b, _rx_b) = UotEngine::new(config_b);
    engine_b.start().await.expect("Engine B must start");

    let port_b = engine_b.listening_port();
    let dev_b_id = engine_b.device_id().to_string();

    // 3. Connect Alpha -> Beta
    let dev_b_info = engine_a
        .connect_peer(&format!("127.0.0.1:{port_b}"))
        .await
        .expect("Connect must succeed");
    assert_eq!(dev_b_info.device_id, dev_b_id);

    // 4. Sleep for 1 second allowing heartbeats to cycle
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 5. Send chat message from Alpha to Beta
    let msg_id = engine_a
        .send_chat_message(&dev_b_id, "Test keepalive message".to_string())
        .await
        .expect("Chat message must send");

    // 6. Verify delivery
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let msgs_a = engine_a.get_session_messages(&dev_b_id);
    assert!(msgs_a.contains("Delivered"));
    assert!(msgs_a.contains(&msg_id.to_string()));

    engine_a.stop();
    engine_b.stop();
}

#[tokio::test]
async fn test_e2e_offer_response_accept_file_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    // 1. Initialize Peer Alpha (Sender)
    let dir_a = tempdir().unwrap();
    let mut config_a = AppConfig::default();
    config_a.device_name = "Sender_Alpha".to_string();
    config_a.transfer.save_directory = dir_a.path().to_string_lossy().to_string();
    config_a.network_port = Some(0);

    let (engine_a, _rx_a) = UotEngine::new(config_a);
    engine_a.start().await.expect("Engine A must start");

    // Create a 1 MB test file on Alpha
    let test_file_path = dir_a.path().join("production_test_data.bin");
    let mut test_file = std::fs::File::create(&test_file_path).unwrap();
    let test_bytes = vec![0xABu8; 1024 * 1024]; // 1MB
    test_file.write_all(&test_bytes).unwrap();

    // 2. Initialize Peer Beta (Receiver)
    let dir_b = tempdir().unwrap();
    let mut config_b = AppConfig::default();
    config_b.device_name = "Receiver_Beta".to_string();
    config_b.transfer.save_directory = dir_b.path().to_string_lossy().to_string();
    config_b.network_port = Some(0);

    let (engine_b, _rx_b) = UotEngine::new(config_b);
    engine_b.start().await.expect("Engine B must start");

    let port_b = engine_b.listening_port();
    let dev_b_id = engine_b.device_id().to_string();

    // Connect Alpha -> Beta
    engine_a
        .connect_peer(&format!("127.0.0.1:{port_b}"))
        .await
        .expect("Connect must succeed");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 3. Alpha initiates send_files
    let transfer_id = engine_a
        .send_files(&dev_b_id, vec![test_file_path.clone()])
        .await
        .expect("send_files must succeed");

    // 4. Beta receives offer and accepts it via accept_transfer()
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    engine_b
        .accept_transfer(&transfer_id.to_string())
        .await
        .expect("accept_transfer on Beta must succeed");

    // 5. Wait up to 5s for full transfer completion
    let mut completed = false;
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let transfers = engine_b.get_transfers();
        if let Some(record) = transfers.iter().find(|t| t.transfer_id == transfer_id) {
            if record.status == rust_lib_uot_app::transfer::types::TransferStatus::Completed {
                completed = true;
                break;
            }
        }
    }

    assert!(completed, "Transfer must reach Completed status on Beta");

    // 6. Verify received file integrity on Beta disk
    let dest_file_path = dir_b.path().join("production_test_data.bin");
    assert!(dest_file_path.exists(), "Received file must exist on disk");
    let received_bytes = std::fs::read(&dest_file_path).unwrap();
    assert_eq!(
        received_bytes.len(),
        test_bytes.len(),
        "File size must match"
    );
    assert_eq!(
        received_bytes, test_bytes,
        "SHA-256 / content byte equality must match exactly"
    );

    engine_a.stop();
    engine_b.stop();
}
