# UOT — Universal Offline Transfer

A cross-platform offline-first file transfer and streaming application built with **Flutter** (UI) and **Rust** (core engine).

## Features

- 📱 **Cross-Platform**: Android, iOS, Windows, macOS, Linux, Web
- 🔒 **Offline-First**: No internet/cloud/account required
- ⚡ **Fast**: Optimized chunked transfers with Rust core
- 🔐 **Secure**: Encrypted communication, device authentication
- 📊 **Reliable**: Pause/resume, retry, crash recovery
- 🎥 **Streaming**: Local video/audio/camera/screen streaming

## Quick Start

### Prerequisites

- [Flutter](https://flutter.dev/) 3.44.6+
- [Rust](https://rustup.rs/) 1.97+
- [flutter_rust_bridge_codegen](https://cjycode.com/flutter_rust_bridge/)

### Setup

```bash
# Clone the repository
git clone <repository-url>
cd UOT

# Generate FRB bindings
flutter_rust_bridge_codegen generate

# Get Flutter dependencies
flutter pub get

# Run on current platform
flutter run
```

### Test

```bash
# Rust tests
cargo test --manifest-path rust/Cargo.toml

# Flutter tests
flutter test
```

## Architecture

```
┌─────────────────────────┐
│     Flutter UI Layer     │
│  (Dart · Material 3)    │
├─────────────────────────┤
│   flutter_rust_bridge    │
│       (FFI · v2)         │
├─────────────────────────┤
│    Rust Core Engine      │
│  ┌───────┬───────────┐  │
│  │ API   │ Protocol  │  │
│  ├───────┼───────────┤  │
│  │Transport│ Security │  │
│  ├───────┼───────────┤  │
│  │Transfer│ Discovery │  │
│  ├───────┼───────────┤  │
│  │Streaming│  Core    │  │
│  └───────┴───────────┘  │
└─────────────────────────┘
```

## Documentation

See [docs/](docs/) for complete documentation including:

- [Technical Architecture](docs/TECHNICAL_ARCHITECTURE.md)
- [Protocol](docs/PROTOCOL.md)
- [Security](docs/SECURITY.md)
- [Setup Guide](docs/SETUP.md)
- [Code Map](docs/CODE_MAP.md)

## License

MIT
