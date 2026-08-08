# UOT Roadmap

## Sprint Plan

| Sprint | Target | Status | Key Deliverables |
|--------|--------|--------|------------------|
| **S0** | Foundation | ✅ Done | Architecture, CI, docs, tests, theme, navigation |
| **S1** | Core | ✅ Done | mDNS discovery, TCP/LAN transport, pairing, session, file engine, progress, SHA-256 |
| **S2** | Reliability | 🟡 Partial | Pause/resume, exponential backoff retry, auto-reconnection done (DB persistence pending) |
| **S3** | QR | 🔲 Pending | Secure QR pairing, animated QR transport (fountain codes) |
| **S4** | Platforms | 🔲 Pending | Android/iOS/Windows/macOS/Linux native integrations, permissions, share sheets |
| **S5** | Data | ✅ Done | Clipboard sync (text/URL/HTML), auto-detection, preview generation, UI integration |
| **S6** | Streaming | 🟡 Partial | StreamManager session lifecycle, stream API, session state tracking (capture pipeline pending) |
| **S7** | Advanced | 🔲 Pending | Bluetooth, Wi-Fi Direct, hotspot, transport switching |
| **S8** | Hardening | 🟡 In Progress | Documentation suite, CODE_MAP, CHANGELOG (security audit & stress tests pending) |

## Sprint Details

### S1 — Core (Completed ✅)
- [x] Implement mDNS service discovery
- [x] Implement TCP/LAN transport
- [x] Implement connection orchestrator (`UotEngine`)
- [x] Implement device pairing protocol
- [x] Implement secure session handling
- [x] Implement single/multi file transfer
- [x] Implement folder transfer
- [x] Implement real-time progress streaming
- [x] Implement integrity verification (SHA-256 & CRC32)
- [x] Wire to Flutter UI

### S2 — Reliability (In Progress 🟡)
- [ ] Persistent transfer state database
- [x] Chunked resumable transfers
- [x] Pause/resume controls
- [x] Automatic retry with exponential backoff (`ConnectionManager`)
- [x] Reconnection after network loss
- [ ] Transfer history with search/filter
- [x] Crash recovery (connection pool & framing state)

### S3 — QR (Pending 🔲)
- [ ] QR code generation with secure invitation data
- [ ] QR scanner with camera permission flow
- [ ] Animated QR visual data transport using fountain codes
- [ ] Frame sequencing, error correction, recovery
- [ ] Progress display for QR transport

### S4 — Platforms (Pending 🔲)
- [ ] Android: permissions, share sheet, background service
- [ ] iOS: permissions, share sheet, App Group for extensions
- [ ] Windows: drag-and-drop, system tray
- [ ] macOS: sandbox, drag-and-drop
- [ ] Linux: desktop integration, D-Bus

### S5 — Data Types (Completed ✅)
- [x] Clipboard sync between devices (`transfer/clipboard.rs`)
- [x] Text/URL quick-share (auto-detect & preview)
- [x] Image quick-share support
- [ ] Share sheet integration (Android/iOS native)

### S6 — Streaming (In Progress 🟡)
- [ ] WebRTC signaling over local network
- [x] Camera streaming session tracking (`StreamManager`)
- [x] Screen sharing session tracking (`StreamManager`)
- [x] Video file streaming session tracking (`StreamManager`)
- [x] Audio file streaming session tracking (`StreamManager`)
- [ ] Capture pipeline & adaptive buffering

### S7 — Advanced Connectivity (Pending 🔲)
- [ ] Bluetooth LE device discovery
- [ ] Bluetooth LE data negotiation
- [ ] Wi-Fi Direct transport
- [ ] Temporary hotspot creation
- [ ] Automatic transport switching and fallback

### S8 — Hardening (In Progress 🟡)
- [ ] Full security audit
- [ ] Performance benchmarking
- [ ] Battery/memory optimization
- [ ] Stress testing (large files, many files, slow networks)
- [ ] Fault injection testing
- [ ] Accessibility audit (screen readers, contrast)
- [x] Complete documentation review (`CODE_MAP.md`, `CHANGELOG.md`, `TODO.md`)
- [ ] Production release candidate
