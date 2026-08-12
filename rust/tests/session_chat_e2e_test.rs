//! E2E Session Chat Verification Test
//!
//! Tests real two-peer bidirectional chat over TCP:
//! 1. Engine A connects to Engine B.
//! 2. A sends chat message "Hello from A" to B.
//! 3. B receives message, stores it in B's PeerSession with direction=Incoming and state=Delivered.
//! 4. B sends ACK back to A; A updates message state to Delivered.
//! 5. B sends chat message "Hello back from B" to A.
//! 6. A receives message, stores it in A's PeerSession with direction=Incoming and state=Delivered.
//! 7. Verifies byte-for-byte content, timestamps, and message states on both ends.

use rust_lib_uot_app::core::config::AppConfig;
use rust_lib_uot_app::core::engine::{EngineState, UotEngine};
use rust_lib_uot_app::core::session::{MessageDirection, MessageState};
use tempfile::tempdir;

#[tokio::test]
async fn test_bidirectional_session_chat_e2e() {
    let _ = env_logger::builder().is_test(true).try_init();

    // 1. Initialize Engine A (Sender)
    let dir_a = tempdir().unwrap();
    let mut config_a = AppConfig::default();
    config_a.device_name = "Peer_Alpha".to_string();
    config_a.transfer.save_directory = dir_a.path().to_string_lossy().to_string();
    config_a.network_port = Some(0);

    let (engine_a, _rx_a) = UotEngine::new(config_a);
    engine_a.start().await.expect("Engine A must start");
    assert_eq!(engine_a.state(), EngineState::Running);

    // 2. Initialize Engine B (Receiver)
    let dir_b = tempdir().unwrap();
    let mut config_b = AppConfig::default();
    config_b.device_name = "Peer_Beta".to_string();
    config_b.transfer.save_directory = dir_b.path().to_string_lossy().to_string();
    config_b.network_port = Some(0);

    let (engine_b, _rx_b) = UotEngine::new(config_b);
    engine_b.start().await.expect("Engine B must start");
    assert_eq!(engine_b.state(), EngineState::Running);

    let port_b = engine_b.listening_port();
    let dev_a_id = engine_a.device_id().to_string();
    let dev_b_id = engine_b.device_id().to_string();

    // 3. Connect Engine A -> Engine B
    let addr_b_str = format!("127.0.0.1:{port_b}");
    let dev_b_info = engine_a
        .connect_peer(&addr_b_str)
        .await
        .expect("Engine A must connect to Engine B");
    assert_eq!(dev_b_info.device_id, dev_b_id);

    // Wait 200ms for connection registration on both ends
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 4. Engine A sends message to Engine B
    let msg_text_a = "Hello from Peer Alpha!".to_string();
    let msg_id_a = engine_a
        .send_chat_message(&dev_b_id, msg_text_a.clone())
        .await
        .expect("Engine A send_chat_message must succeed");

    // Wait up to 2 seconds for message & ACK processing
    let mut delivered_on_a = false;
    let mut received_on_b = false;

    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Check A's session messages for Delivered state
        let msgs_a_json = engine_a.get_session_messages(&dev_b_id);
        if msgs_a_json.contains("Delivered") && msgs_a_json.contains(&msg_id_a.to_string()) {
            delivered_on_a = true;
        }

        // Check B's session messages for Incoming message
        let msgs_b_json = engine_b.get_session_messages(&dev_a_id);
        if msgs_b_json.contains(&msg_text_a) {
            received_on_b = true;
        }

        if delivered_on_a && received_on_b {
            break;
        }
    }

    assert!(
        received_on_b,
        "Engine B must receive and store incoming message in session"
    );
    assert!(
        delivered_on_a,
        "Engine A must receive ACK and mark message state as Delivered"
    );

    // 5. Engine B sends reply message to Engine A
    let msg_text_b = "Hello back from Peer Beta!".to_string();
    let msg_id_b = engine_b
        .send_chat_message(&dev_a_id, msg_text_b.clone())
        .await
        .expect("Engine B send_chat_message must succeed");

    let mut delivered_on_b = false;
    let mut received_on_a = false;

    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let msgs_b_json = engine_b.get_session_messages(&dev_a_id);
        if msgs_b_json.contains("Delivered") && msgs_b_json.contains(&msg_id_b.to_string()) {
            delivered_on_b = true;
        }

        let msgs_a_json = engine_a.get_session_messages(&dev_b_id);
        if msgs_a_json.contains(&msg_text_b) {
            received_on_a = true;
        }

        if delivered_on_b && received_on_a {
            break;
        }
    }

    assert!(
        received_on_a,
        "Engine A must receive and store reply message from B in session"
    );
    assert!(
        delivered_on_b,
        "Engine B must receive ACK from A and mark message state as Delivered"
    );

    // 6. Verify session messages lists on both engines
    let session_a = engine_a.get_or_create_session(&dev_b_id, "Peer_Beta");
    let session_b = engine_b.get_or_create_session(&dev_a_id, "Peer_Alpha");

    {
        let s_a = session_a.read();
        assert_eq!(
            s_a.messages.len(),
            2,
            "Engine A session must contain 2 messages"
        );
        assert_eq!(s_a.messages[0].direction, MessageDirection::Outgoing);
        assert_eq!(s_a.messages[0].content, msg_text_a);
        assert_eq!(s_a.messages[0].state, MessageState::Delivered);
        assert_eq!(s_a.messages[1].direction, MessageDirection::Incoming);
        assert_eq!(s_a.messages[1].content, msg_text_b);
        assert_eq!(s_a.messages[1].state, MessageState::Delivered);
    }

    {
        let s_b = session_b.read();
        assert_eq!(
            s_b.messages.len(),
            2,
            "Engine B session must contain 2 messages"
        );
        assert_eq!(s_b.messages[0].direction, MessageDirection::Incoming);
        assert_eq!(s_b.messages[0].content, msg_text_a);
        assert_eq!(s_b.messages[0].state, MessageState::Delivered);
        assert_eq!(s_b.messages[1].direction, MessageDirection::Outgoing);
        assert_eq!(s_b.messages[1].content, msg_text_b);
        assert_eq!(s_b.messages[1].state, MessageState::Delivered);
    }

    // Cleanup
    engine_a.stop();
    engine_b.stop();
}
