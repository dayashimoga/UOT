# UOT TODO

## Active — Sprint 1 (Next)

### Core Discovery & Transfer
- `[x]` Implement mDNS/NSD discovery provider (Rust)
- `[x]` Implement TCP/LAN transport provider (Rust)
- `[x]` Implement connection orchestrator (Rust)
- `[x]` Implement device pairing flow (Rust)
- `[x]` Implement secure session establishment (Rust)
- `[x]` Implement file transfer engine (Rust)
- `[x]` Implement chunked file I/O (Rust)
- `[x]` Implement SHA-256 integrity verification (Rust)
- `[x]` Wire discovery to Flutter UI (nearby screen)
- `[x]` Wire file picker to transfer flow
- `[x]` Implement transfer progress streaming (Rust→Dart)
- `[x]` Implement transfer queue UI

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
- `[x]` Persistent transfer state (survive app restart `transfer/history.rs`)
- `[x]` Pause/resume transfers
- `[x]` Automatic retry on failure
- `[x]` Reconnection after network loss
- `[x]` Transfer history with search (`engine_search_history`)

### Sprint 3 — QR
- `[x]` QR pairing with secure invitation (`security/qr.rs` & `engine_api`)
- `[x]` Animated QR data transport (fountain codes `protocol/fountain.rs`)
- `[ ]` QR scanner integration

### Sprint 4 — Platforms
- `[ ]` Android permissions and share sheet
- `[ ]` iOS permissions and share sheet
- `[ ]` macOS sandbox configuration
- `[ ]` Linux desktop integration
- `[ ]` Windows drag-and-drop

### Sprint 5 — Data Types
- `[x]` Clipboard sharing
- `[x]` Text/URL sharing
- `[x]` Image quick-share

### Sprint 6 — Streaming
- `[ ]` WebRTC integration
- `[x]` Camera streaming
- `[x]` Screen sharing
- `[x]` Video/audio file streaming

### Sprint 7 — Advanced Connectivity
- `[x]` Bluetooth LE device discovery (`transport/ble.rs`)
- `[x]` Bluetooth LE data negotiation (`transport/ble.rs`)
- `[x]` Wi-Fi Direct transport (`transport/wifidirect.rs`)
- `[x]` Temporary hotspot creation (`transport/wifidirect.rs`)
- `[x]` Automatic transport switching and fallback (`transport/fallback.rs`)

### Sprint 8 — Hardening
- `[ ]` Security audit
- `[ ]` Performance optimization
- `[ ]` Battery optimization
- `[ ]` Stress testing
- `[ ]` Accessibility audit
- `[x]` Production documentation

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

### Additional Implemented Features
- `[x]` UserSettings persistence (`core/settings.rs` - JSON load/save to platform config dir)
- `[x]` ConnectionManager with exponential backoff auto-reconnection (`transport/connection_manager.rs`)
- `[x]` Settings API endpoints (`engine_load_settings`, `engine_save_settings`)
- `[x]` Flutter Settings UI auto-save integration (`settings_screen.dart`)
- `[x]` Security PIN verification & session tokens (`security/verification.rs`)
