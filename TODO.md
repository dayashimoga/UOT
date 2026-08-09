# TODO — Universal Offline Transfer (UOT)

## Active Sprint Tasks (Sprint 12 — Roadmap & Future Enhancements)
- [ ] Universal Send broadcast across multiple LAN targets
- [ ] Adaptive video streaming relay over UOT session
- [ ] Folder two-way sync protocol extension

---

## Complete Milestone History

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
