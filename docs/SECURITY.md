# Security Architecture — UOT

> Threat model, encryption specification, authentication flows, and path validation rules.

## Encryption

### Algorithm: AES-256-GCM (Authenticated Encryption with Associated Data)

- **Key size**: 256 bits (32 bytes)
- **Nonce size**: 96 bits (12 bytes), randomly generated per message
- **Authentication tag**: 128 bits (16 bytes), appended to ciphertext
- **Implementation**: `aes-gcm` crate v0.10 (RustCrypto)

### Key Exchange: X25519 Diffie-Hellman

- **Key pair generation**: `x25519-dalek` crate v2
- **Shared secret derivation**: X25519 DH + HKDF-SHA256 with domain separation (`UOT-session-key-v1`)
- **Session keys**: Ephemeral per connection — never reused

### Wire Protocol Encryption

All TCP frames (Control + Data) are encrypted with the negotiated session key after the key exchange handshake completes. Nonces are unique per frame.

## Authentication

### PIN-Based Verification

1. Receiver generates a 6-digit PIN (TTL: configurable, default 5 minutes)
2. PIN is displayed on receiver's screen
3. Sender enters PIN out-of-band (voice, visual)
4. On successful verification, a session token (SHA-256 of device_id + random bytes) is issued
5. Session token expires after 1 hour
6. Trusted devices can bypass PIN verification

### Trust Management

- `TrustManager` maintains a list of trusted device IDs
- Trust can be granted after successful PIN verification
- Trust can be revoked at any time
- Trusted devices auto-connect without PIN prompt

## Path Validation (Defense-in-Depth)

### `StrictPathValidator` (`security/path_validator.rs`)

Validates all received file paths against:

| Attack Vector | Defense |
|---------------|---------|
| Directory traversal (`../`) | Reject any `ParentDir` component |
| Absolute paths (`/etc/passwd`, `C:\`) | Reject `RootDir` and `Prefix` components |
| Null byte injection (`file\0.txt`) | Reject any path containing `\0` |
| URL-encoded traversal (`%2e%2e`) | Detect and reject encoded sequences |
| Windows reserved names (`CON`, `NUL`) | Reject stem matching reserved names |
| Illegal characters (`<>:"|?*`) | Reject per-character |
| Overly long filenames (>255 bytes) | Reject exceeding limit |
| Symlink attacks | Check `is_symlink()` before writing |
| Base directory escape | Verify resolved path starts with save directory |

### Fallback Sanitization

If validation fails, `sanitize_filename()` provides best-effort cleanup by removing dangerous characters, replacing traversal sequences, and prefixing reserved names.

## Resource Exhaustion Protection

- **Maximum frame size**: 64 MB (`MAX_MESSAGE_SIZE` in `transport/tcp.rs`)
- **Event channel buffer**: 256 messages (`mpsc::channel(256)`)
- **Event log ring buffer**: 200 entries max (`MAX_EVENT_LOG`)
- **Rate limiter**: Token bucket bandwidth throttler (`transfer/ratelimit.rs`)
- **Connection retry**: Exponential backoff with max 5 retries, max 30s delay

## Threat Model

| Threat | Mitigation | Status |
|--------|-----------|--------|
| Man-in-the-middle | AES-256-GCM encryption + X25519 key exchange | ✅ Implemented |
| Replay attacks | Unique nonces per message | ✅ Implemented |
| Data tampering | GCM authentication tag verification | ✅ Implemented |
| Path traversal | StrictPathValidator | ✅ Implemented |
| Symlink attacks | Pre-write symlink check | ✅ Implemented |
| Unauthorized access | PIN verification + session tokens | ✅ Implemented |
| Resource exhaustion | Frame size limits + rate limiter + bounded buffers | ✅ Implemented |
| Key reuse | Ephemeral session keys | ✅ Implemented |
| Secret logging | KeyPair private_key marked "never log" | ✅ Implemented |

## Module Map

| Module | Purpose |
|--------|---------|
| `security/mod.rs` | Trait definitions (`CryptoProvider`, `PathValidator`) |
| `security/crypto.rs` | AES-256-GCM + X25519 implementation |
| `security/path_validator.rs` | Strict path validation and sanitization |
| `security/verification.rs` | PIN verification + session tokens + trust manager |
| `security/qr.rs` | QR invitation encoding with ephemeral keys |
| `security/types.rs` | Security type definitions |
