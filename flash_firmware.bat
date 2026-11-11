@echo off
chcp 65001 >nul
title Прошивка CecDesk Dongle dev/r.1 (ESP32-S3)
echo ==============================================================
echo        Прошивка контроллера CecDesk Dongle (Dimitryi Garutt)
echo ==============================================================
echo.
echo [1/3] Завершение запущенного процесса CecDesk App...
taskkill /f /im CecDesk.exe >nul 2>&1
taskkill /f /im HDMI_CEC_Studio.exe >nul 2>&1
timeout /t 1 /nobreak >nul

echo [2/3] Подготовка контроллера к прошивке...
echo       Если прошивка не начинается автоматически:
echo       Зажмите кнопку BOOT на плате и нажмите RST.
echo.
echo [3/3] Запуск сборки и прошивки через PlatformIO...
cd /d "%~dp0firmware"

where pio >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    pio run -t upload
) else (
    "%LOCALAPPDATA%\Programs\Python\Python311\Scripts\pio.exe" run -t upload
)

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ==============================================================
    echo [УСПЕХ] Контроллер CecDesk Dongle успешно прошит!
    echo ==============================================================
) else (
    echo.
    echo [ОШИБКА] Не удалось прошить контроллер.
    echo Проверьте подключение USB и режим загрузчика (BOOT + RST).
)

echo.
pause
