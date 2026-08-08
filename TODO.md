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
- [x] 147 Rust Tests (100% Pass) & 10 Flutter Tests (100% Pass)

## Future Roadmap (Platform & Hardware Extensions)
- [ ] Platform-native BLE GATT host adapters (Android NDK / iOS CoreBluetooth)
- [ ] Platform-native Wi-Fi Direct P2P Group Owner adapters (Android WifiP2pManager)
- [ ] Camera QR code optical scanner integration (Flutter camera package)
- [ ] Hardware H.264/AAC media payload codec pipeline for live streaming
