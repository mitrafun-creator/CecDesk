@echo off
setlocal
set PATH=C:\rust\mingw\bin;C:\rust\toolchain\bin;%PATH%
set RUSTUP_HOME=C:\rust\rustup
set CARGO_HOME=C:\rust\cargo
set CARGO_TARGET_DIR=C:\rust_target
set LIBRARY_PATH=C:\rust_target\lib
set RUSTFLAGS=-L C:/rust_target/lib
set RUSTC=C:\rust\toolchain\bin\rustc.exe

cd /d "%~dp0tauri_app\src-tauri"
C:\rust\toolchain\bin\cargo.exe build --release

if %ERRORLEVEL% EQU 0 (
    echo [OK] Build succeeded. Updating HDMI_CEC_Studio.exe...
    taskkill /f /im HDMI_CEC_Studio.exe >nul 2>&1
    ping 127.0.0.1 -n 2 >nul
    copy /y "C:\rust_target\release\hdmi_cec_studio.exe" "%~dp0HDMI_CEC_Studio.exe" >nul
    if %ERRORLEVEL% EQU 0 (
        echo [OK] Successfully updated HDMI_CEC_Studio.exe!
    ) else (
        echo [ERROR] Failed to copy hdmi_cec_studio.exe. File may still be running or locked.
        exit /b 1
    )
) else (
    echo [ERROR] Cargo build failed with code %ERRORLEVEL%.
)
