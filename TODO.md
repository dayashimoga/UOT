# TODO — Universal Offline Transfer (UOT)

## Active Sprint Tasks (Sprint 8 — Final Hardening & Verification)
- [x] Integrate `TrustManager` & PIN verification into `UotEngine`
- [x] Implement incoming offer transfer consent gating (`accept_transfer` / `cancel_transfer`)
- [x] Add idle connection timeout (60s) to connection receive loop
- [x] Create Rust integration test suite (`rust/tests/integration_transfer.rs`)
- [x] Create Flutter widget tests for `ReceiveScreen` and `IncomingOfferDialog`
- [x] Add Docker container mesh setup (`Dockerfile` & `docker-compose.yml`)
- [x] Implement `FountainDecoder` with CRC32 verification for QR air-gap stream reconstruction (`protocol/fountain.rs`)
- [x] Integrate `StreamManager` into `UotEngine` (`start_stream`, `stop_stream`, `get_streams`)
- [x] Expose streaming FFI APIs (`engine_start_stream`, `engine_stop_stream`)
- [x] Update documentation suite (`GAP_ANALYSIS.md`, `PRODUCTION_READINESS.md`, `TESTING.md`)

## Complete Milestone History

### Sprint 0 — Foundation (Completed ✅)
- [x] Flutter 3.44 + Rust 1.97 project scaffold via flutter_rust_bridge v2.12
- [x] Core Rust architecture modules (`core/`, `transport/`, `protocol/`, `security/`, `discovery/`, `transfer/`, `streaming/`)
- [x] Flutter shell with 6 feature screens (Nearby, Transfers, Receive, Stream, Devices, Settings)
- [x] Material 3 Dark theme & adaptive navigation
- [x] Developer skill (`.agents/skills/production-development/SKILL.md`)
- [x] Initial CI/CD GitHub Actions workflows

### Sprint 1 — Core (Completed ✅)
- [x] mDNS peer discovery (`discovery/mdns.rs`)
- [x] TCP/LAN length-prefixed binary transport (`transport/tcp.rs`)
- [x] Main engine coordinator (`core/engine.rs`)
- [x] Chunked file transfer engine with CRC32 & SHA-256 integrity (`transfer/engine.rs`)
- [x] Engine FFI API layer (`api/engine_api.rs`)
- [x] Flutter UI integration for discovery & transfers

### Sprint 2 — Reliability & Wiring (Completed ✅)
- [x] Typed `WireMessage` protocol handler (`protocol/handler.rs`)
- [x] Automatic retry with exponential backoff (`transport/connection_manager.rs`)
- [x] UserSettings JSON persistence (`core/settings.rs`)
- [x] Real pause/resume controls with tokio watch channels

### Sprint 3 — Security & Data (Completed ✅)
- [x] AES-256-GCM envelope cipher & X25519 Diffie-Hellman key exchange (`security/crypto.rs`)
- [x] `StrictPathValidator` for path traversal, null-byte, symlink & Windows reserved name security (`security/path_validator.rs`)
- [x] Clipboard quick-transfer module (`transfer/clipboard.rs`)
- [x] `TrustManager`, 6-digit PIN verification, session tokens (`security/verification.rs`)
- [x] Interactive Flutter `IncomingOfferDialog` and offer consent gating

### Sprint 4 — Persistence & Analytics (Completed ✅)
- [x] `TransferHistoryStore` persistent JSON store with query/filtering (`transfer/history.rs`)
- [x] `LifetimeStats` cumulative analytics tracker (`transfer/analytics.rs`)
- [x] `TransferQueueManager` priority batch scheduling (`transfer/queue.rs`)
- [x] Token-bucket `RateLimiter` bandwidth throttling (`transfer/ratelimit.rs`)
- [x] Bounded event log ring buffer (200 entries)

### Sprint 5 — QR & Advanced Transport (Completed ✅)
- [x] Fountain code encoder & `FountainDecoder` with CRC32 validation (`protocol/fountain.rs`)
- [x] Secure QR pairing invitation payload (`security/qr.rs`)
- [x] Subnet active scanner over IPv4 /24 range (`discovery/subnet.rs`)
- [x] `TransportFallbackManager` multi-transport selection (`transport/fallback.rs`)
- [x] Docker multi-node simulation setup (`Dockerfile`, `docker-compose.yml`)

### Sprint 6 — Streaming & Control (Completed ✅)
- [x] `StreamManager` session lifecycle (`streaming/manager.rs`)
- [x] StreamManager integrated into UotEngine (`start_stream`, `stop_stream`, `get_streams`)
- [x] Automated test & quality gate script (`scripts/coverage.ps1`)
### Sprint 7 — Platform Adapters & Hardware Streaming (Partial ⚠️)
- [x] Platform-native BLE GATT host adapter interface (`lib/src/platform/ble_adapter.dart`) — ⚠️ stub/interface only
- [x] Platform-native Wi-Fi Direct P2P Group Owner adapter interface (`lib/src/platform/wifi_direct_adapter.dart`) — ⚠️ stub/interface only
- [x] Mobile camera optical QR code scanner UI dialog (`lib/src/features/nearby/qr_scanner_dialog.dart`) — ⚠️ simulated frames only
- [x] Live media payload H.264/AAC packet container types (`rust/src/streaming/pipeline.rs`) — ⚠️ data structures only
- [x] 150 Rust Tests (100% Pass) & 14 Flutter Tests (100% Pass)

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
- [x] 170 Rust Tests (100% Pass)

## Remaining Gaps (Deferred — Platform Limited)
- [ ] Load/stress testing (>100MB files, thousands of files, concurrent transfers)
- [ ] Multi-device real-hardware E2E (Android↔Windows, etc.)

## Future Roadmap (Platform & Hardware Extensions — PLATFORM LIMITED)
- [ ] Platform-native BLE GATT host adapters (Android NDK / iOS CoreBluetooth) — requires native code
- [ ] Platform-native Wi-Fi Direct P2P Group Owner (WifiP2pManager) — requires Android native
- [ ] Camera QR code optical scanner with native camera access — requires mobile_scanner plugin
- [ ] Hardware H.264/AAC media codec pipeline for live streaming — requires platform encoder APIs
- [ ] Hotspot creation via native API — requires platform-specific implementation

