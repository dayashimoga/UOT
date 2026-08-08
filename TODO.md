# UOT TODO

## ✅ Completed

### Sprint 0 — Foundation
- `[x]` Project scaffold (Flutter 3.44 + Rust 1.97 + FRB v2.12)
- `[x]` 7 architectural modules with trait interfaces
- `[x]` Material 3 dark-first theme, adaptive navigation
- `[x]` 68 unit tests, CI/CD workflows
- `[x]` Documentation (README, CODE_MAP, CHANGELOG)

### Sprint 1 — Core
- `[x]` TCP/LAN transport with length-prefixed framing
- `[x]` mDNS discovery (register/browse/events)
- `[x]` File transfer engine (chunked I/O, CRC32, SHA-256)
- `[x]` UOT engine coordinator (lifecycle, orchestration)
- `[x]` Engine API singleton with tokio runtime

### Sprint 2 — Wiring
- `[x]` FRB bindings for all engine APIs
- `[x]` Live device polling (nearby screen)
- `[x]` File picker integration (files + folders)
- `[x]` Transfer polling (transfers screen)
- `[x]` Protocol handler (WireMessage over TCP frames)

### Sprint 3 — Features
- `[x]` Clipboard module (text/URL/HTML auto-detect)
- `[x]` PIN verification (6-digit, time-limited)
- `[x]` TrustManager (trusted devices, sessions)
- `[x]` Pause/resume/cancel/accept transfer APIs
- `[x]` Clipboard send wired to Flutter UI

### Sprint 4 — Streaming
- `[x]` StreamManager session lifecycle
- `[x]` StreamSession state tracking
- `[x]` Stream API endpoint

### Sprint 5 — Persistence & Reliability
- `[x]` UserSettings persistence (JSON load/save)
- `[x]` ConnectionManager with exponential backoff reconnection
- `[x]` Settings API (load/save from Flutter)
- `[x]` CODE_MAP.md updated with all new files

---

## Active — Next

### Testing & Polish
- `[ ]` Run full Rust test suite, fix any failures
- `[ ]` Add tests for new modules (settings, clipboard, verification, connection_manager)
- `[ ]` Run cargo fmt + dart format
- `[ ]` Flutter widget tests for all screens
- `[ ]` Integration tests for Rust→Dart bridge

### Environment
- `[ ]` Enable Developer Mode on Windows for symlink support
- `[ ]` Set up Docker dev environment

---

## Backlog

### QR Pairing
- `[ ]` QR pairing with secure invitation
- `[ ]` Animated QR data transport (fountain codes)

### Platform Integration
- `[ ]` Android permissions and share sheet
- `[ ]` iOS permissions and share sheet
- `[ ]` macOS sandbox configuration
- `[ ]` Windows drag-and-drop
- `[ ]` Linux desktop integration

### Advanced Features
- `[ ]` Persistent transfer state (survive app restart)
- `[ ]` Transfer history search
- `[ ]` Camera/screen streaming implementation
- `[ ]` Encryption (AES-256-GCM session encryption)
- `[ ]` Wi-Fi Direct transport
- `[ ]` Bluetooth transport
- `[ ]` USB transport
- `[ ]` Cloud relay (optional)
