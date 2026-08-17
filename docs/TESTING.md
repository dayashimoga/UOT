# UOT Automated Testing System & Test Matrix

## Test Suite Hierarchy

| Category | Suite File / Command | Target Scope | Classification Level |
|----------|-----------------------|--------------|----------------------|
| **Dual-Peer E2E Workflow** | `cargo test --test e2e_two_peer_workflow` | TCP, Hello Handshake, X25519, Messaging, File Transfer, SHA-256 | `PROVEN` |
| **Network Fault Harness** | `cargo test --test network_fault_harness` | Closed Ports, Timeout, Expired PIN, Stream Drops | `PROVEN` |
| **Transport Lab E2E Suite** | `cargo test --test transport_lab_e2e` (10 tests) | Multi-Node, Pause/Resume/Retry, Fallback Hierarchy, Multi-Batch Isolation, Checkpoint Resume, Chat Stress | `PROVEN` |
| **QR Payload & Security** | `cargo test --test qr_payload_e2e_test` | QR JSON, URI Schema (`uot://pair`), Malformed Payload, Expiry | `PROVEN` |
| **Rust Unit & Lib Suite** | `cargo test --lib` (250+ tests) | Crypto, Protocol, Security, Queue, Engine | `PROVEN` |
| **Coverage & Edge Cases** | `cargo test --test coverage_tests` (165 tests) | Edge Cases, Multi-File E2E, Engine API Matrix, Chat/Clipboard Loopback, State Machine, Subnet Scan, Transports, Capabilities, Chunk I/O, Wire Messages, Session Cipher, QR, Trust Store, Queue Priorities, Fountain, Security | `PROVEN` |
| **Flutter Widget & Unit Tests** | `flutter test` (17 tests) | UI Components, Theme, QR Decoder, Adapters, Offer Dialog | `PROVEN` |
| **Flutter Integration Driver** | `flutter test integration_test` | Full Flutter App Navigation & Engine Init | `EMULATOR-PROVEN` |
| **Android Emulator Smoke Script** | `bash scripts/android_smoke_test.sh` | APK Install, RESUMED state, Logcat Crash Detection | `EMULATOR-PROVEN` |

---

## Execution Commands

### 1. Execute All Automated Rust Tests
```bash
cargo test --manifest-path rust/Cargo.toml
```

### 2. Execute Specific Multi-Peer E2E & Fault Harness
```bash
cargo test --manifest-path rust/Cargo.toml --test e2e_two_peer_workflow --test network_fault_harness --test qr_payload_e2e_test
```

### 3. Execute All Flutter Tests
```bash
C:\flutter\bin\flutter.bat analyze
C:\flutter\bin\flutter.bat test
```

### 4. Execute Automated Script Harness
```bash
bash scripts/qr_e2e_test.sh
bash scripts/network_fault_test.sh
bash scripts/android_smoke_test.sh
```
