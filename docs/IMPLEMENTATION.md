# UOT Implementation Status

> Last updated: Sprint 12, 2026-08-09.

## Architecture

```
Flutter UI (Dart) ──FRB FFI──> Rust Core Engine
                                ├── transport/tcp.rs (TCP/LAN)
                                ├── protocol/handler.rs (Wire messages)
                                ├── security/crypto.rs (AES-256-GCM + X25519)
                                ├── transfer/engine.rs (Chunking, resume)
                                ├── discovery/mdns.rs (mDNS browse/register)
                                └── streaming/pipeline.rs (H.264/AAC framing)
```

## Module Status

| Module | Lines | Tests | Status |
|--------|-------|-------|--------|
| `rust/src/core/engine.rs` | 55KB | Engine init, state, events | **IMPLEMENTED** |
| `rust/src/transport/tcp.rs` | 526 | 5 unit + E2E | **COMPLETE & PROVEN** |
| `rust/src/protocol/handler.rs` | 280 | Serialize/deserialize | **IMPLEMENTED** |
| `rust/src/security/crypto.rs` | 353 | 14 unit tests | **COMPLETE & PROVEN** |
| `rust/src/security/path_validator.rs` | 423 | 15+ unit tests | **COMPLETE & PROVEN** |
| `rust/src/security/qr.rs` | — | QR invitation | **PARTIAL** (no fountain) |
| `rust/src/transfer/engine.rs` | 15KB | E2E transfer tests | **IMPLEMENTED** |
| `rust/src/transfer/checkpoint.rs` | 230 | Save/load/list | **IMPLEMENTED** |
| `rust/src/discovery/mdns.rs` | 9.5KB | mDNS integration | **IMPLEMENTED** |
| `rust/src/streaming/pipeline.rs` | 190 | Framing tests | **PARTIAL** (no real media) |
| `rust/src/transport/ble.rs` | 37 | Structs only | **NOT IMPLEMENTED** |
| `rust/src/transport/wifidirect.rs` | 51 | Structs only | **NOT IMPLEMENTED** |
| `rust/src/transport/fallback.rs` | 96 | Selection tests | **PARTIAL** |

## Test Suite

| Suite | Count | Status |
|-------|-------|--------|
| Rust unit tests | 168 | ✅ Pass |
| Rust E2E load tests | 4 | ✅ Pass |
| Rust E2E transfer tests | 2 | ✅ Pass |
| Rust security tests | 19 | ✅ Pass |
| Rust doc tests | 0 | ✅ Pass |
| **Rust Total** | **197** | **✅ 100% Pass** |
| Flutter widget tests | 14 | ✅ Pass |

## Sprint 12 Changes

### Code Changes
- `lib/main.dart` — 3-state engine init (loading/failed/ready)
- `lib/src/features/diagnostics/rust_init_failed_screen.dart` — NEW: diagnostic recovery screen
- `android/.../MainActivity.kt` — Guarded plugin registration with `hasSystemFeature()`
- `android/.../network_security_config.xml` — NEW: LAN-only cleartext policy
- `.github/workflows/ci.yml` — Pinned `windows-2022`, `flutter clean`, coverage enforcement
- `rust/tests/security_tests.rs` — NEW: 19 security/fault-injection/recovery tests

### Documentation Changes
- `docs/PROTOCOL.md` — Corrected crypto (AES-256-GCM, not Noise XX)
- `docs/SECURITY.md` — Corrected all crypto references
- `docs/TRANSPORT_MATRIX.md` — NEW: honest transport status
- `docs/PLATFORM_SUPPORT.md` — Rewritten with honest status
- `docs/PRODUCTION_READINESS.md` — Rewritten with honest classification
- `docs/GAP_ANALYSIS.md` — Rewritten from actual audit
- `docs/PERFORMANCE.md` — Rewritten with actual benchmark numbers
