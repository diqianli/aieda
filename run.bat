@echo off
echo ============================================
echo   ARM CPU Emulator - Windows
echo ============================================
echo.

REM Run simulation with sample ELF
cpu_emulator.exe sample.elf 100000

echo.
echo ============================================
echo   Simulation Complete!
echo ============================================
echo.
echo Output files:
echo   sample.json
echo   sample_topdown.json
echo   sample_report.html
echo.
echo To view visualization:
echo   python -m http.server 8080
echo   Open: http://localhost:8080/sample_report.html
echo.
pause
