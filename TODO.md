# UOT TODO

## Active — Sprint 1 (Next)

### Core Discovery & Transfer
- `[ ]` Implement mDNS/NSD discovery provider (Rust)
- `[ ]` Implement TCP/LAN transport provider (Rust)
- `[ ]` Implement connection orchestrator (Rust)
- `[ ]` Implement device pairing flow (Rust)
- `[ ]` Implement secure session establishment (Rust)
- `[ ]` Implement file transfer engine (Rust)
- `[ ]` Implement chunked file I/O (Rust)
- `[ ]` Implement SHA-256 integrity verification (Rust)
- `[ ]` Wire discovery to Flutter UI (nearby screen)
- `[ ]` Wire file picker to transfer flow
- `[ ]` Implement transfer progress streaming (Rust→Dart)
- `[ ]` Implement transfer queue UI

### Environment
- `[ ]` Enable Developer Mode on Windows for symlink support
- `[ ]` Install Android SDK for local Android builds
- `[ ]` Set up Docker dev environment

### Testing
- `[ ]` Add Flutter widget tests for all screens
- `[ ]` Add integration tests for Rust→Dart bridge
- `[ ]` Set up coverage reporting (Rust + Dart)

---

## Backlog

### Sprint 2 — Reliability
- `[ ]` Persistent transfer state (survive app restart)
- `[ ]` Pause/resume transfers
- `[ ]` Automatic retry on failure
- `[ ]` Reconnection after network loss
- `[ ]` Transfer history with search

### Sprint 3 — QR
- `[ ]` QR pairing with secure invitation
- `[ ]` Animated QR data transport (fountain codes)
- `[ ]` QR scanner integration

### Sprint 4 — Platforms
- `[ ]` Android permissions and share sheet
- `[ ]` iOS permissions and share sheet
- `[ ]` macOS sandbox configuration
- `[ ]` Linux desktop integration
- `[ ]` Windows drag-and-drop

### Sprint 5 — Data Types
- `[ ]` Clipboard sharing
- `[ ]` Text/URL sharing
- `[ ]` Image quick-share

### Sprint 6 — Streaming
- `[ ]` WebRTC integration
- `[ ]` Camera streaming
- `[ ]` Screen sharing
- `[ ]` Video/audio file streaming

### Sprint 7 — Advanced Connectivity
- `[ ]` Bluetooth LE discovery
- `[ ]` Wi-Fi Direct transport
- `[ ]` Hotspot assistance
- `[ ]` Transport auto-switching

### Sprint 8 — Hardening
- `[ ]` Security audit
- `[ ]` Performance optimization
- `[ ]` Battery optimization
- `[ ]` Stress testing
- `[ ]` Accessibility audit
- `[ ]` Production documentation

---

## Completed History

### Sprint 0 — Foundation ✅ (2026-08-07)
- [x] Project scaffold (Flutter + Rust + FRB)
- [x] Rust core architecture (7 modules)
- [x] Protocol state machine (15 states)
- [x] Protocol messages (16 types)
- [x] Error hierarchy (30+ variants)
- [x] Configuration system
- [x] Transport abstraction (8 transport types)
- [x] Security/discovery/transfer/streaming traits
- [x] Flutter app shell (6 screens)
- [x] Material 3 theme (dark/light)
- [x] Adaptive navigation
- [x] 68 Rust unit tests (100% pass)
- [x] Developer skill
- [x] CI/CD workflows
- [x] Documentation suite
