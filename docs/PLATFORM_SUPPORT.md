# UOT Platform Support

> Audited against actual source code and CI on 2026-08-09.

## Build Status

| Platform | Runner | Build Status | Runtime Validated | Notes |
|----------|--------|-------------|-------------------|-------|
| **Android** | `ubuntu-latest` + NDK | ✅ Builds (APK 23.9MB) | ❌ **Crash on launch** — P0 fix in progress | Native `.so` for arm64, armv7, x86_64 |
| **Windows** | `windows-2022` | ❌ **CI failing** — P0 fix pushed | ❌ Not validated | CMake generator detection issue |
| **Linux** | `ubuntu-latest` | ✅ Builds | ❌ Not validated | GTK3 desktop app |
| **macOS** | `macos-latest` | ✅ Builds | ❌ Not validated | Sandboxed with network entitlements |
| **iOS** | `macos-latest` | ✅ Builds (`--no-codesign`) | ❌ Not validated | Requires Apple Developer cert for device install |
| **Web** | `ubuntu-latest` | ✅ Builds | ❌ Not validated | WASM Rust support TBD |

## Platform-Specific Configuration

### Android
- **Min SDK**: 21 (Android 5.0)
- **Target SDK**: 35
- **Permissions**: Internet, Network State, Wi-Fi, Bluetooth (scan/advertise/connect), Camera, Storage
- **Network Security**: Cleartext restricted to localhost + RFC1918 private ranges via `network_security_config.xml`
- **Native Plugins**: `BleAdapterPlugin`, `WifiDirectAdapterPlugin` — guarded by `hasSystemFeature()` checks

### iOS
- `NSLocalNetworkUsageDescription` and `NSBonjourServices` (`_uot._tcp`) configured
- BLE and Camera usage descriptions in `Info.plist`

### macOS
- App Sandbox enabled with `network.client`, `network.server`, file read-write entitlements

### Windows
- Requires Visual Studio 2022 with C++ desktop workload (CI)
- Requires Developer Mode enabled (local builds only, for symlink support)

### Linux
- Requires GTK3 development headers (`libgtk-3-dev`, `liblzma-dev`)

## Real-Device Testing Status

**No real-device E2E transfers have been validated on any platform.** All "functional" claims are based on CI build success only.
