### Sprint 22 — Production File-Transfer Progression, Android Chat Stability & Target-Only UX (Completed ✅)
- [x] Fixed sender-side progress stalling by atomically writing `TransferStatus::InProgress` and chunk-level `transferred_bytes` to `self.transfers` in `execute_send_arc`
- [x] Fixed receiver-side per-item progress tracking on `FileStart`, `DataChunk`, and `FileEnd`
- [x] Restricted `Open`, `Folder`, and tap-to-open preview actions strictly to received files (`!isSend && isCompleted && savedPath.isNotEmpty`)
- [x] Upgraded `_buildTransferCard` in `chat_screen.dart` to support multi-item batch transfer cards with per-item progress, status badges, and open actions
- [x] Fixed Android chat scrambled/corrupted font glyph rendering by removing unbundled `fontFamily: 'Inter'` from `app_theme.dart` and replacing `SelectableText` with clean `Text` + copy interactions
- [x] Implemented `ActiveChatSessionTracker` and updated `nearby_screen.dart` to suppress duplicate modals/SnackBars when inside active chat
- [x] Added sender status and transferred bytes assertions to `transport_lab_e2e.rs`
- [x] 100% test pass rate across all Rust suites and Flutter test suite; 0 Clippy warnings; 0 Flutter analysis errors; 100% clean rustfmt

### Sprint 21 — Production Transport Lab, Deterministic E2E & UX Overhaul (Completed ✅)
- [x] Fixed multi-peer connection misrouting & session overwrite bug in `get_peer_session` and `get_peer_connection`
- [x] Fixed receiver `InProgress` status transition on data frame receipt and automatic `.part` touching on `FileStart`
- [x] Fixed receiver atomic rename (`.part` -> final) with copy fallback and populated `saved_path` across transfers
- [x] Fixed chat message JSON corruption on Android/Windows by using structured DTOs with `serde_json` serialization and capped chat history at 1000 messages
- [x] Overhauled `ChatScreen` with unified chronological timeline, rich file icons, in-app preview for images/notes, live speed/ETA display, and open/reveal handlers
- [x] Created `TransportSimulator` (`rust/src/transport/simulator.rs`) with configurable jitter, packet loss, bit flips, and network partition simulation
- [x] Created deterministic animated QR optical transport reassembly test suite (`rust/tests/animated_qr_e2e_test.rs`)
- [x] Created acoustic sound FSK modulation, demodulation, noise injection, and CRC16 test suite (`rust/tests/audio_fsk_e2e_test.rs`)
- [x] Created multi-node deterministic integration test suite (`rust/tests/transport_lab_e2e.rs`) asserting disk persistence, exact byte size, and SHA-256 match
- [x] Created Docker multi-node test container (`docker/Dockerfile.peer`) and 3-peer test network (`docker/docker-compose.test.yml`)
- [x] Built Developer Transport Lab & Diagnostics screen in Flutter (`lib/src/features/diagnostics/transport_lab_screen.dart`) with capability matrix, fault injection sliders, and synthetic loopback benchmarks
- [x] 100% test pass rate across all Rust suites and Flutter test suite; 0 Clippy warnings; 0 Flutter analysis errors; 100% clean rustfmt

### Sprint 20 — File Transfer OfferResponse Socket Routing & Inline UI Overhaul (Completed ✅)
- [x] Resolved critical file transfer failure by introducing `transfer_connections` socket map in `UotEngine`
- [x] Routed `OfferResponse` on the exact `TcpConnection` instance that delivered the `Offer` message
- [x] Upgraded `chat_screen.dart` to poll engine events (`IncomingOffer`, `TransferProgress`, `TransferStatusChanged`)
- [x] Built inline `IncomingOffer` banner with instant Accept/Reject action buttons
- [x] Built inline `TransferCard` widgets displaying real-time progress bars, speed, ETA, direction, and file details in the unified chat timeline
- [x] Verified full loopback transfer with `test_e2e_offer_response_accept_file_transfer` (1MB payload, SHA-256 integrity match, atomic rename)
- [x] All 261 Rust tests passed (100%), Clippy 0 warnings, Rustfmt 0 diffs, Flutter analyze 0 issues

- [x] **Platform Truth Audit**: corrected documentation crypto from "Noise XX / ChaCha20-Poly1305" to actual AES-256-GCM + X25519
- [x] Created `docs/TRANSPORT_MATRIX.md` with honest transport status
- [x] Rewrote `docs/PLATFORM_SUPPORT.md`, `docs/PRODUCTION_READINESS.md`, `docs/GAP_ANALYSIS.md`, `docs/SECURITY.md`, `docs/PROTOCOL.md`
- [x] 178 Rust Tests (100% Pass) & 14 Flutter Tests (100% Pass)

### Sprint 11 — Real-Device Production Certification (Completed ✅)
- [x] Resolved Android Gradle R8 task collision & repository mode conflicts (`android/build.gradle.kts`, `android/settings.gradle.kts`)
- [x] Verified Android release APK build (`build/app/outputs/flutter-apk/app-release.apk` - 44.2 MB)
- [x] Resolved Linux desktop `fl_view_set_background_color` API incompatibility (`linux/runner/my_application.cc`)
- [x] Added `flutter config` desktop flags for Linux and Windows in CI (`.github/workflows/ci.yml`)
- [x] Configured `concurrency` group with `cancel-in-progress: true` in GitHub Actions
- [x] Pinned `FLUTTER_VERSION: '3.24.0'` in CI to align with Docker build environment
- [x] Synchronized complete documentation suite (`REQUIREMENTS.md`, `TECHNICAL_ARCHITECTURE.md`, `PROTOCOL.md`, `NETWORKING.md`, `SECURITY.md`, `PLATFORM_SUPPORT.md`, `CODE_MAP.md`, `TESTING.md`, `TEST_MATRIX.md`, `PERFORMANCE.md`, `INFRASTRUCTURE.md`, `CI_CD.md`, `DEPLOYMENT.md`, `SETUP.md`, `CONFIGURATION.md`, `USER_GUIDE.md`, `TROUBLESHOOTING.md`, `WALKTHROUGHS.md`, `GAP_ANALYSIS.md`, `PRODUCTION_READINESS.md`, `IMPLEMENTATION.md`, `TODO.md`, `ROADMAP.md`, `CHANGELOG.md`)
- [x] 174 Rust Tests (100% Pass) & 14 Flutter Tests (100% Pass)

### Sprint 10 — Load Testing, Native Adapters & Multi-Device E2E (Completed ✅)
- [x] Load/stress tests (`rust/tests/load_stress.rs`): 100MB encrypted transfer, 4 concurrent transfers, 50-file batch, encrypted throughput benchmark
- [x] Native Android BLE (`android/.../BleAdapterPlugin.kt`): GATT server, advertising, scanning via BluetoothLeAdvertiser
- [x] Native Android Wi-Fi Direct (`android/.../WifiDirectAdapterPlugin.kt`): WifiP2pManager group creation, peer discovery, connection
- [x] Native iOS BLE (`ios/Runner/BleAdapterPlugin.swift`): CoreBluetooth CBPeripheralManager/CBCentralManager
- [x] Flutter BLE adapter (`lib/src/platform/ble_adapter.dart`): MethodChannel bridge with graceful fallback
- [x] Flutter Wi-Fi Direct adapter (`lib/src/platform/wifi_direct_adapter.dart`): MethodChannel bridge with peer discovery
- [x] Flutter Camera QR adapter (`lib/src/platform/camera_qr_adapter.dart`): MethodChannel bridge to CameraX/AVFoundation
- [x] Docker multi-device E2E (`docker-compose.yml`): 3-node isolated bridge network (sender, receiver, full test runner)

### Sprint 9 — Gap-Closure Security & Reliability (Completed ✅)
- [x] Wire encryption: AES-256-GCM integrated into all data frame transfers (`rust/src/security/session_cipher.rs`)
- [x] X25519 key exchange at connection start (`WireMessage::KeyExchange`)
- [x] Nonce-counter replay protection on all encrypted frames
- [x] Consent gating frame-loss bug fixed (FileStart re-dispatch after acceptance)
- [x] Queue concurrency enforcement (`can_start()` / `mark_started()` / `mark_completed()`)
- [x] Honest GAP_ANALYSIS.md rewrite with evidence-based audit
- [x] 4 real E2E integration tests (encrypted transfer, zero-byte, Unicode, tamper detection)
- [x] Coverage tooling: cargo-tarpaulin + Flutter --coverage in CI with artifact upload
- [x] ConnectionManager integration with exponential backoff retry (`connect_with_retry()`)
- [x] PlatformCapabilities module: honest runtime detection of available features
- [x] CheckpointStore for persistent resume state (`transfer/checkpoint.rs`)
- [x] PIN enforcement via `accept_transfer_with_pin()` with TrustManager verification
- [x] Coverage threshold enforcement (70% gate in CI via tarpaulin)

### Sprint 14 - Production Gap-Closure & Cross-Platform Validation (Completed)
- [x] Android crash fix: compileSdk/targetSdk 34, minSdk 24, optional hardware features, non-blocking RustLib.init() with timeout & RustInitFailedScreen fallback
- [x] Windows CI fix: PowerShell multi-line syntax, Windows launch EXE smoke test
- [x] Coverage hardening: 80.03% coverage (1395/1743 lines), 388 tests passing across all suites
- [x] E2E edge cases & chaos tests (e2e_edge_cases.rs): 255-char filenames, 10-file batch, nested subdirs, checkpoint restart recovery, corrupted checkpoint handling, disconnect chaos & receiver timeout
- [x] Documentation & sync: IMPLEMENTATION.md, TODO.md, CHANGELOG.md updated

### Sprint 15 - QR Code Pairing, Direct IP Connectivity & LAN Subnet Discovery (Completed)
- [x] Fixed "Scan QR Code" top bar button handler (connected to QrPairingDialog)
- [x] Added QrPairingDialog with dual tabs ("My QR & IP", "Direct IP Connect")
- [x] Integrated qr_flutter package for pure Dart cross-platform QR code generation
- [x] Added _MyDeviceBanner header displaying local IPv4 address, device name, and instant action buttons ("Show QR", "Direct Connect", "Scan Subnet")
- [x] Exposed engine_get_local_ips and engine_connect_peer in Rust FRB engine API
- [x] Updated Rust subnet_scan to automatically populate and broadcast discovered LAN nodes
- [x] Configured periodic background subnet scanning every 6 seconds on NearbyScreen
- [x] Flutter analyze 100% clean (0 errors, 0 warnings), 14 Flutter tests passing, 392 Rust tests passing

### Sprint 16 - Optical Animated QR Air-Gapped Transfer & Connectivity Hardening (Completed)
- [x] Built OpticalQrSenderDialog for air-gapped zero-network animated QR stream data transfer using Fountain Codes (Luby Transform)
- [x] Added "Optical Animated QR Stream" option in _SendBottomSheet for sending files without Wi-Fi or Bluetooth
- [x] Exposed engine_fountain_encode in Rust FRB API for animated QR payload generation
- [x] Added "Scan Peer's QR Code" camera button and manual QR link input/paste field in QrScannerDialog
- [x] Added automatic Windows Defender Firewall port 42000 rule registration on engine initialization
- [x] Added 4-second connection attempt timeout and automatic candidate port fallback ([42000, 42001, 8080, 50000]) in connect_peer
- [x] Added real-time SnackBar & Modal Dialog user feedback when initiating file transfer
- [x] Hardened Rust test coverage to 82.15%, 407 Rust tests passing, Flutter analyze 100% clean

### Sprint 17 - UX Gap Resolution: Camera Permissions, File Transfer Modal, Media Streaming & Engine Status (Completed)
- [x] Fixed persistent "Engine starting..." banner issue by making mDNS failure non-fatal and expanding engine state checks for Running/Partial
- [x] Fixed Camera QR Scanner permission request flow and added "Pick QR Code Image File" option for Desktop/file scanning
- [x] Created ActiveTransferDialog real-time modal with progress bar, transfer speed (MB/s), ETA, current item name, and Cancel action
- [x] Fully implemented StreamScreen: wired Camera, Screen Share, Video File, and Audio File streaming controls with live session list & stop buttons
- [x] Verified 14/14 Flutter tests passing, 407/407 Rust tests passing, Flutter analyze 100% clean

### Sprint 18 - Direct IP Validation, ConfirmSendDialog, InstantChat & Android MobileScanner (Completed)
- [x] Added strict IPv4 octet (0-255) validation to QrPairingDialog to prevent typos (e.g., 292.168.0.111) and added Auto-Fill Subnet chip helper
- [x] Created ConfirmSendDialog with file item list, individual item sizes, batch total size, and explicit "[🚀 Send Files Now]" action button
- [x] Created InstantChatDialog for sending live text messages, connection pings, and delivery receipts between connected nodes
- [x] Integrated mobile_scanner ^7.4.0 in QrScannerDialog for live Android hardware camera QR scanning
- [x] Verified 14/14 Flutter tests passing, 407/407 Rust tests passing, Flutter analyze 100% clean

### Sprint 19 - Scan Loop Debouncing, Error State Pause & Windows Firewall Elevation Helper (Completed)
- [x] Implemented QR scan debouncing and error state pause in QrScannerDialog to eliminate infinite camera scan loops
- [x] Added "Tap to Scan Again" manual scan retry button on viewfinder overlay
- [x] Added engine_fix_windows_firewall in Rust FRB API to trigger Windows PowerShell UAC prompt allowing port 42000
- [x] Added 1-tap "Fix Windows Firewall" helper button in QrScannerDialog and QrPairingDialog
- [x] Verified 14/14 Flutter tests passing, 407/407 Rust tests passing, Flutter analyze 100% clean

### Sprint 20 - Dynamic Bound Port QR Payloads, Self-Loopback Protection & Candidate Port Fallback (Completed)
- [x] Exposed engine_get_listening_port in Rust FRB API returning actual bound TCP socket port
- [x] Updated QrPairingDialog and NearbyScreen device banner to dynamically generate QR payload and IP display with actual bound port
- [x] Added self-loopback check in connect_peer preventing devices from connecting to themselves
- [x] Enabled candidate port fallback [42000, 42001, 42002, 42003, 8080, 50000] when connecting via explicit IP:port strings
- [x] Verified 14/14 Flutter tests passing, 407/407 Rust tests passing, Flutter analyze 100% clean

### Sprint 17 — Production Recovery: Critical Gap Fixes (Completed ✅)
- [x] Evidence-based gap analysis: traced Flutter→FRB→Rust→discovery→connection→handshake→transfer runtime path
- [x] GAP 1 FIX: Stored `event_rx` instead of discarding — engine events now flow to Flutter
- [x] Added `engine_poll_events()` API + `event_forwarder` async task (9 event types → JSON)
- [x] GAP 2 FIX: QR invitation uses actual local IP from `tcp::local_ips()` instead of `127.0.0.1`
- [x] GAP 3 FIX: `connect_peer()` now spawns `handle_incoming_connection()` reader task — bidirectional comms
- [x] GAP 4 FIX: `send_files()` sends Hello/HelloAck before KeyExchange — receiver identifies sender
- [x] GAP 7 FIX: Transfer acceptance timeout increased from 5s to 120s
- [x] GAP 8 FIX: Removed non-elevated netsh from `engine_init()` (firewall via UAC-elevated path only)
- [x] Flutter: wired `_pollEvents()` into refresh timer — IncomingOffer → Accept dialog, ClipboardReceived → notification
- [x] FRB codegen regenerated, cargo clippy clean, flutter analyze clean, 19/19 Rust tests pass

### Remaining P1 Items (Next Sprint)
- [x] GAP 5: Unify connection map keys (device_id vs IP:port) 
- [x] GAP 6: Add receive-side ProgressTracker in handle_incoming_connection
- [x] GAP 9: Add recurring heartbeat Ping every 15s with disconnect detection
- [x] GAP 10: Fix send_clipboard connection lookup to check both device_id and IP:port
- [x] GAP 11: Add engine_get_diagnostics() API with connection health info
- [ ] GAP 12-14: Streaming stubs, transport stubs, cross-session resume (P2)

### Sprint 18 — Session, Chat & Transfer Architecture (In Progress)
- [x] Phase 1: Created `PeerSession` model (`session.rs`) with full state machine (Discovered→SessionReady)
- [x] Phase 1: Added `sessions` map to `UotEngine` with get_or_create_session, get_sessions_json, get_session_messages
- [x] Phase 1: Added `send_chat_message()` with MessageState tracking (Sending→Sent→Delivered→Failed)
- [x] Phase 1: Added heartbeat task (15s Ping, 3-miss disconnect detection)
- [x] Phase 2: Expanded EngineEvent to 22 types (SessionStateChanged, IncomingMessage, MessageDelivered, HeartbeatChanged, OfferAccepted, etc.)
- [x] Phase 2: Updated event_forwarder to serialize all 22 event types
- [x] Phase 3: Added WireMessage variants (ChatMessage, MessageAck, FileStartAck, TransferCompleteAck)
- [x] Phase 3: Added ChatMessage handler with automatic MessageAck reply
- [x] Phase 3: Added OfferResponse handler for ACK-based offer acceptance
- [x] Phase 5: Created unified ChatScreen with chronological conversation, delivery states, message input
- [x] Phase 5: Added _PeerActionSheet (Chat / Send Files options on device tap)
- [x] Phase 5: Added IncomingMessage event routing with "Open Chat" SnackBar action
- [x] Phase API: Added engine_get_sessions(), engine_get_messages(), engine_send_message() APIs
- [x] FRB codegen regenerated, cargo clippy clean, flutter analyze clean, 419 tests pass

### Remaining Sprint 18 Items
- [ ] Phase 4: Transfer transaction model with full state machine (Preparing→Completed)
- [ ] Phase 4: Temp file + SHA-256 + atomic rename for data integrity
- [ ] Phase 5: Transfer detail screen with progress, speed, ETA
- [ ] Phase 6: 27-step E2E dual-engine test
- [ ] Phase 6: Documentation updates

