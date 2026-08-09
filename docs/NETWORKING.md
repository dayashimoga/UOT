# Networking Specification — Universal Offline Transfer (UOT)

> **Network architecture, discovery protocols, transport fallback, and framing rules.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. Network Topology & Discovery

```
                         ┌───────────────────────────┐
                         │   mDNS Peer Discovery     │
                         │    _uot._tcp.local:42000  │
                         └─────────────┬─────────────┘
                                       │ (Fallback if mDNS fails)
                                       ▼
                         ┌───────────────────────────┐
                         │   Subnet Active Scanner   │
                         │    IPv4 /24 Scan (42000)  │
                         └─────────────┬─────────────┘
                                       │
                                       ▼
                         ┌───────────────────────────┐
                         │  X25519 Key Exchange      │
                         │  AES-256-GCM TCP Framing   │
                         └───────────────────────────┘
```

---

## 2. Ports and Protocol Specs

| Component | Protocol | Port / Address | Payload Structure |
|-----------|----------|----------------|-------------------|
| **mDNS Broadcast** | UDP / Multicast | 5353 (`224.0.0.251`) | Service `_uot._tcp.local.`, TXT records (`device_name`, `device_id`, `version`) |
| **TCP Transport** | TCP / Unicast | 42000 (Default) | `[4-byte Length][1-byte Type][12-byte Nonce][Ciphertext][16-byte Tag]` |
| **Subnet Scanner** | TCP Ping | 42000 | Active probe connection on LAN `/24` subnet range |
| **BLE GATT** | Bluetooth LE | Service UUID | Base64 encoded pairing invitation payload |
