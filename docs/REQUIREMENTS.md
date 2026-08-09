# Functional and System Requirements — Universal Offline Transfer (UOT)

> **Traceable requirements specification for UOT core engine and UI.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. System Requirements

- **SR-01 Offline Operation**: Application must operate 100% offline without requiring internet access or cloud servers.
- **SR-02 Cross-Platform**: Engine must build and execute on Android, iOS, Windows, Linux, and macOS.
- **SR-03 High Performance**: Local transfers over Wi-Fi/LAN must exceed 100 MB/s where hardware permits.
- **SR-04 Memory Bounds**: RAM usage during active 5GB+ transfers must remain bounded under 64 MB.

---

## 2. Security Requirements

- **SEC-01 End-to-End Encryption**: All data in transit must be encrypted using AES-256-GCM with X25519 key exchange.
- **SEC-02 Replay Protection**: Nonce counters must strictly increment per frame to prevent replay attacks.
- **SEC-03 Path Traversal Shield**: Receiving engine must reject any file path containing parent traversal (`..`), null bytes, URL encoding, or illegal characters.
- **SEC-04 Authentication**: Device pairing must support 6-digit out-of-band PIN verification.

---

## 3. Transfer Requirements

- **TR-01 Resume Support**: Interrupted transfers must resume from exact checkpoint chunk byte offsets.
- **TR-02 Queue Management**: Transfers must support priority queues (`Low`, `Normal`, `High`, `Urgent`) and enforce concurrency limits.
- **TR-03 Integrity Verification**: Every chunk must verify CRC32; full file must verify SHA-256 upon completion.
