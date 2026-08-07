# UOT Code Map

> Auto-maintained map of the codebase. Updated after every architectural change.

## Directory Structure

```
h:\UOT/
├── .agents/                        # Agent skills
│   └── skills/production-development/
│       └── SKILL.md                # Mandatory dev skill
├── .github/workflows/
│   └── ci.yml                      # CI/CD for all platforms
├── docs/                           # Documentation
├── docker/                         # Docker environments
├── lib/                            # Flutter/Dart UI
│   ├── main.dart                   # App entry point
│   └── src/
│       ├── core/
│       │   ├── theme/
│       │   │   └── app_theme.dart  # Material 3 theme (dark/light)
│       │   └── router/
│       │       └── app_router.dart # Adaptive navigation
│       ├── features/
│       │   ├── nearby/
│       │   │   └── nearby_screen.dart    # Device discovery UI
│       │   ├── transfers/
│       │   │   └── transfers_screen.dart # Transfer queue/history
│       │   ├── receive/
│       │   │   └── receive_screen.dart   # Receive configuration
│       │   ├── stream/
│       │   │   └── stream_screen.dart    # Media streaming
│       │   ├── devices/
│       │   │   └── devices_screen.dart   # Device management
│       │   └── settings/
│       │       └── settings_screen.dart  # App settings
│       └── rust/                   # Auto-generated FRB bindings
├── rust/                           # Rust core engine
│   ├── Cargo.toml                  # Dependencies
│   └── src/
│       ├── lib.rs                  # Module registration
│       ├── frb_generated.rs        # FRB generated code
│       ├── api/                    # FFI API (exposed to Dart)
│       │   ├── mod.rs
│       │   ├── simple.rs           # Scaffold greeting (preserved)
│       │   ├── init.rs             # Version, health check
│       │   └── types.rs            # Shared API types
│       ├── core/                   # Shared utilities
│       │   ├── mod.rs
│       │   ├── config.rs           # App configuration
│       │   ├── error.rs            # Error type hierarchy
│       │   └── version.rs          # Version/build info
│       ├── transport/              # Transport abstraction
│       │   ├── mod.rs              # Traits: TransportConnection, TransportProvider
│       │   └── types.rs            # TransportId, TransportState, etc.
│       ├── protocol/               # Transfer protocol
│       │   ├── mod.rs
│       │   ├── state.rs            # Protocol state machine
│       │   └── messages.rs         # Protocol message types
│       ├── security/               # Security module
│       │   ├── mod.rs              # Traits: CryptoProvider, PathValidator
│       │   └── types.rs            # TrustedDevice, SessionToken, QrInvitation
│       ├── discovery/              # Device discovery
│       │   ├── mod.rs              # Traits: DiscoveryProvider, DiscoveryService
│       │   └── types.rs            # DiscoveredDevice, DeviceType, DiscoveryMethod
│       ├── transfer/               # Transfer engine
│       │   ├── mod.rs              # Trait: TransferEngine
│       │   └── types.rs            # TransferRecord, TransferProgress, etc.
│       └── streaming/              # Media streaming
│           ├── mod.rs              # Traits: StreamCapabilityDetector, StreamManager
│           └── types.rs            # StreamCapability, StreamConfig, StreamStatus
├── rust_builder/                   # FRB Cargokit builder
├── test/                           # Dart tests
├── integration_test/               # Flutter integration tests
├── scripts/                        # Build/test scripts
├── pubspec.yaml                    # Flutter dependencies
├── IMPLEMENTATION.md               # Implementation status
├── TODO.md                         # Current work items
├── CHANGELOG.md                    # Append-only changelog
├── ROADMAP.md                      # Sprint plan
└── README.md                       # Project overview
```

## Module Responsibilities

| Module | Language | Purpose |
|--------|----------|---------|
| `rust/src/api/` | Rust | FFI boundary — thin wrappers exposed to Dart |
| `rust/src/core/` | Rust | Configuration, errors, version, shared utils |
| `rust/src/transport/` | Rust | Transport abstraction (Wi-Fi, BT, QR, USB) |
| `rust/src/protocol/` | Rust | Protocol state machine and message types |
| `rust/src/security/` | Rust | Encryption, auth, key mgmt, path validation |
| `rust/src/discovery/` | Rust | Device discovery (mDNS, BLE, QR) |
| `rust/src/transfer/` | Rust | File transfer engine (chunking, resume) |
| `rust/src/streaming/` | Rust | Media streaming engine |
| `lib/src/core/theme/` | Dart | Material 3 dark/light theme system |
| `lib/src/core/router/` | Dart | Adaptive navigation (mobile/desktop) |
| `lib/src/features/` | Dart | Feature UI screens |

## Key Design Decisions

- **Rust core for performance**: All networking, protocol, security, and transfer logic runs in Rust for performance and memory safety.
- **Flutter for UI**: Cross-platform UI with Material 3, responsive layout.
- **flutter_rust_bridge v2**: Type-safe FFI with streams for progress updates.
- **Trait-based architecture**: All transports/discovery/security implement traits, allowing easy extension.
- **Dark-first theme**: High-contrast dark theme as default for readability.
