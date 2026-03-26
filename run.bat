@echo off
echo ============================================
echo   ARM CPU Emulator - Windows
echo ============================================
echo.

REM Run simulation
cpu_emulator.exe 100000 static\konata_data.json

echo.
echo ============================================
echo   Simulation Complete!
echo ============================================
echo.
echo Output files:
echo   static\konata_data.json
echo   static\konata_data_topdown.json
echo   static\konata_data_report.html
echo.
echo To view visualization:
echo   cd static
echo   python -m http.server 8080
echo   Open: http://localhost:8080/konata_data_report.html
echo.
pause
