# UOT Gap Analysis

> Audited against actual source code on 2026-08-09. Sprint 12.

## Summary

The UOT codebase has a substantial Rust core engine and Flutter UI, but significant gaps exist between documentation claims and actual implementation. This analysis separates real functionality from scaffolding.

## Critical Gaps (P0)

### 1. Android Launch Crash
- **Gap**: App crashes immediately on physical device launch
- **Root Cause**: `RustLib.init()` native library load failure not properly handled; Kotlin plugins crash on devices without BLE/Wi-Fi Direct
- **Fix Status**: In progress (Sprint 12) — diagnostic screen, guarded plugin registration

### 2. Windows CI Build Failure
- **Gap**: GitHub Actions Windows build fails consistently
- **Root Cause**: CMake generator detection conflict with Flutter tooling
- **Fix Status**: In progress (Sprint 12) — pinned runner, clean build, artifact validation

### 3. No Real-Device E2E Validation
- **Gap**: Zero real-device file transfers have been performed on any platform
- **Impact**: Cannot confirm the entire transfer pipeline actually works end-to-end
- **Requirement**: Two physical devices on same network

### 4. Documentation Crypto Mismatch
- **Gap**: Docs claimed "Noise Protocol XX" and "ChaCha20-Poly1305" but code uses AES-256-GCM + X25519
- **Fix Status**: Corrected in Sprint 12

## Major Gaps (P1)

### 5. BLE Transport — NOT IMPLEMENTED
- Code has: GATT UUIDs, `BleAdvertisement` struct (37 lines)
- Code lacks: GATT server, GATT client, BLE scanning, BLE connection, BLE data transfer
- Kotlin adapter exists but is not validated

### 6. Wi-Fi Direct Transport — NOT IMPLEMENTED
- Code has: `WifiDirectGroupInfo` struct (51 lines)
- Code lacks: P2P group creation, negotiation, connection, data transfer
- Kotlin adapter exists but is not validated

### 7. Media Streaming — PARTIAL
- Code has: H.264 NAL framing, AAC ADTS framing, jitter buffer, `StreamManager` session tracking
- Code lacks: Camera capture, microphone capture, screen capture, H.264 encoding, AAC encoding, video decoding, audio decoding, renderer, A/V sync, adaptive bitrate

### 8. Transport Fallback — PARTIAL
- Code has: `TransportFallbackManager` with selection logic
- Code lacks: Runtime transport switching during active session, session migration

### 9. QR Fountain Code — NOT IMPLEMENTED
- Code has: JSON QR invitation with expiry
- Code lacks: Fountain encoding, animated QR display, fountain decoding from camera frames

### 10. Coverage Enforcement
- **Gap**: CI coverage threshold check had `continue-on-error: true`
- **Fix Status**: Corrected in Sprint 12 — now enforced at 70%

## Moderate Gaps (P2)

| Area | Gap |
|------|-----|
| Trusted devices | UI screen exists, no persistent trust store or key revocation |
| Transfer history | In-memory only, no persistent database |
| Conflict handling | No replace/keep/rename/skip on duplicate filenames |
| Native share sheet | Not integrated |
| Automatic transport selection | Selection logic exists, not integrated into transfer flow |
| Large file benchmarks | No performance data at 1GB+ scale |
