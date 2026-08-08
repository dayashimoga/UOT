# UOT Master TODO & Implementation History

> **Note**: This document preserves all planned and implemented items over time. Items are marked `[x]` as completed and never removed.

---

## 1. Core Roadmap (Sprint 0 – 8)

### Sprint 0 — Foundation ✅ (Completed 2026-08-07)
- `[x]` Project scaffold (Flutter + Rust + FRB)
- `[x]` Rust core architecture (7 modules: api, core, transport, protocol, security, discovery, transfer, streaming)
- `[x]` Protocol state machine (15 states)
- `[x]` Protocol messages (16 types)
- `[x]` Error hierarchy (30+ variants)
- `[x]` Configuration system
- `[x]` Transport abstraction (8 transport types)
- `[x]` Security/discovery/transfer/streaming traits
- `[x]` Flutter app shell (6 screens: nearby, transfers, receive, stream, devices, settings)
- `[x]` Material 3 theme (dark/light)
- `[x]` Adaptive navigation
- `[x]` 68 Rust unit tests (100% pass)
- `[x]` Developer skill (`.agents/skills/production-development`)
- `[x]` CI/CD workflows (7 platform matrix)
- `[x]` Documentation suite

### Sprint 1 — Core Discovery & Transfer ✅ (Completed 2026-08-07)
- `[x]` Implement mDNS/NSD discovery provider (Rust `discovery/mdns.rs`)
- `[x]` Implement TCP/LAN transport provider (Rust `transport/tcp.rs`)
- `[x]` Implement connection orchestrator (Rust `core/engine.rs`)
- `[x]` Implement device pairing flow (Rust)
- `[x]` Implement secure session establishment (Rust)
- `[x]` Implement file transfer engine (Rust `transfer/engine.rs`)
- `[x]` Implement chunked file I/O (Rust)
- `[x]` Implement SHA-256 & CRC32 integrity verification (Rust)
- `[x]` Wire discovery to Flutter UI (`nearby_screen.dart` live polling)
- `[x]` Wire file picker to transfer flow (`file_picker` integration)
- `[x]` Implement transfer progress streaming (Rust→Dart real-time polling)
- `[x]` Implement transfer queue UI (`transfers_screen.dart`)

### Sprint 2 — Reliability & Protocol Handling ✅ (Completed 2026-08-07)
- `[x]` WireMessage serialization over TCP frames (`protocol/handler.rs`)
- `[x]` Bidirectional framed I/O on `TcpConnection` (`send_frame` / `recv_frame`)
- `[x]` FRB engine API bindings (`engine_api.rs` / `engine_api.dart`)
- `[x]` Live device polling in Nearby Screen (2s interval)
- `[x]` Real-time transfer polling in Transfers Screen (1s interval)
- `[x]` Pause/resume transfer APIs and UI handlers
- `[x]` Reconnection & retry primitives (`transport/connection_manager.rs`)
- `[ ]` Persistent transfer state (survive app restart)
- `[ ]` Transfer history with search

### Sprint 3 — Security, QR & Clipboard ✅ (Partially Completed 2026-08-07)
- `[x]` Clipboard sharing module (`transfer/clipboard.rs` - text, URL, HTML auto-detect)
- `[x]` Text/URL sharing via engine API (`engine_send_clipboard`)
- `[x]` Clipboard send UI integration in Nearby screen
- `[x]` PIN verification (6-digit, TTL-based `security/verification.rs`)
- `[x]` Session tokens and `TrustManager` for device pairing
- `[ ]` QR pairing with secure invitation
- `[ ]` Animated QR data transport (fountain codes)
- `[ ]` QR scanner integration

### Sprint 4 — Media Streaming & Capabilities ✅ (Completed 2026-08-07)
- `[x]` StreamManager session lifecycle (`streaming/manager.rs`)
- `[x]` StreamSession state tracking (`StreamType`: Camera/Screen/Video/Audio, `StreamState`)
- `[x]` Stream API endpoint (`engine_get_streams`)
- `[ ]` Camera/screen capture streaming pipeline implementation
- `[ ]` WebRTC / RTSP transport integration

### Sprint 5 — Settings & Persistence ✅ (Completed 2026-08-08)
- `[x]` UserSettings persistence (`core/settings.rs` - JSON load/save to platform config dir)
- `[x]` ConnectionManager with exponential backoff auto-reconnection (`transport/connection_manager.rs`)
- `[x]` Settings API endpoints (`engine_load_settings`, `engine_save_settings`)
- `[x]` Flutter Settings UI auto-save integration (`settings_screen.dart`)
- `[x]` Update CODE_MAP.md, CHANGELOG.md, and TODO.md

---

## 2. Platform & OS Integration Backlog

### Platform Features
- `[ ]` Android permissions and share sheet
- `[ ]` iOS permissions and share sheet
- `[ ]` macOS sandbox configuration
- `[ ]` Windows drag-and-drop
- `[ ]` Linux desktop integration

### Advanced Transports & Connectivity
- `[ ]` Bluetooth LE discovery
- `[ ]` Wi-Fi Direct transport
- `[ ]` Hotspot assistance
- `[ ]` Transport auto-switching
- `[ ]` USB transport
- `[ ]` Cloud relay fallback (optional)

---

## 3. Environment, Testing & Hardening Backlog

### Environment Setup
- `[ ]` Enable Developer Mode on Windows for symlink support
- `[ ]` Install Android SDK for local Android builds
- `[ ]` Set up Docker dev environment

### Quality Assurance & Hardening
- `[ ]` Run full Rust test suite and expand test coverage
- `[ ]` Flutter widget tests for all screens
- `[ ]` Integration tests for Rust→Dart bridge
- `[ ]` Set up coverage reporting (Rust + Dart)
- `[ ]` Security audit & penetration testing
- `[ ]` Performance & throughput benchmarks
- `[ ]` Battery optimization
- `[ ]` Stress testing under network latency / loss
- `[ ]` Accessibility audit (WCAG AAA compliance check)
