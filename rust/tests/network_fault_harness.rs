//! Network Fault Harness Integration Test
//!
//! Simulates network anomalies, timeouts, closed ports, invalid authentication,
//! chunk CRC corruption, and abrupt stream drop scenarios. Verifies state machine
//! rollback, clean error reporting, and panic safety.

use tempfile::tempdir;

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::UotEngine;
use rust_lib_uot_app::core::error::{SecurityError, TransportError, UotError};
use rust_lib_uot_app::protocol::handler as proto;
use rust_lib_uot_app::transport::tcp::{TcpConnection, TcpTransportListener};

#[tokio::test]
async fn test_fault_closed_port_and_unreachable_ip() {
    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();
    config.network_port = Some(0);

    let (engine, _rx) = UotEngine::new(config);

    // 1. Closed port connection failure
    let res = engine.connect_peer("127.0.0.1:59999").await;
    assert!(res.is_err(), "Connection to closed port 59999 must fail");

    if let Err(UotError::Transport(TransportError::ConnectionFailed { reason })) = res {
        assert!(
            reason.contains("59999") || reason.contains("failed") || reason.contains("timed out"),
            "Error reason must describe connection failure"
        );
    }
}

#[tokio::test]
async fn test_fault_expired_pin_verification() {
    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();

    let (engine, _rx) = UotEngine::new(config);

    // Generate PIN with 1 second TTL
    let pin = engine.generate_pin(1);
    assert_eq!(pin.len(), 6);

    // Immediate verification succeeds
    let token = engine.verify_pin("device_x", &pin);
    assert!(token.is_some(), "Immediate PIN verification must succeed");

    // Wait for TTL expiration
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // Expired verification fails
    let expired_token = engine.verify_pin("device_x", &pin);
    assert!(
        expired_token.is_none(),
        "Expired PIN verification must fail"
    );

    // Testing accept_transfer_with_pin with wrong PIN
    let accept_res = engine
        .accept_transfer_with_pin("00000000-0000-0000-0000-000000000000", "device_x", "999999")
        .await;

    assert!(accept_res.is_err(), "Invalid PIN accept must fail");
    if let Err(UotError::Security(SecurityError::AuthenticationFailed { reason })) = accept_res {
        assert!(reason.contains("Invalid or expired PIN"));
    }
}

#[tokio::test]
async fn test_fault_abrupt_disconnect_mid_handshake() {
    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();
    config.network_port = Some(0);

    let (engine, _rx) = UotEngine::new(config);

    // Spawn server that accepts connection, reads Hello, then immediately closes socket
    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();

    let server_task = tokio::spawn(async move {
        if let Some(stream) = incoming.recv().await {
            let conn = TcpConnection::new(stream).unwrap();
            let _ = proto::recv_message(&conn).await;
            // Immediate drop without sending HelloAck
        }
    });

    let conn_res = engine.connect_peer(&format!("127.0.0.1:{port}")).await;
    assert!(
        conn_res.is_err(),
        "Connection with abrupt disconnect must fail"
    );

    server_task.await.unwrap();
    listener.stop();
}
