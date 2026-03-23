@echo off
setlocal

cd /d "%~dp0"

echo [SafeClaw] Beende alte Dev-Prozesse...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-Process | Where-Object { $_.ProcessName -like '*safeclaw*' -or $_.ProcessName -like '*crabcage*' -or $_.ProcessName -like '*cargo*' -or $_.ProcessName -like '*node*' } | Stop-Process -Force" >nul 2>nul

echo [SafeClaw] Starte Tauri Dev...
cmd /c npm run tauri dev
