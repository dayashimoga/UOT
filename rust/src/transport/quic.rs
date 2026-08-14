//! QUIC Transport Implementation
//!
//! Provides QUIC-based transport for secure, multiplexed, low-latency transfers.
//! Uses `quinn` (pure Rust QUIC) with self-signed certificates for peer-to-peer mode.

use std::net::SocketAddr;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::error::TransportError;
use crate::transport::tcp::{Frame, FrameType};
use crate::transport::types::{TransportCapabilities, TransportState, TransportStats};

/// Default QUIC port.
pub const QUIC_PORT: u16 = 42001;

/// QUIC transport connection.
pub struct QuicTransport {
    state: RwLock<TransportState>,
    stats: RwLock<TransportStats>,
    local_addr: RwLock<Option<SocketAddr>>,
    endpoint: RwLock<Option<quinn::Endpoint>>,
}

impl QuicTransport {
    /// Create a new QUIC transport instance.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TransportState::Idle),
            stats: RwLock::new(TransportStats::default()),
            local_addr: RwLock::new(None),
            endpoint: RwLock::new(None),
        }
    }
}

impl Default for QuicTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl QuicTransport {
    /// Get transport capabilities.
    pub fn capabilities() -> TransportCapabilities {
        TransportCapabilities {
            bidirectional: true,
            reliable: true,
            requires_network: true,
            max_throughput: 500_000_000, // ~500 MB/s theoretical
            typical_latency_ms: 1,
            max_payload_size: 0, // Stream-based
            supports_streaming: true,
            supports_discovery: false,
            platforms: vec![
                "android".into(),
                "ios".into(),
                "windows".into(),
                "macos".into(),
                "linux".into(),
            ],
            is_simulated: false,
            requires_physical_hardware: false,
        }
    }

    /// Generate self-signed certificate for P2P mode.
    pub fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), TransportError> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let key_pair = rcgen::KeyPair::generate()
            .map_err(|e| TransportError::Connection(format!("Key gen: {e}")))?;
        let params = rcgen::CertificateParams::new(vec!["uot.local".into()])
            .map_err(|e| TransportError::Connection(format!("Params: {e}")))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| TransportError::Connection(format!("Cert gen: {e}")))?;
        let cert_der = cert.der().to_vec();
        let key_der = key_pair.serialize_der();
        Ok((cert_der, key_der))
    }

    /// Create a QUIC server endpoint.
    pub async fn listen(&self, port: u16) -> Result<SocketAddr, TransportError> {
        let (cert_der, key_der) = Self::generate_self_signed_cert()?;

        let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(key_der);

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key.into())
            .map_err(|e| TransportError::Connection(format!("TLS: {e}")))?;
        server_crypto.alpn_protocols = vec![b"uot".to_vec()];

        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .map_err(|e| TransportError::Connection(format!("QUIC config: {e}")))?,
        ));

        let endpoint =
            quinn::Endpoint::server(server_config, format!("0.0.0.0:{port}").parse().unwrap())
                .map_err(|e| TransportError::Connection(format!("Bind: {e}")))?;

        let addr = endpoint
            .local_addr()
            .map_err(|e| TransportError::Connection(format!("Addr: {e}")))?;

        *self.local_addr.write() = Some(addr);
        *self.endpoint.write() = Some(endpoint);
        *self.state.write() = TransportState::Listening;

        log::info!("QUIC listening on {addr}");
        Ok(addr)
    }

    /// Connect to a QUIC server.
    pub async fn connect(&self, addr: SocketAddr) -> Result<QuicConnection, TransportError> {
        let mut client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipVerification))
            .with_no_client_auth();

        client_crypto.alpn_protocols = vec![b"uot".to_vec()];

        let client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
                .map_err(|e| TransportError::Connection(format!("QUIC client: {e}")))?,
        ));

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| TransportError::Connection(format!("Client: {e}")))?;
        endpoint.set_default_client_config(client_config);

        *self.state.write() = TransportState::Connecting;

        let connection = endpoint
            .connect(addr, "uot.local")
            .map_err(|e| TransportError::Connection(format!("Connect: {e}")))?
            .await
            .map_err(|e| TransportError::ConnectionFailed {
                reason: format!("QUIC: {e}"),
            })?;

        *self.state.write() = TransportState::Connected;
        log::info!("QUIC connected to {addr}");

        Ok(QuicConnection { connection })
    }

    pub fn state(&self) -> TransportState {
        *self.state.read()
    }

    pub fn stats(&self) -> TransportStats {
        self.stats.read().clone()
    }
}

/// An active QUIC connection.
pub struct QuicConnection {
    connection: quinn::Connection,
}

impl QuicConnection {
    /// Open a bidirectional stream and send a frame.
    pub async fn send_frame(&self, frame: Frame) -> Result<(), TransportError> {
        let (mut send, _recv) =
            self.connection
                .open_bi()
                .await
                .map_err(|e| TransportError::SendFailed {
                    reason: format!("Stream: {e}"),
                })?;

        let encoded = frame.encode();
        send.write_all(&encoded)
            .await
            .map_err(|e| TransportError::SendFailed {
                reason: format!("Write: {e}"),
            })?;
        send.finish().map_err(|e| TransportError::SendFailed {
            reason: format!("Finish: {e}"),
        })?;

        Ok(())
    }

    /// Accept a bidirectional stream and read a frame.
    pub async fn recv_frame(&self) -> Result<Frame, TransportError> {
        let (_send, mut recv) =
            self.connection
                .accept_bi()
                .await
                .map_err(|e| TransportError::ReceiveFailed {
                    reason: format!("Accept: {e}"),
                })?;

        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| TransportError::ReceiveFailed {
                reason: format!("Read len: {e}"),
            })?;
        let payload_len = u32::from_be_bytes(len_buf) as usize;

        // Read type + payload
        let mut type_buf = [0u8; 1];
        recv.read_exact(&mut type_buf)
            .await
            .map_err(|e| TransportError::ReceiveFailed {
                reason: format!("Read type: {e}"),
            })?;
        let frame_type = FrameType::try_from(type_buf[0])?;

        let mut payload = vec![0u8; payload_len];
        recv.read_exact(&mut payload)
            .await
            .map_err(|e| TransportError::ReceiveFailed {
                reason: format!("Read payload: {e}"),
            })?;

        Ok(Frame {
            frame_type,
            payload,
        })
    }

    /// Close the connection.
    pub fn close(&self) {
        self.connection.close(0u32.into(), b"done");
    }

    /// Get remote address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.connection.remote_address()
    }
}

/// Skip TLS verification for P2P (we use our own encryption layer).
#[derive(Debug)]
struct SkipVerification;

impl rustls::client::danger::ServerCertVerifier for SkipVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_capabilities() {
        let caps = QuicTransport::capabilities();
        assert!(caps.bidirectional);
        assert!(caps.reliable);
        assert!(caps.supports_streaming);
        assert!(caps.max_throughput > 0);
    }

    #[test]
    fn test_quic_transport_creation() {
        let transport = QuicTransport::new();
        assert_eq!(transport.state(), TransportState::Idle);
        assert_eq!(transport.stats().bytes_sent, 0);
    }

    #[test]
    fn test_self_signed_cert_generation() {
        let (cert, key) = QuicTransport::generate_self_signed_cert().unwrap();
        assert!(!cert.is_empty());
        assert!(!key.is_empty());
    }

    #[tokio::test]
    async fn test_quic_listen_and_connect() {
        let server = QuicTransport::new();
        let addr = server.listen(0).await.unwrap(); // Port 0 = OS picks
        assert_eq!(server.state(), TransportState::Listening);

        let server_endpoint = server.endpoint.read().as_ref().unwrap().clone();
        let accept_handle = tokio::spawn(async move {
            if let Some(conn) = server_endpoint.accept().await {
                let _ = conn.await;
            }
        });

        let client_addr: SocketAddr = format!("127.0.0.1:{}", addr.port()).parse().unwrap();
        let client = QuicTransport::new();
        let conn = client.connect(client_addr).await.unwrap();
        assert_eq!(client.state(), TransportState::Connected);
        conn.close();
        let _ = accept_handle.await;
    }
}
