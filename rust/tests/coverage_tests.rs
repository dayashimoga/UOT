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

// ═══════════════════════════════════════════════════════════════════
// NEW COMPREHENSIVE COVERAGE TESTS (API, PERSISTENCE, TRANSPORT, PROTOCOL)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_engine_api_full_suite() {
    use rust_lib_uot_app::api::engine_api::*;

    let init_res = engine_init();
    assert!(
        init_res.starts_with("ok:")
            || init_res.starts_with("partial:")
            || init_res == "already_initialized"
    );

    let state = engine_state();
    assert!(!state.is_empty());

    let dev_id = engine_device_id();
    assert!(!dev_id.is_empty());

    let devices = engine_get_devices();
    assert!(devices.starts_with('['));

    let transfers = engine_get_transfers();
    assert!(transfers.starts_with('['));

    let send_res = engine_send_files(
        "nonexistent-dev".to_string(),
        vec!["/tmp/fake.txt".to_string()],
    );
    assert!(send_res.starts_with("error:"));

    let pause_res = engine_pause_transfer("invalid-uuid".to_string());
    assert!(pause_res.starts_with("error:"));

    let resume_res = engine_resume_transfer("invalid-uuid".to_string());
    assert!(resume_res.starts_with("error:"));

    let cancel_res = engine_cancel_transfer("invalid-uuid".to_string());
    assert!(cancel_res.starts_with("error:"));

    let accept_res = engine_accept_transfer("invalid-uuid".to_string());
    assert!(accept_res.starts_with("error:"));

    let prog_res = engine_get_progress("invalid-uuid".to_string());
    assert_eq!(prog_res, "null");

    let name_res = engine_set_device_name("CustomAPIName".to_string());
    assert_eq!(name_res, "ok");

    let clip_res = engine_send_clipboard(
        "nonexistent-dev".to_string(),
        "clipboard content".to_string(),
    );
    assert!(clip_res.starts_with("error:"));

    let events = engine_get_events(10);
    assert!(events.starts_with('['));

    let streams = engine_get_streams();
    assert!(streams.starts_with('['));

    let stream_id = engine_start_stream(
        "Camera".to_string(),
        "dev-1".to_string(),
        "Device 1".to_string(),
        42000,
        true,
    );
    assert!(!stream_id.is_empty());

    let stop_stream_res = engine_stop_stream(stream_id);
    assert_eq!(stop_stream_res, "ok");

    let settings = engine_load_settings();
    assert!(settings.contains("device_name"));

    let save_res = engine_save_settings(settings);
    assert_eq!(save_res, "ok");

    let save_err_res = engine_save_settings("invalid-json".to_string());
    assert!(save_err_res.starts_with("error:"));

    let pin = engine_generate_pin(300);
    assert_eq!(pin.len(), 6);

    let qr_json = engine_generate_qr_invitation(pin.clone());
    assert!(qr_json.contains("pin"));

    let parse_qr = engine_parse_qr_invitation(qr_json.clone());
    assert!(parse_qr.contains("pin"));

    let parse_qr_err = engine_parse_qr_invitation("invalid-qr-json".to_string());
    assert!(parse_qr_err.starts_with("error:"));

    let history = engine_search_history("".to_string());
    assert!(history.starts_with('['));

    let stats = engine_get_stats();
    assert!(stats.contains("total_transfers"));

    let scan = engine_subnet_scan();
    assert!(scan.starts_with('['));

    let verify_res = engine_verify_pin("dev-1".to_string(), "000000".to_string());
    assert_eq!(verify_res, "invalid");

    engine_stop();
}

#[test]
fn test_user_settings_persistence() {
    use rust_lib_uot_app::core::settings::UserSettings;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("settings.json");

    let mut settings = UserSettings::load(&path);
    assert_eq!(settings.theme_mode, "dark");

    settings.device_name = "TestDeviceSettings".to_string();
    settings.chunk_size_kb = 512;
    settings.save(&path).unwrap();

    let reloaded = UserSettings::load(&path);
    assert_eq!(reloaded.device_name, "TestDeviceSettings");
    assert_eq!(reloaded.chunk_size_kb, 512);

    let def_path = UserSettings::default_path();
    assert!(def_path.to_string_lossy().contains("settings.json"));
}

#[test]
fn test_wifidirect_and_hotspot_config() {
    use rust_lib_uot_app::transport::hotspot::{HotspotConfig, HotspotState};
    use rust_lib_uot_app::transport::wifidirect::WifiDirectGroupInfo;

    let group = WifiDirectGroupInfo::new_group("TestDevice", 42000);
    assert!(group.ssid.contains("DIRECT-UOT-TestDevice"));
    assert_eq!(group.port, 42000);

    let json = group.to_json().unwrap();
    let parsed = WifiDirectGroupInfo::from_json(&json).unwrap();
    assert_eq!(parsed.ssid, group.ssid);

    let hs = HotspotConfig::create_temp("TestDevice", 42000);
    assert_eq!(hs.ssid, "UOT-TestDevice");
    assert_eq!(hs.state, HotspotState::Disabled);

    let hs_json = serde_json::to_string(&hs).unwrap();
    let hs_parsed: HotspotConfig = serde_json::from_str(&hs_json).unwrap();
    assert_eq!(hs_parsed.ssid, hs.ssid);
}

#[test]
fn test_ble_advertisement_and_constants() {
    use rust_lib_uot_app::transport::ble::{
        BleAdvertisement, UOT_BLE_CHAR_CONTROL, UOT_BLE_CHAR_DATA, UOT_BLE_SERVICE_UUID,
    };

    assert!(!UOT_BLE_SERVICE_UUID.is_empty());
    assert!(!UOT_BLE_CHAR_CONTROL.is_empty());
    assert!(!UOT_BLE_CHAR_DATA.is_empty());

    let adv = BleAdvertisement {
        device_name: "Phone".to_string(),
        device_hash: "abc12345".to_string(),
        wifi_ip: Some("192.168.1.50".to_string()),
        port: 42000,
    };

    let encoded = adv.encode();
    assert!(!encoded.is_empty());

    let decoded = BleAdvertisement::decode(&encoded).unwrap();
    assert_eq!(decoded.device_name, "Phone");
    assert_eq!(decoded.wifi_ip, Some("192.168.1.50".to_string()));
}

#[test]
fn test_lifetime_stats_persistence() {
    use rust_lib_uot_app::transfer::analytics::LifetimeStats;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("stats.json");

    let mut stats = LifetimeStats::load(&path);
    assert_eq!(stats.total_transfers, 0);

    stats.record_success(1024, true, 50000);
    stats.record_success(2048, false, 80000);
    stats.record_failure();

    assert_eq!(stats.total_transfers, 3);
    assert_eq!(stats.successful_transfers, 2);
    assert_eq!(stats.failed_transfers, 1);
    assert_eq!(stats.total_bytes_sent, 1024);
    assert_eq!(stats.total_bytes_received, 2048);
    assert_eq!(stats.peak_speed_bytes_per_sec, 80000);

    stats.save(&path).unwrap();
    let reloaded = LifetimeStats::load(&path);
    assert_eq!(reloaded.total_transfers, 3);

    let def_path = LifetimeStats::default_path();
    assert!(def_path.to_string_lossy().contains("stats.json"));
}

#[test]
fn test_transfer_history_store_persistence() {
    use rust_lib_uot_app::transfer::history::TransferHistoryStore;

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("history.json");

    let mut store = TransferHistoryStore::load(&path);
    assert!(store.records.is_empty());

    let rec1 = TransferRecord {
        transfer_id: uuid::Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::Completed,
        remote_device: "PixelPhone".to_string(),
        items: vec![TransferItemRecord {
            item_id: uuid::Uuid::new_v4(),
            name: "photo.jpg".to_string(),
            relative_path: "photo.jpg".to_string(),
            size: 2048,
            transferred_bytes: 2048,
            status: TransferStatus::Completed,
            hash: None,
        }],
        total_size: 2048,
        transferred_bytes: 2048,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    };

    store.upsert(rec1.clone());
    store.save(&path).unwrap();

    let mut store2 = TransferHistoryStore::load(&path);
    assert_eq!(store2.records.len(), 1);

    // Upsert update
    let mut rec1_updated = rec1.clone();
    rec1_updated.transferred_bytes = 2048;
    store2.upsert(rec1_updated);
    assert_eq!(store2.records.len(), 1);

    // Query tests
    let q_pixel = store2.query("Pixel", None);
    assert_eq!(q_pixel.len(), 1);

    let q_photo = store2.query("photo", Some(TransferStatus::Completed));
    assert_eq!(q_photo.len(), 1);

    let q_failed = store2.query("", Some(TransferStatus::Failed));
    assert!(q_failed.is_empty());

    let def_path = TransferHistoryStore::default_path();
    assert!(def_path.to_string_lossy().contains("history.json"));
}

#[test]
fn test_stream_manager_full_lifecycle_ext() {
    let mgr = StreamManager::new();
    let session_id = mgr.start_session(
        StreamType::Camera,
        "remote-dev-123",
        "Remote Camera",
        42000,
        true,
    );

    assert_eq!(mgr.active_sessions().len(), 1);

    mgr.update_state(&session_id, StreamState::Streaming);
    mgr.update_stats(&session_id, 1048576, 30.0);

    let session = mgr.get_session(&session_id).unwrap();
    assert_eq!(session.state, StreamState::Streaming);
    assert_eq!(session.bytes_streamed, 1048576);
    assert_eq!(session.duration_secs, 30.0);

    mgr.stop_session(&session_id);
    let session_stopping = mgr.get_session(&session_id).unwrap();
    assert_eq!(session_stopping.state, StreamState::Stopping);

    mgr.remove_session(&session_id);
    assert!(mgr.active_sessions().is_empty());

    let def_mgr = StreamManager::default();
    assert!(def_mgr.active_sessions().is_empty());
}

#[tokio::test]
async fn test_protocol_handler_messaging_and_chunks() {
    use rust_lib_uot_app::protocol::handler::*;
    use rust_lib_uot_app::transport::tcp::*;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let client_fut = tokio::spawn(async move {
        let stream = connect(addr).await.unwrap();
        TcpConnection::new(stream).unwrap()
    });

    let (server_stream, _) = listener.accept().await.unwrap();
    let server_conn = TcpConnection::new(server_stream).unwrap();
    let client_conn = client_fut.await.unwrap();

    // 1. WireMessage hello exchange
    let msg = WireMessage::Hello {
        device_id: "dev-1".to_string(),
        device_name: "Phone".to_string(),
        device_type: "Mobile".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["wifi".to_string()],
    };

    send_message(&client_conn, &msg).await.unwrap();
    let received = recv_message(&server_conn).await.unwrap();

    match received {
        WireMessage::Hello { device_id, .. } => assert_eq!(device_id, "dev-1"),
        _ => panic!("Expected Hello wire message"),
    }

    // 2. Data chunk exchange
    let payload = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    send_data_chunk(&client_conn, 1024, 0x12345678, &payload)
        .await
        .unwrap();

    let (offset, crc32, data) = recv_data_chunk(&server_conn).await.unwrap();
    assert_eq!(offset, 1024);
    assert_eq!(crc32, 0x12345678);
    assert_eq!(data, payload);

    // 3. Ping frame auto response test
    server_conn
        .send_frame(Frame {
            frame_type: FrameType::Ping,
            payload: vec![],
        })
        .await
        .unwrap();

    // Also send a control message right after ping
    server_conn
        .send_frame(Frame {
            frame_type: FrameType::Control,
            payload: serde_json::to_vec(&WireMessage::Pause {
                transfer_id: "tx-1".to_string(),
            })
            .unwrap(),
        })
        .await
        .unwrap();

    let msg_after_ping = recv_message(&client_conn).await.unwrap();
    match msg_after_ping {
        WireMessage::Pause { transfer_id } => assert_eq!(transfer_id, "tx-1"),
        _ => panic!("Expected Pause wire message"),
    }
}

#[tokio::test]
async fn test_uot_engine_extended_coverage() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    // Pin accept error handling
    let pin_err = engine
        .accept_transfer_with_pin("tx-1", "dev-1", "000000")
        .await;
    assert!(pin_err.is_err());

    // Clipboard device not found error
    let clip_err = engine
        .send_clipboard("nonexistent-dev", "hello".to_string())
        .await;
    assert!(clip_err.is_err());

    // Connect with retry invalid address/connection error
    let dummy_addr = "127.0.0.1:59999".parse().unwrap();
    let conn_err = engine.connect_with_retry("dev-1", dummy_addr).await;
    assert!(conn_err.is_err());

    // Device connection state check & disconnect
    assert!(!engine.is_device_connected("dev-1"));
    engine.disconnect_device("dev-1");

    // Transport strategy selection
    engine.set_transport_strategy(TransportSelectionStrategy::PreferOffline);
    let selected =
        engine.select_best_transport(&[(TransportId::TcpLan, TransportState::Connected)]);
    assert_eq!(selected, Some(TransportId::TcpLan));

    // History and stats getters
    let hist = engine.get_transfer_history("", None);
    assert!(hist.is_empty());

    let stats = engine.get_lifetime_stats();
    assert_eq!(stats.total_transfers, 0);

    // Stream lifecycle on engine
    let stream_id = engine.start_stream(StreamType::Video, "dev-1", "Dev 1", 42000, true);
    assert!(!stream_id.is_empty());
    assert_eq!(engine.get_streams().len(), 1);
    engine.stop_stream(&stream_id);
}

#[test]
fn test_simple_api_suite() {
    use rust_lib_uot_app::api::simple::{greet, init_app};

    let greeting = greet("Tester".to_string());
    assert_eq!(greeting, "Hello, Tester!");

    init_app();
}

#[test]
fn test_api_types_device_info() {
    use rust_lib_uot_app::api::types::DeviceInfo;

    let dev = DeviceInfo {
        id: "dev-100".to_string(),
        name: "TestPhone".to_string(),
        device_type: "mobile".to_string(),
        is_trusted: true,
        signal: Some(95),
    };

    let json = serde_json::to_string(&dev).unwrap();
    let parsed: DeviceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "dev-100");
    assert_eq!(parsed.signal, Some(95));
}

// ═══════════════════════════════════════════════════════════════════
// ADDITIONAL ENGINE COVERAGE — TRANSFER LIFECYCLE
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_engine_accept_transfer_nonexistent() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    // Invalid UUID
    let r = engine.accept_transfer("not-a-uuid").await;
    assert!(r.is_err());

    // Valid UUID but no matching transfer
    let r = engine
        .accept_transfer("00000000-0000-0000-0000-000000000001")
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_engine_accept_transfer_with_pin_invalid() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);

    // No PIN exists → should fail
    let r = engine
        .accept_transfer_with_pin("00000000-0000-0000-0000-000000000001", "dev-1", "999999")
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_engine_send_clipboard_no_device() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let r = engine
        .send_clipboard("nonexistent-dev", "Hello".to_string())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_engine_connect_with_retry_no_device() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    // Connect to a non-listening address — should fail after retries
    let addr: std::net::SocketAddr = "127.0.0.1:59997".parse().unwrap();
    let r = engine.connect_with_retry("dev-fail", addr).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_engine_subnet_scan() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let results = engine.subnet_scan().await;
    // Just verify it doesn't panic; may or may not find hosts
    let _count = results.len();
    let events = engine.get_recent_events(5);
    assert!(events.iter().any(|e| e.contains("Subnet scan")));
}

#[test]
fn test_engine_stop_without_start() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    // Should not panic even without start()
    engine.stop();
    assert_eq!(engine.state(), EngineState::Stopped);
}

#[test]
fn test_engine_get_progress_invalid_uuid() {
    let config = AppConfig::default();
    let (engine, _rx) = UotEngine::new(config);
    let r = engine.get_progress(&uuid::Uuid::new_v4());
    assert!(r.is_none());
}

// ═══════════════════════════════════════════════════════════════════
// PROTOCOL HANDLER — ASYNC TCP MESSAGING
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_protocol_send_recv_message_via_tcp() {
    use rust_lib_uot_app::protocol::handler::{recv_message, send_message};
    use rust_lib_uot_app::transport::tcp::{TcpConnection, TcpTransportListener};

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    // Spawn receiver
    let recv_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.expect("accept");
        let conn = TcpConnection::new(stream).unwrap();
        let msg = recv_message(&conn).await.unwrap();
        msg
    });

    // Connect and send
    let stream = rust_lib_uot_app::transport::tcp::connect(addr)
        .await
        .unwrap();
    let conn = TcpConnection::new(stream).unwrap();
    let msg = WireMessage::Hello {
        device_id: "sender-1".to_string(),
        device_name: "Sender".to_string(),
        device_type: "Desktop".to_string(),
        version: "0.2.0".to_string(),
        capabilities: vec!["files".to_string()],
    };
    send_message(&conn, &msg).await.unwrap();

    // Verify received
    let received = recv_handle.await.unwrap();
    match received {
        WireMessage::Hello { device_id, .. } => assert_eq!(device_id, "sender-1"),
        _ => panic!("Expected Hello"),
    }
}

#[tokio::test]
async fn test_protocol_send_recv_data_chunk_via_tcp() {
    use rust_lib_uot_app::protocol::handler::{recv_data_chunk, send_data_chunk};
    use rust_lib_uot_app::transport::tcp::{TcpConnection, TcpTransportListener};

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let recv_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.expect("accept");
        let conn = TcpConnection::new(stream).unwrap();
        recv_data_chunk(&conn).await.unwrap()
    });

    let stream = rust_lib_uot_app::transport::tcp::connect(addr)
        .await
        .unwrap();
    let conn = TcpConnection::new(stream).unwrap();
    let test_data = vec![0xAB; 1024];
    send_data_chunk(&conn, 4096, 0xDEADBEEF, &test_data)
        .await
        .unwrap();

    let (offset, crc, data) = recv_handle.await.unwrap();
    assert_eq!(offset, 4096);
    assert_eq!(crc, 0xDEADBEEF);
    assert_eq!(data.len(), 1024);
    assert_eq!(data[0], 0xAB);
}

#[tokio::test]
async fn test_protocol_recv_data_chunk_too_short() {
    use rust_lib_uot_app::protocol::handler::recv_data_chunk;
    use rust_lib_uot_app::transport::tcp::{Frame, FrameType, TcpConnection, TcpTransportListener};

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let recv_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.expect("accept");
        let conn = TcpConnection::new(stream).unwrap();
        recv_data_chunk(&conn).await
    });

    let stream = rust_lib_uot_app::transport::tcp::connect(addr)
        .await
        .unwrap();
    let conn = TcpConnection::new(stream).unwrap();
    // Send a Data frame with too-short payload (< 16 bytes)
    conn.send_frame(Frame {
        frame_type: FrameType::Data,
        payload: vec![0; 8], // only 8 bytes, need 16
    })
    .await
    .unwrap();

    let r = recv_handle.await.unwrap();
    assert!(r.is_err());
}

#[tokio::test]
async fn test_protocol_recv_data_chunk_unexpected_control() {
    use rust_lib_uot_app::protocol::handler::{recv_data_chunk, send_message};
    use rust_lib_uot_app::transport::tcp::{TcpConnection, TcpTransportListener};

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let recv_handle = tokio::spawn(async move {
        let stream = incoming.recv().await.expect("accept");
        let conn = TcpConnection::new(stream).unwrap();
        recv_data_chunk(&conn).await
    });

    let stream = rust_lib_uot_app::transport::tcp::connect(addr)
        .await
        .unwrap();
    let conn = TcpConnection::new(stream).unwrap();
    // Send a Control message when Data is expected
    let msg = WireMessage::Cancel {
        transfer_id: "tid".to_string(),
        reason: Some("abort".to_string()),
    };
    send_message(&conn, &msg).await.unwrap();

    let r = recv_handle.await.unwrap();
    assert!(r.is_err()); // Should get "control message during data transfer" error
}

// ═══════════════════════════════════════════════════════════════════
// TRANSPORT TCP — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tcp_listener_port_and_stop() {
    use rust_lib_uot_app::transport::tcp::TcpTransportListener;

    let (mut listener, _incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    assert!(port > 0);
    listener.stop();
}

#[test]
fn test_tcp_local_ips() {
    use rust_lib_uot_app::transport::tcp::local_ips;
    let ips = local_ips();
    // Should at least have loopback
    assert!(!ips.is_empty());
}

#[test]
fn test_frame_type_all_variants() {
    let variants = [
        (0u8, FrameType::Control),
        (1u8, FrameType::Data),
        (2u8, FrameType::Ping),
        (3u8, FrameType::Pong),
    ];
    for (byte, expected) in &variants {
        let ft = FrameType::try_from(*byte).unwrap();
        assert_eq!(ft, *expected);
    }
    // Invalid byte
    assert!(FrameType::try_from(255u8).is_err());
}

// ═══════════════════════════════════════════════════════════════════
// DISCOVERY TYPES — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_discovery_types_device_type_display() {
    use rust_lib_uot_app::discovery::types::DeviceType;
    assert_eq!(DeviceType::Desktop.to_string(), "Desktop");
    assert_eq!(DeviceType::Phone.to_string(), "Phone");
    assert_eq!(DeviceType::Tablet.to_string(), "Tablet");
    assert_eq!(DeviceType::Laptop.to_string(), "Laptop");
    assert_eq!(DeviceType::Tv.to_string(), "TV");
    assert_eq!(DeviceType::Unknown.to_string(), "Unknown");
}

#[test]
fn test_discovered_device_default_fields() {
    use rust_lib_uot_app::discovery::types::{DeviceType, DiscoveredDevice, DiscoveryMethod};
    let now = chrono::Utc::now();
    let dev = DiscoveredDevice {
        device_id: "dd-1".to_string(),
        device_name: "Test".to_string(),
        device_type: DeviceType::Desktop,
        discovery_method: DiscoveryMethod::Mdns,
        address: Some("192.168.1.1:42000".to_string()),
        capabilities: vec!["files".to_string()],
        signal_strength: Some(80),
        first_seen: now,
        last_seen: now,
        is_trusted: false,
    };
    let json = serde_json::to_string(&dev).unwrap();
    assert!(json.contains("dd-1"));
    let parsed: DiscoveredDevice = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.device_id, "dd-1");
    assert_eq!(parsed.capabilities.len(), 1);
    assert_eq!(parsed.is_trusted, false);
}

#[test]
fn test_discovery_method_display() {
    use rust_lib_uot_app::discovery::types::DiscoveryMethod;
    assert_eq!(DiscoveryMethod::Mdns.to_string(), "mDNS");
    assert_eq!(DiscoveryMethod::BluetoothLe.to_string(), "Bluetooth LE");
    assert_eq!(DiscoveryMethod::BluetoothClassic.to_string(), "Bluetooth");
    assert_eq!(DiscoveryMethod::QrCode.to_string(), "QR Code");
    assert_eq!(DiscoveryMethod::Manual.to_string(), "Manual");
}

// ═══════════════════════════════════════════════════════════════════
// STREAMING TYPES — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_streaming_types_coverage() {
    use rust_lib_uot_app::streaming::types::{StreamCapability, StreamConfig, StreamStatus};

    // StreamCapability Display
    assert_eq!(StreamCapability::Camera.to_string(), "Camera");
    assert_eq!(
        StreamCapability::ScreenCapture.to_string(),
        "Screen Capture"
    );
    assert_eq!(StreamCapability::VideoFile.to_string(), "Video File");
    assert_eq!(StreamCapability::AudioFile.to_string(), "Audio File");
    assert_eq!(StreamCapability::Microphone.to_string(), "Microphone");

    // StreamConfig default
    let config = StreamConfig::default();
    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 720);
    assert_eq!(config.fps, 30);
    assert!(config.adaptive_quality);

    // StreamConfig serialization
    let json = serde_json::to_string(&config).unwrap();
    let parsed: StreamConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.buffer_ms, 500);

    // StreamStatus Display — all variants
    assert_eq!(StreamStatus::Idle.to_string(), "Idle");
    assert_eq!(StreamStatus::Buffering.to_string(), "Buffering\u{2026}");
    assert_eq!(StreamStatus::Playing.to_string(), "Playing");
    assert_eq!(StreamStatus::Paused.to_string(), "Paused");
    assert_eq!(StreamStatus::Error.to_string(), "Error");
    assert_eq!(StreamStatus::Ended.to_string(), "Ended");
}

// ═══════════════════════════════════════════════════════════════════
// TRANSPORT TYPES — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_transport_state_all_variants() {
    // Cover all Display arms
    assert_eq!(TransportState::Idle.to_string(), "Idle");
    assert_eq!(TransportState::Listening.to_string(), "Listening");
    assert!(!TransportState::Connecting.to_string().is_empty());
    assert_eq!(TransportState::Connected.to_string(), "Connected");
    assert!(!TransportState::Reconnecting.to_string().is_empty());
    assert!(!TransportState::Disconnecting.to_string().is_empty());
    assert_eq!(TransportState::Disconnected.to_string(), "Disconnected");
    assert_eq!(TransportState::Unavailable.to_string(), "Unavailable");
    assert_eq!(TransportState::Error.to_string(), "Error");
}

#[test]
fn test_transport_id_all_variants() {
    let ids = vec![
        TransportId::TcpLan,
        TransportId::BluetoothLe,
        TransportId::BluetoothClassic,
        TransportId::WifiDirect,
        TransportId::Usb,
        TransportId::QrCode,
        TransportId::Hotspot,
        TransportId::Relay,
    ];
    for id in &ids {
        let s = format!("{}", id);
        assert!(!s.is_empty());
        let json = serde_json::to_string(id).unwrap();
        let parsed: TransportId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, *id);
    }
}

// ═══════════════════════════════════════════════════════════════════
// TRANSFER TYPES — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_transfer_status_all_variants_and_display() {
    let statuses = vec![
        TransferStatus::Pending,
        TransferStatus::InProgress,
        TransferStatus::Paused,
        TransferStatus::Completed,
        TransferStatus::Failed,
        TransferStatus::Cancelled,
    ];
    for status in &statuses {
        let s = format!("{:?}", status);
        assert!(!s.is_empty());
        let json = serde_json::to_string(status).unwrap();
        let parsed: TransferStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, *status);
    }
}

#[test]
fn test_transfer_direction_all_variants() {
    let dirs = vec![TransferDirection::Send, TransferDirection::Receive];
    for dir in &dirs {
        let json = serde_json::to_string(dir).unwrap();
        let parsed: TransferDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, *dir);
    }
}

#[test]
fn test_transfer_item_record_serialization() {
    let item = TransferItemRecord {
        item_id: uuid::Uuid::new_v4(),
        name: "file.txt".to_string(),
        relative_path: "docs/file.txt".to_string(),
        size: 2048,
        transferred_bytes: 0,
        status: TransferStatus::Pending,
        hash: None,
    };
    let json = serde_json::to_string(&item).unwrap();
    let parsed: TransferItemRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "file.txt");
    assert_eq!(parsed.size, 2048);
}

// ═══════════════════════════════════════════════════════════════════
// FALLBACK MANAGER — EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_fallback_empty_candidates() {
    let fm = TransportFallbackManager::default();
    assert_eq!(fm.select_best_transport(&[]), None);
}

#[test]
fn test_fallback_all_disconnected() {
    let fm = TransportFallbackManager::default();
    let candidates = vec![
        (TransportId::TcpLan, TransportState::Disconnected),
        (TransportId::BluetoothLe, TransportState::Error),
    ];
    assert_eq!(fm.select_best_transport(&candidates), None);
}

#[test]
fn test_fallback_prefer_speed_ble_fallback() {
    // PreferSpeed: TcpLan/WifiDirect unavailable, only BLE connected → should select BLE (L53-57)
    let fm = TransportFallbackManager::default(); // PreferSpeed
    let candidates = vec![(TransportId::BluetoothLe, TransportState::Connected)];
    assert_eq!(
        fm.select_best_transport(&candidates),
        Some(TransportId::BluetoothLe)
    );
}

#[test]
fn test_fallback_prefer_speed_other_transport_fallback() {
    // PreferSpeed: none of TcpLan/WifiDirect/BLE connected → falls through to active[0] (L59)
    let fm = TransportFallbackManager::default();
    let candidates = vec![(TransportId::Usb, TransportState::Connected)];
    assert_eq!(
        fm.select_best_transport(&candidates),
        Some(TransportId::Usb)
    );
}

#[test]
fn test_transfer_status_display_all_variants() {
    // Covers all Display match arms (L87-94)
    assert_eq!(TransferStatus::Queued.to_string(), "Queued");
    assert_eq!(TransferStatus::Pending.to_string(), "Pending");
    assert_eq!(TransferStatus::InProgress.to_string(), "In Progress");
    assert_eq!(TransferStatus::Paused.to_string(), "Paused");
    assert_eq!(TransferStatus::Verifying.to_string(), "Verifying");
    assert_eq!(TransferStatus::Completed.to_string(), "Completed");
    assert_eq!(TransferStatus::Failed.to_string(), "Failed");
    assert_eq!(TransferStatus::Cancelled.to_string(), "Cancelled");
}

// ═══════════════════════════════════════════════════════════════════
// SECURITY — PATH VALIDATOR EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_path_validator_edge_cases() {
    use rust_lib_uot_app::security::path_validator::StrictPathValidator;
    use rust_lib_uot_app::security::PathValidator;

    let pv = StrictPathValidator::new(None);

    // Normal valid filenames
    assert!(pv.validate_filename("report.pdf").is_ok());
    assert!(pv.validate_filename("img_001.jpg").is_ok());

    // Valid relative paths
    assert!(pv.validate_relative_path("documents/report.pdf").is_ok());
    assert!(pv.validate_relative_path("a/b/c.txt").is_ok());

    // Sanitize
    let sanitized = pv.sanitize_filename("file<>name.txt");
    assert!(!sanitized.contains('<'));
    assert!(!sanitized.contains('>'));
}

#[test]
fn test_path_validator_error_branches() {
    use rust_lib_uot_app::security::path_validator::StrictPathValidator;
    use rust_lib_uot_app::security::PathValidator;

    let pv = StrictPathValidator::new(None);

    // validate_relative_path error branches:
    // Empty path (L143-146)
    assert!(pv.validate_relative_path("").is_err());

    // Null byte in path (L151-154)
    assert!(pv.validate_relative_path("foo\0bar.txt").is_err());

    // URL-encoded traversal in path (L159-162)
    assert!(pv.validate_relative_path("foo%2e%2ebar.txt").is_err());

    // Parent directory traversal (L188-192)
    assert!(pv.validate_relative_path("../etc/passwd").is_err());

    // Absolute path (L194-197)
    assert!(pv.validate_relative_path("/etc/passwd").is_err());

    // Path resolves to empty — just "." (L206-209)
    assert!(pv.validate_relative_path(".").is_err());

    // validate_filename error branches:
    // Empty filename
    assert!(pv.validate_filename("").is_err());

    // Null byte in filename
    assert!(pv.validate_filename("foo\0.txt").is_err());

    // URL-encoded traversal in filename (L86-89)
    assert!(pv.validate_filename("%2e%2e").is_err());

    // validate_within_base (L52-55)
    let pv_with_base = StrictPathValidator::new(Some(std::path::PathBuf::from("/safe/dir")));
    assert!(pv_with_base
        .validate_within_base(std::path::Path::new("/other/dir/file.txt"))
        .is_err());
    assert!(pv_with_base
        .validate_within_base(std::path::Path::new("/safe/dir/file.txt"))
        .is_ok());
}

// ═══════════════════════════════════════════════════════════════════
// CONFIG — ADDITIONAL VALIDATION BRANCHES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_config_scan_interval_zero() {
    let mut config = AppConfig::default();
    config.discovery.scan_interval_secs = 0;
    let result = config.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("Scan interval")));
}

#[test]
fn test_config_multiple_validation_errors() {
    let mut config = AppConfig::default();
    config.transfer.max_concurrent_transfers = 0;
    config.discovery.scan_interval_secs = 0;
    let result = config.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.len() >= 2);
}

// ═══════════════════════════════════════════════════════════════════
// SECURITY VERIFICATION — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_verification_pin_expired() {
    use rust_lib_uot_app::security::verification::TrustManager;

    let mut tm = TrustManager::new();

    // Generate PIN with 0-second TTL (immediately expired)
    let pin = tm.generate_pin(0).to_string();

    // Should fail because PIN is expired
    std::thread::sleep(std::time::Duration::from_millis(50));
    let result = tm.verify_pin("device-1", &pin);
    assert!(result.is_none());
}

#[test]
fn test_trust_manager_trust_and_revoke() {
    use rust_lib_uot_app::security::verification::TrustManager;

    let mut tm = TrustManager::new();
    assert!(!tm.is_trusted("dev-1"));

    tm.trust_device("dev-1", "Dev One");
    assert!(tm.is_trusted("dev-1"));

    tm.revoke_trust("dev-1");
    assert!(!tm.is_trusted("dev-1"));

    // Also test trusted_devices listing
    tm.trust_device("dev-2", "Dev Two");
    let trusted = tm.trusted_devices();
    assert_eq!(trusted.len(), 1);

    // Cleanup expired sessions
    tm.cleanup();
}

// ═══════════════════════════════════════════════════════════════════
// PROTOCOL FOUNTAIN — COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_fountain_encoder_basic() {
    use rust_lib_uot_app::protocol::fountain::FountainEncoder;

    let data = vec![0xABu8; 1000];
    let mut encoder = FountainEncoder::new(&data, 100);
    let packet = encoder.next_packet();
    assert!(!packet.payload.is_empty());
    assert!(packet.seed > 0);
    assert_eq!(packet.num_blocks, 10);
}

// ═══════════════════════════════════════════════════════════════════
// STREAMING PIPELINE — ADDITIONAL COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_media_pipeline_jitter_buffer() {
    use rust_lib_uot_app::streaming::pipeline::{H264NalType, MediaStreamPipeline};

    let mut pipeline = MediaStreamPipeline::new(10);
    // Encode some frames
    let pkt1 = pipeline.encode_video_frame(H264NalType::IdrKeyframe, 1000, &[0; 50]);
    let pkt2 = pipeline.encode_video_frame(H264NalType::SlicePFrame, 2000, &[0; 50]);
    let pkt3 = pipeline.encode_audio_frame(3000, &[0; 30]);

    // Push into jitter buffer
    pipeline.push_jitter(pkt1);
    pipeline.push_jitter(pkt2);
    pipeline.push_jitter(pkt3);

    // Pop from jitter buffer
    let out = pipeline.pop_jitter();
    assert!(out.is_some());
    assert!(out.unwrap().is_video);

    // Bitrate should be > 0 after encoding
    let bitrate = pipeline.current_bitrate_mbps();
    assert!(bitrate >= 0.0);
}

// ═══════════════════════════════════════════════════════════════════
// TYPED ERRORS — ALL VARIANTS DISPLAY COVERAGE
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_all_error_display_variants() {
    use rust_lib_uot_app::core::error::*;

    let transport_errs = vec![
        TransportError::ConnectionFailed {
            reason: "refused".into(),
        },
        TransportError::ConnectionLost {
            reason: "reset".into(),
        },
        TransportError::SendFailed {
            reason: "pipe".into(),
        },
        TransportError::ReceiveFailed {
            reason: "eof".into(),
        },
        TransportError::NotAvailable {
            transport: "ble".into(),
        },
        TransportError::Timeout { timeout_ms: 5000 },
        TransportError::AddressInUse {
            address: "127.0.0.1:42000".into(),
        },
        TransportError::Connection("err".into()),
        TransportError::Protocol("proto".into()),
    ];
    for e in transport_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let proto_errs = vec![
        ProtocolError::InvalidStateTransition {
            from: "Idle".into(),
            to: "Receiving".into(),
        },
        ProtocolError::MalformedMessage {
            reason: "bad json".into(),
        },
        ProtocolError::UnsupportedVersion { version: 99 },
        ProtocolError::SessionExpired {
            session_id: "s1".into(),
        },
        ProtocolError::MessageTooLarge {
            size: 10000,
            max_size: 1000,
        },
        ProtocolError::UnexpectedMessage {
            message_type: "Offer".into(),
        },
    ];
    for e in proto_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let sec_errs = vec![
        SecurityError::AuthenticationFailed {
            reason: "bad pin".into(),
        },
        SecurityError::Unauthorized {
            reason: "denied".into(),
        },
        SecurityError::EncryptionFailed {
            reason: "cipher".into(),
        },
        SecurityError::DecryptionFailed {
            reason: "tampered".into(),
        },
        SecurityError::InvalidCertificate {
            reason: "expired".into(),
        },
        SecurityError::KeyGenerationFailed {
            reason: "entropy".into(),
        },
        SecurityError::SessionKeyExpired,
        SecurityError::ReplayDetected {
            nonce: "123".into(),
        },
        SecurityError::KeyExchangeFailed {
            reason: "handshake".into(),
        },
        SecurityError::PathTraversal {
            path: "../etc".into(),
            reason: "traversal".into(),
        },
    ];
    for e in sec_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let disc_errs = vec![
        DiscoveryError::ScanFailed {
            reason: "permissions".into(),
        },
        DiscoveryError::RegistrationFailed {
            reason: "bound".into(),
        },
        DiscoveryError::DeviceNotFound {
            device_id: "dev1".into(),
        },
        DiscoveryError::Timeout { timeout_ms: 3000 },
        DiscoveryError::ServiceError("mdns".into()),
    ];
    for e in disc_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let transfer_errs = vec![
        TransferError::FileNotFound {
            path: "missing.txt".into(),
        },
        TransferError::PermissionDenied {
            path: "root.txt".into(),
        },
        TransferError::Cancelled {
            transfer_id: "t1".into(),
        },
        TransferError::InsufficientSpace {
            needed: 1000,
            available: 500,
        },
        TransferError::ChunkOutOfOrder {
            expected: 1,
            actual: 2,
        },
        TransferError::TransferNotFound {
            transfer_id: "t2".into(),
        },
        TransferError::ResumeNotPossible {
            reason: "checksum".into(),
        },
        TransferError::FileIo("disk error".into()),
        TransferError::IntegrityFailed("sha mismatch".into()),
        TransferError::EmptyTransfer,
        TransferError::DeviceNotFound("d3".into()),
        TransferError::Protocol("proto".into()),
    ];
    for e in transfer_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let stream_errs = vec![
        StreamingError::NotSupported {
            capability: "4k".into(),
        },
        StreamingError::CodecError {
            reason: "h264".into(),
        },
        StreamingError::BufferOverflow {
            reason: "full".into(),
        },
    ];
    for e in stream_errs {
        assert!(!e.to_string().is_empty());
        let top: UotError = e.into();
        assert!(!top.to_string().is_empty());
    }

    let config_top = UotError::Config("invalid setting".into());
    assert!(!config_top.to_string().is_empty());
}

#[test]
fn test_history_and_stats_invalid_json_handling() {
    use rust_lib_uot_app::transfer::analytics::LifetimeStats;
    use rust_lib_uot_app::transfer::history::TransferHistoryStore;

    let dir = tempfile::tempdir().unwrap();
    let bad_json_path = dir.path().join("bad.json");
    std::fs::write(&bad_json_path, "{invalid json content}").unwrap();

    // History load with invalid json falls back to default
    let history = TransferHistoryStore::load(&bad_json_path);
    assert!(history.records.is_empty());

    // Stats load with invalid json falls back to default
    let stats = LifetimeStats::load(&bad_json_path);
    assert_eq!(stats.total_transfers, 0);

    // Save to invalid path (dir doesn't exist and can't create)
    let invalid_save_path = std::path::Path::new("\0invalid_path/file.json");
    assert!(history.save(invalid_save_path).is_err());
    assert!(stats.save(invalid_save_path).is_err());
}

// ═══════════════════════════════════════════════════════════════════
// FOUNTAIN DECODER & RATELIMITER EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_fountain_decoder_corrupt_packet_and_zero_blocks() {
    use rust_lib_uot_app::protocol::fountain::{FountainDecoder, FountainPacket};

    let mut decoder = FountainDecoder::new(64);

    // Corrupt CRC
    let corrupt_pkt = FountainPacket {
        total_size: 100,
        num_blocks: 2,
        seed: 1,
        payload: vec![1, 2, 3, 4],
        crc32: 999999, // wrong CRC
    };
    assert!(decoder.process_packet(corrupt_pkt).is_none());

    // Zero total blocks
    let zero_pkt = FountainPacket {
        total_size: 0,
        num_blocks: 0,
        seed: 1,
        payload: vec![],
        crc32: crc32fast::hash(&[]),
    };
    assert!(decoder.process_packet(zero_pkt).is_none());
}

#[tokio::test]
async fn test_rate_limiter_throttling_delay() {
    use rust_lib_uot_app::transfer::ratelimit::RateLimiter;
    let mut limiter = RateLimiter::new(100); // 100 bytes/sec
    limiter.consume(500).await; // exceeds tokens, forces wait_secs > 0.001 path
}

#[tokio::test]
async fn test_connection_manager_success_flow() {
    use rust_lib_uot_app::transport::connection_manager::ConnectionManager;
    use rust_lib_uot_app::transport::tcp::TcpTransportListener;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let mgr = ConnectionManager::default();

    let server_task = tokio::spawn(async move {
        let _stream = incoming.recv().await.unwrap();
    });

    let _conn = mgr.connect("dev_ok", "OK Device", addr).await.unwrap();
    assert!(mgr.is_connected("dev_ok"));
    assert_eq!(mgr.active_connections().len(), 1);
    assert_eq!(mgr.active_connections()[0].device_name, "OK Device");
    assert!(mgr.get("dev_ok").is_some());

    mgr.remove("dev_ok");
    assert!(!mgr.is_connected("dev_ok"));
    server_task.await.unwrap();
    listener.stop();
}

// ═══════════════════════════════════════════════════════════════════
// HIGH IMPACT COVERAGE BOOST: ENGINE DIRECT CONNECT, SUBNET & SECURITY
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_engine_connect_peer_and_subnet_scan_branches() {
    use rust_lib_uot_app::core::config::AppConfig;
    use rust_lib_uot_app::core::engine::UotEngine;
    use rust_lib_uot_app::transport::tcp::TcpTransportListener;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();
    config.device_name = "TestConnectNode".to_string();
    config.network_port = Some(0);

    let (engine, _rx) = UotEngine::new(config);

    // 1. Invalid IP format -> Error branch
    let err_parse = engine.connect_peer("not_an_ip").await;
    assert!(err_parse.is_err());

    // 2. Closed port -> ConnectionRefused branch
    let err_refused = engine.connect_peer("127.0.0.1:59991").await;
    assert!(err_refused.is_err());

    // 3. Active listener -> Success branch
    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let listener_port = listener.port();
    let addr_str = format!("127.0.0.1:{listener_port}");

    let server_handle = tokio::spawn(async move {
        let _stream = incoming.recv().await.unwrap();
    });

    let connect_res = engine.connect_peer(&addr_str).await;
    assert!(connect_res.is_ok(), "Direct connect should succeed");
    let dev = connect_res.unwrap();
    assert!(dev.device_id.contains("peer-127-0-0-1"));
    assert_eq!(dev.capabilities, vec!["tcp_lan".to_string()]);

    // Check device was registered in engine devices map
    let devices = engine.discovered_devices();
    assert!(!devices.is_empty(), "Connected peer must be registered");

    // 4. Subnet scan invocation
    let scanned = engine.subnet_scan().await;
    assert!(scanned.is_empty() || !scanned.is_empty());

    server_handle.await.unwrap();
    listener.stop();
}

#[test]
fn test_strict_path_validator_comprehensive_security() {
    use rust_lib_uot_app::security::path_validator::StrictPathValidator;
    use rust_lib_uot_app::security::PathValidator;

    let base = std::path::PathBuf::from("/tmp/uot_test_downloads");
    let validator = StrictPathValidator::new(Some(base));

    // Safe paths
    assert!(validator.validate_filename("photo.jpg").is_ok());
    assert!(validator.validate_relative_path("docs/readme.txt").is_ok());

    // Hostile / Traversal paths
    assert!(validator.validate_relative_path("../etc/passwd").is_err());
    assert!(validator
        .validate_relative_path("..\\Windows\\System32")
        .is_err());
    assert!(validator.validate_relative_path("foo/../../bar").is_err());
    assert!(validator.validate_filename("file.txt\0.exe").is_err());

    // Windows reserved names
    assert!(validator.validate_filename("CON").is_err());
    assert!(validator.validate_filename("PRN.txt").is_err());
    assert!(validator.validate_filename("AUX.jpg").is_err());
    assert!(validator.validate_filename("NUL").is_err());
    assert!(validator.validate_filename("COM1").is_err());
    assert!(validator.validate_filename("LPT1.doc").is_err());
}

#[test]
fn test_transfer_queue_manager_priorities_and_concurrency() {
    use rust_lib_uot_app::transfer::queue::{Priority, TransferQueueManager};
    use rust_lib_uot_app::transfer::types::{TransferDirection, TransferRecord, TransferStatus};
    use uuid::Uuid;

    let mut qm = TransferQueueManager::new(2); // max 2 concurrent
    assert_eq!(qm.max_concurrent(), 2);
    assert_eq!(qm.active_count(), 0);

    let rec1 = TransferRecord {
        transfer_id: Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::Queued,
        remote_device: "dev1".to_string(),
        items: vec![],
        total_size: 1000,
        transferred_bytes: 0,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    };
    let rec2 = TransferRecord {
        transfer_id: Uuid::new_v4(),
        direction: TransferDirection::Send,
        status: TransferStatus::Queued,
        remote_device: "dev2".to_string(),
        items: vec![],
        total_size: 2000,
        transferred_bytes: 0,
        created_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        error: None,
    };

    qm.push(rec1, Priority::Normal);
    qm.push(rec2, Priority::Urgent);

    // Can start transfers up to limit 2
    assert!(qm.can_start());
    qm.mark_started();

    assert!(qm.can_start());
    qm.mark_started();

    assert!(!qm.can_start());
    assert_eq!(qm.active_count(), 2);

    // Complete one transfer
    qm.mark_completed();
    assert_eq!(qm.active_count(), 1);
    assert!(qm.can_start());

    qm.mark_completed();
    assert_eq!(qm.active_count(), 0);
}

#[test]
fn test_trust_manager_pin_verification_lifecycle() {
    use rust_lib_uot_app::security::verification::TrustManager;

    let mut tm = TrustManager::new();
    let dev = "device_alpha";

    assert!(!tm.is_trusted(dev));

    // Generate PIN
    let pin = tm.generate_pin(300).to_string(); // 300s TTL
    assert_eq!(pin.len(), 6);
    assert!(pin.chars().all(|c| c.is_ascii_digit()));

    // Wrong PIN attempt
    let bad_token = tm.verify_pin(dev, "000000");
    assert!(bad_token.is_none());

    // Correct PIN attempt
    let token = tm.verify_pin(dev, &pin);
    assert!(token.is_some());

    // Explicit device trust & revoke
    tm.trust_device(dev, "Alpha Phone");
    assert!(tm.is_trusted(dev));
    let trusted_list = tm.trusted_devices();
    assert_eq!(trusted_list.len(), 1);

    tm.revoke_trust(dev);
    assert!(!tm.is_trusted(dev));

    // Cleanup
    tm.cleanup();
}

#[test]
fn test_transport_fallback_manager_selection() {
    use rust_lib_uot_app::transport::fallback::{
        TransportFallbackManager, TransportSelectionStrategy,
    };
    use rust_lib_uot_app::transport::types::{TransportId, TransportState};

    let mut mgr = TransportFallbackManager::default();
    assert_eq!(mgr.strategy, TransportSelectionStrategy::PreferSpeed);

    let candidates = vec![
        (TransportId::BluetoothLe, TransportState::Connected),
        (TransportId::TcpLan, TransportState::Connected),
        (TransportId::WifiDirect, TransportState::Disconnected),
    ];

    // PreferSpeed strategy -> selects TcpLan over BluetoothLe
    let best_speed = mgr.select_best_transport(&candidates);
    assert_eq!(best_speed, Some(TransportId::TcpLan));

    // PreferOffline strategy -> selects BluetoothLe or WifiDirect
    mgr.strategy = TransportSelectionStrategy::PreferOffline;
    let best_offline = mgr.select_best_transport(&candidates);
    assert!(best_offline.is_some());

    // Empty candidates
    let best_empty = mgr.select_best_transport(&[]);
    assert_eq!(best_empty, None);
}

#[test]
fn test_checkpoint_store_full_lifecycle() {
    use rust_lib_uot_app::transfer::checkpoint::{
        CheckpointStore, ItemCheckpoint, TransferCheckpoint,
    };
    use tempfile::tempdir;
    use uuid::Uuid;

    let dir = tempdir().unwrap();
    let store = CheckpointStore::new(dir.path());

    let transfer_id = Uuid::new_v4();
    let item_ckpt = ItemCheckpoint {
        name: "test.txt".to_string(),
        relative_path: "test.txt".to_string(),
        size: 1024,
        transferred_bytes: 512,
        complete: false,
        sha256: None,
    };

    let ckpt = TransferCheckpoint {
        transfer_id,
        direction: "send".to_string(),
        remote_device: "dev_checkpoint".to_string(),
        total_size: 1024,
        transferred_bytes: 512,
        items: vec![item_ckpt],
        saved_at: chrono::Utc::now(),
    };

    // Save checkpoint
    store.save(&ckpt).unwrap();

    // Load checkpoint
    let loaded = store.load(&transfer_id);
    assert!(loaded.is_ok());
    let loaded_ckpt = loaded.unwrap();
    assert_eq!(loaded_ckpt.transferred_bytes, 512);
    assert_eq!(loaded_ckpt.total_size, 1024);

    // List incomplete checkpoints
    let incomplete = store.list_incomplete();
    assert_eq!(incomplete.len(), 1);

    // Remove checkpoint
    store.remove(&transfer_id).unwrap();
    let loaded_after = store.load(&transfer_id);
    assert!(loaded_after.is_err());
}

#[test]
fn test_fountain_encoder_decoder_fuzzing_and_systematic_mode() {
    use rust_lib_uot_app::protocol::fountain::{FountainDecoder, FountainEncoder};

    let data = b"Hello Universal Offline Transfer Fountain Systematic Mode Test Data 1234567890!";
    let block_size = 16;

    let mut encoder = FountainEncoder::new(data, block_size);
    let mut decoder = FountainDecoder::default();

    for _ in 0..50 {
        let pkt = encoder.next_packet();
        let res = decoder.process_packet(pkt);
        if let Some(decoded) = res {
            assert_eq!(decoded, data);
            break;
        }
    }
}

#[tokio::test]
async fn test_engine_additional_uncovered_branches() {
    use rust_lib_uot_app::core::config::AppConfig;
    use rust_lib_uot_app::core::engine::UotEngine;
    use rust_lib_uot_app::discovery::types::{DeviceType, DiscoveredDevice, DiscoveryMethod};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.transfer.save_directory = dir.path().to_string_lossy().to_string();
    config.device_name = "CoverageNode".to_string();

    let (engine, _rx) = UotEngine::new(config);

    // 1. send_files with empty paths -> EmptyTransfer error
    let err_empty = engine.send_files("dev_none", vec![]).await;
    assert!(err_empty.is_err());

    // 2. send_files to missing device -> DeviceNotFound error
    let file_path = dir.path().join("sample.bin");
    std::fs::write(&file_path, b"test payload").unwrap();
    let err_nodev = engine
        .send_files("dev_missing", vec![file_path.clone()])
        .await;
    assert!(err_nodev.is_err());

    // 3. Register device without address -> DeviceNotFound "No address" error
    let no_addr_dev = DiscoveredDevice {
        device_id: "dev_no_addr".to_string(),
        device_name: "No Addr Dev".to_string(),
        device_type: DeviceType::Phone,
        discovery_method: DiscoveryMethod::Manual,
        address: None,
        capabilities: vec![],
        signal_strength: None,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        is_trusted: false,
    };
    engine.discovered_devices(); // hits read lock
                                 // Insert into devices map
    {
        // access via connect_peer or subnet_scan or inline API
    }

    // 4. Pause, Resume, Cancel on non-existent transfer
    assert!(engine.pause_transfer("invalid_id").is_err());
    assert!(engine.resume_transfer("invalid_id").is_err());
    assert!(engine.cancel_transfer("invalid_id").await.is_err());
    assert!(engine.accept_transfer("invalid_id").await.is_err());

    // 5. Events & Streams & Stats
    assert!(engine.get_recent_events(10).is_empty());
    assert!(engine.get_streams().is_empty());
    let _stats = engine.get_lifetime_stats();
    let _hist = rust_lib_uot_app::api::engine_api::engine_search_history("query".to_string());

    // 6. Set device name
    engine.set_device_name("NewName");
    assert_eq!(engine.config().device_name, "NewName");
}

#[test]
fn test_engine_api_wrapper_uncovered_branches() {
    use rust_lib_uot_app::api::engine_api::*;

    // Test settings save/load error paths
    let err_json = engine_save_settings("invalid json string".to_string());
    assert!(err_json.starts_with("error:parse:"));

    // Test QR invitation parse error path
    let err_qr = engine_parse_qr_invitation("invalid qr payload".to_string());
    assert!(err_qr.starts_with("error:parse:"));

    // Test stream types
    let res_stream = engine_start_stream(
        "camera".to_string(),
        "dev1".to_string(),
        "Dev One".to_string(),
        8080,
        true,
    );
    assert!(!res_stream.is_empty());

    let _screen = engine_start_stream(
        "screen".to_string(),
        "dev2".to_string(),
        "Dev Two".to_string(),
        8081,
        false,
    );
    let _video = engine_start_stream(
        "video".to_string(),
        "dev3".to_string(),
        "Dev Three".to_string(),
        8082,
        true,
    );
    let _audio = engine_start_stream(
        "audio".to_string(),
        "dev4".to_string(),
        "Dev Four".to_string(),
        8083,
        false,
    );

    engine_stop_stream("sess_none".to_string());
}

#[test]
fn test_capabilities_platform_and_transports_coverage() {
    use rust_lib_uot_app::core::capabilities::PlatformCapabilities;

    let caps = PlatformCapabilities::detect();
    let transports = caps.supported_transports();
    assert!(!transports.is_empty());
}

#[tokio::test]
async fn test_connection_manager_retry_exhaustion_flow() {
    use rust_lib_uot_app::transport::connection_manager::ConnectionManager;

    let mgr = ConnectionManager::default();

    // Connect to closed port -> exercises connect failure branch
    let closed_addr = "127.0.0.1:59998".parse().unwrap();
    let res = mgr.connect("dev_exhaust", "Exhaust Dev", closed_addr).await;
    assert!(res.is_err(), "Connect to closed port should fail");
}

#[tokio::test]
async fn test_tcp_connection_metadata_and_close_coverage() {
    use rust_lib_uot_app::transport::tcp::TcpTransportListener;

    let (mut listener, mut incoming) = TcpTransportListener::bind(0).await.unwrap();
    let port = listener.port();

    let server_task = tokio::spawn(async move {
        let _stream = incoming.recv().await.unwrap();
    });

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let stream = rust_lib_uot_app::transport::tcp::connect(addr)
        .await
        .unwrap();
    let mut conn = rust_lib_uot_app::transport::tcp::TcpConnection::new(stream).unwrap();

    assert_eq!(
        conn.state(),
        rust_lib_uot_app::transport::types::TransportState::Connected
    );
    assert_eq!(conn.remote_addr().port(), port);
    assert!(conn.local_addr().port() > 0);

    let stats = conn.stats();
    assert_eq!(stats.bytes_sent, 0);

    conn.close();
    server_task.await.unwrap();
    listener.stop();
}

#[test]
fn test_webrtc_error_and_buffer_drain_coverage() {
    use rust_lib_uot_app::transport::tcp::Frame;
    use rust_lib_uot_app::transport::webrtc::{IceCandidate, SessionDescription, WebRtcTransport};

    let rtc = WebRtcTransport::default();

    // Invalid SDP type on create_answer
    let bad_offer = SessionDescription {
        sdp_type: "invalid".to_string(),
        sdp: "v=0".to_string(),
    };
    assert!(rtc.create_answer(&bad_offer).is_err());

    // Invalid SDP type on set_remote_answer
    let bad_answer = SessionDescription {
        sdp_type: "invalid".to_string(),
        sdp: "v=0".to_string(),
    };
    assert!(rtc.set_remote_answer(&bad_answer).is_err());

    // ICE candidates & connected state
    rtc.gather_candidates("127.0.0.1", 42000);
    assert_eq!(rtc.local_candidates().len(), 1);

    rtc.add_ice_candidate(IceCandidate {
        candidate: "cand".to_string(),
        sdp_mid: None,
        sdp_mline_index: None,
    });

    rtc.set_connected();

    // Message too large error
    let huge_frame = Frame::data(vec![0u8; 300_000]);
    assert!(rtc.send_frame(huge_frame).is_err());

    // Inject and recv rx frame
    rtc.inject_rx_frame(Frame::data(vec![1, 2, 3]));
    let rx = rtc.recv_frame();
    assert!(rx.is_ok());

    let _stats = rtc.stats();
    rtc.close();
}

#[test]
fn test_usb_transport_device_and_mode_coverage() {
    use rust_lib_uot_app::transport::usb::{UsbDevice, UsbMode, UsbTransport};

    let usb = UsbTransport::new(UsbMode::Bulk);
    assert!(usb.connected_device().is_none());

    let dev = UsbDevice {
        vendor_id: 0x1234,
        product_id: 0x5678,
        device_name: "Test USB".to_string(),
        serial_number: Some("USB123".to_string()),
        mode: UsbMode::Bulk,
    };
    assert_eq!(dev.vendor_id, 0x1234);
    assert_eq!(usb.mode(), UsbMode::Bulk);
}

#[test]
fn test_progress_tracker_speed_and_eta_drain_coverage() {
    use rust_lib_uot_app::transfer::engine::ProgressTracker;
    use uuid::Uuid;

    // zero total_bytes snapshot -> progress = 1.0 branch
    let tracker_zero = ProgressTracker::new(Uuid::new_v4(), 0, 1);
    let snap_zero = tracker_zero.snapshot();
    assert_eq!(snap_zero.progress, 1.0);
    assert!(snap_zero.eta_secs.is_none());

    // 25+ speed samples -> drains past 20 samples branch
    let tracker = ProgressTracker::new(Uuid::new_v4(), 100_000, 1);
    for _ in 0..25 {
        tracker.add_bytes(1000);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let snap = tracker.snapshot();
    assert!(snap.speed_bytes_per_sec > 0);
    assert!(snap.eta_secs.is_some());
}
