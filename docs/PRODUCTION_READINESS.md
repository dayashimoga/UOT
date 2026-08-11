# UOT Production Readiness & Evidence Matrix

> Audited against actual source code and automated test evidence on 2026-08-11.

## Classification Summary

| Feature | Status | Automated Evidence |
|---------|--------|-------------------|
| **TCP/LAN File Transfer** | `PROVEN` | `e2e_two_peer_workflow.rs` automated dual-engine transfer with SHA-256 byte-for-byte hash equality |
| **AES-256-GCM + X25519 Encryption** | `PROVEN` | `security_tests.rs` full key exchange, encryption, decryption, tamper detection tests |
| **Hello/HelloAck Handshake** | `PROVEN` | `e2e_two_peer_workflow.rs` & `coverage_tests.rs` 5s handshake verification |
| **SHA-256 File Integrity Verification** | `PROVEN` | Source and destination file hashes match byte-for-byte in automated E2E test |
| **Instant Messaging & Event Delivery** | `PROVEN` | Real event polling & `ClipboardReceived` delivery verified in dual-peer workflow |
| **Path Traversal Protection** | `PROVEN` | `StrictPathValidator` with 15+ test cases |
| **QR Payload Parsing & Expiry** | `PROVEN` | `qr_payload_e2e_test.rs` & `qr_payload_parsing_test.dart` URI parameter extraction |
| **Transfer History Persistence** | `PROVEN` | `test_transfer_history_store_persistence` JSON disk save/load test |
| **Android Launch & Startup** | `EMULATOR-PROVEN` | `scripts/android_smoke_test.sh` & `full_workflow_test.dart` clean launch on Android emulator |
| **Windows Build & Execution** | `PROVEN` | Native Windows compilation, `netsh` elevation helper, and local suite execution |
| **Optical QR Barcode Adapter** | `SIMULATED` | Simulated camera barcode scanner (`platform_adapters_test.dart`) & JSON payload tests |
| **BLE GATT Transport** | `SIMULATED` | `BleGattAdapter` simulation mode; physical hardware validation required |
| **Wi-Fi Direct P2P Transport** | `SIMULATED` | `WifiDirectAdapter` simulation mode; physical hardware validation required |
| **Media Streaming Pipeline** | `PARTIAL` | NAL/ADTS framing & jitter buffer in Rust; real camera/mic/codec capture pending |
| **Physical Camera Sensor** | `HARDWARE-REQUIRED` | Requires physical device camera hardware |
| **Physical Radio Transports (BLE/P2P)** | `HARDWARE-REQUIRED` | Requires physical Android/iOS multi-device wireless radio hardware |

---

## Production Qualification Gates

- [x] **Rust Engine Compile & Clippy**: Clean pass with 0 warnings (`cargo clippy -- -D warnings`).
- [x] **Rust Formatting**: Clean pass (`cargo fmt -- --check`).
- [x] **Rust Test Suite**: 407+ tests passed, 0 failed.
- [x] **Automated Dual-Peer E2E Transfer**: `e2e_two_peer_workflow.rs` verifies complete transfer with SHA-256 equality.
- [x] **Network Fault Harness**: Closed port, timeout, expired PIN, and abrupt disconnect handling verified in `network_fault_harness.rs`.
- [x] **Flutter Analysis**: 0 issues found (`flutter analyze`).
- [x] **Flutter Tests**: 14 tests passed (`flutter test`).
- [x] **Android Emulator Smoke Test**: Script `scripts/android_smoke_test.sh` passes without logcat crashes.
- [x] **Self-Device Filter & Safety**: Local listener IP/port filtered out from discovery.
- [x] **Platform Firewall Guard**: Restricted to Windows OS; Android renders diagnostic instructions.
