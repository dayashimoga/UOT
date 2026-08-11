# UOT Gap Analysis & Feature Evidence Matrix

> Audited against actual source code and automated test evidence on 2026-08-11.

## Summary

The UOT codebase features a robust Rust core engine and Flutter UI with **PROVEN** end-to-end TCP/LAN transport, Hello/HelloAck bidirectional handshakes, X25519 authenticated sessions, AES-256-GCM encryption, live instant messaging, and SHA-256 verified file transfers.

Features are classified honestly based on empirical automated evidence according to the **6-Level Evidence Policy**.

---

## 6-Level Feature Classification Matrix

| Feature | Classification | Proof / Evidence |
|---------|----------------|------------------|
| **TCP/LAN Transport Engine** | `PROVEN` | Automated dual-peer integration test (`e2e_two_peer_workflow.rs`) with SHA-256 byte-for-byte verification |
| **Hello/HelloAck Handshake** | `PROVEN` | Automated dual-peer test verifying 5s handshake & state transitions (`SessionReady`) |
| **AES-256-GCM + X25519 Encryption** | `PROVEN` | Verified in unit & integration test suites (`security_tests.rs`) |
| **Instant Messaging & Delivery** | `PROVEN` | Real event polling & `ClipboardReceived` delivery verified in dual-peer workflow |
| **Direct IP Connect & QR Pair Parsing** | `PROVEN` | `qr_payload_e2e_test.rs` & `qr_payload_parsing_test.dart` URI parameter extraction |
| **Transfer History Persistence** | `PROVEN` | `test_transfer_history_store_persistence` JSON disk save/load test |
| **Lifetime Statistics & Rate Limiting** | `PROVEN` | `test_lifetime_stats_save_load_roundtrip` & token bucket rate-limiter tests |
| **Android Startup & UI Render** | `EMULATOR-PROVEN` | `scripts/android_smoke_test.sh` & `full_workflow_test.dart` on Android emulator |
| **Optical QR Fallback & Fountain** | `SIMULATED` | Simulated camera barcode scanner (`platform_adapters_test.dart`) & JSON payload tests |
| **BLE GATT Transport** | `SIMULATED` | `BleGattAdapter` simulation mode (`platform_adapters_test.dart`); requires hardware |
| **Wi-Fi Direct P2P Transport** | `SIMULATED` | `WifiDirectAdapter` simulation mode (`platform_adapters_test.dart`); requires hardware |
| **Media Streaming Pipeline** | `PARTIAL` | NAL/ADTS framing & jitter buffer in Rust; real camera/mic/codec capture pending |
| **Physical Camera Sensor** | `HARDWARE-REQUIRED` | Requires physical device camera hardware |
| **Physical Radio Transports (BLE/P2P)** | `HARDWARE-REQUIRED` | Requires physical Android/iOS multi-device wireless radio hardware |

---

## Proven Core Capabilities (P0)

1. **Protocol Handshake & Verification (`PROVEN`)**:
   - `connect_peer()` performs TCP connect → `Hello` → `HelloAck` → `Ping`/`Pong` handshake.
   - Prevents false-positive "Connected" states on raw socket connect.

2. **Verified File Transfer Pipeline (`PROVEN`)**:
   - Source-to-destination SHA-256 hash equality automated in `e2e_two_peer_workflow.rs`.
   - Chunked transfer with AES-256-GCM encryption and disk persistence.

3. **Real Live Instant Messaging (`PROVEN`)**:
   - `ClipboardData` framing delivered to receiver with real event polling (`ClipboardReceived`).

4. **Self-Device Filter & Safety (`PROVEN`)**:
   - Local listener IP/port filtered out from discovery list (`discovered_devices()`, `subnet_scan()`).

5. **Platform Firewall Guard (`PROVEN`)**:
   - `Fix Windows Firewall` button restricted to Windows OS; Android renders diagnostic instructions.

---

## Remaining Hardware-Dependent Requirements

| Feature | Requirement for Full Hardware Proof |
|---------|--------------------------------------|
| **Physical Camera QR** | Deployment onto physical Android/iOS device with camera sensor |
| **BLE GATT Scanning** | Two physical devices with BLE 4.2+ GATT hardware enabled |
| **Wi-Fi Direct Group Owner** | Two physical Android devices with Wi-Fi P2P hardware enabled |
