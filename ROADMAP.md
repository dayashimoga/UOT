# UOT Roadmap

## Sprint Plan

| Sprint | Target | Status | Key Deliverables |
|--------|--------|--------|------------------|
| **S0** | Foundation | ✅ Done | Architecture, CI, docs, tests, theme, navigation |
| **S1** | Core | 🔲 Next | mDNS discovery, LAN transfer, pairing, sessions, files, progress |
| **S2** | Reliability | 🔲 | Chunking, pause/resume, retry, reconnect, crash recovery, history |
| **S3** | QR | 🔲 | Secure QR pairing, animated QR transport (fountain codes) |
| **S4** | Platforms | 🔲 | Android/iOS/Windows/macOS/Linux validation, permissions, integration |
| **S5** | Data | 🔲 | Clipboard, text, URLs, images, share sheet |
| **S6** | Streaming | 🔲 | WebRTC, camera, screen, video/audio streaming |
| **S7** | Advanced | 🔲 | Bluetooth, Wi-Fi Direct, hotspot, transport switching |
| **S8** | Hardening | 🔲 | Security audit, perf, stress tests, accessibility, production RC |

## Sprint Details

### S1 — Core (Next)
- Implement mDNS service discovery
- Implement TCP/LAN transport
- Implement connection orchestrator
- Implement device pairing protocol
- Implement secure session with ephemeral keys
- Implement single/multi file transfer
- Implement folder transfer
- Implement real-time progress streaming
- Implement integrity verification (SHA-256)
- Wire to Flutter UI

### S2 — Reliability
- Persistent transfer state database
- Chunked resumable transfers
- Pause/resume controls
- Automatic retry with backoff
- Reconnection after network loss
- Transfer history with search/filter
- Crash recovery (resume from last good chunk)

### S3 — QR
- QR code generation with secure invitation data
- QR scanner with camera permission flow
- Animated QR visual data transport using fountain codes
- Frame sequencing, error correction, recovery
- Progress display for QR transport

### S4 — Platforms
- Android: permissions, share sheet, background service
- iOS: permissions, share sheet, App Group for extensions
- Windows: drag-and-drop, system tray
- macOS: sandbox, drag-and-drop
- Linux: desktop integration, D-Bus

### S5 — Data Types
- Clipboard sync between devices
- Text/URL quick-share
- Image quick-share
- Share sheet integration (Android/iOS)

### S6 — Streaming
- WebRTC signaling over local network
- Camera streaming
- Screen sharing
- Video file streaming
- Audio file streaming
- Adaptive quality/buffering

### S7 — Advanced Connectivity
- Bluetooth LE device discovery
- Bluetooth LE data negotiation
- Wi-Fi Direct transport
- Temporary hotspot creation
- Automatic transport switching and fallback

### S8 — Hardening
- Full security audit
- Performance benchmarking
- Battery/memory optimization
- Stress testing (large files, many files, slow networks)
- Fault injection testing
- Accessibility audit (screen readers, contrast)
- Complete documentation review
- Production release candidate
