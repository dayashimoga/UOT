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
│       │   │   ├── nearby_screen.dart    # Device discovery + send (file_picker, engine)
│       │   │   └── widgets/quick_actions.dart # ★ QuickActionsBar (Files, Clipboard, QR, Subnet)
│       │   ├── transfers/
│       │   │   ├── transfers_screen.dart # Transfer queue/history (pause/cancel wired)
│       │   │   └── widgets/transfer_search.dart # ★ TransferSearchBar (text query & status filter)
│       │   ├── receive/
│       │   │   └── receive_screen.dart   # Receive configuration
│       │   ├── stream/
│       │   │   └── stream_screen.dart    # Media streaming
│       │   ├── devices/
│       │   │   └── devices_screen.dart   # Device management
│       │   └── settings/
│       │       └── settings_screen.dart  # App settings
│       └── rust/                   # Auto-generated FRB bindings
│           └── api/engine_api.dart # Generated engine API bindings
├── rust/                           # Rust core engine
│   ├── Cargo.toml                  # Dependencies
│   └── src/
│       ├── lib.rs                  # Module registration
│       ├── frb_generated.rs        # FRB generated code
│       ├── api/                    # FFI API (exposed to Dart)
│       │   ├── mod.rs
│       │   ├── simple.rs           # Scaffold greeting
│       │   ├── init.rs             # Version, health check
│       │   ├── types.rs            # Shared API types
│       │   └── engine_api.rs       # ★ Engine singleton API (init/stop/devices/transfers/send/pause/resume/cancel/clipboard/settings/streams)
│       ├── core/                   # Shared utilities
│       │   ├── mod.rs
│       │   ├── config.rs           # App configuration
│       │   ├── error.rs            # Error type hierarchy
│       │   ├── version.rs          # Version/build info
│       │   ├── engine.rs           # ★ UotEngine coordinator (lifecycle, mDNS+TCP, send/receive)
│       │   ├── settings.rs         # ★ UserSettings persistence (JSON load/save)
│       │   └── benchmark.rs        # ★ Real-time throughput & bandwidth benchmark engine
│       ├── transport/              # Transport layer
│       │   ├── mod.rs              # Traits: TransportConnection, TransportProvider
│       │   ├── types.rs            # TransportState, TransportStats
│       │   ├── tcp.rs              # ★ TCP/LAN transport (framing, send_frame/recv_frame)
│       │   ├── connection_manager.rs # ★ Auto-reconnect with exponential backoff
│       │   ├── ble.rs              # ★ BLE GATT service & advertisement
│       │   ├── wifidirect.rs       # ★ Wi-Fi Direct P2P group & AP hotspot
│       │   ├── fallback.rs         # ★ Transport auto-switching & fallback orchestrator
│       │   └── hotspot.rs          # ★ Access Point hotspot assist & status tracking
│       ├── protocol/               # Transfer protocol
│       │   ├── mod.rs
│       │   ├── state.rs            # Protocol state machine
│       │   ├── messages.rs         # Protocol message types
│       │   ├── handler.rs          # ★ WireMessage serialization over TCP frames
│       │   └── fountain.rs         # ★ Fountain/LT codes for animated QR transport
│       ├── security/               # Security module
│       │   ├── mod.rs              # Traits: CryptoProvider, PathValidator
│       │   ├── types.rs            # TrustedDevice, SessionToken
│       │   ├── qr.rs               # ★ Secure QR invitation & pairing data
│       │   ├── verification.rs     # ★ PIN verification, TrustManager
│       │   └── crypto.rs           # ★ SoftwareCryptoProvider (AES-256-GCM session encryption)
│       ├── discovery/              # Device discovery
│       │   ├── mod.rs              # Traits: DiscoveryProvider
│       │   ├── types.rs            # DiscoveredDevice, DeviceType
│       │   ├── mdns.rs             # ★ mDNS service registration/browsing
│       │   ├── subnet.rs           # ★ LAN subnet active scanner fallback
│       │   └── interface.rs        # ★ Network interface enumeration & IP binding helper
│       ├── transfer/               # Transfer engine
│       │   ├── mod.rs              # Trait: TransferEngine
│       │   ├── types.rs            # TransferRecord, TransferProgress
│       │   ├── engine.rs           # ★ Chunked I/O, CRC32+SHA-256, progress
│       │   ├── clipboard.rs        # ★ Clipboard text/URL/HTML transfer
│       │   ├── history.rs          # ★ Persistent transfer history & search store
│       │   ├── queue.rs            # ★ Transfer queue priority & scheduling manager
│       │   ├── analytics.rs        # ★ Lifetime transfer statistics & analytics manager
│       │   └── ratelimit.rs        # ★ Token bucket rate limiter & bandwidth throttler
│       └── streaming/              # Media streaming
│           ├── mod.rs              # Traits: StreamCapabilityDetector
│           ├── types.rs            # StreamCapability, StreamConfig
│           └── manager.rs          # ★ StreamManager session lifecycle
├── rust_builder/                   # FRB Cargokit builder
├── pubspec.yaml                    # Flutter deps (file_picker, path_provider)
├── CHANGELOG.md                    # Append-only changelog
├── TODO.md                         # Current work items
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
