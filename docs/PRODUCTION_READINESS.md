# Production Readiness Classification

> Updated: 2026-08-08 (Post-Validation Audit)

All features in Universal Offline Transfer (UOT) are strictly classified according to verifiable implementation and test evidence:

---

## 1. COMPLETE & PROVEN
- **AES-256-GCM Envelope Encryption**: Authenticated encryption with random 12-byte nonces and X25519 static Diffie-Hellman key exchange (`security/crypto.rs`).
- **StrictPathValidator**: Path traversal protection rejecting `..`, null-byte injections, URL-encoded traversal, Windows reserved filenames (`CON`, `NUL`), and symlink overwrite attempts (`security/path_validator.rs`).
- **TCP Length-Prefixed Transport**: Framed binary network protocol over TCP sockets with CRC32 integrity checks (`transport/tcp.rs`).
- **mDNS Service Discovery**: Automatic local network peer discovery via `_uot._tcp.local.` mDNS broadcasting (`discovery/mdns.rs`).
- **Subnet Fallback Scanner**: Active IPv4 /24 subnet scanner on port 42000 (`discovery/subnet.rs`).
- **File Transfer Engine**: Chunked file I/O, progress tracking, speed calculation, and SHA-256 integrity verification (`transfer/engine.rs`).
- **Transfer Queue Manager**: Priority scheduling (`Low`, `Normal`, `High`, `Urgent`) enforcing `max_concurrent_transfers` limits (`transfer/queue.rs`).
- **TrustManager & PIN Authentication**: 6-digit PIN verification, session token generation, and device trust lists (`security/verification.rs`, `core/engine.rs`).
- **Flutter Consent & PIN UI**: Interactive Material 3 offer dialog and incoming offer cards supporting accept/decline and PIN verification (`lib/src/features/receive/`).
- **Lifetime Analytics & Persistent History**: JSON store for cumulative transfer statistics and text-searchable transfer history (`transfer/analytics.rs`, `transfer/history.rs`).
- **Docker Multi-Node Simulation**: Multi-stage `Dockerfile` and `docker-compose.yml` subnet bridge network for protocol integration testing.

---

## 2. PLATFORM LIMITED (Requires Native Mobile / OS SDKs)
- **Bluetooth Low Energy (BLE) Transport**: GATT service definitions (`UOT_BLE_SERVICE_UUID`) and advertisement payload framing implemented in Rust (`transport/ble.rs`); requires Android CoreBluetooth / iOS native GATT host bridge.
- **Wi-Fi Direct P2P Group Negotiation**: P2P group configuration and credential structures (`transport/wifidirect.rs`); requires Android WifiP2pManager native binding.
- **Hotspot Access Point Assist**: Local AP configuration helper (`transport/hotspot.rs`); requires OS system network privileges.

---

## 3. PENDING (Future Milestone Roadmap)
- **Real Media Byte Streaming**: Stream session state tracking is complete (`streaming/manager.rs`); raw audio/video payload relay and hardware H.264/AAC codec pipeline are pending future milestone development.
- **Fountain Code QR Receiver**: Luby Transform (LT) encoder is complete (`protocol/fountain.rs`); camera QR decoder and fountain packet reconstruction pipeline are pending.
