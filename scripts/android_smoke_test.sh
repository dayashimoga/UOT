#!/usr/bin/env bash
set -e

echo "=== Installing APK ==="
adb install apk/app-release.apk

echo "=== Launching app ==="
adb shell am start -n com.uot.uot_app/.MainActivity

echo "=== Waiting for app to start (15 seconds) ==="
sleep 15

echo "=== Checking if app process is alive ==="
if adb shell pidof com.uot.uot_app > /dev/null 2>&1; then
  echo "✅ App process is alive"
else
  echo "::error::App process is NOT alive — crash detected"
  adb logcat -d > emulator-logcat.txt 2>&1
  exit 1
fi

echo "=== Checking for ANR ==="
if adb shell dumpsys activity processes | grep -q "not responding"; then
  echo "::error::ANR detected"
  adb logcat -d > emulator-logcat.txt 2>&1
  exit 1
fi

echo "=== Capturing logcat ==="
adb logcat -d > emulator-logcat.txt 2>&1

echo "=== Checking logcat for fatal errors ==="
if grep -qE "FATAL EXCEPTION|Process: com.uot.uot_app.*SIGABRT|UnsatisfiedLinkError" emulator-logcat.txt; then
  echo "::error::Fatal exception found in logcat"
  cat emulator-logcat.txt | tail -100
  exit 1
fi

echo "✅ Android emulator smoke test passed"
