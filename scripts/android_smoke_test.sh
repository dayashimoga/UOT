#!/usr/bin/env bash
set -e

echo "=== Installing APK ==="
adb install -r build/app/outputs/flutter-apk/app-release.apk || adb install -r apk/app-release.apk

echo "=== Launching app ==="
adb shell am start -n com.uot.uot_app/.MainActivity

echo "=== Bounded wait for MainActivity RESUMED state (10 seconds) ==="
sleep 10

echo "=== Verifying app process is alive ==="
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

echo "=== Checking logcat for fatal exceptions and crashes ==="
if grep -qE "FATAL EXCEPTION|Process: com.uot.uot_app.*SIGABRT|SIGSEGV|UnsatisfiedLinkError|MissingPluginException|SecurityException" emulator-logcat.txt; then
  echo "::error::Fatal exception or crash found in logcat"
  grep -C 5 -E "FATAL EXCEPTION|SIGABRT|SIGSEGV|UnsatisfiedLinkError|MissingPluginException|SecurityException" emulator-logcat.txt | tail -100
  exit 1
fi

echo "✅ Android emulator smoke test passed cleanly"
