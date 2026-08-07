//! Protocol Messages
//!
//! Defines all message types exchanged between UOT peers.
//! Messages are versioned and serializable for wire transport.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Protocol message wrapper with header and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Message header (always present).
    pub header: MessageHeader,
    /// Message payload (type-specific).
    pub payload: MessagePayload,
}

/// Common message header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Unique message identifier.
    pub message_id: Uuid,
    /// Session identifier (set after session creation).
    pub session_id: Option<Uuid>,
    /// Protocol version.
    pub protocol_version: u32,
    /// Monotonically increasing sequence number (for replay protection).
    pub sequence: u64,
    /// Timestamp of message creation.
    pub timestamp: DateTime<Utc>,
    /// Sender device identifier.
    pub sender_id: String,
}

impl MessageHeader {
    /// Create a new message header.
    pub fn new(sender_id: String, session_id: Option<Uuid>, sequence: u64) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            session_id,
            protocol_version: crate::core::version::PROTOCOL_VERSION,
            sequence,
            timestamp: Utc::now(),
            sender_id,
        }
    }
}

/// All possible message payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    // === Discovery ===
    /// Device discovery announcement.
    Discover(DiscoverPayload),
    /// Response to a discovery announcement.
    DiscoverResponse(DiscoverResponsePayload),

    // === Pairing ===
    /// Initiate pairing with a device.
    PairRequest(PairRequestPayload),
    /// Response to a pairing request.
    PairResponse(PairResponsePayload),

    // === Session ===
    /// Create a new transfer session.
    CreateSession(CreateSessionPayload),
    /// Session creation acknowledgement.
    SessionCreated(SessionCreatedPayload),

    // === Transfer ===
    /// Offer to send files/data.
    Offer(OfferPayload),
    /// Accept or reject an offer.
    OfferResponse(OfferResponsePayload),
    /// Start the transfer.
    Start(StartPayload),
    /// A chunk of data.
    Chunk(ChunkPayload),
    /// Acknowledgement of received chunk(s).
    Ack(AckPayload),

    // === Control ===
    /// Pause the transfer.
    Pause(PausePayload),
    /// Resume the transfer.
    Resume(ResumePayload),
    /// Cancel the transfer.
    Cancel(CancelPayload),
    /// Request reconnection.
    Reconnect(ReconnectPayload),
    /// Retry a failed chunk.
    Retry(RetryPayload),

    // === Verification ===
    /// Request integrity verification.
    Verify(VerifyPayload),
    /// Verification result.
    VerifyResult(VerifyResultPayload),

    // === Completion ===
    /// Transfer completed.
    Complete(CompletePayload),
    /// Error notification.
    Error(ErrorPayload),

    // === Heartbeat ===
    /// Keep-alive ping.
    Ping,
    /// Keep-alive pong.
    Pong,
}

// === Payload Definitions ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverPayload {
    pub device_name: String,
    pub device_type: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResponsePayload {
    pub device_name: String,
    pub device_type: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequestPayload {
    pub device_name: String,
    pub public_key: Vec<u8>,
    pub qr_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResponsePayload {
    pub accepted: bool,
    pub public_key: Option<Vec<u8>>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionPayload {
    pub session_type: SessionType,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreatedPayload {
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Session types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    /// File transfer session.
    Transfer,
    /// Media streaming session.
    Streaming,
    /// Clipboard/text data session.
    Clipboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferPayload {
    pub transfer_id: Uuid,
    pub items: Vec<OfferItem>,
    pub total_size: u64,
}

/// Describes a single item in a transfer offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferItem {
    pub item_id: Uuid,
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub is_directory: bool,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferResponsePayload {
    pub transfer_id: Uuid,
    pub accepted: bool,
    pub reason: Option<String>,
    /// Specific items to skip (already exist, etc.).
    pub skip_items: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPayload {
    pub transfer_id: Uuid,
    /// Byte offset to resume from (0 for new transfers).
    pub resume_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPayload {
    pub transfer_id: Uuid,
    pub item_id: Uuid,
    pub chunk_index: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    pub checksum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPayload {
    pub transfer_id: Uuid,
    pub item_id: Uuid,
    pub chunk_index: u64,
    pub received_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PausePayload {
    pub transfer_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumePayload {
    pub transfer_id: Uuid,
    pub resume_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelPayload {
    pub transfer_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectPayload {
    pub session_id: Uuid,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPayload {
    pub transfer_id: Uuid,
    pub item_id: Uuid,
    pub chunk_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyPayload {
    pub transfer_id: Uuid,
    pub item_id: Uuid,
    pub hash_algorithm: String,
    pub expected_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResultPayload {
    pub transfer_id: Uuid,
    pub item_id: Uuid,
    pub verified: bool,
    pub actual_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePayload {
    pub transfer_id: Uuid,
    pub total_bytes: u64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub transfer_id: Option<Uuid>,
    pub error_code: u32,
    pub message: String,
    pub recoverable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_header_creation() {
        let header = MessageHeader::new("device-1".to_string(), None, 1);
        assert_eq!(header.sender_id, "device-1");
        assert_eq!(header.sequence, 1);
        assert!(header.session_id.is_none());
        assert_eq!(
            header.protocol_version,
            crate::core::version::PROTOCOL_VERSION
        );
    }

    #[test]
    fn test_protocol_message_serialization() {
        let msg = ProtocolMessage {
            header: MessageHeader::new("device-1".to_string(), None, 0),
            payload: MessagePayload::Ping,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ProtocolMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.header.sender_id, "device-1");
    }

    #[test]
    fn test_transfer_item_serialization() {
        let item = OfferItem {
            item_id: Uuid::new_v4(),
            name: "photo.jpg".to_string(),
            relative_path: "photos/photo.jpg".to_string(),
            size: 1024 * 1024,
            mime_type: Some("image/jpeg".to_string()),
            is_directory: false,
            hash: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: OfferItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "photo.jpg");
        assert_eq!(deserialized.size, 1024 * 1024);
    }

    #[test]
    fn test_offer_payload() {
        let offer = OfferPayload {
            transfer_id: Uuid::new_v4(),
            items: vec![OfferItem {
                item_id: Uuid::new_v4(),
                name: "test.txt".to_string(),
                relative_path: "test.txt".to_string(),
                size: 100,
                mime_type: Some("text/plain".to_string()),
                is_directory: false,
                hash: None,
            }],
            total_size: 100,
        };
        let json = serde_json::to_string(&offer).unwrap();
        assert!(json.contains("test.txt"));
    }

    #[test]
    fn test_session_type_serialization() {
        let st = SessionType::Transfer;
        let json = serde_json::to_string(&st).unwrap();
        let deserialized: SessionType = serde_json::from_str(&json).unwrap();
        assert_eq!(st, deserialized);
    }

    #[test]
    fn test_chunk_payload() {
        let chunk = ChunkPayload {
            transfer_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            chunk_index: 0,
            offset: 0,
            data: vec![1, 2, 3, 4],
            checksum: 0xDEAD_BEEF,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: ChunkPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.data, vec![1, 2, 3, 4]);
        assert_eq!(deserialized.checksum, 0xDEAD_BEEF);
    }
}
