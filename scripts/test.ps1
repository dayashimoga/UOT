# UOT Test Runner
# Runs all Rust and Dart tests
# Usage: . .\scripts\test.ps1

$ErrorActionPreference = "Continue"
$env:Path = "C:\flutter\bin;" + $env:Path

$exitCode = 0

Write-Host "=== UOT Test Suite ===" -ForegroundColor Cyan

# Rust tests
Write-Host "`n--- Rust Tests ---" -ForegroundColor Yellow
cargo test --manifest-path rust/Cargo.toml --verbose 2>&1
if ($LASTEXITCODE -ne 0) { $exitCode = 1 }

# Dart tests
Write-Host "`n--- Dart/Flutter Tests ---" -ForegroundColor Yellow
flutter test 2>&1
if ($LASTEXITCODE -ne 0) { $exitCode = 1 }

# Summary
if ($exitCode -eq 0) {
    Write-Host "`n=== ALL TESTS PASSED ===" -ForegroundColor Green
} else {
    Write-Host "`n=== SOME TESTS FAILED ===" -ForegroundColor Red
}

exit $exitCode
