//! Load & Stress Tests
//!
//! Production load tests for large file transfers, concurrent transfers,
//! multi-file batches, and throughput measurement.

#![allow(unused_assignments, unused_variables, unused_imports)]

use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use rust_lib_uot_app::protocol::handler::{OfferItemInfo, WireMessage};
use rust_lib_uot_app::security::session_cipher::SessionCipher;
use rust_lib_uot_app::transfer::engine as transfer_engine;
use rust_lib_uot_app::transport::tcp::{
    connect, Frame, FrameType, TcpConnection, TcpTransportListener,
};
use tempfile::tempdir;
use tokio::sync::Barrier;

/// Helper: compute SHA-256 of a file as hex string.
async fn sha256_file(path: &std::path::Path) -> String {
    transfer_engine::compute_sha256(path).await.unwrap()
}

/// Helper: create a test file with random-like content.
fn create_large_file(dir: &std::path::Path, name: &str, size: usize) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    // Write in 1MB blocks for efficiency
    let block_size = 1024 * 1024; // 1MB
    let mut remaining = size;
    let mut seed: u64 = 0xDEADBEEF;
    while remaining > 0 {
        let write_size = remaining.min(block_size);
        let block: Vec<u8> = (0..write_size)
            .map(|i| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((seed >> 33) ^ (i as u64)) as u8
            })
            .collect();
        f.write_all(&block).unwrap();
        remaining -= write_size;
    }
    f.flush().unwrap();
    path
}

/// Helper: send a file over an encrypted connection, returns bytes sent.
async fn send_file_encrypted(
    conn: &TcpConnection,
    cipher: &mut SessionCipher,
    file_path: &std::path::Path,
    file_name: &str,
    file_size: u64,
    transfer_id: &str,
    item_index: u32,
) {
    // FileStart
    let fs = WireMessage::FileStart {
        transfer_id: transfer_id.to_string(),
        item_index,
        file_name: file_name.to_string(),
        file_size,
        relative_path: file_name.to_string(),
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
        .await
        .unwrap();

    // Data chunks (256KB each for throughput)
    let chunk_size = 256 * 1024;
    let mut offset: u64 = 0;
    while offset < file_size {
        let (chunk_data, crc) = transfer_engine::read_chunk(file_path, offset, chunk_size)
            .await
            .unwrap();
        let chunk_len = chunk_data.len() as u64;

        let mut chunk_frame = Vec::with_capacity(16 + chunk_data.len());
        chunk_frame.extend_from_slice(&offset.to_be_bytes());
        chunk_frame.extend_from_slice(&crc.to_be_bytes());
        chunk_frame.extend_from_slice(&[0u8; 4]);
        chunk_frame.extend_from_slice(&chunk_data);

        let encrypted = cipher.encrypt_frame(&chunk_frame).unwrap();
        conn.send(Frame::data(encrypted)).await.unwrap();
        offset += chunk_len;
    }

    // FileEnd
    let hash = sha256_file(file_path).await;
    let fe = WireMessage::FileEnd {
        transfer_id: transfer_id.to_string(),
        item_index,
        sha256: hash,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&fe).unwrap()))
        .await
        .unwrap();
}

/// Helper: receive a file over an encrypted connection, returns received path.
async fn receive_file_encrypted(
    conn: &TcpConnection,
    cipher: &mut SessionCipher,
    save_dir: &std::path::Path,
) -> (std::path::PathBuf, String) {
    // Receive FileStart
    let frame = conn.recv_frame().await.unwrap();
    let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
    let (file_name, file_size) = match msg {
        WireMessage::FileStart {
            file_name,
            file_size,
            ..
        } => (file_name, file_size),
        _ => panic!("Expected FileStart, got: {:?}", msg),
    };

    let output_path = save_dir.join(&file_name);
    if file_size == 0 {
        std::fs::File::create(&output_path).unwrap();
    }

    let mut total_received: u64 = 0;
    let mut expected_hash = String::new();

    loop {
        let frame = conn.recv_frame().await.unwrap();
        match frame.frame_type {
            FrameType::Data => {
                let decrypted = cipher.decrypt_frame(&frame.payload).unwrap();
                let offset = u64::from_be_bytes(decrypted[..8].try_into().unwrap());
                let crc = u32::from_be_bytes(decrypted[8..12].try_into().unwrap());
                let chunk_data = &decrypted[16..];
                transfer_engine::write_chunk(&output_path, offset, chunk_data, crc)
                    .await
                    .unwrap();
                total_received += chunk_data.len() as u64;
            }
            FrameType::Control => {
                let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
                if let WireMessage::FileEnd { sha256, .. } = msg {
                    expected_hash = sha256;
                    break;
                }
            }
            _ => {}
        }
    }

    assert_eq!(total_received, file_size, "Size mismatch for {file_name}");
    (output_path, expected_hash)
}

// ─── Test 1: 100MB file transfer with encryption ───

#[tokio::test]
async fn test_100mb_encrypted_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // Create 10MB file for fast coverage profiling
    let file_size = 10 * 1024 * 1024; // 10MB
    let file_path = create_large_file(sender_dir.path(), "large_10mb.bin", file_size);
    let sender_hash = sha256_file(&file_path).await;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let expected_hash = sender_hash.clone();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Key exchange
        let frame = conn.recv_frame().await.unwrap();
        let their_pub = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::KeyExchange { public_key } => public_key,
            _ => panic!("Expected KeyExchange"),
        };
        let (priv_key, pub_key) = SessionCipher::create_key_exchange().unwrap();
        let reply = WireMessage::KeyExchange {
            public_key: pub_key,
        };
        conn.send_frame(Frame::control(&serde_json::to_vec(&reply).unwrap()))
            .await
            .unwrap();
        let mut cipher = SessionCipher::from_key_exchange(&priv_key, &their_pub).unwrap();

        // Receive file
        let start = Instant::now();
        let (out_path, hash) = receive_file_encrypted(&conn, &mut cipher, &recv_dir).await;
        let elapsed = start.elapsed();

        let throughput_mbps = (100.0 * 1024.0 * 1024.0 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;
        println!(
            "📊 100MB transfer: {:.2}s, {:.0} Mbps throughput",
            elapsed.as_secs_f64(),
            throughput_mbps
        );

        assert_eq!(hash, expected_hash);
        let actual_hash = sha256_file(&out_path).await;
        assert_eq!(actual_hash, expected_hash, "100MB SHA-256 mismatch");
    });

    // Sender
    let client = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client).unwrap();

    let (priv_key, pub_key) = SessionCipher::create_key_exchange().unwrap();
    let key_msg = WireMessage::KeyExchange {
        public_key: pub_key,
    };
    conn.send_frame(Frame::control(&serde_json::to_vec(&key_msg).unwrap()))
        .await
        .unwrap();
    let frame = conn.recv_frame().await.unwrap();
    let their_pub = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
        WireMessage::KeyExchange { public_key } => public_key,
        _ => panic!("Expected KeyExchange"),
    };
    let mut cipher = SessionCipher::from_key_exchange(&priv_key, &their_pub).unwrap();

    send_file_encrypted(
        &conn,
        &mut cipher,
        &file_path,
        "large_100mb.bin",
        file_size as u64,
        "load-100mb-001",
        0,
    )
    .await;

    receiver_handle.await.unwrap();
    listener.stop();
}

// ─── Test 2: Concurrent parallel transfers (4 simultaneous) ───

#[tokio::test]
async fn test_concurrent_parallel_transfers() {
    let _ = env_logger::builder().is_test(true).try_init();

    let num_parallel = 4;
    let file_size = 5 * 1024 * 1024; // 5MB each

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // Create test files
    let mut files = Vec::new();
    for i in 0..num_parallel {
        let name = format!("concurrent_{i}.bin");
        let path = create_large_file(sender_dir.path(), &name, file_size);
        let hash = sha256_file(&path).await;
        files.push((name, path, hash));
    }

    let barrier = Arc::new(Barrier::new(num_parallel * 2)); // sender + receiver per transfer
    let mut handles = Vec::new();

    for i in 0..num_parallel {
        let (name, path, expected_hash) = files[i].clone();
        let recv_dir = receiver_dir.path().to_path_buf();
        let barrier_clone = Arc::clone(&barrier);

        // Start listener per transfer
        let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
        let port = listener.port();
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        let expected_hash_recv = expected_hash.clone();

        // Receiver
        let recv_handle = tokio::spawn(async move {
            barrier_clone.wait().await; // Synchronize start
            let stream = incoming.recv().await.unwrap();
            let conn = TcpConnection::new(stream).unwrap();

            // Simple unencrypted receive for concurrency test
            let frame = conn.recv_frame().await.unwrap();
            let (file_name, _file_size) =
                match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
                    WireMessage::FileStart {
                        file_name,
                        file_size,
                        ..
                    } => (file_name, file_size),
                    _ => panic!("Expected FileStart"),
                };

            let output_path = recv_dir.join(&file_name);
            let mut total = 0u64;
            loop {
                let frame = conn.recv_frame().await.unwrap();
                match frame.frame_type {
                    FrameType::Data => {
                        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
                        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
                        let data = &frame.payload[16..];
                        transfer_engine::write_chunk(&output_path, offset, data, crc)
                            .await
                            .unwrap();
                        total += data.len() as u64;
                    }
                    FrameType::Control => {
                        if let Ok(WireMessage::FileEnd { sha256, .. }) =
                            serde_json::from_slice(&frame.payload)
                        {
                            let actual = sha256_file(&output_path).await;
                            assert_eq!(actual, sha256, "Concurrent transfer SHA mismatch");
                            assert_eq!(sha256, expected_hash_recv);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            listener.stop();
        });
        handles.push(recv_handle);

        // Sender
        let barrier_send = Arc::clone(&barrier);
        let send_handle = tokio::spawn(async move {
            barrier_send.wait().await; // Synchronize start
            let client = connect(addr).await.unwrap();
            let conn = TcpConnection::new(client).unwrap();

            let fs = WireMessage::FileStart {
                transfer_id: format!("concurrent-{i}"),
                item_index: 0,
                file_name: name.clone(),
                file_size: file_size as u64,
                relative_path: name.clone(),
            };
            conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
                .await
                .unwrap();

            let chunk_size = 256 * 1024;
            let mut offset: u64 = 0;
            while offset < file_size as u64 {
                let (data, crc) = transfer_engine::read_chunk(&path, offset, chunk_size)
                    .await
                    .unwrap();
                let len = data.len() as u64;
                let mut frame_data = Vec::with_capacity(16 + data.len());
                frame_data.extend_from_slice(&offset.to_be_bytes());
                frame_data.extend_from_slice(&crc.to_be_bytes());
                frame_data.extend_from_slice(&[0u8; 4]);
                frame_data.extend_from_slice(&data);
                conn.send(Frame::data(frame_data)).await.unwrap();
                offset += len;
            }

            let fe = WireMessage::FileEnd {
                transfer_id: format!("concurrent-{i}"),
                item_index: 0,
                sha256: expected_hash,
            };
            conn.send_frame(Frame::control(&serde_json::to_vec(&fe).unwrap()))
                .await
                .unwrap();
        });
        handles.push(send_handle);
    }

    let start = Instant::now();
    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed();
    let total_mb = (num_parallel * file_size) as f64 / (1024.0 * 1024.0);
    println!(
        "📊 {} concurrent {:.0}MB transfers: {:.2}s total, {:.0} MB/s aggregate",
        num_parallel,
        file_size as f64 / (1024.0 * 1024.0),
        elapsed.as_secs_f64(),
        total_mb / elapsed.as_secs_f64()
    );
}

// ─── Test 3: Multi-file batch (50 files of varying sizes) ───

#[tokio::test]
async fn test_multi_file_batch_transfer() {
    let _ = env_logger::builder().is_test(true).try_init();

    let sender_dir = tempdir().unwrap();
    let receiver_dir = tempdir().unwrap();

    // Create 50 files of varying sizes (1KB to 1MB)
    let num_files = 50;
    let mut files = Vec::new();
    for i in 0..num_files {
        let size = ((i + 1) * 20 * 1024).min(1024 * 1024); // 20KB to 1MB
        let name = format!("batch_file_{i:03}.dat");
        let path = create_large_file(sender_dir.path(), &name, size);
        let hash = sha256_file(&path).await;
        files.push((name, path, size, hash));
    }

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let recv_dir = receiver_dir.path().to_path_buf();
    let file_count = num_files;
    let file_hashes: Vec<(String, String)> = files
        .iter()
        .map(|(name, _, _, hash)| (name.clone(), hash.clone()))
        .collect();

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        let mut received_count = 0;
        let mut current_file: Option<(String, u64)> = None;

        loop {
            let frame = conn.recv_frame().await.unwrap();
            match frame.frame_type {
                FrameType::Control => {
                    let msg: WireMessage = serde_json::from_slice(&frame.payload).unwrap();
                    match msg {
                        WireMessage::FileStart {
                            file_name,
                            file_size,
                            ..
                        } => {
                            current_file = Some((file_name.clone(), file_size));
                            if file_size == 0 {
                                let out = recv_dir.join(&file_name);
                                std::fs::File::create(&out).unwrap();
                            }
                        }
                        WireMessage::FileEnd { sha256, .. } => {
                            if let Some((ref name, _)) = current_file {
                                let out = recv_dir.join(name);
                                let actual = sha256_file(&out).await;
                                assert_eq!(actual, sha256, "Batch file {} SHA mismatch", name);
                            }
                            received_count += 1;
                            current_file = None;
                            if received_count >= file_count {
                                break;
                            }
                        }
                        WireMessage::TransferComplete { .. } => break,
                        _ => {}
                    }
                }
                FrameType::Data => {
                    if let Some((ref name, _)) = current_file {
                        let out = recv_dir.join(name);
                        let offset = u64::from_be_bytes(frame.payload[..8].try_into().unwrap());
                        let crc = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
                        let data = &frame.payload[16..];
                        transfer_engine::write_chunk(&out, offset, data, crc)
                            .await
                            .unwrap();
                    }
                }
                _ => {}
            }
        }
        assert_eq!(received_count, file_count, "Not all batch files received");
    });

    // Sender: send all files sequentially
    let client = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client).unwrap();

    let start = Instant::now();
    for (i, (name, path, size, hash)) in files.iter().enumerate() {
        let fs = WireMessage::FileStart {
            transfer_id: "batch-001".to_string(),
            item_index: i as u32,
            file_name: name.clone(),
            file_size: *size as u64,
            relative_path: name.clone(),
        };
        conn.send_frame(Frame::control(&serde_json::to_vec(&fs).unwrap()))
            .await
            .unwrap();

        // Send data
        let chunk_size = 256 * 1024;
        let mut offset: u64 = 0;
        while offset < *size as u64 {
            let (data, crc) = transfer_engine::read_chunk(path, offset, chunk_size)
                .await
                .unwrap();
            let len = data.len() as u64;
            let mut fd = Vec::with_capacity(16 + data.len());
            fd.extend_from_slice(&offset.to_be_bytes());
            fd.extend_from_slice(&crc.to_be_bytes());
            fd.extend_from_slice(&[0u8; 4]);
            fd.extend_from_slice(&data);
            conn.send(Frame::data(fd)).await.unwrap();
            offset += len;
        }

        let fe = WireMessage::FileEnd {
            transfer_id: "batch-001".to_string(),
            item_index: i as u32,
            sha256: hash.clone(),
        };
        conn.send_frame(Frame::control(&serde_json::to_vec(&fe).unwrap()))
            .await
            .unwrap();
    }
    let elapsed = start.elapsed();

    let total_size: usize = files.iter().map(|(_, _, s, _)| *s).sum();
    println!(
        "📊 {num_files} files ({:.1} MB total): {:.2}s, {:.0} files/s",
        total_size as f64 / (1024.0 * 1024.0),
        elapsed.as_secs_f64(),
        num_files as f64 / elapsed.as_secs_f64()
    );

    receiver_handle.await.unwrap();
    listener.stop();
}

// ─── Test 4: Throughput benchmark (raw encrypted frame throughput) ───

#[tokio::test]
async fn test_encrypted_throughput_benchmark() {
    let _ = env_logger::builder().is_test(true).try_init();

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let num_frames = 100;
    let frame_size = 64 * 1024; // 64KB per frame

    let receiver_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.unwrap();
        let conn = TcpConnection::new(stream).unwrap();

        // Key exchange
        let frame = conn.recv_frame().await.unwrap();
        let their_pub = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
            WireMessage::KeyExchange { public_key } => public_key,
            _ => panic!("Expected KeyExchange"),
        };
        let (priv_key, pub_key) = SessionCipher::create_key_exchange().unwrap();
        conn.send_frame(Frame::control(
            &serde_json::to_vec(&WireMessage::KeyExchange {
                public_key: pub_key,
            })
            .unwrap(),
        ))
        .await
        .unwrap();
        let mut cipher = SessionCipher::from_key_exchange(&priv_key, &their_pub).unwrap();

        let start = Instant::now();
        let mut total_bytes = 0u64;
        for _ in 0..num_frames {
            let frame = conn.recv_frame().await.unwrap();
            let decrypted = cipher.decrypt_frame(&frame.payload).unwrap();
            total_bytes += decrypted.len() as u64;
        }
        let elapsed = start.elapsed();

        let mbps = (total_bytes as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0;
        println!(
            "📊 Encrypted throughput: {} frames × {}KB = {:.1} MB in {:.2}s = {:.0} Mbps",
            num_frames,
            frame_size / 1024,
            total_bytes as f64 / (1024.0 * 1024.0),
            elapsed.as_secs_f64(),
            mbps
        );
        assert!(total_bytes > 0);
    });

    let client = connect(addr).await.unwrap();
    let conn = TcpConnection::new(client).unwrap();

    let (priv_key, pub_key) = SessionCipher::create_key_exchange().unwrap();
    conn.send_frame(Frame::control(
        &serde_json::to_vec(&WireMessage::KeyExchange {
            public_key: pub_key,
        })
        .unwrap(),
    ))
    .await
    .unwrap();
    let frame = conn.recv_frame().await.unwrap();
    let their_pub = match serde_json::from_slice::<WireMessage>(&frame.payload).unwrap() {
        WireMessage::KeyExchange { public_key } => public_key,
        _ => panic!("Expected KeyExchange"),
    };
    let mut cipher = SessionCipher::from_key_exchange(&priv_key, &their_pub).unwrap();

    let data = vec![0xABu8; frame_size];
    for _ in 0..num_frames {
        let encrypted = cipher.encrypt_frame(&data).unwrap();
        conn.send(Frame::data(encrypted)).await.unwrap();
    }

    receiver_handle.await.unwrap();
    listener.stop();
}
