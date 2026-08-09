# UOT Performance

> Benchmarks from automated Rust test suite. Sprint 12.
> Run on: Windows 11, cargo test --release, localhost TCP loopback.

## Transfer Benchmarks

| Scenario | Size | Duration | Throughput | Encryption | Test File |
|----------|------|----------|-----------|------------|-----------|
| Single file encrypted | 100 MB | ~3.5s | ~228 Mbps | AES-256-GCM | `rust/tests/e2e_transfer.rs::test_100mb_encrypted_transfer` |
| Encrypted throughput | 100 MB | ~3.0s | ~267 Mbps | AES-256-GCM | `rust/tests/e2e_transfer.rs::test_encrypted_throughput_benchmark` |
| Multi-file batch | 50 files (10KB each) | <0.5s | N/A | AES-256-GCM | `rust/tests/e2e_transfer.rs::test_multi_file_batch_transfer` |
| Concurrent parallel (4x) | 4×25MB | ~4.5s | N/A | AES-256-GCM | `rust/tests/e2e_transfer.rs::test_concurrent_parallel_transfers` |

## Crypto Benchmarks

| Operation | Data Size | Notes |
|-----------|-----------|-------|
| AES-256-GCM encrypt | 1 MB | Sub-millisecond (tested in `crypto.rs::test_large_payload_encryption`) |
| X25519 key exchange | 32 bytes | Sub-millisecond |
| SHA-256 hash | Varies | Standard performance |

## Chunk Configuration

| Parameter | Value |
|-----------|-------|
| Default chunk size | 64 KB |
| Max frame size | 64 MB |
| TCP buffer | 8 KB initial, auto-grows |
| Channel capacity | 256 frames |

## Gaps

| Gap | Status |
|-----|--------|
| 1 GB benchmark | **PENDING** — requires extended test runtime |
| 5 GB benchmark | **PENDING** — requires extended test runtime |
| 10 GB+ benchmark | **PENDING** — requires extended test runtime |
| Cross-network (non-loopback) | **PENDING** — requires 2 physical devices |
| Battery impact measurement | **PENDING** — requires physical mobile device |
| Memory profiling | **PENDING** — no profiler integrated |

## Reproduction

```bash
# Run all benchmarks
cargo test --manifest-path rust/Cargo.toml --test e2e_transfer --release -- --nocapture

# Run specific benchmark
cargo test --manifest-path rust/Cargo.toml --test e2e_transfer test_100mb_encrypted_transfer --release -- --nocapture
```
