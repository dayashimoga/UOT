# Coverage & Test Suite Enforcement Script
$ErrorActionPreference = "Stop"

Write-Host "=== Running Rust Test Suite (100% Pass Required) ===" -ForegroundColor Cyan
cargo test --manifest-path rust/Cargo.toml
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust tests failed!"
    exit 1
}

Write-Host "=== Running Rust Format Check ===" -ForegroundColor Cyan
cargo fmt --manifest-path rust/Cargo.toml -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust formatting check failed!"
    exit 1
}

Write-Host "=== Running Rust Clippy Lint (0 Warnings Required) ===" -ForegroundColor Cyan
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "Clippy lints failed!"
    exit 1
}

Write-Host "=== Running Dart Format Check ===" -ForegroundColor Cyan
& "C:\flutter\bin\cache\dart-sdk\bin\dart.exe" format --set-exit-if-changed .
if ($LASTEXITCODE -ne 0) {
    Write-Error "Dart formatting check failed!"
    exit 1
}

Write-Host "=== Running Flutter Test Suite (100% Pass Required) ===" -ForegroundColor Cyan
& "C:\flutter\bin\flutter.bat" test --coverage
if ($LASTEXITCODE -ne 0) {
    Write-Error "Flutter tests failed!"
    exit 1
}

Write-Host "=== All Test & Quality Gates Passed Successfully! ===" -ForegroundColor Green
