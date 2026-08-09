# UOT Transport Matrix

> Audited against actual source code on 2026-08-09.

| Transport | Status | Implementation | Evidence |
|-----------|--------|---------------|----------|
| **TCP/LAN** | **COMPLETE & PROVEN** | Full framed TCP with reader/writer tasks, keepalive, length-prefixed framing | `rust/src/transport/tcp.rs` (526 lines), 5 unit tests, E2E transfer tests |
| **mDNS Discovery** | **COMPLETE & PROVEN** | `mdns-sd` based service registration and browsing | `rust/src/discovery/mdns.rs` (9.5KB), integrated into engine |
| **Subnet Scan** | **IMPLEMENTED BUT UNPROVEN** | TCP port probe scan of /24 subnet | `rust/src/discovery/subnet.rs`, no real-device validation |
| **BLE** | **NOT IMPLEMENTED** | Data structures only (`BleAdvertisement`, GATT UUIDs). No GATT client, no GATT server, no scan, no connect, no data transfer | `rust/src/transport/ble.rs` (37 lines — structs only) |
| **Wi-Fi Direct** | **NOT IMPLEMENTED** | Data structures only (`WifiDirectGroupInfo`). No P2P group negotiation, no connection | `rust/src/transport/wifidirect.rs` (51 lines — structs only) |
| **Hotspot** | **PARTIAL** | Configuration struct exists | `rust/src/transport/hotspot.rs` — not integrated |
| **Transport Fallback** | **PARTIAL** | Selection logic only (`TransportFallbackManager`). No actual session migration between transports at runtime | `rust/src/transport/fallback.rs` (96 lines) |
| **QUIC** | **NOT IMPLEMENTED** | Not in codebase | — |
| **WebRTC** | **NOT IMPLEMENTED** | Not in codebase | — |
| **QR Fountain Code** | **NOT IMPLEMENTED** | JSON QR invitation only, no fountain encoding/decoding | `rust/src/security/qr.rs` |
| **USB** | **NOT IMPLEMENTED** | Not in codebase | — |

## Kotlin/Dart Platform Adapters

| Adapter | Status | Notes |
|---------|--------|-------|
| `BleAdapterPlugin` (Kotlin) | **PARTIAL** | MethodChannel registered, underlying Android BLE calls may exist but not E2E validated |
| `WifiDirectAdapterPlugin` (Kotlin) | **PARTIAL** | MethodChannel registered, underlying Wi-Fi Direct calls may exist but not E2E validated |
| `camera_qr_adapter.dart` (Dart) | **PARTIAL** | QR scan UI exists, fountain code reconstruction not implemented |
