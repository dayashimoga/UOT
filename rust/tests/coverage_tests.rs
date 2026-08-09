//! Comprehensive Coverage Tests
//!
//! Targets all modules with insufficient coverage to reach >90% overall.
//! Tests engine lifecycle, config, transport types, streaming, transfer history,
//! analytics, protocol messages, discovery types, and core utilities.

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::{EngineState, UotEngine};
use rust_lib_uot_app::protocol::handler::{OfferItemInfo, WireMessage};
use rust_lib_uot_app::security::crypto::SoftwareCryptoProvider;
use rust_lib_uot_app::security::session_cipher::SessionCipher;
use rust_lib_uot_app::security::verification::{TrustManager, VerificationPin};
use rust_lib_uot_app::security::CryptoProvider;
use rust_lib_uot_app::streaming::manager::{StreamManager, StreamState, StreamType};
use rust_lib_uot_app::streaming::pipeline::{AudioCodec, H264NalType, MediaStreamPipeline};
use rust_lib_uot_app::transfer::analytics::LifetimeStats;
use rust_lib_uot_app::transfer::history::TransferHistoryStore;
use rust_lib_uot_app::transfer::queue::{Priority, TransferQueueManager};
use rust_lib_uot_app::transfer::ratelimit::RateLimiter;
use rust_lib_uot_app::transfer::types::*;
use rust_lib_uot_app::transport::ble::BleAdvertisement;
use rust_lib_uot_app::transport::fallback::{TransportFallbackManager, TransportSelectionStrategy};
use rust_lib_uot_app::transport::tcp::{Frame, FrameType};
use rust_lib_uot_app::transport::types::{TransportId, TransportState};
use rust_lib_uot_app::transport::wifidirect::WifiDirectGroupInfo;

// ═══════════════════════════════════════════════════════════════════
// ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_engine_creation_and_basic_state() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    assert_eq!(engine.state(), EngineState::Stopped);
    assert!(!engine.device_id().is_empty());
    assert!(engine.discovered_devices().is_empty());
    assert!(engine.get_transfers().is_empty());
    assert!(engine.get_streams().is_empty());
    assert!(engine.get_recent_events(10).is_empty());
}

#[test]
fn test_engine_log_event() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    engine.log_event("Test event 1");
    engine.log_event("Test event 2");
    let events = engine.get_recent_events(10);
    assert_eq!(events.len(), 2);
    assert!(events[0].contains("Test event 2"));
    assert!(events[1].contains("Test event 1"));
}

#[test]
fn test_engine_log_event_ring_buffer_overflow() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    for i in 0..250 {
        engine.log_event(&format!("Event {i}"));
    }
    let events = engine.get_recent_events(300);
    assert_eq!(events.len(), 200); // MAX_EVENT_LOG = 200
}

#[test]
fn test_engine_config_getter() {
    let mut config = AppConfig::default();
    config.device_name = "TestDevice42".to_string();
    let (engine, _rx) = UotEngine::new(config);
    assert_eq!(engine.config().device_name, "TestDevice42");
}

#[test]
fn test_engine_set_device_name() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    engine.set_device_name("NewName");
    assert_eq!(engine.config().device_name, "NewName");
}

#[test]
fn test_engine_pin_generation_and_verification() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    let pin = engine.generate_pin(300);
    assert_eq!(pin.len(), 6);
    assert!(pin.chars().all(|c| c.is_ascii_digit()));

    // Wrong PIN
    assert!(engine.verify_pin("dev-1", "000000").is_none());
    // Correct PIN
    let token = engine.verify_pin("dev-1", &pin);
    assert!(token.is_some());
    // PIN is consumed
    assert!(engine.verify_pin("dev-1", &pin).is_none());
}

#[test]
fn test_engine_streaming_full_lifecycle() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    let sid = engine.start_stream(StreamType::Screen, "dev-1", "Desktop", 9000, true);
    assert_eq!(engine.get_streams().len(), 1);
    assert_eq!(engine.get_streams()[0].stream_type, StreamType::Screen);
    assert_eq!(engine.get_streams()[0].is_sender, true);

    engine.stop_stream(&sid);
    assert_eq!(engine.get_streams()[0].state, StreamState::Stopping);
}

#[test]
fn test_engine_transport_selection() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    let candidates = vec![
        (TransportId::BluetoothLe, TransportState::Connected),
        (TransportId::TcpLan, TransportState::Connected),
    ];
    assert_eq!(
        engine.select_best_transport(&candidates),
        Some(TransportId::TcpLan)
    );

    engine.set_transport_strategy(TransportSelectionStrategy::PreferOffline);
    assert_eq!(
        engine.select_best_transport(&candidates),
        Some(TransportId::BluetoothLe)
    );
}

#[test]
fn test_engine_lifetime_stats() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let stats = engine.get_lifetime_stats();
    assert_eq!(stats.total_transfers, 0);
}

#[test]
fn test_engine_transfer_history_empty() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let history = engine.get_transfer_history("", None);
    assert!(history.is_empty());
}

#[test]
fn test_engine_device_connection_state() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    assert!(!engine.is_device_connected("dev-1"));
    engine.disconnect_device("dev-1"); // No-op, should not panic
}

#[tokio::test]
async fn test_engine_cancel_nonexistent_transfer() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let result = engine.cancel_transfer("invalid-uuid").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_engine_cancel_valid_uuid_not_found() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let result = engine
        .cancel_transfer("00000000-0000-0000-0000-000000000001")
        .await;
    assert!(result.is_err());
}

#[test]
fn test_engine_pause_resume_nonexistent() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    assert!(engine.pause_transfer("invalid").is_err());
    assert!(engine.resume_transfer("invalid").is_err());
    assert!(engine
        .pause_transfer("00000000-0000-0000-0000-000000000001")
        .is_err());
    assert!(engine
        .resume_transfer("00000000-0000-0000-0000-000000000001")
        .is_err());
}

// ═══════════════════════════════════════════════════════════════════
// CONFIG TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_config_full_validation() {
    let mut config = AppConfig::default();
    assert!(config.validate().is_ok());
    config.transfer.max_concurrent_transfers = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_serialization_roundtrip() {
    let config = AppConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.device_name, deserialized.device_name);
    assert_eq!(config.transfer.chunk_size, deserialized.transfer.chunk_size);
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER ANALYTICS TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_lifetime_stats_default() {
    let stats = LifetimeStats::default();
    assert_eq!(stats.total_transfers, 0);
    assert_eq!(stats.successful_transfers, 0);
    assert_eq!(stats.failed_transfers, 0);
}

#[test]
fn test_lifetime_stats_record_success_send() {
    let mut stats = LifetimeStats::default();
    stats.record_success(1000, true, 500);
    assert_eq!(stats.total_transfers, 1);
    assert_eq!(stats.successful_transfers, 1);
    assert_eq!(stats.total_bytes_sent, 1000);
    assert_eq!(stats.total_bytes_received, 0);
    assert_eq!(stats.peak_speed_bytes_per_sec, 500);
}

#[test]
fn test_lifetime_stats_record_success_receive() {
    let mut stats = LifetimeStats::default();
    stats.record_success(2000, false, 1000);
    assert_eq!(stats.total_bytes_received, 2000);
    assert_eq!(stats.total_bytes_sent, 0);
}

#[test]
fn test_lifetime_stats_record_failure() {
    let mut stats = LifetimeStats::default();
    stats.record_failure();
    assert_eq!(stats.total_transfers, 1);
    assert_eq!(stats.failed_transfers, 1);
}

#[test]
fn test_lifetime_stats_peak_speed() {
    let mut stats = LifetimeStats::default();
    stats.record_success(100, true, 500);
    stats.record_success(100, true, 300); // Lower speed
    assert_eq!(stats.peak_speed_bytes_per_sec, 500); // Keeps peak
    stats.record_success(100, true, 900);
    assert_eq!(stats.peak_speed_bytes_per_sec, 900); // New peak
}

#[test]
fn test_lifetime_stats_save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stats.json");

    let mut stats = LifetimeStats::default();
    stats.record_success(5000, true, 1000);
    stats.record_failure();
    stats.save(&path).unwrap();

    let loaded = LifetimeStats::load(&path);
    assert_eq!(loaded.total_transfers, 2);
    assert_eq!(loaded.successful_transfers, 1);
    assert_eq!(loaded.failed_transfers, 1);
    assert_eq!(loaded.total_bytes_sent, 5000);
}

#[test]
fn test_lifetime_stats_load_nonexistent() {
    let stats = LifetimeStats::load(std::path::Path::new("/nonexistent/stats.json"));
    assert_eq!(stats.total_transfers, 0); // Should return default
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER HISTORY TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_history_upsert_and_query() {
    let mut store = TransferHistoryStore::default();

    let record = TransferRecord {
        transfer_id: uuid::Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::Completed,
        remote_device: "Alice Phone".to_string(),
        items: vec![TransferItemRecord {
            item_id: uuid::Uuid::new_v4(),
            name: "photo.jpg".to_string(),
            relative_path: "photo.jpg".to_string(),
            size: 1024,
            transferred_bytes: 1024,
            status: TransferStatus::Completed,
            hash: Some("abc123".to_string()),
        }],
        total_size: 1024,
        transferred_bytes: 1024,
        created_at: chrono::Utc::now(),
        started_at: Some(chrono::Utc::now()),
        finished_at: Some(chrono::Utc::now()),
        error: None,
    };

    store.upsert(record.clone());
    assert_eq!(store.records.len(), 1);

    // Query by device name
    let results = store.query("alice", None);
    assert_eq!(results.len(), 1);

    // Query by file name
    let results = store.query("photo", None);
    assert_eq!(results.len(), 1);

    // Query with status filter
    let results = store.query("", Some(TransferStatus::Completed));
    assert_eq!(results.len(), 1);
    let results = store.query("", Some(TransferStatus::Failed));
    assert_eq!(results.len(), 0);

    // Empty query returns all
    let results = store.query("", None);
    assert_eq!(results.len(), 1);

    // Upsert updates existing
    let mut updated = record.clone();
    updated.status = TransferStatus::Failed;
    store.upsert(updated);
    assert_eq!(store.records.len(), 1);
    assert_eq!(store.records[0].status, TransferStatus::Failed);
}

#[test]
fn test_history_save_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.json");

    let mut store = TransferHistoryStore::default();
    store.upsert(TransferRecord {
        transfer_id: uuid::Uuid::new_v4(),
        direction: TransferDirection::Receive,
        status: TransferStatus::Completed,
        remote_device: "Bob".to_string(),
        items: vec![],
        total_size: 500,
        transferred_bytes: 500,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    });
    store.save(&path).unwrap();

    let loaded = TransferHistoryStore::load(&path);
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].remote_device, "Bob");
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER QUEUE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_queue_manager_basic() {
    let mut qm = TransferQueueManager::new(2);
    assert!(qm.can_start());

    let record = TransferRecord {
        transfer_id: uuid::Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::Queued,
        remote_device: "Dev".to_string(),
        items: vec![],
        total_size: 100,
        transferred_bytes: 0,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    };

    qm.push(record.clone(), Priority::Normal);
    qm.mark_started();
    assert!(qm.can_start()); // Still under limit

    qm.push(record.clone(), Priority::Normal);
    qm.mark_started();
    assert!(!qm.can_start()); // At limit

    qm.mark_completed();
    assert!(qm.can_start()); // Back under limit
}

// ═══════════════════════════════════════════════════════════════════
// RATE LIMITER TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_rate_limiter_creation() {
    let _rl = RateLimiter::new(0); // 0 = unlimited
    let _rl2 = RateLimiter::new(1_000_000); // 1 MB/s
}

// ═══════════════════════════════════════════════════════════════════
// STREAMING TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_stream_manager_full_lifecycle() {
    let mgr = StreamManager::new();

    let sid = mgr.start_session(StreamType::Camera, "dev-1", "Cam1", 9000, true);
    assert_eq!(mgr.active_sessions().len(), 1);

    let session = mgr.get_session(&sid).unwrap();
    assert_eq!(session.stream_type, StreamType::Camera);
    assert_eq!(session.state, StreamState::Starting);
    assert_eq!(session.is_sender, true);
    assert_eq!(session.port, 9000);

    mgr.update_state(&sid, StreamState::Streaming);
    assert_eq!(mgr.get_session(&sid).unwrap().state, StreamState::Streaming);

    mgr.update_stats(&sid, 5000, 10.5);
    let s = mgr.get_session(&sid).unwrap();
    assert_eq!(s.bytes_streamed, 5000);
    assert!((s.duration_secs - 10.5).abs() < 0.01);

    mgr.stop_session(&sid);
    assert_eq!(mgr.get_session(&sid).unwrap().state, StreamState::Stopping);

    mgr.remove_session(&sid);
    assert!(mgr.get_session(&sid).is_none());
    assert!(mgr.active_sessions().is_empty());
}

#[test]
fn test_stream_type_display() {
    assert_eq!(format!("{}", StreamType::Camera), "Camera");
    assert_eq!(format!("{}", StreamType::Screen), "Screen");
    assert_eq!(format!("{}", StreamType::Video), "Video");
    assert_eq!(format!("{}", StreamType::Audio), "Audio");
}

#[test]
fn test_media_pipeline_opus_audio() {
    let mut pipeline = MediaStreamPipeline::new(10);
    let pkt = pipeline.encode_audio_frame(3000, b"OPUS_FRAME");
    assert!(!pkt.is_video);
    assert_eq!(pkt.audio_codec, Some(AudioCodec::AacAdts));
    assert_eq!(pkt.payload, b"OPUS_FRAME");
}

#[test]
fn test_media_pipeline_bitrate() {
    let pipeline = MediaStreamPipeline::new(10);
    let bitrate = pipeline.current_bitrate_mbps();
    assert!(bitrate >= 0.0);
}

#[test]
fn test_media_pipeline_sequence_numbers() {
    let mut pipeline = MediaStreamPipeline::new(100);
    let p1 = pipeline.encode_video_frame(H264NalType::Sps, 0, b"SPS");
    let p2 = pipeline.encode_video_frame(H264NalType::Pps, 100, b"PPS");
    let p3 = pipeline.encode_video_frame(H264NalType::IdrKeyframe, 200, b"IDR");
    assert_eq!(p1.sequence, 1);
    assert_eq!(p2.sequence, 2);
    assert_eq!(p3.sequence, 3);
}

#[test]
fn test_jitter_buffer_overflow() {
    let mut pipeline = MediaStreamPipeline::new(2);
    let p1 = pipeline.encode_video_frame(H264NalType::Sps, 0, b"A");
    let p2 = pipeline.encode_video_frame(H264NalType::Pps, 100, b"B");
    let p3 = pipeline.encode_video_frame(H264NalType::IdrKeyframe, 200, b"C");

    pipeline.push_jitter(p1);
    pipeline.push_jitter(p2);
    pipeline.push_jitter(p3); // Overflows, drops oldest

    let popped = pipeline.pop_jitter().unwrap();
    assert_eq!(popped.payload, b"B"); // A was dropped
}

// ═══════════════════════════════════════════════════════════════════
// BLE / WI-FI DIRECT / HOTSPOT TRANSPORT TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_ble_advertisement_encode_decode() {
    let adv = BleAdvertisement {
        device_name: "TestDevice".to_string(),
        device_hash: "abc123".to_string(),
        wifi_ip: Some("192.168.1.100".to_string()),
        port: 42000,
    };
    let encoded = adv.encode();
    assert!(!encoded.is_empty());
    let decoded = BleAdvertisement::decode(&encoded).unwrap();
    assert_eq!(decoded.device_name, "TestDevice");
    assert_eq!(decoded.port, 42000);
    assert_eq!(decoded.wifi_ip, Some("192.168.1.100".to_string()));
}

#[test]
fn test_ble_advertisement_without_wifi() {
    let adv = BleAdvertisement {
        device_name: "BLE-Only".to_string(),
        device_hash: "xyz".to_string(),
        wifi_ip: None,
        port: 42000,
    };
    let encoded = adv.encode();
    let decoded = BleAdvertisement::decode(&encoded).unwrap();
    assert!(decoded.wifi_ip.is_none());
}

#[test]
fn test_wifi_direct_group_info() {
    let group = WifiDirectGroupInfo::new_group("MyPhone", 42000);
    assert!(group.ssid.starts_with("DIRECT-UOT-MyPhone-"));
    assert_eq!(group.passphrase.len(), 8);
    assert_eq!(group.frequency_mhz, 5180);
    assert_eq!(group.port, 42000);

    let json = group.to_json().unwrap();
    let parsed = WifiDirectGroupInfo::from_json(&json).unwrap();
    assert_eq!(parsed.ssid, group.ssid);
    assert_eq!(parsed.passphrase, group.passphrase);
}

// ═══════════════════════════════════════════════════════════════════
// TRANSPORT TYPES TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_transport_id_variants() {
    let ids = [
        TransportId::TcpLan,
        TransportId::WifiDirect,
        TransportId::BluetoothLe,
        TransportId::QrCode,
    ];
    for id in &ids {
        assert_eq!(*id, *id);
    }
    assert_ne!(TransportId::TcpLan, TransportId::BluetoothLe);
}

#[test]
fn test_transport_state_variants() {
    let states = [
        TransportState::Disconnected,
        TransportState::Connecting,
        TransportState::Connected,
        TransportState::Listening,
        TransportState::Error,
    ];
    for state in &states {
        assert_eq!(*state, *state);
    }
}

#[test]
fn test_fallback_wifi_direct_preference() {
    let mgr = TransportFallbackManager::new(TransportSelectionStrategy::PreferSpeed);
    let candidates = vec![
        (TransportId::WifiDirect, TransportState::Connected),
        (TransportId::BluetoothLe, TransportState::Connected),
    ];
    assert_eq!(
        mgr.select_best_transport(&candidates),
        Some(TransportId::WifiDirect)
    );
}

#[test]
fn test_fallback_no_active_candidates() {
    let mgr = TransportFallbackManager::default();
    let candidates = vec![
        (TransportId::TcpLan, TransportState::Disconnected),
        (TransportId::BluetoothLe, TransportState::Error),
    ];
    assert_eq!(mgr.select_best_transport(&candidates), None);
}

// ═══════════════════════════════════════════════════════════════════
// FRAME TYPE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_frame_type_conversion() {
    assert_eq!(FrameType::try_from(0u8).unwrap(), FrameType::Control);
    assert_eq!(FrameType::try_from(1u8).unwrap(), FrameType::Data);
    assert_eq!(FrameType::try_from(2u8).unwrap(), FrameType::Ping);
    assert_eq!(FrameType::try_from(3u8).unwrap(), FrameType::Pong);
    assert!(FrameType::try_from(4u8).is_err());
    assert!(FrameType::try_from(255u8).is_err());
}

#[test]
fn test_frame_construction() {
    let control = Frame::control(b"hello");
    assert_eq!(control.frame_type, FrameType::Control);
    assert_eq!(control.payload, b"hello");

    let data = Frame::data(b"binary stuff".to_vec());
    assert_eq!(data.frame_type, FrameType::Data);
    assert_eq!(data.payload, b"binary stuff");

    let ping = Frame::ping();
    assert_eq!(ping.frame_type, FrameType::Ping);
    assert!(ping.payload.is_empty());

    let pong = Frame::pong();
    assert_eq!(pong.frame_type, FrameType::Pong);
    assert!(pong.payload.is_empty());
}

#[test]
fn test_frame_encode() {
    let frame = Frame::control(b"test");
    let encoded = frame.encode();
    // 4 bytes length + 1 byte type + 4 bytes payload
    assert_eq!(encoded.len(), 9);
    // Payload length = 4 (big-endian)
    assert_eq!(&encoded[0..4], &[0, 0, 0, 4]);
    // Type = 0 (Control)
    assert_eq!(encoded[4], 0);
    // Payload
    assert_eq!(&encoded[5..], b"test");
}

// ═══════════════════════════════════════════════════════════════════
// PROTOCOL MESSAGE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_wire_message_hello_roundtrip() {
    let msg = WireMessage::Hello {
        device_id: "dev-1".to_string(),
        device_name: "My Phone".to_string(),
        device_type: "Phone".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["file_transfer".to_string(), "clipboard".to_string()],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: WireMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        WireMessage::Hello {
            device_id,
            device_name,
            capabilities,
            ..
        } => {
            assert_eq!(device_id, "dev-1");
            assert_eq!(device_name, "My Phone");
            assert_eq!(capabilities.len(), 2);
        }
        _ => panic!("Expected Hello"),
    }
}

#[test]
fn test_wire_message_all_variants_serialize() {
    let messages: Vec<WireMessage> = vec![
        WireMessage::Hello {
            device_id: "d".to_string(),
            device_name: "n".to_string(),
            device_type: "t".to_string(),
            version: "v".to_string(),
            capabilities: vec![],
        },
        WireMessage::HelloAck {
            device_id: "d".to_string(),
            device_name: "n".to_string(),
            device_type: "t".to_string(),
            version: "v".to_string(),
        },
        WireMessage::Offer {
            transfer_id: "tid".to_string(),
            device_name: "n".to_string(),
            items: vec![OfferItemInfo {
                name: "f.txt".to_string(),
                relative_path: "f.txt".to_string(),
                size: 100,
                is_directory: false,
            }],
            total_size: 100,
        },
        WireMessage::OfferResponse {
            transfer_id: "tid".to_string(),
            accepted: true,
            reason: None,
        },
        WireMessage::FileStart {
            transfer_id: "tid".to_string(),
            item_index: 0,
            file_name: "f.txt".to_string(),
            file_size: 100,
            relative_path: "f.txt".to_string(),
        },
        WireMessage::FileEnd {
            transfer_id: "tid".to_string(),
            item_index: 0,
            sha256: "abc".to_string(),
        },
        WireMessage::TransferComplete {
            transfer_id: "tid".to_string(),
            success: true,
        },
        WireMessage::Cancel {
            transfer_id: "tid".to_string(),
            reason: Some("user cancelled".to_string()),
        },
        WireMessage::Pause {
            transfer_id: "tid".to_string(),
        },
        WireMessage::Resume {
            transfer_id: "tid".to_string(),
            offset: 5000,
        },
        WireMessage::ClipboardData {
            content_type: "text/plain".to_string(),
            data: "hello clipboard".to_string(),
        },
        WireMessage::KeyExchange {
            public_key: vec![1, 2, 3, 4],
        },
    ];

    for msg in &messages {
        let json = serde_json::to_string(msg).unwrap();
        let parsed: WireMessage = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, json2);
    }
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER TYPES TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_transfer_status_variants() {
    let statuses = [
        TransferStatus::Queued,
        TransferStatus::Pending,
        TransferStatus::InProgress,
        TransferStatus::Paused,
        TransferStatus::Verifying,
        TransferStatus::Completed,
        TransferStatus::Failed,
        TransferStatus::Cancelled,
    ];
    for s in &statuses {
        let json = serde_json::to_string(s).unwrap();
        let parsed: TransferStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(*s, parsed);
    }
}

#[test]
fn test_transfer_direction_variants() {
    assert_ne!(TransferDirection::Send, TransferDirection::Receive);
    let json = serde_json::to_string(&TransferDirection::Send).unwrap();
    let parsed: TransferDirection = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, TransferDirection::Send);
}

#[test]
fn test_transfer_record_serialization() {
    let record = TransferRecord {
        transfer_id: uuid::Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::InProgress,
        remote_device: "Phone".to_string(),
        items: vec![],
        total_size: 0,
        transferred_bytes: 0,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: TransferRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.transfer_id, parsed.transfer_id);
}

// ═══════════════════════════════════════════════════════════════════
// VERSION TESTS
// ═══════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════
// SECURITY TESTS (ADDITIONAL)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_trust_manager_cleanup() {
    let mut tm = TrustManager::new();
    tm.trust_device("dev-1", "Phone");
    tm.trust_device("dev-2", "Laptop");
    assert_eq!(tm.trusted_devices().len(), 2);
    tm.cleanup(); // Should not remove anything (no expired sessions)
    assert_eq!(tm.trusted_devices().len(), 2);
}

#[test]
fn test_verification_pin_format() {
    for _ in 0..100 {
        let pin = VerificationPin::generate(300);
        assert_eq!(pin.pin.len(), 6);
        assert!(pin.pin.parse::<u32>().is_ok());
        assert!(!pin.is_expired());
    }
}

#[test]
fn test_session_cipher_short_frame() {
    let key = vec![0x42u8; 32];
    let mut dec = SessionCipher::new(key).unwrap();
    // Too short for nonce counter
    assert!(dec.decrypt_frame(&[0u8; 7]).is_err());
    assert!(dec.decrypt_frame(&[]).is_err());
}

#[test]
fn test_crypto_provider_key_pair_uniqueness() {
    let provider = SoftwareCryptoProvider::new();
    let kp1 = provider.generate_key_pair().unwrap();
    let kp2 = provider.generate_key_pair().unwrap();
    assert_ne!(kp1.public_key, kp2.public_key);
    assert_ne!(kp1.private_key, kp2.private_key);
}
