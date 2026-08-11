#!/usr/bin/env bash
set -e

echo "=== Android E2E Automated Test Suite ==="

# Check connected device
if ! adb get-state > /dev/null 2>&1; then
  echo "::warning::No ADB device/emulator connected — skipping physical Android E2E test"
  exit 0
fi

echo "=== Running Flutter Integration Driver on ADB device ==="
C:/flutter/bin/flutter.bat test integration_test/full_workflow_test.dart

echo "✅ Android E2E integration test completed"
