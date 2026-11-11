@echo off
setlocal
echo ====================================================
echo  HDMI-CEC Emulator - RP2040-Zero Quick Flasher
echo ====================================================

set FIRMWARE_UF2=%~dp0firmware\.pio\build\rp2040-zero\firmware.uf2

cd /d "%~dp0firmware"
echo [1/3] Compiling RP2040-Zero firmware...
pio run -e rp2040-zero
if errorlevel 1 (
    echo [ERROR] PlatformIO build failed.
    pause
    exit /b 1
)

echo.
echo [2/3] Checking for RPI-RP2 bootloader drive...
for %%d in (D E F G H I J K L M N O P Q R S T U V W X Y Z) do (
    if exist "%%d:\INFO_UF2.TXT" goto :found_drive
)

echo Triggering 1200bps USB reset into BOOTSEL mode...
if not "%~1"=="" (
    mode %~1 BAUD=1200 PARITY=N DATA=8 >nul 2>&1
    ping 127.0.0.1 -n 4 >nul
) else (
    echo Note: pass COM port as argument if board is not already in BOOTSEL ^(e.g. flash_rp2040.bat COM5^)
)

:found_drive
echo [3/3] Searching for RPI-RP2 drive...
for %%d in (D E F G H I J K L M N O P Q R S T U V W X Y Z) do (
    if exist "%%d:\INFO_UF2.TXT" (
        echo [OK] Found RP2040 bootloader drive on %%d:\
        echo Flashing %FIRMWARE_UF2% to %%d:\...
        copy /y "%FIRMWARE_UF2%" "%%d:\" >nul
        if errorlevel 0 (
            echo.
            echo [SUCCESS] Firmware copied successfully!
            echo The board will now reboot automatically as a USB CDC CEC-EMU adapter.
            ping 127.0.0.1 -n 3 >nul
            exit /b 0
        ) else (
            echo [ERROR] Copy failed.
            pause
            exit /b 1
        )
    )
)

echo [WARNING] No RP2040 bootloader drive (RPI-RP2) found!
echo Please hold the BOOT button on RP2040-Zero and plug the USB cable into PC.
pause
exit /b 1
