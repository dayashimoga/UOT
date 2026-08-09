# UOT Wire Protocol Specification

## Overview

UOT uses a custom length-prefixed binary framing protocol over TCP sockets for all peer-to-peer communication.

## Cryptography

| Component | Algorithm | Status |
|-----------|-----------|--------|
| Key Exchange | X25519 ECDH | **IMPLEMENTED** |
| Key Derivation | SHA-256 HKDF with domain separator `UOT-session-key-v1` | **IMPLEMENTED** |
| Authenticated Encryption | AES-256-GCM (32-byte key, 12-byte nonce, 16-byte tag) | **IMPLEMENTED** |
| Integrity Hash | SHA-256 (per-file and per-chunk verification) | **IMPLEMENTED** |

> **Note**: Previous documentation incorrectly referenced "Noise Protocol XX" and "ChaCha20-Poly1305". These are **NOT** implemented. The actual implementation uses AES-256-GCM with X25519 key exchange as described above.

## Wire Frame Format

```
+-------------------+------------+------------------+
| Payload Length (4B)| Type (1B)  | Payload (N bytes)|
+-------------------+------------+------------------+
```

- **Payload Length**: 4-byte big-endian unsigned integer (max 64 MB)
- **Frame Type**: 1-byte enum:
  - `0x00` — Control (JSON protocol message)
  - `0x01` — Data (binary file chunk)
  - `0x02` — Ping (keepalive)
  - `0x03` — Pong (keepalive response)

## Protocol Messages (JSON, Frame Type 0x00)

All control messages use tagged JSON (`"type"` field):

| Message | Direction | Purpose |
|---------|-----------|---------|
| `hello` | Initiator → Responder | Device announcement (id, name, type, version, capabilities) |
| `hello_ack` | Responder → Initiator | Acknowledge hello |
| `key_exchange` | Bidirectional | X25519 public key for session encryption |
| `offer` | Sender → Receiver | Propose file transfer (items, sizes) |
| `offer_response` | Receiver → Sender | Accept/reject offer |
| `file_start` | Sender → Receiver | Begin sending file (name, size, path) |
| `file_end` | Sender → Receiver | File complete (SHA-256 hash) |
| `transfer_complete` | Sender → Receiver | All files transferred |
| `cancel` | Either → Either | Cancel transfer |
| `pause` | Either → Either | Pause transfer |
| `resume` | Either → Either | Resume transfer (with offset) |
| `clipboard_data` | Either → Either | Text/clipboard sharing |

## Connection Lifecycle

```
Initiator                    Responder
    |--- Hello ------------------>|
    |<-- HelloAck ----------------|
    |--- KeyExchange ------------>|
    |<-- KeyExchange -------------|
    |   (derive shared AES key)   |
    |--- Offer ------------------>|
    |<-- OfferResponse -----------|
    |--- FileStart -------------->|
    |--- Data (chunks) --------->|
    |--- FileEnd ---------------->|
    |--- TransferComplete ------->|
```

## Chunk Transfer

- Default chunk size: 64 KB
- Each chunk sent as Frame Type `0x01` (Data)
- Per-file SHA-256 hash verified at `FileEnd`
- Checkpoint state saved to disk for resume after interruption
