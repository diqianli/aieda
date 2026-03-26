# ARM CPU Emulator - Windows PowerShell Build Script
# Run this script in PowerShell: .\build_windows.ps1

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  ARM CPU Emulator - Windows Build" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check Rust installation
Write-Host "[CHECK] Verifying Rust installation..." -ForegroundColor Yellow
$cargoPath = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoPath) {
    Write-Host "[ERROR] Rust/Cargo not found!" -ForegroundColor Red
    Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor Yellow
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}

$rustVersion = rustc --version
Write-Host "[OK] Found: $rustVersion" -ForegroundColor Green

# Build steps
Write-Host ""
Write-Host "[1/4] Building release binary..." -ForegroundColor Yellow
cargo build --release --features visualization
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Build failed!" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host "[OK] Build complete" -ForegroundColor Green

Write-Host ""
Write-Host "[2/4] Building examples..." -ForegroundColor Yellow
cargo build --release --examples --features visualization
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Examples build failed!" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host "[OK] Examples built" -ForegroundColor Green

Write-Host ""
Write-Host "[3/4] Running simulation (100K instructions)..." -ForegroundColor Yellow
$simCmd = "cargo run --release --features visualization --example cpu_emulator 100000 visualization\static\konata_data.json"
Invoke-Expression $simCmd
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Simulation failed!" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host "[OK] Simulation complete" -ForegroundColor Green

Write-Host ""
Write-Host "[4/4] Creating output directory..." -ForegroundColor Yellow
if (-not (Test-Path "output")) {
    New-Item -ItemType Directory -Path "output" | Out-Null
}
Copy-Item "visualization\static\konata_data.json" "output\" -ErrorAction SilentlyContinue
Copy-Item "visualization\static\konata_data_topdown.json" "output\" -ErrorAction SilentlyContinue
Copy-Item "visualization\static\konata_data_report.html" "output\" -ErrorAction SilentlyContinue
Write-Host "[OK] Output files copied" -ForegroundColor Green

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Build Complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output files in output\:" -ForegroundColor White
Write-Host "  - konata_data.json          (Pipeline data)" -ForegroundColor Gray
Write-Host "  - konata_data_topdown.json  (TopDown analysis)" -ForegroundColor Gray
Write-Host "  - konata_data_report.html   (Visualization report)" -ForegroundColor Gray
Write-Host ""
Write-Host "To view visualization:" -ForegroundColor White
Write-Host "  cd visualization\static" -ForegroundColor Gray
Write-Host "  python -m http.server 8080" -ForegroundColor Gray
Write-Host "  Then open: http://localhost:8080/konata_data_report.html" -ForegroundColor Gray
Write-Host ""

Read-Host "Press Enter to exit"
