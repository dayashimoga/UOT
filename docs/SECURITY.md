# UOT Security Architecture

> Audited against actual source code on 2026-08-09.

## Cryptographic Primitives

| Primitive | Algorithm | Library | Status |
|-----------|-----------|---------|--------|
| Key Exchange | X25519 ECDH | `x25519-dalek` v2 | **IMPLEMENTED** |
| Key Derivation | SHA-256 HKDF (domain: `UOT-session-key-v1`) | `sha2` v0.10 | **IMPLEMENTED** |
| Authenticated Encryption | AES-256-GCM | `aes-gcm` v0.10 | **IMPLEMENTED** |
| File Integrity | SHA-256 | `sha2` v0.10 | **IMPLEMENTED** |
| Frame Checksum | CRC32 | `crc32fast` v1 | **IMPLEMENTED** |
| Nonce Generation | 12-byte CSPRNG | `rand` v0.9 + `OsRng` | **IMPLEMENTED** |

> **Correction**: Previous documentation referenced "Noise Protocol XX" and "ChaCha20-Poly1305". These are **NOT** implemented. The actual implementation uses **AES-256-GCM** with **X25519** key exchange.

## Session Security

- Ephemeral X25519 key pairs generated per session
- Shared secret derived via Diffie-Hellman + HKDF-SHA256
- AES-256-GCM provides authenticated encryption (confidentiality + integrity + authentication)
- 12-byte random nonces per encryption operation
- Key material: 32 bytes (256 bits)

## Input Validation & Path Security

- **Path traversal protection**: Blocks `../`, `..\\`, encoded variants, null bytes
- **Reserved name blocking**: Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`)
- **Symlink protection**: Does not follow symlinks from received files
- **Filename sanitization**: Removes/replaces dangerous characters

## Network Security

- **Android**: Cleartext traffic restricted to localhost + RFC1918 private ranges via `network_security_config.xml`
- **All platforms**: Application-layer AES-256-GCM encryption on all transferred data regardless of transport security

## Known Gaps

| Gap | Status | Risk |
|-----|--------|------|
| No replay protection (nonce reuse detection) | **PENDING** | Medium — nonces are random but not tracked |
| No session expiry/timeout | **PENDING** | Low |
| No rate limiting on PIN verification | **PENDING** | Medium — brute-force 6-digit PIN |
| No secure storage for keys | **PENDING** | Low — ephemeral keys only |
| Secrets in logs not audited | **PENDING** | Medium |
| No formal security audit | **PENDING** | High |
