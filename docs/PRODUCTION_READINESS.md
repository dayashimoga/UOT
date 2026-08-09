# UOT Production Readiness

> Audited against actual source code on 2026-08-09. Sprint 12.

## Feature Classification

| Feature | Status | Evidence |
|---------|--------|----------|
| **TCP/LAN File Transfer** | **IMPLEMENTED BUT UNPROVEN** | Engine, protocol handler, chunking, framing all implemented. 178 Rust tests pass. No real-device E2E transfer validated. |
| **AES-256-GCM Encryption** | **COMPLETE & PROVEN** | Full encrypt/decrypt/key-exchange with 14 unit tests including tamper, wrong key, wrong nonce, large payload |
| **X25519 Key Exchange** | **COMPLETE & PROVEN** | DH shared secret derivation tested |
| **SHA-256 File Integrity** | **COMPLETE & PROVEN** | Hash computation tested, used in FileEnd messages |
| **mDNS Device Discovery** | **IMPLEMENTED BUT UNPROVEN** | `mdns-sd` integration implemented, not validated on real network |
| **Transfer Pause/Resume/Cancel** | **IMPLEMENTED BUT UNPROVEN** | Engine API exists, protocol messages defined, not E2E tested |
| **Checkpoint Resume** | **IMPLEMENTED BUT UNPROVEN** | Checkpoint store with save/load/delete, unit tested, not validated across app restart |
| **Path Traversal Protection** | **COMPLETE & PROVEN** | Validator with 15+ test cases |
| **QR Invitation** | **PARTIAL** | JSON generation/parsing with expiry. No fountain code. No animated QR. |
| **PIN Verification** | **IMPLEMENTED BUT UNPROVEN** | 6-digit PIN generation/verification in engine |
| **Clipboard Transfer** | **IMPLEMENTED BUT UNPROVEN** | Engine API and protocol message exist |
| **BLE Transport** | **NOT IMPLEMENTED** | Data structures only |
| **Wi-Fi Direct Transport** | **NOT IMPLEMENTED** | Data structures only |
| **QUIC Transport** | **NOT IMPLEMENTED** | Not in codebase |
| **WebRTC Transport** | **NOT IMPLEMENTED** | Not in codebase |
| **Transport Fallback** | **PARTIAL** | Selection logic only, no runtime migration |
| **Media Streaming** | **PARTIAL** | H.264/AAC packet framing + jitter buffer. No capture/encode/decode/render. |
| **Adaptive Bitrate** | **NOT IMPLEMENTED** | Not in codebase |
| **Trusted Devices** | **PARTIAL** | UI screen exists, no persistent trust store |
| **Android App** | **BLOCKED** | Crash on launch — P0 fix in progress |
| **Windows App** | **BLOCKED** | CI build failing — P0 fix pushed |
| **Linux/macOS/iOS Apps** | **IMPLEMENTED BUT UNPROVEN** | CI builds succeed, no runtime validation |

## Production Blockers (P0)

1. **Android crash on launch** — `RustLib.init()` native library load failure
2. **Windows CI build failure** — CMake generator detection
3. **No real-device E2E transfer validated on any platform**
4. **Documentation contained false crypto claims** (Noise XX/ChaCha20 vs actual AES-256-GCM/X25519) — corrected in Sprint 12

## NOT Production Ready

This application is NOT production-ready. Minimum requirements for production:
- [ ] Android launches successfully on physical device
- [ ] Windows CI produces verified release artifact
- [ ] At least one real-device file transfer works end-to-end
- [ ] >90% line+branch coverage enforced
- [ ] All P0 blockers resolved with evidence
