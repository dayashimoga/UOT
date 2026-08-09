//! Real End-to-End Transfer Integration Tests
//!
//! These tests exercise the actual TCP transport layer with encrypted frames,
//! protocol message exchange, and SHA-256 file integrity verification.
//! Unlike the engine-level test, these bypass mDNS (which may fail in CI)
//! and directly test: connect → key exchange → offer → accept → transfer → verify.

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use rust_lib_uot_app::protocol::handler::{OfferItemInfo, WireMessage};
use rust_lib_uot_app::security::session_cipher::SessionCipher;
use rust_lib_uot_app::transfer::engine as transfer_engine;
use rust_lib_uot_app::transport::tcp::{
    connect, Frame, FrameType, TcpConnection, TcpTransportListener,
};
use tempfile::tempdir;

/// Helper: compute SHA-256 of a file as hex string.
async fn sha256_file(path: &std::path::Path) -> String {
    transfer_engine::compute_sha256(path).await.unwrap()
}

/// Helper: read a file into bytes.
fn read_file(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
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

// ─── Test 1: Real encrypted file transfer over TCP loopback ───

#[tokio::test]
async fn test_real_encrypted_file_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Setup directories
    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // Create a 256KB test file with deterministic content
    let test_content: Vec<u8> = (0..262_144u32).map(|i| (i % 251) as u8).collect();
    let file_path = create_test_file(sender_dir.path(), "test_data.bin", &test_content);
    let sender_hash = sha256_file(&file_path).await;

    // Start TCP listener (receiver side)
    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Spawn receiver task
    let recv_dir = receiver_dir.path().to_path_buf();
    let expected_hash = sender_hash.clone();
    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.expect("accept connection");
        let conn = TcpConnection::new(stream).unwrap();

        // Receive KeyExchange from sender
        let frame = conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Control);
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        let their_public = match msg {
            WireMessage::KeyExchange { public_key } => public_key,
            _ => panic!("Expected KeyExchange, got: {:?}", msg),
        };

        // Generate our keypair and send back
        let (our_private, our_public) = SessionCipher::create_key_exchange().unwrap();
        let reply = WireMessage::KeyExchange {
            public_key: our_public,
        };
        let reply_bytes = serde_json::to_vec(&reply).unwrap();
        conn.send_frame(Frame::control(&reply_bytes)).await.unwrap();

        // Derive session cipher
        let mut cipher = SessionCipher::from_key_exchange(&our_private, &their_public).unwrap();

        // Receive Offer
        let frame = conn.recv_frame().await.unwrap();
        let offer: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        let (transfer_id, file_name, file_size) = match offer {
            WireMessage::Offer {
                transfer_id,
                items,
                total_size,
                ..
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(total_size, 262_144);
                (transfer_id, items[0].name.clone(), items[0].size)
            }
            _ => panic!("Expected Offer"),
        };

        // Receive FileStart
        let frame = conn.recv_frame().await.unwrap();
        let file_start: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match file_start {
            WireMessage::FileStart {
                file_name: name,
                file_size: size,
                ..
            } => {
                assert_eq!(name, file_name);
                assert_eq!(size, file_size);
            }
            _ => panic!("Expected FileStart"),
        }

        // Receive encrypted data chunks and write to file
        let output_path = recv_dir.join(&file_name);
        let mut total_received: u64 = 0;

        loop {
            let frame = conn.recv_frame().await.unwrap();
            match frame.frame_type {
                FrameType::Data => {
                    // Decrypt the frame
                    let decrypted = cipher.decrypt_frame(&frame.payload).unwrap();
                    assert!(decrypted.len() >= 16, "Decrypted frame too small");

                    let offset = u64::from_be_bytes(decrypted[..8].try_into().unwrap());
                    let _crc = u32::from_be_bytes(decrypted[8..12].try_into().unwrap());
                    let chunk_data = &decrypted[16..];

                    transfer_engine::write_chunk(&output_path, offset, chunk_data, _crc)
                        .await
                        .unwrap();
                    total_received += chunk_data.len() as u64;
                }
                FrameType::Control => {
                    let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
                    match msg {
                        WireMessage::FileEnd { sha256, .. } => {
                            assert_eq!(sha256, expected_hash, "SHA-256 mismatch in FileEnd");
                            break;
                        }
                        WireMessage::TransferComplete { success, .. } => {
                            assert!(success);
                            break;
                        }
                        _ => {} // Skip other control messages
                    }
                }
                _ => {}
            }
        }

        assert_eq!(total_received, file_size, "Received bytes mismatch");

        // Verify SHA-256 of the received file
        let received_hash = sha256_file(&output_path).await;
        assert_eq!(
            received_hash, expected_hash,
            "Received file SHA-256 does not match sender's hash"
        );

        // Verify byte-for-byte content
        let received_content = read_file(&output_path);
        assert_eq!(
            received_content.len(),
            262_144,
            "Received file size mismatch"
        );

        output_path
    });

    // Sender side: connect, key exchange, send file
    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    // Key exchange
    let (our_private, our_public) = SessionCipher::create_key_exchange().unwrap();
    let key_msg = WireMessage::KeyExchange {
        public_key: our_public,
    };
    let key_bytes = serde_json::to_vec(&key_msg).unwrap();
    conn.send_frame(Frame::control(&key_bytes)).await.unwrap();

    // Receive their public key
    let frame = conn.recv_frame().await.unwrap();
    let their_public = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
        WireMessage::KeyExchange { public_key } => public_key,
        _ => panic!("Expected KeyExchange reply"),
    };

    let mut cipher = SessionCipher::from_key_exchange(&our_private, &their_public).unwrap();

    // Send Offer
    let offer = WireMessage::Offer {
        transfer_id: "test-transfer-001".to_string(),
        device_name: "TestSender".to_string(),
        items: vec![OfferItemInfo {
            name: "test_data.bin".to_string(),
            relative_path: "test_data.bin".to_string(),
            size: 262_144,
            is_directory: false,
        }],
        total_size: 262_144,
    };
    let offer_bytes = serde_json::to_vec(&offer).unwrap();
    conn.send_frame(Frame::control(&offer_bytes)).await.unwrap();

    // Send FileStart
    let file_start = WireMessage::FileStart {
        transfer_id: "test-transfer-001".to_string(),
        item_index: 0,
        file_name: "test_data.bin".to_string(),
        file_size: 262_144,
        relative_path: "test_data.bin".to_string(),
    };
    let fs_bytes = serde_json::to_vec(&file_start).unwrap();
    conn.send_frame(Frame::control(&fs_bytes)).await.unwrap();

    // Send file chunks (64KB each)
    let chunk_size = 65_536;
    let mut offset: u64 = 0;
    while offset < 262_144 {
        let (chunk_data, crc) = transfer_engine::read_chunk(&file_path, offset, chunk_size)
            .await
            .unwrap();
        let chunk_len = chunk_data.len() as u64;

        // Build chunk frame: offset(8) + crc(4) + reserved(4) + data
        let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
        chunk_frame.extend_from_slice(&offset.to_be_bytes());
        chunk_frame.extend_from_slice(&crc.to_be_bytes());
        chunk_frame.extend_from_slice(&[0u8; 4]);
        chunk_frame.extend_from_slice(&chunk_data);

        // Encrypt and send
        let encrypted = cipher.encrypt_frame(&chunk_frame).unwrap();
        conn.send(Frame::data(encrypted)).await.unwrap();

        offset += chunk_len;
    }

    // Send FileEnd with SHA-256
    let file_end = WireMessage::FileEnd {
        transfer_id: "test-transfer-001".to_string(),
        item_index: 0,
        sha256: sender_hash.clone(),
    };
    let fe_bytes = serde_json::to_vec(&file_end).unwrap();
    conn.send_frame(Frame::control(&fe_bytes)).await.unwrap();

    // Wait for receiver to verify
    let received_path = receiver_handle.await.unwrap();
    assert!(received_path.exists(), "Received file must exist");

    listener.stop();
}

// ─── Test 2: Zero-byte file transfer ───

#[tokio::test]
async fn test_zero_byte_file_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    let file_path = create_test_file(sender_dir.path(), "empty.txt", &[]);
    let sender_hash = sha256_file(&file_path).await;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let expected_hash = sender_hash.clone();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive FileStart
        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::FileStart {
                file_name,
                file_size,
                ..
            } => {
                assert_eq!(file_name, "empty.txt");
                assert_eq!(file_size, 0);
                // Create empty file
                let out = recv_dir.join("empty.txt");
                std::fs::File::create(&out).unwrap();
            }
            _ => panic!("Expected FileStart"),
        }

        // Receive FileEnd (no data chunks for zero-byte file)
        let frame = conn.recv_frame().await.unwrap();
        let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
        match msg {
            WireMessage::FileEnd { sha256, .. } => {
                assert_eq!(sha256, expected_hash);
            }
            _ => panic!("Expected FileEnd"),
        }

        // Verify
        let out_path = recv_dir.join("empty.txt");
        let hash = sha256_file(&out_path).await;
        assert_eq!(hash, expected_hash);
        assert_eq!(std::fs::metadata(&out_path).unwrap().len(), 0);
    });

    // Sender: send zero-byte file
    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let fs = WireMessage::FileStart {
        transfer_id: "zero-byte-001".to_string(),
        item_index: 0,
        file_name: "empty.txt".to_string(),
        file_size: 0,
        relative_path: "empty.txt".to_string(),
    };
    let fs_bytes = serde_json::to_vec(&fs).unwrap();
    conn.send_frame(Frame::control(&fs_bytes)).await.unwrap();

    // No data chunks — file is zero bytes

    let fe = WireMessage::FileEnd {
        transfer_id: "zero-byte-001".to_string(),
        item_index: 0,
        sha256: sender_hash,
    };
    let fe_bytes = serde_json::to_vec(&fe).unwrap();
    conn.send_frame(Frame::control(&fe_bytes)).await.unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}

// ─── Test 3: Unicode filename transfer ───

#[tokio::test]
async fn test_unicode_filename_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    let unicode_name = "文件_données_ファイル.txt";
    let content = b"Unicode filename test content 12345";
    let file_path = create_test_file(sender_dir.path(), unicode_name, content);
    let sender_hash = sha256_file(&file_path).await;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let expected_hash = sender_hash.clone();
    let fname = unicode_name.to_string();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive FileStart
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

        // Receive data chunk (small enough for one chunk)
        let frame = conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Data);
        assert!(frame.payload.len() >= 16);
        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
        let chunk = &frame.payload[16..];
        let out = recv_dir.join(&fname);
        transfer_engine::write_chunk(&out, offset, chunk, crc)
            .await
            .unwrap();

        // Receive FileEnd
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

    // Sender
    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let fs = WireMessage::FileStart {
        transfer_id: "unicode-001".to_string(),
        item_index: 0,
        file_name: unicode_name.to_string(),
        file_size: content.len() as u64,
        relative_path: unicode_name.to_string(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
        .await
        .unwrap();

    // Send data (unencrypted for this test — tests protocol, not encryption)
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
        transfer_id: "unicode-001".to_string(),
        item_index: 0,
        sha256: sender_hash,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fe).unwrap()))
        .await
        .unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}

// ─── Test 4: Session cipher tamper-in-transit detection ───

#[tokio::test]
async fn test_encrypted_transfer_tamper_detected() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Receive KeyExchange
        let frame = conn.recv_frame().await.unwrap();
        let their_public = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::KeyExchange { public_key } => public_key,
            _ => panic!("Expected KeyExchange"),
        };

        // Reply with our key
        let (our_private, our_public) = SessionCipher::create_key_exchange().unwrap();
        let reply = WireMessage::KeyExchange {
            public_key: our_public,
        };
        conn.send_frame(Frame::control(&serde_json::to_vec(&reply).unwrap()))
            .await
            .unwrap();

        let mut cipher = SessionCipher::from_key_exchange(&our_private, &their_public).unwrap();

        // Receive tampered data frame — decryption should fail
        let frame = conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Data);
        let result = cipher.decrypt_frame(&frame.payload);
        assert!(result.is_err(), "Tampered frame must fail decryption");
    });

    // Sender: send a tampered encrypted frame
    let client_stream = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client_stream).unwrap();

    let (our_private, our_public) = SessionCipher::create_key_exchange().unwrap();
    let key_msg = WireMessage::KeyExchange {
        public_key: our_public,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&key_msg).unwrap()))
        .await
        .unwrap();

    let frame = conn.recv_frame().await.unwrap();
    let their_public = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
        WireMessage::KeyExchange { public_key } => public_key,
        _ => panic!("Expected KeyExchange reply"),
    };

    let mut cipher = SessionCipher::from_key_exchange(&our_private, &their_public).unwrap();

    // Encrypt legitimate data
    let data = b"This is sensitive transfer data";
    let mut encrypted = cipher.encrypt_frame(data).unwrap();

    // TAMPER: flip a byte in the ciphertext
    if encrypted.len() > 15 {
        encrypted[15] ^= 0xFF;
    }

    // Send tampered frame
    conn.send(Frame::data(encrypted)).await.unwrap();

    receiver_handle.await.unwrap();
    listener.stop();
}
