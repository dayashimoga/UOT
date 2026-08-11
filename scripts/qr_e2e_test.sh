#!/usr/bin/env bash
set -e

echo "=== Optical QR Payload E2E Automated Test Suite ==="

echo "1. Running Rust QR payload & security tests..."
cargo test --manifest-path rust/Cargo.toml --test qr_payload_e2e_test

echo "2. Running Dart QR payload unit & widget tests..."
C:/flutter/bin/flutter.bat test test/qr_payload_parsing_test.dart

echo "✅ Optical QR E2E automation passed"
