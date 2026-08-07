---
name: UOT Production Development
description: >
  Mandatory development skill for Universal Offline Transfer (UOT).
  Enforces architecture, coding standards, testing, security, Docker,
  CI/CD, documentation, sprints, releases, and definition-of-done rules.
  Must be read before every implementation task.
---

# UOT Production Development Skill

## Project Overview

**Universal Offline Transfer (UOT)** — A cross-platform offline-first file transfer
and streaming application built with **Flutter (UI) + Rust (core engine)**.

**Key paths:**
- Project root: `h:\UOT`
- Flutter app: `lib/`
- Rust core: `rust/src/`
- Rust API (FFI): `rust/src/api/`
- Tests (Dart): `test/`
- Tests (Rust): `rust/src/` (inline) + `rust/tests/`
- Integration tests: `integration_test/`
- Documentation: `docs/`
- CI/CD: `.github/workflows/`
- Docker: `docker/`
- Scripts: `scripts/`
- Code map: `docs/CODE_MAP.md`
- Architecture: `docs/TECHNICAL_ARCHITECTURE.md`

**Flutter path (not on system PATH):** `C:\flutter\bin\flutter.bat`
Always use full path or set `$env:Path = "C:\flutter\bin;" + $env:Path` first.

---

## Architecture Rules

### Layer Separation
1. **Rust Core** handles: protocol, networking, security, transfer engine, discovery,
   transport abstraction, integrity, state persistence, streaming engine.
2. **Flutter/Dart** handles: UI, navigation, state management, platform adapters,
   theme, accessibility, internationalization.
3. **flutter_rust_bridge v2** connects them via FFI. All Rust→Dart communication
   uses the generated bindings in `lib/src/rust/`.
4. **Never** put networking/protocol/security logic in Dart.
5. **Never** put UI logic in Rust.

### Module Organization
- `rust/src/api/` — Public API exposed to Dart via FRB. Keep thin.
- `rust/src/core/` — Configuration, errors, version, shared utilities.
- `rust/src/transport/` — Transport abstraction (traits + implementations).
- `rust/src/protocol/` — Transfer protocol state machine, messages.
- `rust/src/security/` — Encryption, authentication, key management.
- `rust/src/discovery/` — Device discovery (mDNS, BLE, etc.).
- `rust/src/transfer/` — Transfer engine (chunking, resume, integrity).
- `rust/src/streaming/` — Media streaming engine.

### Flutter Organization
- `lib/src/core/` — Theme, router, DI, l10n.
- `lib/src/features/` — Feature modules (nearby, transfers, receive, stream, devices, settings).
- `lib/src/shared/` — Reusable widgets and utilities.
- `lib/src/platform/` — Platform-specific adapters.

---

## Coding Standards

### Rust
- Edition: 2021+
- Format: `cargo fmt` (rustfmt defaults)
- Lint: `cargo clippy -- -D warnings`
- Use `thiserror` for error types
- Use `serde` for serialization
- Use `tokio` for async runtime
- Prefer `Arc<T>` over `Rc<T>` for thread safety
- No `unwrap()` in production code — use `?` or explicit error handling
- No `unsafe` without documented justification and review
- All public items must have doc comments (`///`)
- Use `#[cfg(test)]` for inline test modules

### Dart
- Format: `dart format .`
- Lint: `dart analyze` with strict rules
- Use `final` for immutable variables
- Use `const` constructors where possible
- Prefer `Widget` composition over inheritance
- All public APIs must have dartdoc comments
- No `print()` in production — use structured logging
- Use `freezed`/`riverpod` patterns for state management
- Avoid `dynamic` types

### General
- No hardcoded values — use configuration/constants
- No placeholder/mock/dummy implementations
- No suppressed warnings or `// ignore:` without documented reason
- No `TODO` comments without tracking in `TODO.md`

---

## Testing Requirements

### Coverage
- **Minimum: >90% line coverage** for both Rust and Dart
- **Critical modules** (security, protocol, transfer): target >95%
- Coverage must be **enforced by CI** — builds fail if coverage drops
- Coverage must **not be gamed** (no ignoring files, no empty tests)

### Test Categories
1. **Unit tests** — Every public function/method
2. **Integration tests** — Cross-module interaction
3. **Protocol tests** — State machine transitions, message parsing
4. **Transport tests** — Connection lifecycle, data flow
5. **Security tests** — Encryption, auth, key management, path traversal
6. **UI tests** — Widget rendering, navigation, interaction
7. **Platform tests** — Platform-specific behavior
8. **E2E tests** — Full user workflows
9. **Fault injection** — Disconnects, timeouts, corruption, resource exhaustion
10. **Regression tests** — Every bug fix gets a test

### Test Rules
- All tests must pass (100% pass rate)
- Tests must be deterministic — no flaky tests
- Tests must be fast — use mocks for I/O in unit tests
- Integration tests may use real I/O
- Never suppress failing tests to get green CI

---

## Security Rules

1. **Encrypt all communication** — TLS/noise protocol
2. **Authenticate devices/sessions** — mutual authentication
3. **Ephemeral session keys** — never reuse
4. **No secrets in logs** — no private keys, file contents, passwords
5. **Validate all inputs** — filenames, paths, sizes, message types
6. **Path traversal protection** — sanitize all file paths
7. **Symlink protection** — never follow symlinks from received files
8. **Resource exhaustion protection** — bound all buffers, queues, connections
9. **Session expiry** — all sessions must have timeouts
10. **Replay protection** — nonces/sequence numbers on all messages
11. **Secure storage** — use platform keychain/keystore for credentials
12. **Least privilege** — request minimum OS permissions

---

## Docker Rules

1. **Never install globally** what can run in Docker
2. **Detect existing tools** before adding Docker alternatives
3. **Multi-stage builds** to minimize image size
4. **Pin dependency versions** in Dockerfiles
5. **Provide `docker-compose.yml`** for multi-service setups
6. **Document all Docker commands** in `docs/INFRASTRUCTURE.md`

---

## CI/CD Rules

1. **Every PR** runs: format → lint → test → coverage → build
2. **Platform matrix**: Android, iOS (macOS runner), Windows, Linux, macOS, Web
3. **Never publish broken artifacts**
4. **Artifacts include**: commit SHA, version, platform, architecture, checksums
5. **Coverage gates** — PR fails if coverage drops below 90%
6. **Security scanning** — run `cargo audit` and dependency checks
7. **Caching** — cache Cargo, pub, and build artifacts for speed

---

## Documentation Rules

1. **Documentation must match reality** — no stale docs
2. **CHANGELOG.md is append-only** — never overwrite history
3. **TODO.md preserves history** — completed items move to history section
4. **ADRs for architectural changes** — document why, alternatives, consequences
5. **Update CODE_MAP.md** when adding/removing/moving files
6. **Update TEST_MATRIX.md** when adding test categories
7. **Traceability**: Requirement → Implementation → Test → Documentation

---

## Sprint Rules

1. **Work only in sprints** — each sprint ships working, tested, documented code
2. **Repository must be releasable** after every sprint
3. **Never ship scaffolding-only** unless the scaffold itself is validated
4. **End-of-sprint checklist** (see Definition of Done below)
5. **Sprint sequence**: S0→S1→S2→S3→S4→S5→S6→S7→S8

---

## Definition of Done

A task is complete **only when ALL of the following are true**:

- [ ] Implemented (real code, not placeholders)
- [ ] Integrated (works with existing code)
- [ ] Tested (unit + integration + relevant category tests)
- [ ] >90% code coverage maintained
- [ ] 100% test pass rate
- [ ] Secure (security review for relevant code)
- [ ] Formatted (`cargo fmt` + `dart format`)
- [ ] Linted (`cargo clippy` + `dart analyze`)
- [ ] Platform-validated (builds on target platforms)
- [ ] Documented (code docs + docs/ updates)
- [ ] CI passes (all workflows green)
- [ ] No regressions (existing functionality preserved)
- [ ] IMPLEMENTATION.md updated
- [ ] TODO.md updated
- [ ] CHANGELOG.md appended
- [ ] CODE_MAP.md updated if architecture changed

---

## Mandatory End-of-Iteration Updates

After every implementation iteration:

1. Update `IMPLEMENTATION.md`
2. Update `TODO.md`
3. **Append** to `CHANGELOG.md` (never overwrite)
4. Update `docs/REQUIREMENTS.md` if impacted
5. Update `docs/TECHNICAL_ARCHITECTURE.md` if impacted
6. Update `docs/TESTING.md` / `docs/TEST_MATRIX.md` if impacted
7. Update `docs/CODE_MAP.md` if architecture changed
8. Run all tests — require 100% pass
9. Verify >90% coverage
10. Run format + lint + static analysis
11. Ensure repository remains buildable/releasable

---

## AI Efficiency Rules

1. **Read this skill** before every implementation task
2. **Read `docs/CODE_MAP.md`** to understand existing code before making changes
3. **Use focused inspection** — don't re-read entire files unnecessarily
4. **Incremental changes** — small, targeted edits over full rewrites
5. **Reuse scripts** in `scripts/` for common operations
6. **Reference documentation** instead of re-analyzing
