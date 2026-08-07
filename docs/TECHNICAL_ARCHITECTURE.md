# UOT Technical Architecture

## Overview

UOT uses a layered architecture: **Flutter UI → flutter_rust_bridge FFI → Rust Core Engine**.

```
┌──────────────────────────────────────────────────┐
│                  Flutter UI Layer                 │
│  ┌────────┬────────┬────────┬────────┬────────┐  │
│  │ Nearby │Transfer│Receive │ Stream │Settings│  │
│  └────────┴────────┴────────┴────────┴────────┘  │
│  ┌─────────────────────────────────────────────┐  │
│  │  Core: Theme · Router · DI · L10n           │  │
│  └─────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────┤
│            flutter_rust_bridge (FFI)              │
├──────────────────────────────────────────────────┤
│                 Rust Core Engine                  │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐      │
│  │    API    │ │  Protocol │ │  Security │      │
│  └───────────┘ └───────────┘ └───────────┘      │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐      │
│  │ Transport │ │ Discovery │ │ Transfer  │      │
│  └───────────┘ └───────────┘ └───────────┘      │
│  ┌───────────┐ ┌───────────┐                     │
│  │ Streaming │ │   Core    │                     │
│  └───────────┘ └───────────┘                     │
└──────────────────────────────────────────────────┘
```

## Data Flow

```
User selects files → Flutter UI → Rust API (FFI) → Transfer Engine
  → Protocol State Machine → Transport Layer → Network → Remote Device
                                                          ↓
Remote: Network → Transport → Protocol → Transfer Engine → File System
```

## Module Descriptions

### Rust Core (`rust/src/`)

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `api/` | FFI boundary to Dart | `get_version()`, `health_check()` |
| `core/` | Config, errors, version | `AppConfig`, `UotError`, `BuildInfo` |
| `transport/` | Network abstraction | `TransportConnection`, `TransportProvider` |
| `protocol/` | State machine + messages | `ProtocolState`, `ProtocolMessage` |
| `security/` | Crypto + auth | `CryptoProvider`, `PathValidator` |
| `discovery/` | Device finding | `DiscoveryProvider`, `DiscoveredDevice` |
| `transfer/` | File transfer engine | `TransferEngine`, `TransferProgress` |
| `streaming/` | Media streaming | `StreamCapabilityDetector`, `StreamManager` |

### Flutter UI (`lib/src/`)

| Module | Purpose |
|--------|---------|
| `core/theme/` | Material 3 dark/light theme |
| `core/router/` | Adaptive navigation (mobile/desktop) |
| `features/nearby/` | Device discovery screen |
| `features/transfers/` | Transfer queue and history |
| `features/receive/` | Receive configuration |
| `features/stream/` | Media streaming UI |
| `features/devices/` | Device management |
| `features/settings/` | Application settings |

## Key Design Principles

1. **All heavy lifting in Rust** — networking, protocol, security, file I/O
2. **Thin FFI layer** — API module is a pass-through, not business logic
3. **Trait-based abstraction** — every transport/discovery/security module implements traits
4. **Platform-agnostic core** — Rust code compiles for all targets
5. **Responsive UI** — adaptive layout for mobile (NavigationBar) and desktop (NavigationRail)
