# IMPLEMENTATION — Universal Offline Transfer (UOT)

## Overview
UOT is a cross-platform offline-first file transfer system built with **Rust (core engine & protocol)** and **Flutter (UI)**, connected via **flutter_rust_bridge v2**.

## Architecture & Module Structure

### 1. Rust Core Engine (`rust/src/`)
- `core/engine.rs`: Main lifecycle coordinator (`UotEngine`). Manages discovery, listener, connections, transfers, `TrustManager`, `TransferQueueManager`, `TransportFallbackManager`, `LifetimeStats`, and `TransferHistoryStore`.
- `core/config.rs`: `AppConfig` with validated device name, network ports, transfer limits, and discovery intervals.
- `core/capabilities.rs`: Platform-aware capability detection (Wi-Fi Direct, BLE, USB, streaming).
- `security/crypto.rs` & `security/session_cipher.rs`: AES-256-GCM envelope cipher and X25519 Diffie-Hellman key exchange with monotonic nonce replay protection.
- `security/path_validator.rs`: `StrictPathValidator` sanitizing paths against traversal, null-bytes, encoded sequences, Windows reserved names, and symlink attacks.
- `security/verification.rs`: `TrustManager`, 6-digit `VerificationPin`, session token management, and device trust/revoke lifecycle.
- `transfer/engine.rs`: Chunked file I/O, sliding-window speed calculation, CRC32, and SHA-256 integrity verification.
- `transfer/queue.rs`: `TransferQueueManager` handling priority scheduling (`Low`, `Normal`, `High`, `Urgent`) and concurrent limits.
- `transfer/checkpoint.rs`: `CheckpointStore` for persistent resume-after-restart. JSON checkpoint per transfer with per-item progress.
- `transport/tcp.rs`: Length-prefixed binary framing protocol over TCP sockets.
- `transport/fallback.rs`: `TransportFallbackManager` with `PreferSpeed`, `PreferOffline`, and `Manual` strategies.
- `transport/connection_manager.rs`: Exponential backoff reconnection with jitter.
- `protocol/handler.rs`: Wire protocol messages (KeyExchange, Offer, FileStart, Data, FileEnd, TransferComplete).
- `protocol/fountain.rs`: Luby Transform fountain code encoder/decoder for animated QR transport.
- `streaming/pipeline.rs`: `MediaStreamPipeline` with H.264 NAL framing, jitter buffer, and bitrate tracking.
- `discovery/mdns.rs`: Peer discovery over mDNS broadcasting (`_uot._tcp.local.`).
- `discovery/subnet.rs`: Fallback IPv4 /24 subnet scanner on port 42000.

### 2. Flutter UI (`lib/src/`)
- `main.dart`: Non-blocking engine initialization with 15s timeout, loading spinner, and `RustInitFailedScreen` on failure.
- `features/diagnostics/rust_init_failed_screen.dart`: Professional error screen with retry button and clipboard diagnostics.
- `features/receive/incoming_offer_dialog.dart`: Material 3 offer consent modal with file list preview and PIN verification input.
- `features/receive/receive_screen.dart`: Receive settings and interactive incoming transfer offer cards.
- `features/transfers/`: Queue, progress indicators, pause/resume, and searchable history.
- `features/devices/`: Trusted device management and QR invitation pairing.
- `platform/`: Platform adapters for BLE, Wi-Fi Direct, and Camera QR scanner.

### 3. Android Configuration
- `build.gradle.kts`: compileSdk=34, targetSdk=34, minSdk=24, Kotlin 1.9.22, JVM target 11.
- `AndroidManifest.xml`: Optional hardware features (BLE, Wi-Fi Direct, Camera), foreground service for Android 14+, network security config.

### 4. CI/CD & Build Matrix (`.github/workflows/ci.yml`)
- **Rust Check**: Format, Clippy, 379+ tests, tarpaulin coverage ≥80%.
- **Flutter Check**: dart format, flutter analyze, flutter test.
- **Android APK build** with cargo-ndk (arm64-v8a, armv7, x86_64) + emulator smoke test.
- **Windows desktop build** with smoke test (launch EXE, verify 5s alive).
- **Linux desktop build** (`flutter config --enable-linux-desktop`).
- **macOS desktop build**.
- **iOS simulator** release compilation with codesign handling.
- **Web release build**.

## Test Suite

### Rust Tests (379+ tests)
| Suite | Tests | Description |
|-------|-------|-------------|
| Unit tests | 251 | All modules: engine, crypto, checkpoint, queue, types |
| E2E transfer | 4 | Encrypted TCP, zero-byte, Unicode, tamper detection |
| E2E edge cases | 8 | Long filenames, large batch, nested dirs, recovery, chaos |
| Integration | 4 | Two-engine loopback, protocol handler, connection manager |
| Security | 19 | Malformed messages, path traversal, PIN brute-force, replay |
| Coverage | 99 | Targeted tests for Display impls, error branches, fallback |

### Coverage
- **80.03%** line coverage (1395/1743 lines) via `cargo-tarpaulin`.
- Excluded: `frb_generated` (auto-gen), `load_stress` (benchmarks), `src/testing` (infra), `discovery/mdns` (needs multicast), `transport/quic` (needs sockets), `streaming/capture` (needs hardware).
- `core/engine.rs` at ~47% — async I/O methods require real TCP+mDNS, tested by E2E suite.

## Verification & Status
- **Rust Test Suite**: 379+ tests passing (`cargo test --manifest-path rust/Cargo.toml`)
- **Rust Line Coverage**: ≥80% enforced via `cargo-tarpaulin` CI pipeline
- **Flutter Test Suite**: 14 tests passing (`flutter test --coverage`)
- **Clippy Lint**: Clean (`cargo clippy -- -D warnings`)
- **Dart Analyzer**: Clean (`flutter analyze`) — 0 errors, 0 warnings
- **Dart Format**: 50 files, 0 changes needed
