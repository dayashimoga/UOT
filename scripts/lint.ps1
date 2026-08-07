# UOT Lint Runner
# Runs formatting and linting for Rust and Dart
# Usage: . .\scripts\lint.ps1

$ErrorActionPreference = "Continue"
$env:Path = "C:\flutter\bin;" + $env:Path

$exitCode = 0

Write-Host "=== UOT Lint Suite ===" -ForegroundColor Cyan

# Rust formatting
Write-Host "`n--- Rust Format Check ---" -ForegroundColor Yellow
cargo fmt --manifest-path rust/Cargo.toml -- --check 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  Rust formatting issues found. Run: cargo fmt --manifest-path rust/Cargo.toml" -ForegroundColor Red
    $exitCode = 1
}

# Rust linting
Write-Host "`n--- Rust Clippy ---" -ForegroundColor Yellow
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  Clippy warnings found." -ForegroundColor Red
    $exitCode = 1
}

# Dart formatting
Write-Host "`n--- Dart Format Check ---" -ForegroundColor Yellow
dart format --set-exit-if-changed lib/ test/ 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  Dart formatting issues found. Run: dart format ." -ForegroundColor Red
    $exitCode = 1
}

# Dart analysis
Write-Host "`n--- Dart Analyze ---" -ForegroundColor Yellow
flutter analyze 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  Analysis issues found." -ForegroundColor Red
    $exitCode = 1
}

# Summary
if ($exitCode -eq 0) {
    Write-Host "`n=== ALL LINT CHECKS PASSED ===" -ForegroundColor Green
} else {
    Write-Host "`n=== LINT ISSUES FOUND ===" -ForegroundColor Red
}

exit $exitCode
