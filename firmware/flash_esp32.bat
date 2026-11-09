@echo off
chcp 65001 >nul
cd /d "%~dp0"
set "PY_EXE=%LOCALAPPDATA%\Programs\Python\Python311\python.exe"
if not exist "%PY_EXE%" set "PY_EXE=python"
echo [*] Сборка и прошивка ESP32-S3...
"%PY_EXE%" -m platformio run -t upload
pause
