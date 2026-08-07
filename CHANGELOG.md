# CHANGELOG

All notable changes to UOT (Universal Offline Transfer) are documented here.
This file is **append-only** — history is never overwritten.

## [0.1.0-alpha] - 2026-08-07

### Sprint 0 — Foundation

#### Added
- **Project scaffold**: Flutter 3.44.6 + Rust 1.97.1 via flutter_rust_bridge v2.12.0
- **Rust core engine** with 7 architectural modules:
  - `core/` — Configuration, error types, version info
  - `transport/` — Transport abstraction traits (TransportConnection, TransportProvider)
  - `protocol/` — Protocol state machine and message types (16 message types)
  - `security/` — Crypto traits, path validation, session/device types
  - `discovery/` — Discovery traits and device types
  - `transfer/` — Transfer engine traits, progress tracking, speed/ETA formatting
  - `streaming/` — Stream capability detection, config, status types
- **Flutter app shell** with 6 feature screens:
  - Nearby (device discovery with scanning animation)
  - Transfers (queue and history)
  - Receive (visibility toggle, incoming requests)
  - Stream (camera, screen, video, audio options)
  - Devices (trusted device management)
  - Settings (theme, transfer, discovery, security, about)
- **Theme system**: Material 3 dark-first design with high-contrast text
- **Adaptive navigation**: NavigationBar (mobile) / NavigationRail (desktop ≥800px)
- **68 Rust unit tests** — all passing, covering errors, config, version, protocol state, messages, transport types, security types, discovery types, transfer types, streaming types
- **Mandatory developer skill**: `.agents/skills/production-development/SKILL.md`
- **GitHub Actions CI/CD**: Workflows for Rust check, Flutter check, and builds for Web, Android, Windows, Linux, macOS, iOS
- **Documentation**: README, CODE_MAP, CHANGELOG, IMPLEMENTATION, TODO, ROADMAP, GAP_ANALYSIS
- **Rust API endpoints** exposed to Dart: `getVersion()`, `getProtocolVersion()`, `getBuildInfo()`, `healthCheck()`

#### Technical Details
- Protocol state machine: 15 states with validated transitions
- Transport abstraction: 8 transport types (TCP/LAN, Wi-Fi Direct, BT Classic, BLE, QR, USB, Hotspot, Relay)
- Configuration system with validation (device name, chunk size, concurrent transfers, scan intervals)
- Error hierarchy with 7 error categories and 30+ specific error variants
- Transfer progress with speed formatting (B/s → KB/s → MB/s → GB/s) and ETA display
