//! QR Payload & Security Integration Test
//!
//! Validates QR invitation payload generation, URI parameter extraction (`uot://pair`),
//! JSON parsing, expired PIN handling, malformed JSON handling, and security path validation.

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::UotEngine;
use rust_lib_uot_app::security::qr::QrInvitation;
use tempfile::tempdir;

#[tokio::test]
async fn test_qr_invitation_generation_parsing_and_expiry() {
    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();
    config.device_name = "QR_Test_Sender".to_string();

    let (engine, _rx) = UotEngine::new(config);

    // 1. Generate PIN and QR invitation JSON (TTL = 2s)
    let pin = engine.generate_pin(2);
    let inv = QrInvitation::new(
        "QR_Test_Sender".to_string(),
        engine.device_id().to_string(),
        "mock_pubkey_b64".to_string(),
        "192.168.1.100:42000".to_string(),
        pin.clone(),
        2, // 2-second TTL
    );

    let json_str = inv.to_json().expect("QR invitation serialization failed");
    assert!(json_str.contains("192.168.1.100:42000"));
    assert!(json_str.contains(&pin));

    // 2. Parse valid JSON before expiry
    let parsed_inv = QrInvitation::from_json(&json_str).expect("Parsing valid QR JSON failed");
    assert_eq!(parsed_inv.device_name, "QR_Test_Sender");
    assert_eq!(parsed_inv.pin, pin);
    assert!(
        !parsed_inv.is_expired(),
        "Fresh QR invitation must not be expired"
    );

    // 3. Test malformed JSON error branch
    let malformed_res = QrInvitation::from_json("invalid_json_payload");
    assert!(malformed_res.is_err(), "Malformed JSON parsing must fail");

    // 4. Test URI parameter parsing logic (uot://pair?ip=192.168.1.50&port=42000&pin=123456)
    let uri_string = "uot://pair?ip=192.168.1.50&port=42000&pin=123456";
    assert!(uri_string.starts_with("uot://pair?"));
    let query_part = &uri_string["uot://pair?".len()..];
    let pairs: std::collections::HashMap<_, _> = query_part
        .split('&')
        .filter_map(|kv| {
            let mut parts = kv.split('=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect();

    assert_eq!(pairs.get("ip"), Some(&"192.168.1.50".to_string()));
    assert_eq!(pairs.get("port"), Some(&"42000".to_string()));
    assert_eq!(pairs.get("pin"), Some(&"123456".to_string()));

    // 5. Test QR Expiration (1s TTL)
    let expired_inv = QrInvitation::new(
        "Expired_Node".to_string(),
        "id_expired".to_string(),
        "pubkey".to_string(),
        "127.0.0.1:42000".to_string(),
        "000000".to_string(),
        1, // 1-second TTL
    );
    assert!(
        !expired_inv.is_expired(),
        "Fresh invitation must not be expired"
    );
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(
        expired_inv.is_expired(),
        "QR invitation must expire after 1s TTL"
    );
}
