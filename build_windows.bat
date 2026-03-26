@echo off
REM ARM CPU Emulator - Windows Build Script
REM This script builds the CPU emulator and generates visualization data

echo ============================================
echo   ARM CPU Emulator - Windows Build
echo ============================================
echo.

REM Check Rust installation
where cargo >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Rust/Cargo not found!
    echo Please install Rust from: https://rustup.rs/
    echo.
    pause
    exit /b 1
)

echo [1/4] Building release binary...
cargo build --release --features visualization
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Build failed!
    pause
    exit /b 1
)

echo.
echo [2/4] Building example programs...
cargo build --release --examples --features visualization
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Examples build failed!
    pause
    exit /b 1
)

echo.
echo [3/4] Running simulation (100K instructions)...
cargo run --release --features visualization --example cpu_emulator 100000 visualization\static\konata_data.json
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Simulation failed!
    pause
    exit /b 1
)

echo.
echo [4/4] Creating output directory...
if not exist "output" mkdir output
copy visualization\static\konata_data.json output\ >nul 2>&1
copy visualization\static\konata_data_topdown.json output\ >nul 2>&1
copy visualization\static\konata_data_report.html output\ >nul 2>&1

echo.
echo ============================================
echo   Build Complete!
echo ============================================
echo.
echo Output files in output\:
echo   - konata_data.json          (Pipeline data)
echo   - konata_data_topdown.json  (TopDown analysis)
echo   - konata_data_report.html   (Visualization report)
echo.
echo To view visualization:
echo   cd visualization\static
echo   python -m http.server 8080
echo   Then open: http://localhost:8080/konata_data_report.html
echo.
pause
