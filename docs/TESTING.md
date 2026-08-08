# Testing Strategy — UOT

## Test Categories & Coverage

| Category | Framework | Location | Count | Status |
|----------|-----------|----------|-------|--------|
| Rust unit tests | `#[cfg(test)]` inline | `rust/src/**/*.rs` | 126 | ✅ All pass |
| Flutter widget tests | `flutter_test` | `test/` | 8 | ✅ All pass |
| Flutter integration tests | `integration_test` | `integration_test/` | 1 | ⚠️ FRB scaffold only |
| Rust integration tests | `cargo test` | `rust/tests/` | 0 | 🔴 Planned |
| E2E tests | TBD | TBD | 0 | 🔴 Planned |

## Rust Unit Test Breakdown

### Security Tests (33 tests)
- `security::crypto::tests` — AES-256-GCM encrypt/decrypt roundtrip, tampered data rejection, wrong key/nonce rejection, key exchange DH shared secret matching, nonce uniqueness, edge cases (empty data, large payload, invalid lengths)
- `security::path_validator::tests` — Directory traversal, absolute paths, null bytes, URL-encoded attacks, Windows reserved names, illegal characters, filename length limits, sanitization fallback

### Core Tests (21 tests)
- `core::config::tests` — Config defaults, validation, serialization
- `core::engine::tests` — Engine lifecycle (new, start, stop)
- `core::error::tests` — Error display formatting, downcasting
- `core::version::tests` — Version parsing, comparison

### Transfer Tests (17 tests)
- `transfer::engine::tests` — Chunked read/write, CRC32 integrity, SHA-256 hashing, directory collection, progress tracking, transfer record creation
- `transfer::types::tests` — Serialization, status display, speed formatting, ETA display

### Transport Tests (12 tests)
- `transport::tcp::tests` — Frame encoding, type conversion, listener bind, connect/accept, send/receive
- `transport::types::tests` — Transport ID display/serialization, capabilities, stats

### Protocol Tests (5 tests)
- `protocol::state::tests` — State machine transitions
- `protocol::messages::tests` — Message serialization
- `protocol::fountain::tests` — Fountain code encoding

### Discovery Tests (2 tests)
- `discovery::mdns::tests` — mDNS initialization
- `discovery::types::tests` — Device type display

### API Tests (5 tests)
- `api::engine_api::tests` — Engine state before init
- `api::init::tests` — Version, protocol version, build info
- `api::types::tests` — Connection info serialization, platform capabilities

## Running Tests

```bash
# All Rust tests
cargo test --manifest-path rust/Cargo.toml

# Specific module
cargo test --manifest-path rust/Cargo.toml security::crypto

# With output
cargo test --manifest-path rust/Cargo.toml -- --nocapture

# Flutter tests
C:\flutter\bin\flutter.bat test

# Clippy lint
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings

# Format check
cargo fmt --manifest-path rust/Cargo.toml -- --check
```

## Coverage Targets

| Module | Target | Rationale |
|--------|--------|-----------|
| `security/` | >95% | Critical security code |
| `protocol/` | >95% | Wire protocol correctness |
| `transfer/` | >90% | File I/O reliability |
| `transport/` | >90% | Network reliability |
| `core/` | >90% | Engine lifecycle |
| `api/` | >80% | Thin FFI layer |
| Overall | >90% | Production requirement |

## CI Integration

Tests run on every push/PR via `.github/workflows/ci.yml`:
- `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test` → `cargo audit`
- `dart format --set-exit-if-changed` → `flutter analyze` → `flutter test`
