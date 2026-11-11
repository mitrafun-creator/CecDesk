@echo off
setlocal
set "MODE=%~1"
if "%MODE%"=="" set "MODE=check"
set "VER=%~2"

if "%VER%"=="" (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts_dev\release_pipeline.ps1" -Mode %MODE%
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts_dev\release_pipeline.ps1" -Mode %MODE% -Version %VER%
)
exit /b %ERRORLEVEL%
