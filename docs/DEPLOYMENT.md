# Deployment & Build Guide — Universal Offline Transfer (UOT)

> **Building release binaries for Android, Windows, Linux, macOS, iOS, and Web.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. Prerequisites

- **Flutter SDK**: 3.24.0
- **Rust Toolchain**: 1.83+ (stable)
- **flutter_rust_bridge_codegen**: v2.12.0
- **Android NDK**: r26b (for Android cross-compilation)

---

## 2. Build Commands

### Android Release APK
```bash
flutter build apk --release
# Artifact generated: build/app/outputs/flutter-apk/app-release.apk (44.2 MB)
```

### Windows Release
```bash
flutter config --enable-windows-desktop
flutter build windows --release
# Artifact generated: build/windows/x64/runner/Release/
```

### Linux Release
```bash
flutter config --enable-linux-desktop
flutter build linux --release
# Artifact generated: build/linux/x64/release/bundle/
```

### Web Release
```bash
flutter build web --release
# Artifact generated: build/web/
```
