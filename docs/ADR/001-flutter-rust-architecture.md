# ADR-001: Flutter + Rust Architecture

## Status: Accepted

## Date: 2026-08-07

## Context

UOT requires a cross-platform application supporting Android, iOS, Windows, macOS, Linux, and Web. It must perform high-throughput file transfers, handle networking/security in native code, and provide a polished, responsive UI.

## Decision

Use **Flutter** for the UI/cross-platform application layer and **Rust** for the core engine (networking, protocol, security, transfer, discovery), connected via **flutter_rust_bridge v2**.

## Rationale

### Why Flutter?
- Single codebase for 6 platforms (Android, iOS, Windows, macOS, Linux, Web)
- Material 3 design system with excellent theming support
- Hot reload for rapid UI development
- Large ecosystem of platform-specific plugins
- Strong community and corporate backing (Google)

### Why Rust?
- Memory safety without garbage collection
- Performance comparable to C/C++
- Excellent async/concurrency model (tokio)
- Strong type system prevents entire categories of bugs
- Cross-compilation to all target platforms
- Ideal for networking, security, and file I/O code

### Why flutter_rust_bridge?
- Type-safe FFI between Dart and Rust
- Supports streams for real-time progress updates
- Auto-generates Dart bindings from Rust code
- Active maintenance (v2.12.0 as of 2026)
- Handles memory management across the FFI boundary

## Alternatives Considered

1. **Pure Flutter (Dart only)**: Rejected because Dart's performance for file I/O, encryption, and networking is significantly lower than Rust. Battery/memory impact would be worse.

2. **Flutter + C/C++ FFI**: Rejected because C/C++ lacks Rust's memory safety guarantees. Higher risk of memory leaks, buffer overflows, and use-after-free bugs in production.

3. **React Native + Rust**: Rejected because React Native has weaker desktop support and more complex build systems for native code integration.

4. **Kotlin Multiplatform**: Rejected because it lacks iOS/desktop maturity and Rust's performance characteristics for file transfer workloads.

## Consequences

### Positive
- Shared core logic across all platforms
- Performance-critical code runs natively
- Memory-safe core prevents security vulnerabilities
- Type-safe FFI boundary catches interface bugs at compile time

### Negative
- Developers must know both Dart and Rust
- Build system is more complex (Cargo + pub + FRB codegen)
- FRB code generation adds a build step
- Web platform requires WASM compilation for Rust code
