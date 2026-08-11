#!/usr/bin/env bash
set -e

echo "=== Network Fault & Recovery Injection Test Suite ==="

echo "Executing Cargo Network Fault Harness..."
cargo test --manifest-path rust/Cargo.toml --test network_fault_harness

echo "✅ Network fault injection tests passed"
