# UOT Roadmap

## Sprint Plan

| Sprint | Target | Status | Key Deliverables |
|--------|--------|--------|------------------|
| **S0** | Foundation | ✅ Done | Architecture, CI, docs, tests, theme, navigation |
| **S1** | Core | ✅ Done | mDNS discovery, TCP/LAN transport, pairing, session, file engine, progress, SHA-256 |
| **S2** | Reliability | ✅ Done | Pause/resume (watch channels), exponential backoff retry, UserSettings persistence |
| **S3** | QR & Security | ✅ Done | AES-256-GCM+X25519, StrictPathValidator, TrustManager, secure QR pairing, FountainEncoder & FountainDecoder |
| **S4** | Data & Queue | ✅ Done | Clipboard sync, TransferQueueManager priority scheduling, RateLimiter bandwidth throttling |
| **S5** | Analytics & Subnet | ✅ Done | TransferHistoryStore search, LifetimeStats analytics, SubnetScanner fallback discovery |
| **S6** | Streaming | ✅ Done | StreamManager lifecycle integrated in UotEngine, stream start/stop APIs |
| **S7** | Advanced & Docker | ✅ Done | TransportFallbackManager orchestrator, Docker multi-node simulation mesh |
| **S8** | Validation & Docs | ✅ Done | 130 Rust tests (100% pass), 10 Flutter tests (100% pass), GAP_ANALYSIS, PRODUCTION_READINESS, TESTING |

---

## Complete Sprint Details & History

### S0 — Foundation (Completed ✅)
- [x] Flutter 3.44 + Rust 1.97 project scaffold via flutter_rust_bridge v2.12
- [x] Core Rust architecture modules (`core/`, `transport/`, `protocol/`, `security/`, `discovery/`, `transfer/`, `streaming/`)
- [x] Flutter shell with 6 feature screens (Nearby, Transfers, Receive, Stream, Devices, Settings)
- [x] Material 3 Dark theme & adaptive navigation
- [x] Developer skill (`.agents/skills/production-development/SKILL.md`)
- [x] GitHub Actions CI/CD workflows (`.github/workflows/ci.yml`)

### S1 — Core (Completed ✅)
- [x] Implement mDNS service discovery (`discovery/mdns.rs`)
- [x] Implement TCP/LAN length-prefixed transport (`transport/tcp.rs`)
- [x] Implement connection orchestrator (`UotEngine`)
- [x] Implement device pairing protocol
- [x] Implement secure session handling
- [x] Implement single/multi file transfer
- [x] Implement folder transfer with relative path preservation
- [x] Implement real-time progress tracking with sliding-window speed calculation
- [x] Implement integrity verification (SHA-256 & CRC32)
- [x] Wire to Flutter UI

### S2 — Reliability & Wiring (Completed ✅)
- [x] UserSettings JSON persistence (`core/settings.rs`)
- [x] Chunked resumable transfers
- [x] Real pause/resume controls with tokio watch channels
- [x] Automatic retry with exponential backoff (`transport/connection_manager.rs`)
- [x] Reconnection after network loss
- [x] Typed `WireMessage` protocol handler (`protocol/handler.rs`)
- [x] Framing state recovery

### S3 — QR & Security (Completed ✅)
- [x] AES-256-GCM envelope encryption & X25519 Diffie-Hellman key exchange (`security/crypto.rs`)
- [x] `StrictPathValidator` protection against path traversal, null-bytes, encoded attacks, symlinks, Windows reserved filenames (`security/path_validator.rs`)
- [x] `TrustManager`, 6-digit PIN verification, session tokens (`security/verification.rs`)
- [x] QR invitation generation with ephemeral X25519 key (`security/qr.rs`)
- [x] Animated QR visual data transport using Luby Transform fountain codes (`protocol/fountain.rs`)
- [x] `FountainDecoder` with CRC32 packet integrity validation

### S4 — Data & Queue Scheduling (Completed ✅)
- [x] Clipboard quick-transfer between devices (`transfer/clipboard.rs`)
- [x] Text/URL quick-share auto-detection & preview
- [x] `TransferQueueManager` priority scheduling (`Low`, `Normal`, `High`, `Urgent`) (`transfer/queue.rs`)
- [x] Token-bucket `RateLimiter` bandwidth throttling (`transfer/ratelimit.rs`)
- [x] Interactive Flutter `IncomingOfferDialog` consent modal & PIN verification UI

### S5 — Analytics & Subnet Discovery (Completed ✅)
- [x] Persistent `TransferHistoryStore` JSON store with text search & status filtering (`transfer/history.rs`)
- [x] `LifetimeStats` cumulative analytics tracker (`transfer/analytics.rs`)
- [x] Active IPv4 /24 subnet scanner fallback (`discovery/subnet.rs`)
- [x] Bounded event log ring buffer (200 entries) in `UotEngine`

### S6 — Streaming (Completed ✅)
- [x] `StreamManager` session lifecycle (`streaming/manager.rs`)
- [x] Camera, Screen, Video, Audio stream session tracking
- [x] `StreamManager` integrated into `UotEngine` (`start_stream`, `stop_stream`, `get_streams`)
- [x] FFI streaming endpoints (`engine_start_stream`, `engine_stop_stream`)

### S7 — Advanced Connectivity & Docker (Completed ✅)
- [x] BLE GATT service UUID definitions and advertisement payload framing (`transport/ble.rs`)
- [x] Wi-Fi Direct P2P Group negotiation structure (`transport/wifidirect.rs`)
- [x] Temporary Access Point hotspot configuration helper (`transport/hotspot.rs`)
- [x] `TransportFallbackManager` multi-transport selection (`transport/fallback.rs`)
- [x] Multi-stage `Dockerfile` and 2-node isolated bridge `docker-compose.yml`

### S8 — Hardening & Validation (Completed ✅)
- [x] 126 Rust unit tests + 2 integration tests (128 total, 100% pass)
- [x] 10 Flutter widget tests (100% pass)
- [x] Clippy lint clean (`cargo clippy -- -D warnings`)
- [x] Updated documentation suite (`GAP_ANALYSIS.md`, `PRODUCTION_READINESS.md`, `TESTING.md`, `TODO.md`, `IMPLEMENTATION.md`, `CHANGELOG.md`)

---

## Future Hardware & Native OS Extensions

### Native Mobile Adapters
- [x] Android: BLE GATT host adapter (`lib/src/platform/ble_adapter.dart`)
- [x] Android: Wi-Fi Direct P2P Group Owner adapter (`lib/src/platform/wifi_direct_adapter.dart`)
- [x] iOS: CoreBluetooth GATT peripheral/central adapter (`lib/src/platform/ble_adapter.dart`)
- [x] iOS: Multipeer Connectivity framework integration (`lib/src/platform/wifi_direct_adapter.dart`)

### Hardware Payload Pipelines
- [x] Flutter camera package optical QR scanner UI (`lib/src/features/nearby/qr_scanner_dialog.dart`)
- [x] Hardware H.264 / AAC video streaming encoder/decoder pipeline (`rust/src/streaming/pipeline.rs`)
