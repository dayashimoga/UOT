# UOT Setup Guide

## Prerequisites

| Tool | Required Version | Install |
|------|-----------------|---------|
| Flutter | 3.44.6+ | [flutter.dev](https://flutter.dev/docs/get-started/install) |
| Rust | 1.97+ | [rustup.rs](https://rustup.rs/) |
| flutter_rust_bridge_codegen | 2.12.0 | `cargo install flutter_rust_bridge_codegen` |
| Git | 2.0+ | [git-scm.com](https://git-scm.com/) |

### Optional
| Tool | Purpose | Install |
|------|---------|---------|
| Docker | Linux builds, CI parity | [docker.com](https://www.docker.com/) |
| Android SDK | Android builds | [developer.android.com](https://developer.android.com/studio) |
| Visual Studio | Windows native builds | Desktop development with C++ workload |

## Quick Setup (Windows)

```powershell
# 1. Ensure Flutter is on PATH
$env:Path = "C:\flutter\bin;" + $env:Path

# 2. Install FRB codegen (if not installed)
cargo install flutter_rust_bridge_codegen

# 3. Generate bindings
flutter_rust_bridge_codegen generate

# 4. Get dependencies
flutter pub get

# 5. Run (Web — works without Visual Studio/Android SDK)
flutter run -d chrome
```

Or use the setup script:
```powershell
. .\scripts\setup.ps1
```

## Verification

```powershell
# Run all Rust tests
cargo test --manifest-path rust/Cargo.toml

# Run Flutter tests
flutter test

# Run linting
. .\scripts\lint.ps1
```

## Platform-Specific Notes

### Windows
- Enable Developer Mode for symlink support: `start ms-settings:developers`
- Install Visual Studio with "Desktop development with C++" for native Windows builds

### Android
- Install Android SDK via Android Studio
- Configure: `flutter config --android-sdk <path>`
- Add Rust targets: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
- Install cargo-ndk: `cargo install cargo-ndk`

### macOS/iOS
- Xcode required for iOS builds
- Add Rust targets: `rustup target add aarch64-apple-ios x86_64-apple-ios`

### Linux
- Install: `sudo apt install clang cmake ninja-build pkg-config libgtk-3-dev`
