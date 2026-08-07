# UOT Build & Test Scripts
# Usage: . .\scripts\setup.ps1

$ErrorActionPreference = "Continue"

# Add Flutter to PATH
$env:Path = "C:\flutter\bin;" + $env:Path

Write-Host "=== UOT Development Setup ===" -ForegroundColor Cyan

# Check tools
Write-Host "`nChecking tools..." -ForegroundColor Yellow

$tools = @{
    "Flutter" = { flutter --version 2>$null | Select-Object -First 1 }
    "Dart"    = { dart --version 2>$null }
    "Rust"    = { rustc --version 2>$null }
    "Cargo"   = { cargo --version 2>$null }
    "Git"     = { git --version 2>$null }
    "Docker"  = { docker --version 2>$null }
    "FRB"     = { flutter_rust_bridge_codegen --version 2>$null }
}

foreach ($tool in $tools.GetEnumerator()) {
    try {
        $result = & $tool.Value
        Write-Host "  [OK] $($tool.Key): $result" -ForegroundColor Green
    } catch {
        Write-Host "  [!!] $($tool.Key): NOT FOUND" -ForegroundColor Red
    }
}

# Generate FRB bindings
Write-Host "`nGenerating FRB bindings..." -ForegroundColor Yellow
flutter_rust_bridge_codegen generate

# Get Flutter dependencies
Write-Host "`nGetting Flutter dependencies..." -ForegroundColor Yellow
flutter pub get

Write-Host "`n=== Setup Complete ===" -ForegroundColor Green
Write-Host "Run 'flutter run' to start the app" -ForegroundColor Cyan
