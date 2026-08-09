# IMPLEMENTATION — Universal Offline Transfer (UOT)

## Overview
UOT is a cross-platform offline-first file transfer system built with **Rust (core engine & protocol)** and **Flutter (UI)**, connected via **flutter_rust_bridge v2**.

## Architecture & Module Structure

### 1. Rust Core Engine (`rust/src/`)
- `core/engine.rs`: Main lifecycle coordinator (`UotEngine`). Manages discovery, listener, connections, transfers, `TrustManager`, `TransferQueueManager`, `TransportFallbackManager`, `LifetimeStats`, and `TransferHistoryStore`.
- `security/crypto.rs` & `security/session_cipher.rs`: AES-256-GCM envelope cipher and X25519 Diffie-Hellman key exchange with monotonic nonce replay protection.
- `security/path_validator.rs`: `StrictPathValidator` sanitizing paths against traversal, null-bytes, encoded sequences, Windows reserved names, and symlink attacks.
- `security/verification.rs`: `TrustManager`, 6-digit `VerificationPin`, and session token manager.
- `transfer/engine.rs`: Chunked file I/O, sliding-window speed calculation, CRC32, and SHA-256 integrity verification.
- `transfer/queue.rs`: `TransferQueueManager` handling priority scheduling (`Low`, `Normal`, `High`, `Urgent`) and concurrent limits.
- `transport/tcp.rs`: Length-prefixed binary framing protocol over TCP sockets.
- `discovery/mdns.rs`: Peer discovery over mDNS broadcasting (`_uot._tcp.local.`).
- `discovery/subnet.rs`: Fallback IPv4 /24 subnet scanner on port 42000.

### 2. Flutter UI (`lib/src/`)
- `features/receive/incoming_offer_dialog.dart`: Material 3 offer consent modal with file list preview and PIN verification input.
- `features/receive/receive_screen.dart`: Receive settings and interactive incoming transfer offer cards.
- `features/transfers/`: Queue, progress indicators, pause/resume, and searchable history.
- `features/devices/`: Trusted device management and QR invitation pairing.
- `platform/`: Platform adapters for BLE, Wi-Fi Direct, and Camera QR scanner.

### 3. CI/CD & Build Matrix (`.github/workflows/ci.yml`)
- Android APK build (verified 44.2 MB release binary).
- Linux desktop build (`flutter config --enable-linux-desktop`).
- Windows desktop build (`flutter config --enable-windows-desktop`).
- iOS simulator & release compilation step with codesign handling.
- Web release build.

## Verification & Status
- **Rust Test Suite**: 174 tests passing (`cargo test --manifest-path rust/Cargo.toml`)
- **Flutter Test Suite**: 14 tests passing (`flutter test --coverage`)
- **Clippy Lint**: Clean (`cargo clippy --manifest-path rust/Cargo.toml -- -D warnings`)
- **Analyzer**: Clean (`flutter analyze`) — 0 errors, 0 warnings
- **Docker APK**: Built `app-release.apk` (44.2 MB)
