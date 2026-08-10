## Active Sprint Tasks (Sprint 14 — Hardware Certification & CI Polish)
- [ ] Validate Android APK launch on physical device
- [ ] Validate Windows release .exe launch
- [ ] Perform real-device file transfer (Android↔Windows or Android↔Android)
- [ ] Physical BLE interoperability test
- [ ] Physical Wi-Fi Direct P2P test
- [ ] Physical QR camera scan test
- [ ] Reach >90% Flutter test coverage
- [ ] QUIC transport implementation
- [ ] WebRTC transport implementation

---

## Complete Milestone History

### Sprint 13 — Hardware-Free Validation Lab (Completed ✅)
- [x] Created `testing` module with hardware abstraction traits (TransportAdapter, BleAdapter, WifiDirectAdapter, CameraAdapter, VideoSource, AudioSource)
- [x] Implemented universal `TransferSession` model supporting transport migration + chunk verification
- [x] Built `FaultNetwork` with deterministic packet loss, latency, jitter, bandwidth limit, corruption, disconnect/reconnect
- [x] Built `FakeBleAdapter` with advertise/scan/connect/MTU fragmentation/disconnect + fault injection
- [x] Built `FakeWifiDirectAdapter` with discover/group/connect/disconnect + failure injection
- [x] Built `FountainEncoder`/`FountainDecoder` for QR fountain code E2E with frame loss resilience
- [x] Built `SyntheticVideoSource`/`SyntheticAudioSource` for streaming simulation
- [x] Built `FakeCameraAdapter` for QR scan simulation
- [x] Created `VirtualUotNode` two-node E2E harness with full protocol flow (discovery→key exchange→offer→accept→transfer→SHA-256→complete)
- [x] Implemented checkpoint/resume simulation (fail at N%, save checkpoint, reconnect, resume, SHA-256 match)
- [x] Created 10 chaos test scenarios (clean, multi-file, zero-byte, Unicode, resume, migration, large file, batch, duplicates)
- [x] Created `docs/HARDWARE_CERTIFICATION.md` with SOFTWARE PROVEN / EMULATOR PROVEN / HARDWARE PENDING matrix
- [x] 38+ new testing module tests, all passing
- [x] Raised coverage threshold to 90%, excluded `frb_generated.rs`

### Sprint 12 — Production Blocker & Certification Fix (Completed ✅)
- [x] Android crash fix: diagnostic recovery screen (`RustInitFailedScreen`) on `RustLib.init()` failure
- [x] Android: guarded Kotlin plugin registration with `hasSystemFeature()` checks
- [x] Android: `network_security_config.xml` restricts cleartext to LAN only (replaces blanket `usesCleartextTraffic`)
- [x] Windows CI: pinned `windows-2022` runner, added `flutter clean`/`flutter doctor -v`/artifact validation
- [x] CI: coverage enforcement now mandatory (removed `continue-on-error: true`)
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
