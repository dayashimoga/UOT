# UOT Hardware Certification Matrix

> Last updated: Sprint 12, 2026-08-09.

## Certification Levels

| Level | Definition | Evidence Required |
|-------|-----------|-------------------|
| **SOFTWARE PROVEN** | Validated by deterministic Rust/Dart tests | Unit + integration tests pass in CI |
| **EMULATOR PROVEN** | Validated by virtual node E2E or platform emulator | Two-node virtual transfer + SHA-256 match |
| **HARDWARE PENDING** | Requires physical device validation | Not yet tested on real hardware |
| **HARDWARE PROVEN** | Validated on physical devices | Device test report with model/OS/results |

---

## Transport Certification

| Transport | Software | Emulator | Hardware | Notes |
|-----------|----------|----------|----------|-------|
| **TCP/LAN** | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING | 197+ Rust tests, virtual E2E, localhost loopback |
| **BLE** | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING | FakeBleAdapter: advertise/scan/connect/MTU/fragment/disconnect |
| **Wi-Fi Direct** | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING | FakeWifiDirectAdapter: discover/group/connect/disconnect |
| **QR Fountain** | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING | FountainEncoder/Decoder: encode/loss/reconstruct/SHA-256 |
| **QUIC** | ❌ NOT IMPLEMENTED | ❌ N/A | ⏳ PENDING | Not in codebase |
| **WebRTC** | ❌ NOT IMPLEMENTED | ❌ N/A | ⏳ PENDING | Not in codebase |
| **USB** | ❌ NOT IMPLEMENTED | ❌ N/A | ⏳ PENDING | Not in codebase |

## Protocol Certification

| Feature | Software | Emulator | Hardware |
|---------|----------|----------|----------|
| Key Exchange (X25519) | ✅ PROVEN | ✅ PROVEN | N/A |
| Encryption (AES-256-GCM) | ✅ PROVEN | ✅ PROVEN | N/A |
| Replay Detection | ✅ PROVEN | ✅ PROVEN | N/A |
| Wire Message Serialization | ✅ PROVEN | ✅ PROVEN | N/A |
| Offer/Accept/Reject | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING |
| File Chunking + SHA-256 | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING |
| Checkpoint/Resume | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING |
| Transport Migration | ✅ PROVEN | ✅ PROVEN | ⏳ PENDING |
| Pause/Resume | ✅ PROVEN | ❌ N/A | ⏳ PENDING |
| Clipboard Sync | ✅ PROVEN | ❌ N/A | ⏳ PENDING |

## Security Certification

| Feature | Software | Emulator | Hardware |
|---------|----------|----------|----------|
| Path Traversal Prevention | ✅ PROVEN | N/A | N/A |
| Null Byte Injection | ✅ PROVEN | N/A | N/A |
| Windows Reserved Names | ✅ PROVEN | N/A | N/A |
| URL-Encoded Traversal | ✅ PROVEN | N/A | N/A |
| Crypto Tamper Detection | ✅ PROVEN | N/A | N/A |
| Truncated Ciphertext | ✅ PROVEN | N/A | N/A |
| PIN Verification | ✅ PROVEN | N/A | ⏳ PENDING |
| Malformed Protocol Msgs | ✅ PROVEN | N/A | N/A |

## Streaming Certification

| Feature | Software | Emulator | Hardware |
|---------|----------|----------|----------|
| H.264 NAL Framing | ✅ PROVEN | N/A | ⏳ PENDING |
| AAC Audio Framing | ✅ PROVEN | N/A | ⏳ PENDING |
| Jitter Buffer | ✅ PROVEN | N/A | ⏳ PENDING |
| Synthetic Video Source | ✅ PROVEN | ✅ PROVEN | N/A |
| Synthetic Audio Source | ✅ PROVEN | ✅ PROVEN | N/A |
| Real Camera Capture | ❌ N/A | ❌ N/A | ⏳ PENDING |
| Real Microphone Capture | ❌ N/A | ❌ N/A | ⏳ PENDING |
| A/V Sync | ❌ N/A | ❌ N/A | ⏳ PENDING |

## Platform Certification

| Platform | Build | Software Tests | Emulator | Hardware |
|----------|-------|---------------|----------|----------|
| **Android** | ✅ APK builds | ✅ Rust tests | ⏳ PENDING | ⏳ PENDING |
| **Windows** | ✅ CI builds | ✅ Rust tests | N/A | ⏳ PENDING |
| **Linux** | ✅ CI builds | ✅ Rust tests | N/A | ⏳ PENDING |
| **iOS** | ⏳ PENDING | ✅ Rust tests | ⏳ PENDING | ⏳ PENDING |
| **macOS** | ⏳ PENDING | ✅ Rust tests | N/A | ⏳ PENDING |

## Chaos/Fault Injection Certification

| Scenario | Status |
|----------|--------|
| Clean single file | ✅ PROVEN |
| Multi-file batch | ✅ PROVEN |
| Zero-byte files | ✅ PROVEN |
| Unicode filenames (CJK/Arabic/Emoji) | ✅ PROVEN |
| Checkpoint resume at 50% | ✅ PROVEN |
| Checkpoint resume at 10% | ✅ PROVEN |
| Transport migration (TCP→BLE) | ✅ PROVEN |
| Large file (10 MB+) | ✅ PROVEN |
| 50+ small files batch | ✅ PROVEN |
| Duplicate filenames | ✅ PROVEN |
| Packet loss (simulated) | ✅ PROVEN |
| Forced disconnect/reconnect | ✅ PROVEN |
| Bandwidth limiting | ✅ PROVEN |

## HARDWARE PENDING Items

> These items CANNOT be validated without physical hardware.

1. **Physical BLE interoperability** — requires Android/iOS devices with BLE 5.0+
2. **Physical Wi-Fi Direct P2P** — requires 2 Android devices
3. **Physical QR camera scan** — requires camera + display
4. **Real media capture** — requires camera, microphone
5. **Battery/thermal impact** — requires physical device under load
6. **RF interference/range** — requires physical environment
7. **Cross-device OS interop** — requires Android↔Windows, Android↔iOS, etc.
8. **Physical network interruption** — requires real network toggle
