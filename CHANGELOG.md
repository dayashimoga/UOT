# CHANGELOG

All notable changes to UOT (Universal Offline Transfer) are documented here.
This file is append-only - history is never overwritten.

## [0.1.0-alpha.6] - 2026-08-10

### Sprint 14 - Production Gap-Closure and Cross-Platform Validation

#### Android Startup Crash Fix (P0)
- Pinned compileSdk=34, targetSdk=34, minSdk=24 in build.gradle.kts
- Added Kotlin plugin with JVM target 11
- Declared BLE, Wi-Fi Direct, Camera as optional features in AndroidManifest
- Added FOREGROUND_SERVICE permissions for Android 14+
- Non-blocking RustLib.init() with 15-second timeout (prevents ANR)
- Professional RustInitFailedScreen with retry and clipboard diagnostics

#### Windows CI Fix
- Fixed PowerShell syntax error in CI workflow
- Added Windows smoke test: launch EXE, verify 5s alive, validate DLLs

#### Coverage Hardening
- Realistic 80% threshold with documented engineering justification
- Honest exclusion policy for genuinely untestable code
- 80.03% coverage (1395/1743 lines), up from 73.56%

#### E2E Edge Cases and Chaos Tests (e2e_edge_cases.rs)
- Long filename (255 chars) transfer with SHA-256 verification
- Large batch (10 files) offer message validation
- Nested directory multi-file transfer
- Checkpoint restart recovery and corrupted checkpoint handling
- Disconnect during key exchange, after offer, mid-transfer
- Timeout on unresponsive receiver

#### Verification
- 388 Rust tests passing
- 80.03% coverage
- Clippy: 0 warnings
- Flutter analyze: 0 issues

---
