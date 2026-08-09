# CHANGELOG

All notable changes to UOT (Universal Offline Transfer) are documented here.
This file is **append-only** — history is never overwritten.

## [0.1.0-alpha.4] - 2026-08-08

### Gap-Closure Sprint — P0/P1 Security & Reliability Fixes

#### Security (P0)
- **Wire encryption**: Integrated AES-256-GCM encryption into all data frame transfers via new `SessionCipher` module (`rust/src/security/session_cipher.rs`)
- **X25519 key exchange**: Added `WireMessage::KeyExchange` for automatic session key establishment at connection start
- **Replay protection**: Monotonic nonce counter per session — replayed frames are detected and rejected
- **7 new security tests**: roundtrip, multi-frame, replay detection, tamper detection, key exchange, wrong key, invalid key length

#### Bug Fixes (P0)
- **Consent gating frame-loss**: Fixed bug where first `FileStart` frame after UI acceptance was consumed but not processed. Frame is now manually re-dispatched to the correct handler.
- **Android Gradle 8 APK build failure**: Fixed cargokit Gradle 8 incompatibility in `rust_builder/cargokit/gradle/plugin.gradle` by replacing Java `Action<ExecSpec>` anonymous inner classes with Groovy closures `project.exec { spec -> ... }`. Resolves `Could not find method exec() for arguments [CargoKitBuildTask$1]` during `flutter build apk --release`.
- **Dart Format CI Validation**: Formatted `lib/src/platform/ble_adapter.dart`, `lib/src/platform/camera_qr_adapter.dart`, and `lib/src/platform/wifi_direct_adapter.dart` using standard `dart format`, resolving `dart format --set-exit-if-changed .` CI check failure.

#### Reliability (P1)
- **Queue concurrency enforcement**: `send_files()` now checks `can_start()` before spawning transfers, respecting `max_concurrent_transfers` limit
- **Active transfer tracking**: Added `mark_started()` / `mark_completed()` lifecycle tracking to `TransferQueueManager`
- **2 new queue tests**: concurrency enforcement, priority ordering

#### Documentation (P0)
- **Honest GAP_ANALYSIS.md**: Rewrote `docs/GAP_ANALYSIS.md` with evidence-based audit — every claim verified against actual source code and test execution
- **Classified all features**: COMPLETE & PROVEN / IMPLEMENTED BUT UNPROVEN / PARTIAL / PLATFORM LIMITED / PENDING
- **Documented remaining gaps**: BLE/Wi-Fi Direct/Camera/Streaming classified as PLATFORM LIMITED stubs

#### E2E Testing (P0)
- **4 real E2E integration tests** (`rust/tests/e2e_transfer.rs`): encrypted file transfer with SHA-256 verification, zero-byte file, Unicode filename, tamper-in-transit detection
- **Real TCP loopback**: tests exercise actual TCP transport, key exchange, frame encryption/decryption, and file integrity verification

#### Coverage (P1)
- **cargo-tarpaulin** integrated into CI (`ci.yml`) for Rust coverage measurement with artifact upload
- **Flutter coverage** (`flutter test --coverage`) with lcov output and artifact upload

#### Network Recovery (P1)
- **ConnectionManager integration**: `connect_with_retry()`, `is_device_connected()`, `disconnect_device()` methods added to `UotEngine`
- **Exponential backoff**: auto-reconnection with 1s→2s→4s→...→30s cap, up to 5 retries

#### Platform Capabilities (P1)
- **PlatformCapabilities module** (`rust/src/core/capabilities.rs`): honest runtime detection of available transports and features per platform
- **Capability API**: `detect()`, `supported_transports()`, `unsupported_features()` with compile-time platform detection
- **3 new tests**: platform detection, supported transports, unsupported features on desktop

#### Checkpoint Resume (P2)
- **CheckpointStore module** (`rust/src/transfer/checkpoint.rs`): persistent transfer state save/load/list/remove via JSON files
- **Resume support**: `list_incomplete()` finds interrupted transfers for restart
- **4 new tests**: save/load roundtrip, list incomplete, remove, nonexistent load

#### PIN Enforcement (P2)
- **`accept_transfer_with_pin()`**: new method on `UotEngine` that verifies TrustManager PIN before accepting transfers
- **Untrusted device warning**: `accept_transfer()` now logs warning when accepting from untrusted device

#### Coverage Threshold (P2)
- Added 70% line coverage gate step in CI — parses tarpaulin output and fails if below threshold

## [0.1.0-alpha] - 2026-08-07

### Sprint 0 — Foundation

#### Added
- **Project scaffold**: Flutter 3.44.6 + Rust 1.97.1 via flutter_rust_bridge v2.12.0
- **Rust core engine** with 7 architectural modules:
  - `core/` — Configuration, error types, version info
  - `transport/` — Transport abstraction traits (TransportConnection, TransportProvider)
  - `protocol/` — Protocol state machine and message types (16 message types)
  - `security/` — Crypto traits, path validation, session/device types
  - `discovery/` — Discovery traits and device types
  - `transfer/` — Transfer engine traits, progress tracking, speed/ETA formatting
  - `streaming/` — Stream capability detection, config, status types
- **Flutter app shell** with 6 feature screens:
  - Nearby (device discovery with scanning animation)
  - Transfers (queue and history)
  - Receive (visibility toggle, incoming requests)
  - Stream (camera, screen, video, audio options)
  - Devices (trusted device management)
  - Settings (theme, transfer, discovery, security, about)
- **Theme system**: Material 3 dark-first design with high-contrast text
- **Adaptive navigation**: NavigationBar (mobile) / NavigationRail (desktop ≥800px)
- **68 Rust unit tests** — all passing, covering errors, config, version, protocol state, messages, transport types, security types, discovery types, transfer types, streaming types
- **Mandatory developer skill**: `.agents/skills/production-development/SKILL.md`
- **GitHub Actions CI/CD**: Workflows for Rust check, Flutter check, and builds for Web, Android, Windows, Linux, macOS, iOS
- **Documentation**: README, CODE_MAP, CHANGELOG, IMPLEMENTATION, TODO, ROADMAP, GAP_ANALYSIS
- **Rust API endpoints** exposed to Dart: `getVersion()`, `getProtocolVersion()`, `getBuildInfo()`, `healthCheck()`

#### Technical Details
- Protocol state machine: 15 states with validated transitions
- Transport abstraction: 8 transport types (TCP/LAN, Wi-Fi Direct, BT Classic, BLE, QR, USB, Hotspot, Relay)
- Configuration system with validation (device name, chunk size, concurrent transfers, scan intervals)
- Error hierarchy with 7 error categories and 30+ specific error variants
- Transfer progress with speed formatting (B/s → KB/s → MB/s → GB/s) and ETA display

## [0.2.0-alpha] - 2026-08-07

### Sprint 1 — Core

#### Added
- **TCP/LAN Transport** (`transport/tcp.rs`):
  - Length-prefixed framing protocol (4-byte length + 1-byte type + payload)
  - Async reader/writer with tokio split streams
  - Connection keepalive, graceful shutdown
  - TCP listener with incoming connection channel
  - Frame types: Control (JSON), Data (binary), Ping, Pong
- **mDNS Discovery** (`discovery/mdns.rs`):
  - Service registration (`_uot._tcp.local.`)
  - Service browsing with device found/lost/updated events
  - TXT record properties (device_id, type, version, capabilities)
  - Automatic own-service filtering
- **File Transfer Engine** (`transfer/engine.rs`):
  - Chunked file I/O with configurable chunk size
  - CRC32 per-chunk integrity verification
  - SHA-256 per-file hash verification
  - Recursive folder collection with relative path preservation
  - Progress tracking with sliding-window speed calculation and ETA
  - TransferRecord and TransferItemRecord for queue management
- **UOT Engine Coordinator** (`core/engine.rs`):
  - Lifecycle management (start/stop)
  - mDNS + TCP integration
  - Send/receive file transfer orchestration
  - Event channel for UI updates (device events, transfer progress)
  - Connection management
- **Engine API** (`api/engine_api.rs`):
  - Singleton engine with tokio runtime
  - `engine_init()`, `engine_stop()`, `engine_state()`
  - `engine_get_devices()`, `engine_get_transfers()`
  - `engine_send_files()` with device ID and file paths
- **Flutter UI Rewrite**:
  - Nearby: Ripple scanning animation, device cards with type icons, send bottom sheet (files/folder/clipboard)
  - Transfers: Active queue with progress bars + history tab, pause/cancel controls
  - Receive: Visibility toggle, auto-accept, PIN settings, save location
  - Stream: Camera/screen/video/audio streaming options
  - Devices: This-device card with gradient, trusted device management, QR pairing
  - Settings: Chunk size slider, SHA-256 toggle, discovery, security, about section

#### Changed
- `TransportState` enum: added `Disconnected` variant
- `TransferError`: added `FileIo`, `IntegrityFailed(String)`, `EmptyTransfer`, `DeviceNotFound`, `Protocol` variants
- `TransportError`: added `Connection`, `Protocol` tuple variants
- `DiscoveryError`: added `ServiceError` tuple variant
- `AppConfig`: added `network_port`, `save_directory` fields

## [0.3.0-alpha] - 2026-08-07

### Sprint 2 — Wiring

#### Added
- **FRB bindings** for engine API (devices, transfers, send, stop)
- **File picker integration**: Files + folders via `file_picker` package
- **Live device polling**: Nearby screen polls `engine_get_devices()` every 2s
- **Transfer polling**: Transfers screen polls `engine_get_transfers()` every 1s
- **Protocol handler** (`protocol/handler.rs`): `WireMessage` enum, `send_message()`, `recv_message()`, `send_data_chunk()`, `recv_data_chunk()`
- **TcpConnection**: added `send_frame()` and `recv_frame()` for bidirectional framed I/O

## [0.4.0-alpha] - 2026-08-07

### Sprint 3 — Features

#### Added
- **Clipboard module** (`transfer/clipboard.rs`): `ClipboardItem`, auto-detect text/URL/HTML, preview generation
- **Security verification** (`security/verification.rs`): `VerificationPin` (6-digit, TTL), `VerificationSession` (SHA-256 tokens), `TrustManager`
- **Transfer control APIs**: `pause_transfer()`, `resume_transfer()`, `cancel_transfer()`, `accept_transfer()`
- **Clipboard send API**: `engine_send_clipboard()` with system clipboard wiring
- **Events API**: `engine_get_events()` for event log

## [0.5.0-alpha] - 2026-08-07

### Sprint 4 — Streaming

#### Added
- **StreamManager** (`streaming/manager.rs`): Session lifecycle (start/stop/pause/update)
- **StreamSession**: `StreamType` (Camera/Screen/Video/Audio), `StreamState` (Idle→Streaming→Stopping)
- **Stream API**: `engine_get_streams()` for Flutter

## [0.6.0-alpha] - 2026-08-08

### Sprint 5 — Persistence & Reliability

#### Added
- **UserSettings** (`core/settings.rs`): JSON-based settings persistence (device name, theme, chunk size, SHA-256 toggle, auto-accept, PIN, save directory, port, scan interval, concurrent transfers)
- **ConnectionManager** (`transport/connection_manager.rs`): Exponential backoff reconnection (configurable max retries, base/max delay), connection pooling, device tracking
- **Settings API**: `engine_load_settings()`, `engine_save_settings()`
- **deps**: `dirs-next` for platform config/download directories

#### Changed
- `docs/CODE_MAP.md`: Updated with all Sprint 1-5 files (★ markers for new modules)
- `TODO.md`: Reorganized with completed/active/backlog sections

## [0.7.0-alpha] - 2026-08-08

### Sprint 6 — QR & Advanced Transports

#### Added
- **Fountain Encoder** (`protocol/fountain.rs`): Luby Transform (LT) encoder with CRC32 verification for optical QR transport streams
- **QR Invitation & Pairing** (`security/qr.rs`): Encrypted QR pairing structure with OTP PIN, device ID, ephemeral key, and TTL validation
- **BLE Transport Abstraction** (`transport/ble.rs`): GATT service UUIDs (`UOT_BLE_SERVICE_UUID`), GATT characteristics, and BLE advertisement payload serialization
- **QR FFI APIs** (`api/engine_api.rs` / Dart FFI): `engine_generate_qr_invitation()`, `engine_parse_qr_invitation()`
- **Persistent Transfer History** (`transfer/history.rs`): JSON store with text search & status filtering (`query()`)
- **Wi-Fi Direct P2P Group** (`transport/wifidirect.rs`): `WifiDirectGroupInfo` SSID, WPA2/WPA3 passphrase, 5GHz channel negotiation
- **Transport Fallback Orchestrator** (`transport/fallback.rs`): `TransportFallbackManager` with priority selection (TcpLan -> WifiDirect -> BluetoothLe -> QrCode)
- **History FFI API**: `engine_search_history()`
- **Cryptographic Provider** (`security/crypto.rs`): `SoftwareCryptoProvider` implementing `CryptoProvider` trait (AES-256-GCM envelope cipher & SHA-256 derivation)
- **Hotspot Assist** (`transport/hotspot.rs`): `HotspotConfig` for local Access Point configuration and status tracking
- **Throughput Benchmark** (`core/benchmark.rs`): `ThroughputBenchmark` real-time bandwidth calculator and Mbps snapshotting
- **Subnet Active Scanner** (`discovery/subnet.rs`): `SubnetScanner` fallback scan over IPv4 /24 range on port 42000
- **Transfer Queue Manager** (`transfer/queue.rs`): `TransferQueueManager` with priority scheduling (`Low`, `Normal`, `High`, `Urgent`)
- **Lifetime Statistics & Analytics** (`transfer/analytics.rs`): `LifetimeStats` cumulative bytes/transfers/peak speed tracker (`engine_get_stats()`)
- **Network Interface Enumerator** (`discovery/interface.rs`): `InterfaceEnumerator` active IPv4/IPv6 interface listing
- **Transfer Rate Limiter** (`transfer/ratelimit.rs`): `RateLimiter` token bucket bandwidth throttler

## [0.8.0-alpha] - 2026-08-08

### Sprint 7 — Production Validation & Gap-Closure

#### Added
- **TrustManager Integration** (`core/engine.rs`): Integrated `TrustManager` & PIN verification into `UotEngine` lifecycle.
- **PIN Verification APIs** (`api/engine_api.rs`): `engine_generate_pin()`, `engine_verify_pin()` FFI endpoints for Dart.
- **Offer Consent Gating** (`core/engine.rs`): Incoming file transfers gated until UI calls `accept_transfer()`.
- **Idle Connection Timeout**: Added 60s idle timeout to connection frame processing loop.
- **Rust Integration Test Suite** (`rust/tests/integration_transfer.rs`): Two-engine loopback transfer test, queue manager scheduling test.
- **Flutter Widget Tests** (`test/receive_screen_test.dart`, `test/incoming_offer_dialog_test.dart`): UI tests for `ReceiveScreen` settings/visibility and `IncomingOfferDialog` consent/PIN flow (10 widget tests passing).
- **Docker Mesh Container Setup** (`Dockerfile`, `docker-compose.yml`): Multi-stage container build and isolated 2-node subnet bridge simulation network.
- **Fountain Decoder & Reconstruction** (`protocol/fountain.rs`): `FountainDecoder` with CRC32 integrity validation for Luby Transform optical QR payload reconstruction.
- **Coverage & Quality Gate Script** (`scripts/coverage.ps1`): Automated PowerShell script enforcing 100% test pass rate for Rust & Flutter and zero Clippy warnings.
- **Unit Test Suite Expansion**: Expanded unit test coverage across `benchmark`, `subnet`, `interface`, `fallback`, `connection_manager`, `ratelimit`, `clipboard`, and `qr` modules (147 Rust tests passing).
- **BLE GATT Host Platform Adapter** (`lib/src/platform/ble_adapter.dart`): `BleGattAdapter` managing GATT service UUID (`6E400001-B5A3-F393-E0A9-E50E24DCCA9E`), control & data characteristics, and advertisement broadcast.
- **Wi-Fi Direct P2P Platform Adapter** (`lib/src/platform/wifi_direct_adapter.dart`): `WifiDirectAdapter` for P2P Group Owner creation, SSID broadcast, 5GHz channel negotiation, and TCP bridge binding.
- **Camera Optical QR Scanner Adapter & UI Dialog** (`lib/src/platform/camera_qr_adapter.dart`, `lib/src/features/nearby/qr_scanner_dialog.dart`): Interactive Material 3 QR Scanner modal with camera preview and Luby Transform fountain code stream reconstruction progress tracking.
- **Live Media Payload H.264/AAC Streaming Pipeline** (`rust/src/streaming/pipeline.rs`): `MediaStreamPipeline` providing H.264 NAL unit framing (SPS/PPS/IDR/P-Frame), AAC ADTS audio frame encapsulation, ring-buffer jitter smoothing, and CRC32 packet checksums (150 Rust tests & 14 Flutter tests passing).
- **Updated Documentation Suite**: Evidence-based audit updates across `GAP_ANALYSIS.md`, `PRODUCTION_READINESS.md`, `TESTING.md`, `TODO.md`, `IMPLEMENTATION.md`.





