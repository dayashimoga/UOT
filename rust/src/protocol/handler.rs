//! Protocol Handler
//!
//! Handles serialization/deserialization of protocol messages
//! over TCP frames. Provides request-response and streaming APIs.
use serde::{Deserialize, Serialize};

use crate::core::error::TransportError;
use crate::transport::tcp::{Frame, FrameType, TcpConnection};

/// High-level protocol message types for the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WireMessage {
    /// Device announces itself.
    Hello {
        device_id: String,
        device_name: String,
        device_type: String,
        version: String,
        capabilities: Vec<String>,
    },
    /// Respond to hello.
    HelloAck {
        device_id: String,
        device_name: String,
        device_type: String,
        version: String,
    },
    /// Offer to send files.
    Offer {
        transfer_id: String,
        device_name: String,
        items: Vec<OfferItemInfo>,
        total_size: u64,
    },
    /// Respond to offer.
    OfferResponse {
        transfer_id: String,
        accepted: bool,
        reason: Option<String>,
    },
    /// Signal start of a file within a transfer.
    FileStart {
        transfer_id: String,
        item_index: u32,
        file_name: String,
        file_size: u64,
        relative_path: String,
    },
    /// Signal end of a file with hash.
    FileEnd {
        transfer_id: String,
        item_index: u32,
        sha256: String,
    },
    /// Signal transfer complete.
    TransferComplete { transfer_id: String, success: bool },
    /// Request to cancel a transfer.
    Cancel {
        transfer_id: String,
        reason: Option<String>,
    },
    /// Request to pause a transfer.
    Pause { transfer_id: String },
    /// Request to resume a transfer.
    Resume { transfer_id: String, offset: u64 },
    /// Clipboard/text data.
    ClipboardData { content_type: String, data: String },
    /// X25519 public key exchange for session encryption.
    KeyExchange { public_key: Vec<u8> },
}

/// Item info in an offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferItemInfo {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub is_directory: bool,
}

/// Send a protocol message over a TCP connection.
pub async fn send_message(conn: &TcpConnection, msg: &WireMessage) -> Result<(), TransportError> {
    let json = serde_json::to_vec(msg)
        .map_err(|e| TransportError::Protocol(format!("Failed to serialize message: {e}")))?;
    conn.send_frame(Frame {
        frame_type: FrameType::Control,
        payload: json,
    })
    .await
}

/// Receive a protocol message from a TCP connection.
pub async fn recv_message(conn: &TcpConnection) -> Result<WireMessage, TransportError> {
    let frame = conn.recv_frame().await?;
    match frame.frame_type {
        FrameType::Control => serde_json::from_slice(&frame.payload)
            .map_err(|e| TransportError::Protocol(format!("Failed to deserialize message: {e}"))),
        FrameType::Ping => {
            // Auto-reply to pings
            conn.send_frame(Frame {
                frame_type: FrameType::Pong,
                payload: vec![],
            })
            .await?;
            // Recursively get the next real message
            Box::pin(recv_message(conn)).await
        }
        _ => Err(TransportError::Protocol(format!(
            "Expected control frame, got {:?}",
            frame.frame_type
        ))),
    }
}

/// Send file data chunk over a TCP connection.
pub async fn send_data_chunk(
    conn: &TcpConnection,
    offset: u64,
    crc32: u32,
    data: &[u8],
) -> Result<(), TransportError> {
    // Data frame header: 8 bytes offset + 4 bytes CRC32 + 4 bytes reserved
    let mut header = Vec::with_capacity(16 + data.len());
    header.extend_from_slice(&offset.to_be_bytes());
    header.extend_from_slice(&crc32.to_be_bytes());
    header.extend_from_slice(&[0u8; 4]); // reserved
    header.extend_from_slice(data);

    conn.send_frame(Frame {
        frame_type: FrameType::Data,
        payload: header,
    })
    .await
}

/// Receive a file data chunk from a TCP connection.
/// Returns (offset, crc32, data).
pub async fn recv_data_chunk(conn: &TcpConnection) -> Result<(u64, u32, Vec<u8>), TransportError> {
    let frame = conn.recv_frame().await?;
    match frame.frame_type {
        FrameType::Data => {
            if frame.payload.len() < 16 {
                return Err(TransportError::Protocol("Data frame too short".to_string()));
            }
            let offset = u64::from_be_bytes(frame.payload[0..8].try_into().unwrap());
            let crc32 = u32::from_be_bytes(frame.payload[8..12].try_into().unwrap());
            let data = frame.payload[16..].to_vec();
            Ok((offset, crc32, data))
        }
        FrameType::Control => {
            // Might be a cancel/pause message during transfer
            let msg: WireMessage = serde_json::from_slice(&frame.payload)
                .map_err(|e| TransportError::Protocol(format!("Unexpected control: {e}")))?;
            Err(TransportError::Protocol(format!(
                "Received control message during data transfer: {:?}",
                msg
            )))
        }
        _ => Err(TransportError::Protocol(format!(
            "Expected data frame, got {:?}",
            frame.frame_type
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_message_serialization() {
        let msg = WireMessage::Hello {
            device_id: "test-123".to_string(),
            device_name: "Test Device".to_string(),
            device_type: "Desktop".to_string(),
            version: "0.2.0".to_string(),
            capabilities: vec!["files".to_string(), "clipboard".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"hello\""));
        assert!(json.contains("test-123"));

        let deserialized: WireMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            WireMessage::Hello { device_id, .. } => {
                assert_eq!(device_id, "test-123");
            }
            _ => panic!("Expected Hello"),
        }
    }

    #[test]
    fn test_offer_serialization() {
        let msg = WireMessage::Offer {
            transfer_id: "test-transfer-123".to_string(),
            device_name: "Test Device".to_string(),
            items: vec![OfferItemInfo {
                name: "test.txt".to_string(),
                relative_path: "test.txt".to_string(),
                size: 1024,
                is_directory: false,
            }],
            total_size: 1024,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"offer\""));

        let _: WireMessage = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_all_message_types_serialize() {
        let messages = vec![
            WireMessage::HelloAck {
                device_id: "id".to_string(),
                device_name: "name".to_string(),
                device_type: "Desktop".to_string(),
                version: "0.1.0".to_string(),
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
                sha256: "abc123".to_string(),
            },
            WireMessage::TransferComplete {
                transfer_id: "tid".to_string(),
                success: true,
            },
            WireMessage::Cancel {
                transfer_id: "tid".to_string(),
                reason: Some("user".to_string()),
            },
            WireMessage::Pause {
                transfer_id: "tid".to_string(),
            },
            WireMessage::Resume {
                transfer_id: "tid".to_string(),
                offset: 512,
            },
            WireMessage::ClipboardData {
                content_type: "text/plain".to_string(),
                data: "hello".to_string(),
            },
        ];

        for msg in &messages {
            let json = serde_json::to_string(msg).unwrap();
            let _: WireMessage = serde_json::from_str(&json).unwrap();
        }
    }
}
