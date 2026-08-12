//! TCP/LAN Transport Implementation
//!
//! Provides TCP-based transport for local network file transfer.
//! Uses length-prefixed framing for reliable message delivery.
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Buf, BufMut, BytesMut};
use parking_lot::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use crate::core::error::TransportError;
use crate::transport::types::{TransportState, TransportStats};

/// Default port for UOT TCP transport.
pub const DEFAULT_PORT: u16 = 42000;

/// Maximum message size (64 MB).
const MAX_MESSAGE_SIZE: u32 = 64 * 1024 * 1024;

/// Frame header: 4 bytes length + 1 byte message type.
const FRAME_HEADER_SIZE: usize = 5;

/// Message types for the wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// Protocol/control message (JSON).
    Control = 0,
    /// File data chunk (binary).
    Data = 1,
    /// Keepalive ping.
    Ping = 2,
    /// Keepalive pong.
    Pong = 3,
}

impl TryFrom<u8> for FrameType {
    type Error = TransportError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Control),
            1 => Ok(Self::Data),
            2 => Ok(Self::Ping),
            3 => Ok(Self::Pong),
            _ => Err(TransportError::Protocol(format!(
                "Unknown frame type: {value}"
            ))),
        }
    }
}

/// A framed message on the wire.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Type of this frame.
    pub frame_type: FrameType,
    /// Payload data.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new control frame from a JSON-serializable message.
    pub fn control(data: &[u8]) -> Self {
        Self {
            frame_type: FrameType::Control,
            payload: data.to_vec(),
        }
    }

    /// Create a new data frame for file chunks.
    pub fn data(data: Vec<u8>) -> Self {
        Self {
            frame_type: FrameType::Data,
            payload: data,
        }
    }

    /// Create a ping frame.
    pub fn ping() -> Self {
        Self {
            frame_type: FrameType::Ping,
            payload: Vec::new(),
        }
    }

    /// Create a pong frame.
    pub fn pong() -> Self {
        Self {
            frame_type: FrameType::Pong,
            payload: Vec::new(),
        }
    }

    /// Encode this frame into bytes (length-prefixed).
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u32;
        let mut buf = Vec::with_capacity(FRAME_HEADER_SIZE + self.payload.len());
        buf.put_u32(payload_len);
        buf.put_u8(self.frame_type as u8);
        buf.extend_from_slice(&self.payload);
        buf
    }
}

/// TCP connection wrapper with framed I/O.
pub struct TcpConnection {
    /// Remote address.
    remote_addr: SocketAddr,
    /// Local address.
    local_addr: SocketAddr,
    /// Connection state.
    state: Arc<RwLock<TransportState>>,
    /// Stats tracker.
    stats: Arc<RwLock<TransportStats>>,
    /// Sender for outgoing frames.
    tx: mpsc::Sender<Frame>,
    /// Receiver for incoming frames.
    incoming_rx: tokio::sync::Mutex<mpsc::Receiver<Frame>>,
    /// Shutdown signal.
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TcpConnection {
    /// Create a new connection from an established TCP stream.
    pub fn new(stream: TcpStream) -> Result<Self, TransportError> {
        let remote_addr = stream
            .peer_addr()
            .map_err(|e| TransportError::Connection(format!("No peer addr: {e}")))?;
        let local_addr = stream
            .local_addr()
            .map_err(|e| TransportError::Connection(format!("No local addr: {e}")))?;

        let state = Arc::new(RwLock::new(TransportState::Connected));
        let stats = Arc::new(RwLock::new(TransportStats::default()));

        let (tx, rx) = mpsc::channel::<Frame>(256);
        let (incoming_tx, incoming_rx) = mpsc::channel::<Frame>(256);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn reader/writer tasks
        let read_state = Arc::clone(&state);
        let read_stats = Arc::clone(&stats);
        let (reader, writer) = stream.into_split();

        // Writer task
        tokio::spawn(Self::writer_task(writer, rx));

        // Reader task (with write_tx for automatic Pong replies)
        tokio::spawn(Self::reader_task(
            reader,
            tx.clone(),
            incoming_tx,
            read_state,
            read_stats,
            shutdown_rx,
        ));

        Ok(Self {
            remote_addr,
            local_addr,
            state,
            stats,
            tx,
            incoming_rx: tokio::sync::Mutex::new(incoming_rx),
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Send a frame.
    pub async fn send(&self, frame: Frame) -> Result<(), TransportError> {
        let encoded_len = frame.payload.len() as u64;
        self.tx
            .send(frame)
            .await
            .map_err(|_| TransportError::Connection("Channel closed".to_string()))?;
        self.stats.write().bytes_sent += encoded_len;
        Ok(())
    }

    /// Send a frame (alias for protocol handler).
    pub async fn send_frame(&self, frame: Frame) -> Result<(), TransportError> {
        self.send(frame).await
    }

    /// Receive the next incoming frame.
    pub async fn recv_frame(&self) -> Result<Frame, TransportError> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| TransportError::Connection("Connection closed".to_string()))
    }

    /// Get current state.
    pub fn state(&self) -> TransportState {
        *self.state.read()
    }

    /// Get remote address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Get local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get connection stats.
    pub fn stats(&self) -> TransportStats {
        self.stats.read().clone()
    }

    /// Gracefully close the connection.
    pub fn close(&mut self) {
        *self.state.write() = TransportState::Disconnected;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Writer task: reads frames from channel and writes to TCP.
    async fn writer_task(
        mut writer: tokio::net::tcp::OwnedWriteHalf,
        mut rx: mpsc::Receiver<Frame>,
    ) {
        while let Some(frame) = rx.recv().await {
            let encoded = frame.encode();
            if writer.write_all(&encoded).await.is_err() {
                break;
            }
        }
    }

    /// Reader task: reads frames from TCP and sends to channel.
    async fn reader_task(
        mut reader: tokio::net::tcp::OwnedReadHalf,
        write_tx: mpsc::Sender<Frame>,
        tx: mpsc::Sender<Frame>,
        state: Arc<RwLock<TransportState>>,
        stats: Arc<RwLock<TransportStats>>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let mut buf = BytesMut::with_capacity(8192);

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                result = reader.read_buf(&mut buf) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            stats.write().bytes_received += n as u64;
                        }
                        Err(_) => break,
                    }
                }
            }

            // Try to decode frames from buffer
            while buf.len() >= FRAME_HEADER_SIZE {
                let payload_len = (&buf[..4]).get_u32() as usize;

                if payload_len > MAX_MESSAGE_SIZE as usize {
                    log::error!("Frame too large: {payload_len} bytes");
                    *state.write() = TransportState::Error;
                    return;
                }

                let total_frame_len = FRAME_HEADER_SIZE + payload_len;
                if buf.len() < total_frame_len {
                    break; // Need more data
                }

                // We have a full frame
                let _ = buf.split_to(4); // length
                let frame_type_byte = buf.split_to(1)[0];
                let payload = buf.split_to(payload_len).to_vec();

                let frame_type = match FrameType::try_from(frame_type_byte) {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                let frame = Frame {
                    frame_type,
                    payload,
                };

                // Handle ping/pong internally
                if frame.frame_type == FrameType::Ping {
                    let _ = write_tx.try_send(Frame::pong());
                    continue;
                }
                if frame.frame_type == FrameType::Pong {
                    log::trace!("Pong received");
                    continue;
                }

                if tx.send(frame).await.is_err() {
                    break;
                }
            }
        }

        *state.write() = TransportState::Disconnected;
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        self.close();
    }
}

/// TCP transport listener — accepts incoming connections.
pub struct TcpTransportListener {
    /// The address we're listening on.
    local_addr: SocketAddr,
    /// Shutdown signal.
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TcpTransportListener {
    /// Start listening on the given port (with automatic fallbacks to 42000-42004). Returns the listener and a receiver
    /// for incoming connections (each as a TcpStream).
    pub async fn bind(port: u16) -> Result<(Self, mpsc::Receiver<TcpStream>), TransportError> {
        let target_ports = if port == 0 {
            vec![DEFAULT_PORT, 42001, 42002, 42003, 42004, 0]
        } else {
            vec![port, DEFAULT_PORT, 42001, 42002, 42003, 42004, 0]
        };

        let mut bound_listener = None;
        let mut last_err = None;

        for p in target_ports {
            let addr = SocketAddr::from(([0, 0, 0, 0], p));
            match TcpListener::bind(addr).await {
                Ok(l) => {
                    bound_listener = Some(l);
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        let listener = bound_listener.ok_or_else(|| {
            TransportError::Connection(format!(
                "Bind failed on requested port {port} and fallbacks: {:?}",
                last_err
            ))
        })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| TransportError::Connection(format!("No local addr: {e}")))?;

        log::info!("TCP transport listening on {local_addr}");

        let (tx, rx) = mpsc::channel::<TcpStream>(32);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                log::info!("Incoming connection from {addr}");
                                if tx.send(stream).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!("Accept failed: {e}");
                            }
                        }
                    }
                }
            }
        });

        Ok((
            Self {
                local_addr,
                shutdown_tx: Some(shutdown_tx),
            },
            rx,
        ))
    }

    /// Get the listening address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get the listening port.
    pub fn port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Stop listening.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for TcpTransportListener {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Connect to a remote peer via TCP.
pub async fn connect(addr: SocketAddr) -> Result<TcpStream, TransportError> {
    let stream = TcpStream::connect(addr)
        .await
        .map_err(|e| TransportError::Connection(format!("Connect to {addr} failed: {e}")))?;

    stream
        .set_nodelay(true)
        .map_err(|e| TransportError::Connection(format!("Set nodelay failed: {e}")))?;

    log::info!("Connected to {addr}");
    Ok(stream)
}

/// Get the local IP addresses of this machine.
pub fn local_ips() -> Vec<std::net::IpAddr> {
    let mut ips = Vec::new();
    if let Ok(addrs) = std::net::UdpSocket::bind("0.0.0.0:0") {
        // Connect to a public address to determine local IP
        if addrs.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = addrs.local_addr() {
                ips.push(local.ip());
            }
        }
    }
    ips
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_encode_control() {
        let data = b"hello";
        let frame = Frame::control(data);
        let encoded = frame.encode();
        assert_eq!(encoded.len(), FRAME_HEADER_SIZE + data.len());
        // First 4 bytes: length = 5
        assert_eq!(&encoded[..4], &[0, 0, 0, 5]);
        // 5th byte: frame type = 0 (Control)
        assert_eq!(encoded[4], 0);
        // Payload
        assert_eq!(&encoded[5..], data);
    }

    #[test]
    fn test_frame_encode_data() {
        let data = vec![1, 2, 3, 4];
        let frame = Frame::data(data.clone());
        let encoded = frame.encode();
        assert_eq!(encoded[4], 1); // Data type
        assert_eq!(&encoded[5..], &data);
    }

    #[test]
    fn test_frame_encode_ping() {
        let frame = Frame::ping();
        let encoded = frame.encode();
        assert_eq!(encoded.len(), FRAME_HEADER_SIZE);
        assert_eq!(&encoded[..4], &[0, 0, 0, 0]); // Zero payload
        assert_eq!(encoded[4], 2); // Ping type
    }

    #[test]
    fn test_frame_type_try_from() {
        assert_eq!(FrameType::try_from(0).unwrap(), FrameType::Control);
        assert_eq!(FrameType::try_from(1).unwrap(), FrameType::Data);
        assert_eq!(FrameType::try_from(2).unwrap(), FrameType::Ping);
        assert_eq!(FrameType::try_from(3).unwrap(), FrameType::Pong);
        assert!(FrameType::try_from(99).is_err());
    }

    #[tokio::test]
    async fn test_tcp_listener_bind() {
        let (mut listener, _rx) = TcpTransportListener::bind(0).await.unwrap();
        assert_ne!(listener.port(), 0); // OS assigned a port
        listener.stop();
    }

    #[tokio::test]
    async fn test_tcp_connect_and_accept() {
        let (mut listener, mut rx) = TcpTransportListener::bind(0).await.unwrap();
        let port = listener.port();

        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        // Connect
        let client_stream = connect(addr).await.unwrap();
        assert_eq!(client_stream.peer_addr().unwrap().port(), port);

        // Accept
        let server_stream = rx.recv().await.unwrap();
        assert!(server_stream.peer_addr().is_ok());

        listener.stop();
    }

    #[tokio::test]
    async fn test_tcp_connection_send_receive() {
        let (mut listener, mut rx) = TcpTransportListener::bind(0).await.unwrap();
        let port = listener.port();
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        // Client connects
        let client_stream = connect(addr).await.unwrap();
        let server_stream = rx.recv().await.unwrap();

        // Create framed connections
        let client_conn = TcpConnection::new(client_stream).unwrap();
        let server_conn = TcpConnection::new(server_stream).unwrap();

        // Client sends a control frame
        let msg = b"{\"type\":\"hello\"}";
        client_conn.send(Frame::control(msg)).await.unwrap();

        // Server receives it
        let frame = server_conn.recv_frame().await.unwrap();
        assert_eq!(frame.frame_type, FrameType::Control);
        assert_eq!(&frame.payload, msg);

        listener.stop();
    }

    #[test]
    fn test_local_ips() {
        let ips = local_ips();
        // May be empty in CI, but should not panic
        for ip in &ips {
            assert!(ip.is_ipv4() || ip.is_ipv6());
        }
    }
}
