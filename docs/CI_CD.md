# CI/CD Pipeline Specification — Universal Offline Transfer (UOT)

> **GitHub Actions workflow pipeline, quality gates, and artifact publishing.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. Pipeline Stages (`.github/workflows/ci.yml`)

Every push and pull request triggers parallel jobs:

```
                      ┌──────────────────────┐
                      │    Trigger: Push/PR   │
                      └──────────┬───────────┘
                                 │
              ┌──────────────────┴──────────────────┐
              ▼                                     ▼
   ┌────────────────────┐                ┌────────────────────┐
   │    Rust Check      │                │   Flutter Check    │
   │  - cargo fmt       │                │  - dart format     │
   │  - cargo clippy    │                │  - flutter analyze │
   │  - cargo test      │                │  - flutter test    │
   │  - cargo tarpaulin │                │    (--coverage)    │
   └──────────┬─────────┘                └──────────┬─────────┘
              │                                     │
              └──────────────────┬──────────────────┘
                                 │ (Needs: rust-check, flutter-check)
                                 ▼
   ┌─────────────────────────────────────────────────────────────┐
   │                      Multi-Platform Builds                  │
   │  - Web       - Android APK   - Windows   - Linux   - macOS │
   └─────────────────────────────────────────────────────────────┘
```

---

## 2. Quality Gates

- **Formatting Gate**: `cargo fmt --check` and `dart format --set-exit-if-changed .` must return 0 diffs.
- **Linter Gate**: `cargo clippy -- -D warnings` and `flutter analyze` must produce 0 warnings/errors.
- **Test Gate**: 100% pass rate across 174 Rust tests and 14 Flutter widget tests.
- **Coverage Gate**: Cargo tarpaulin coverage threshold gate enforced in CI.
- **Concurrency & Cancel In Progress**: Outdated builds automatically cancelled via `concurrency.cancel-in-progress: true`.
