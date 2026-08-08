# GAP Analysis — UOT Production Audit

> **Evidence-based audit** — every claim verified by code inspection.
> Updated: 2026-08-08 (Post-Sprint 8 Audit)

## Audit Methodology

For every module/feature, verification follows the chain:
**implemented → integrated → executable → platform-supported → E2E-tested → covered → secure → documented → CI-validated**

---

## Executive Summary

| Metric | Value |
|--------|-------|
| Rust source files | 48 |
| Rust source lines (excl. generated) | ~6,500 |
| Rust unit tests | 93 (all pass) |
| Flutter test files | 1 (theme_test.dart, 8 assertions) |
| Integration tests | 1 stub (FRB hello-world only) |
| Rust integration tests | 0 (no `rust/tests/` directory) |
| E2E tests | 0 |
| Coverage reporting | Not configured |
| Docker | Not created |
| CI/CD | 1 workflow (ci.yml), never validated on runners |
| Platform adapters (Dart) | 0 (`lib/src/platform/` doesn't exist) |
| Security: encryption in transfer pipeline | ❌ Not integrated |
| Security: auth in transfer pipeline | ❌ Not integrated |
| Streaming: actual data flow | ❌ State-only (no actual bytes streamed) |

---

## Detailed Module Audit

### 1. Core Engine (`core/engine.rs` — 782 lines) ✅ REAL

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | Full lifecycle: start/stop, discovery, TCP, send/receive |
| Integrated | ✅ | Uses `MdnsDiscovery`, `TcpTransportListener`, `TransferEngine` |
| Executable | ✅ | Tests `test_engine_start_stop` bind real TCP/mDNS |
| E2E tested | ❌ | No multi-device transfer E2E test |
| Secure | ⚠️ | Path traversal: basic `.replace("..", "_")` only (L507); no encryption on wire |
| Gaps | See P0-SEC-1, P0-SEC-2 |

**Critical findings:**
- `set_device_name()` (L699-703): Does nothing — just logs. Config is immutable.
- `get_recent_events()` (L742-744): Returns empty `Vec::new()` — stub.
- `get_streams()` (L747-751): Returns empty vec — streaming not integrated.
- Auto-accepts all incoming transfers (L482, comment "will add consent UI later").
- No encryption on any frame — plaintext TCP.
- Path sanitization (L507) is insufficient: only replaces `..` but doesn't handle encoded paths, absolute paths, or OS-specific traversal.

---

### 2. TCP Transport (`transport/tcp.rs` — 526 lines) ✅ REAL

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | Full framing (length-prefix), connect/bind/send/recv |
| Integrated | ✅ | Used by `UotEngine` for all transfers |
| Tested | ✅ | `test_tcp_listener_bind`, `test_tcp_connect_and_accept`, `test_tcp_connection_send_receive` |
| Secure | ❌ | No TLS/noise — plaintext TCP |
| Gap | P0-SEC-1 |

---

### 3. mDNS Discovery (`discovery/mdns.rs` — 254 lines) ✅ REAL

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | Uses `mdns-sd` crate, register/browse/stop |
| Integrated | ✅ | Used by `UotEngine::start()` |
| Tested | ✅ | `test_mdns_discovery_new` |
| Gap | No fallback when mDNS fails (subnet scanner exists but isn't integrated) |

---

### 4. Transfer Engine (`transfer/engine.rs` — 494 lines) ✅ REAL

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | Chunked read/write, CRC32, SHA-256 verify, progress tracking |
| Integrated | ✅ | Used by `UotEngine::send_files()` and `handle_incoming_connection()` |
| Tested | ✅ | 7 tests covering items, chunks, CRC, SHA-256, directory traversal |
| Secure | ⚠️ | CRC32+SHA-256 for integrity, but no encryption |
| Gaps | No resume-from-offset (pause just sets status flag, doesn't stop I/O) |

---

### 5. Security Module ⚠️ PARTIALLY REAL

#### 5a. Crypto (`security/crypto.rs`) — ❌ FAKE ENCRYPTION
| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ⚠️ | Has encrypt/decrypt functions |
| **CRITICAL** | ❌ | **Uses XOR cipher, NOT AES-256-GCM** (L62-66). Claims "AES-256-GCM" in docstring but implements byte-by-byte XOR with key/nonce. 4-byte truncated SHA-256 MAC is cryptographically weak. |
| Integrated | ❌ | **Not used anywhere in the transfer pipeline.** No frame encryption. |
| Gap | P0-SEC-1, P0-SEC-2 |

#### 5b. Verification (`security/verification.rs`) — ✅ REAL but not integrated
| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | PIN generation, session tokens, trust manager |
| Integrated | ❌ | **Not used by engine.** Engine auto-accepts all connections. |
| Gap | P0-SEC-3 |

#### 5c. QR Pairing (`security/qr.rs`) — ✅ REAL
| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | QR invitation encode/decode with expiry |
| Integrated | ⚠️ | API exists (`engine_generate_qr_invitation`) but uses placeholder values (L240-241) |

---

### 6. Protocol Handler (`protocol/handler.rs`) — ⚠️ DUAL SYSTEM

**Critical finding:** Two parallel protocol systems exist:
1. `protocol/handler.rs` — Typed `WireMessage` enum with proper `send_message`/`recv_message`
2. `core/engine.rs` — Raw `serde_json::json!()` with string-matched `"type"` field

The engine (item 2) is what actually runs. The protocol handler (item 1) is **dead code** — never called from engine.

| Gap | P1-ARCH-1: Protocol handler not integrated |

---

### 7. Streaming Module — ❌ STATE-ONLY STUB

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ⚠️ | `StreamManager` manages session metadata only |
| Integrated | ❌ | `engine.get_streams()` returns empty vec |
| Data flow | ❌ | **No actual streaming I/O.** No TCP stream reading, no media encoding, no codec integration. |
| Gap | P2-STREAM-1 |

---

### 8. BLE / Wi-Fi Direct / Hotspot — ❌ DATA-TYPE-ONLY STUBS

| Module | Lines | Real? | Evidence |
|--------|-------|-------|----------|
| `transport/ble.rs` | 37 | ❌ | Data struct + JSON ser/deser only. No GATT, no BLE stack. |
| `transport/wifidirect.rs` | 51 | ❌ | Config struct only. No P2P negotiation, no OS API. |
| `transport/hotspot.rs` | 41 | ❌ | Config struct only. No OS hotspot creation. |
| `discovery/interface.rs` | 42 | ⚠️ | Wraps `tcp::local_ips()`. Hardcodes "WLAN/Ethernet" name. |
| `discovery/subnet.rs` | 58 | ✅ | Real async subnet scanner, but not integrated into engine. |

---

### 9. Auxiliary Transfer Modules — ✅ REAL but not integrated

| Module | Real? | Integrated? | Notes |
|--------|-------|-------------|-------|
| `transfer/analytics.rs` | ✅ | ❌ | `LifetimeStats` loads/saves JSON but never called by engine on transfer completion |
| `transfer/history.rs` | ✅ | ❌ | `TransferHistoryStore` works but engine never calls `upsert()` |
| `transfer/queue.rs` | ✅ | ❌ | Priority queue exists but engine sends directly without queuing |
| `transfer/ratelimit.rs` | ✅ | ❌ | Token bucket rate limiter exists but not used in `execute_send()` |
| `transfer/clipboard.rs` | ✅ | ⚠️ | Engine has `send_clipboard()` but no receive handler |
| `protocol/fountain.rs` | ✅ | ❌ | Fountain encoder works (has test) but no decoder, no QR display |
| `transport/fallback.rs` | ✅ | ❌ | Strategy selection works but not called by engine |
| `transport/connection_manager.rs` | ✅ | ❌ | Reconnect logic works but engine manages connections directly |

---

### 10. FFI API Layer (`api/engine_api.rs`) — ⚠️ WORKS but has issues

| Check | Status | Evidence |
|-------|--------|----------|
| Implemented | ✅ | 20+ FFI endpoints for Flutter |
| Tested | ⚠️ | 1 test only (`test_engine_state_before_init`) |
| Issues | ⚠️ | QR invitation uses hardcoded "ephemeral_key_placeholder" (L240) |

---

### 11. Flutter UI — ⚠️ SHELL ONLY

| Check | Status | Evidence |
|-------|--------|----------|
| Screens | ✅ | 6 feature modules exist |
| Platform adapters | ❌ | `lib/src/platform/` doesn't exist |
| Rust integration | ⚠️ | Calls FFI via `frb_generated.dart` but screens show mock/sample data |
| Widget tests | ⚠️ | 1 file (theme_test.dart, 8 assertions) |
| Integration tests | ❌ | 1 stub (FRB hello-world, doesn't test any feature) |

---

### 12. CI/CD — ⚠️ CONFIGURED but incomplete

| Check | Status | Evidence |
|-------|--------|----------|
| Workflow exists | ✅ | `ci.yml` with 7 jobs |
| Rust checks | ✅ | fmt + clippy + test |
| Flutter checks | ✅ | format + analyze + test |
| Platform builds | ✅ | Web, Android, Windows, Linux, macOS, iOS |
| Coverage gates | ❌ | No coverage reporting or enforcement |
| Security scanning | ❌ | No `cargo audit` |
| Docker | ❌ | No `docker/` directory |
| Checksums | ⚠️ | Only Android APK has checksum step |
| Never run | ⚠️ | No evidence of successful CI runs |

---

### 13. Documentation — ⚠️ STALE

| Doc | Status | Evidence |
|-----|--------|----------|
| GAP_ANALYSIS.md | ❌ | Was Sprint 0 vintage (claims "traits only" when implementations exist) |
| CODE_MAP.md | ⚠️ | Exists but may not match current file structure |
| PROTOCOL.md | ⚠️ | Exists but describes typed WireMessage, not the JSON engine uses |
| TECHNICAL_ARCHITECTURE.md | ⚠️ | Exists |
| SETUP.md | ⚠️ | Exists |
| SECURITY.md | ❌ | Doesn't exist |
| TESTING.md | ❌ | Doesn't exist |
| TEST_MATRIX.md | ❌ | Doesn't exist |
| INFRASTRUCTURE.md | ❌ | Doesn't exist |

---

## Priority-Ranked Gap List

### P0 — Security (Must Fix Before Any Release)

| ID | Gap | Impact |
|----|-----|--------|
| P0-SEC-1 | **Crypto is XOR, not AES-256-GCM** | All data sent in plaintext or with broken encryption |
| P0-SEC-2 | **No encryption in transfer pipeline** | All file transfers are plaintext TCP |
| P0-SEC-3 | **No authentication integrated** | Engine auto-accepts all connections (L482) |
| P0-SEC-4 | **Path traversal protection insufficient** | Only `.replace("..", "_")` — bypassable |
| P0-SEC-5 | **QR invitation uses placeholder key** | "ephemeral_key_placeholder" in production API |

### P1 — Architecture / Integration (Required for functional product)

| ID | Gap | Impact |
|----|-----|--------|
| P1-ARCH-1 | Protocol handler (`WireMessage`) not used by engine | Dead code; two parallel protocol systems |
| P1-INT-1 | Analytics not integrated | Stats never recorded |
| P1-INT-2 | History not integrated | Transfer history never persisted |
| P1-INT-3 | Queue not integrated | No priority/scheduling |
| P1-INT-4 | Rate limiter not integrated | No bandwidth control |
| P1-INT-5 | Connection manager not integrated | No auto-reconnect |
| P1-INT-6 | Fallback manager not integrated | No transport switching |
| P1-INT-7 | Subnet scanner not integrated | No fallback discovery |
| P1-INT-8 | `set_device_name()` is no-op | Config is immutable |
| P1-INT-9 | `get_recent_events()` returns empty | Event log not implemented |
| P1-INT-10 | Pause/Resume only sets status flag | Doesn't stop actual I/O |

### P2 — Feature Completeness

| ID | Gap | Impact |
|----|-----|--------|
| P2-STREAM-1 | Streaming is state-only (no data flow) | Feature doesn't work |
| P2-BLE-1 | BLE transport is data-types only | No real BLE capability |
| P2-WIFI-1 | Wi-Fi Direct is data-types only | No real Wi-Fi Direct |
| P2-HOTSPOT-1 | Hotspot is data-types only | No real hotspot creation |
| P2-FOUNTAIN-1 | Fountain decoder missing | QR transport can't receive |
| P2-CLIPBOARD-1 | Clipboard receive handler missing | One-way clipboard only |
| P2-PLATFORM-1 | No Flutter platform adapters | No OS-specific integrations |

### P3 — Testing / CI / Quality

| ID | Gap | Impact |
|----|-----|--------|
| P3-TEST-1 | 0 Rust integration tests | No cross-module validation |
| P3-TEST-2 | 0 E2E tests | No full-flow validation |
| P3-TEST-3 | 1 Flutter test file (theme only) | No UI functional coverage |
| P3-TEST-4 | No coverage reporting | Can't enforce 90% gate |
| P3-TEST-5 | No fault injection tests | No resilience validation |
| P3-TEST-6 | No security tests | No crypto/auth validation |
| P3-CI-1 | No `cargo audit` in CI | No dependency vulnerability scanning |
| P3-CI-2 | No coverage gates in CI | Tests pass without coverage check |
| P3-CI-3 | No Docker configuration | No containerized testing |
| P3-DOC-1 | 5 required docs missing | SECURITY, TESTING, TEST_MATRIX, INFRASTRUCTURE, REQUIREMENTS |
| P3-DOC-2 | PROTOCOL.md describes wrong system | Documents WireMessage not engine's JSON |
