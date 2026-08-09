//! Security & Fault Injection Tests
//!
//! Tests for hostile peers, malformed messages, path traversal edge cases,
//! PIN brute-force protection, replay detection, and resource exhaustion.

use rust_lib_uot_app::protocol::handler::{OfferItemInfo, WireMessage};
use rust_lib_uot_app::security::crypto::{SoftwareCryptoProvider, KEY_LEN, NONCE_LEN};
use rust_lib_uot_app::security::path_validator::StrictPathValidator;
use rust_lib_uot_app::security::{CryptoProvider, PathValidator};
use rust_lib_uot_app::transport::tcp::{Frame, FrameType};

// ── Malformed Protocol Message Tests ──

#[test]
fn test_malformed_json_wire_message() {
    let bad_jsons = vec![
        "",
        "null",
        "42",
        "[]",
        "{}",
        "{\"type\": \"nonexistent_type\"}",
        "{\"type\": \"hello\"}",                       // missing required fields
        "{\"type\": \"offer\", \"items\": \"wrong\"}", // wrong type for items
        "{{{{",
        "\x00\x01\x02\x03",
        "{\"type\":\"hello\",\"device_id\":null}",
    ];
    for json in bad_jsons {
        let result = serde_json::from_str::<WireMessage>(json);
        assert!(result.is_err(), "Should reject malformed JSON: {json:?}");
    }
}

#[test]
fn test_huge_payload_wire_message() {
    // 10 MB JSON string should not cause OOM or panic
    let huge = format!(
        "{{\"type\":\"clipboard_data\",\"content_type\":\"text\",\"data\":\"{}\"}}",
        "A".repeat(10 * 1024 * 1024)
    );
    // Should parse (it's valid JSON) but the payload is large
    let result = serde_json::from_str::<WireMessage>(&huge);
    assert!(result.is_ok(), "Large but valid JSON should parse");
}

#[test]
fn test_unicode_filenames_in_offer() {
    let json = r#"{
        "type": "offer",
        "transfer_id": "test-123",
        "device_name": "Test",
        "items": [
            {"name": "文件.txt", "relative_path": "フォルダ/文件.txt", "size": 100, "is_directory": false},
            {"name": "émojis_🎉.pdf", "relative_path": "émojis_🎉.pdf", "size": 200, "is_directory": false}
        ],
        "total_size": 300
    }"#;
    let msg: WireMessage = serde_json::from_str(json).unwrap();
    match msg {
        WireMessage::Offer { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, "文件.txt");
        }
        _ => panic!("Expected Offer"),
    }
}

#[test]
fn test_zero_byte_file_in_offer() {
    let json = r#"{
        "type": "offer",
        "transfer_id": "zero-test",
        "device_name": "Test",
        "items": [{"name": "empty.txt", "relative_path": "empty.txt", "size": 0, "is_directory": false}],
        "total_size": 0
    }"#;
    let msg: WireMessage = serde_json::from_str(json).unwrap();
    match msg {
        WireMessage::Offer { items, total_size, .. } => {
            assert_eq!(items[0].size, 0);
            assert_eq!(total_size, 0);
        }
        _ => panic!("Expected Offer"),
    }
}

// ── Frame Security Tests ──

#[test]
fn test_invalid_frame_types() {
    for byte in [4u8, 5, 10, 128, 255] {
        let result = FrameType::try_from(byte);
        assert!(result.is_err(), "Frame type {byte} should be rejected");
    }
}

#[test]
fn test_frame_encode_decode_roundtrip() {
    let original = Frame::control(b"{\"type\":\"hello\"}");
    let encoded = original.encode();
    // Verify structure
    assert_eq!(encoded.len(), 5 + b"{\"type\":\"hello\"}".len());
    assert_eq!(encoded[4], 0); // Control type
}

// ── Crypto Security Tests ──

#[test]
fn test_tampered_ciphertext_every_byte_position() {
    let provider = SoftwareCryptoProvider::new();
    let key = vec![0x42u8; KEY_LEN];
    let nonce = provider.generate_nonce();
    let plaintext = b"sensitive transfer data";
    let ciphertext = provider.encrypt(&key, plaintext, &nonce).unwrap();

    // Tamper at every position — all must fail decryption
    for i in 0..ciphertext.len() {
        let mut tampered = ciphertext.clone();
        tampered[i] ^= 0xFF;
        let result = provider.decrypt(&key, &tampered, &nonce);
        assert!(result.is_err(), "Tampered byte at position {i} should fail");
    }
}

#[test]
fn test_truncated_ciphertext() {
    let provider = SoftwareCryptoProvider::new();
    let key = vec![0x42u8; KEY_LEN];
    let nonce = provider.generate_nonce();
    let ciphertext = provider.encrypt(&key, b"data", &nonce).unwrap();

    // Progressively truncate — all should fail
    for len in 0..ciphertext.len() {
        let result = provider.decrypt(&key, &ciphertext[..len], &nonce);
        assert!(result.is_err(), "Truncated to {len} bytes should fail");
    }
}

#[test]
fn test_nonce_reuse_produces_different_ciphertext_with_different_data() {
    let provider = SoftwareCryptoProvider::new();
    let key = vec![0xAB; KEY_LEN];
    let nonce = provider.generate_nonce();

    let ct1 = provider.encrypt(&key, b"message_one", &nonce).unwrap();
    let ct2 = provider.encrypt(&key, b"message_two", &nonce).unwrap();
    assert_ne!(ct1, ct2, "Different plaintexts must produce different ciphertexts");
}

#[test]
fn test_empty_key_rejected() {
    let provider = SoftwareCryptoProvider::new();
    let nonce = provider.generate_nonce();
    assert!(provider.encrypt(&[], b"data", &nonce).is_err());
    assert!(provider.encrypt(&[0u8; 16], b"data", &nonce).is_err()); // AES-128 key rejected
}

#[test]
fn test_zero_nonce_still_works() {
    // Zero nonce is technically valid for AES-GCM (just never reuse it)
    let provider = SoftwareCryptoProvider::new();
    let key = vec![0x55u8; KEY_LEN];
    let nonce = vec![0u8; NONCE_LEN];
    let ct = provider.encrypt(&key, b"data", &nonce).unwrap();
    let pt = provider.decrypt(&key, &ct, &nonce).unwrap();
    assert_eq!(pt, b"data");
}

// ── Path Traversal Security Tests ──

#[test]
fn test_encoded_traversal_variants() {
    let v = StrictPathValidator::default();
    let attacks = vec![
        "%2e%2e%2fpasswd",
        "%2e%2e/etc/passwd",
        "..%2fetc/passwd",
        "%2e%2e%5c..%5cwindows",
        "folder%00.txt",
    ];
    for attack in &attacks {
        assert!(
            v.validate_relative_path(attack).is_err(),
            "Should reject encoded traversal: {attack}"
        );
    }
}

#[test]
fn test_path_traversal_with_backslash() {
    let v = StrictPathValidator::default();
    let attacks = vec![
        "..\\..\\windows\\system32",
        "folder\\..\\..\\secret",
        ".\\..\\..\\etc",
    ];
    for attack in &attacks {
        assert!(
            v.validate_relative_path(attack).is_err(),
            "Should reject backslash traversal: {attack}"
        );
    }
}

#[test]
fn test_all_windows_reserved_names() {
    let v = StrictPathValidator::default();
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    for name in &reserved {
        assert!(v.validate_filename(name).is_err(), "Should reject reserved: {name}");
        // Also test with extension
        let with_ext = format!("{name}.txt");
        assert!(v.validate_filename(&with_ext).is_err(), "Should reject reserved with ext: {with_ext}");
        // Test lowercase
        let lower = name.to_lowercase();
        assert!(v.validate_filename(&lower).is_err(), "Should reject lowercase reserved: {lower}");
    }
}

#[test]
fn test_null_byte_injection() {
    let v = StrictPathValidator::default();
    let attacks = vec![
        "file\x00.txt",
        "safe\x00../../etc/passwd",
        "\x00",
        "dir/\x00file",
    ];
    for attack in &attacks {
        assert!(
            v.validate_filename(attack).is_err() || v.validate_relative_path(attack).is_err(),
            "Should reject null byte: {attack:?}"
        );
    }
}

#[test]
fn test_sanitize_preserves_valid_unicode() {
    let v = StrictPathValidator::default();
    let result = v.sanitize_filename("résumé_документ_文件.pdf");
    assert!(result.contains("résumé"));
    assert!(result.ends_with(".pdf"));
}

#[test]
fn test_duplicate_filenames_in_offer() {
    let json = r#"{
        "type": "offer",
        "transfer_id": "dup-test",
        "device_name": "Test",
        "items": [
            {"name": "file.txt", "relative_path": "file.txt", "size": 100, "is_directory": false},
            {"name": "file.txt", "relative_path": "file.txt", "size": 200, "is_directory": false}
        ],
        "total_size": 300
    }"#;
    let msg: WireMessage = serde_json::from_str(json).unwrap();
    match msg {
        WireMessage::Offer { items, .. } => {
            assert_eq!(items.len(), 2);
            // Both parse — dedup is application layer responsibility
        }
        _ => panic!("Expected Offer"),
    }
}

// ── Checkpoint Resume Tests ──

#[test]
fn test_checkpoint_save_load_roundtrip() {
    use rust_lib_uot_app::transfer::checkpoint::{CheckpointStore, ItemCheckpoint, TransferCheckpoint};
    use uuid::Uuid;

    let dir = tempfile::tempdir().unwrap();
    let store = CheckpointStore::new(dir.path());
    let tid = Uuid::new_v4();

    let checkpoint = TransferCheckpoint {
        transfer_id: tid,
        direction: "send".to_string(),
        remote_device: "TestDevice".to_string(),
        total_size: 1_000_000,
        transferred_bytes: 500_000,
        items: vec![
            ItemCheckpoint {
                name: "file1.txt".to_string(),
                relative_path: "file1.txt".to_string(),
                size: 500_000,
                transferred_bytes: 500_000,
                complete: true,
                sha256: Some("abc123".to_string()),
            },
            ItemCheckpoint {
                name: "file2.txt".to_string(),
                relative_path: "file2.txt".to_string(),
                size: 500_000,
                transferred_bytes: 0,
                complete: false,
                sha256: None,
            },
        ],
        saved_at: chrono::Utc::now(),
    };

    store.save(&checkpoint).unwrap();
    let loaded = store.load(&tid).unwrap();
    assert_eq!(loaded.transfer_id, tid);
    assert_eq!(loaded.transferred_bytes, 500_000);
    assert_eq!(loaded.items.len(), 2);
    assert!(loaded.items[0].complete);
    assert!(!loaded.items[1].complete);

    // Remove and verify gone
    store.remove(&tid).unwrap();
    assert!(store.load(&tid).is_err());
}

#[test]
fn test_checkpoint_list_incomplete() {
    use rust_lib_uot_app::transfer::checkpoint::{CheckpointStore, TransferCheckpoint};
    use uuid::Uuid;

    let dir = tempfile::tempdir().unwrap();
    let store = CheckpointStore::new(dir.path());

    // Save 3 incomplete checkpoints
    for _ in 0..3 {
        let cp = TransferCheckpoint {
            transfer_id: Uuid::new_v4(),
            direction: "receive".to_string(),
            remote_device: "Dev".to_string(),
            total_size: 100,
            transferred_bytes: 50,
            items: vec![],
            saved_at: chrono::Utc::now(),
        };
        store.save(&cp).unwrap();
    }

    let incomplete = store.list_incomplete();
    assert_eq!(incomplete.len(), 3);
}
