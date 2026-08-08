# UOT Testing Strategy & Matrix

## Test Suite Overview

| Category | Command | Count | Pass Rate |
|----------|---------|-------|-----------|
| **Rust Unit Tests** | `cargo test --manifest-path rust/Cargo.toml` | 126 | 100% |
| **Rust Integration Tests** | `cargo test --test integration_transfer` | 2 | 100% |
| **Flutter Widget Tests** | `flutter test` | 10 | 100% |
| **Rust Clippy Lint** | `cargo clippy --manifest-path rust/Cargo.toml -- -D warnings` | Clean (0 warnings) | 100% |
| **Flutter Analysis** | `dart analyze` | Clean | 100% |

---

## Test Inventory

### 1. Security Tests (`rust/src/security/`)
- AES-256-GCM encrypt/decrypt roundtrip
- Tampered ciphertext detection
- Invalid key & nonce length handling
- X25519 Diffie-Hellman key exchange shared secret verification
- Nonce uniqueness check
- Path traversal rejection (`../`, encoded sequences, null-bytes)
- Windows reserved filename sanitization (`CON`, `NUL`, `AUX`)
- Absolute path rejection & symlink check
- PIN expiry and verification test
- Session token generation and validation

### 2. Protocol Tests (`rust/src/protocol/`)
- WireMessage snake_case serialization roundtrips
- Fountain encoder packet generation
- Header creation and frame payload validation
- Protocol state machine transitions and invalid state error handling

### 3. Transport & Engine Integration Tests (`rust/src/transport/`, `rust/tests/`)
- TCP listener bind & socket accept
- TCP connection bidirectional framed read/write
- Engine lifecycle (start/stop)
- Two-engine loopback transfer integration test
- Queue manager batch priority scheduling test
