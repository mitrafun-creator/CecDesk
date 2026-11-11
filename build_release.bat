@echo off
setlocal
set PATH=C:\rust\mingw\bin;C:\rust\toolchain\bin;%PATH%
set RUSTUP_HOME=C:\rust\rustup
set CARGO_HOME=C:\rust\cargo
set CARGO_TARGET_DIR=C:\rust_target
set LIBRARY_PATH=C:\rust_target\lib
set RUSTFLAGS=
set RUSTC=C:\rust\toolchain\bin\rustc.exe
set "CEC_REPO_DIR=%~dp0"
set "CEC_REPO_NOSLASH=%CEC_REPO_DIR:~0,-1%"
set "BUILD_DRIVE="
for %%D in (Z Y X W V U) do (
    if not defined BUILD_DRIVE (
        if not exist %%D:\ (
            subst %%D: "%CEC_REPO_NOSLASH%" >nul 2>&1
            if not errorlevel 1 set "BUILD_DRIVE=%%D:"
        )
    )
)

if defined BUILD_DRIVE (
    cd /d "%BUILD_DRIVE%\tauri_app\src-tauri"
) else (
    cd /d "%~dp0tauri_app\src-tauri"
)

powershell -NoProfile -ExecutionPolicy Bypass -Command "$sep = [string][char]0x1f; $repoBack = $env:CEC_REPO_NOSLASH; $repoFwd = $repoBack -replace '\\', '/'; $upBack = $env:USERPROFILE.TrimEnd('\'); $upFwd = $upBack -replace '\\', '/'; $flags = @('-L', 'C:/rust_target/lib', ('--remap-path-prefix=' + $upBack + '=C:\build\user'), ('--remap-path-prefix=' + $upFwd + '=C:/build/user'), ('--remap-path-prefix=' + $repoBack + '=C:\build\src'), ('--remap-path-prefix=' + $repoFwd + '=C:/build/src'), '--remap-path-prefix=C:\rust\cargo=C:\build\cargo', '--remap-path-prefix=C:/rust/cargo=C:/build/cargo'); if ($env:BUILD_DRIVE) { $drv = $env:BUILD_DRIVE; $flags += @(('--remap-path-prefix=' + $drv + '=C:\build\src'), ('--remap-path-prefix=' + $drv + '/=C:/build/src/')) }; $env:CARGO_ENCODED_RUSTFLAGS = ($flags -join $sep); & 'C:\rust\toolchain\bin\cargo.exe' build --release; exit $LASTEXITCODE"
set "BUILD_EC=%ERRORLEVEL%"

cd /d "%CEC_REPO_DIR%"
if defined BUILD_DRIVE subst %BUILD_DRIVE% /D >nul 2>&1

if %BUILD_EC% EQU 0 (
    echo [OK] Build succeeded. Updating HDMI_CEC_Studio.exe...
    taskkill /f /im HDMI_CEC_Studio.exe >nul 2>&1
    ping 127.0.0.1 -n 2 >nul
    copy /y "C:\rust_target\release\hdmi_cec_studio.exe" "%~dp0HDMI_CEC_Studio.exe" >nul
    if %ERRORLEVEL% EQU 0 (
        if exist "C:\rust_target\release\WebView2Loader.dll" (
            copy /y "C:\rust_target\release\WebView2Loader.dll" "%~dp0WebView2Loader.dll" >nul
        )
        echo [OK] Successfully updated HDMI_CEC_Studio.exe!
        echo [OK] Packaging HDMI_CEC_Studio_Windows_x64.zip...
        set "CEC_REPO_DIR=%~dp0"
        powershell -NoProfile -ExecutionPolicy Bypass -Command "Add-Type -AssemblyName System.IO.Compression.FileSystem; $ErrorActionPreference = 'Stop'; $repo = $env:CEC_REPO_DIR.TrimEnd('\'); $zipPath = Join-Path $repo 'HDMI_CEC_Studio_Windows_x64.zip'; $stageRoot = Join-Path $env:TEMP ('hdmi_cec_pkg_' + $PID); $pkgDir = Join-Path $stageRoot 'HDMI_CEC_Studio_Windows_x64'; if (Test-Path $stageRoot) { Remove-Item $stageRoot -Recurse -Force }; New-Item -ItemType Directory -Path $pkgDir -Force | Out-Null; Copy-Item (Join-Path $repo 'HDMI_CEC_Studio.exe') $pkgDir -Force; $wv = Join-Path $repo 'WebView2Loader.dll'; if (-not (Test-Path $wv)) { throw 'Missing WebView2Loader.dll in repo root' }; Copy-Item $wv $pkgDir -Force; Copy-Item (Join-Path $repo 'README.md') $pkgDir -Force; Copy-Item (Join-Path $repo 'LICENSE') $pkgDir -Force; Copy-Item (Join-Path $repo 'THIRD_PARTY_NOTICES.md') $pkgDir -Force; Copy-Item (Join-Path $repo 'bin') (Join-Path $pkgDir 'bin') -Recurse -Force; Copy-Item (Join-Path $repo 'tv_profiles') (Join-Path $pkgDir 'tv_profiles') -Recurse -Force; $scriptsDir = Join-Path $pkgDir 'scripts'; New-Item -ItemType Directory -Path $scriptsDir -Force | Out-Null; if (Test-Path (Join-Path $repo 'scripts\README.md')) { Copy-Item (Join-Path $repo 'scripts\README.md') $scriptsDir -Force }; if (Test-Path $zipPath) { Remove-Item $zipPath -Force }; [System.IO.Compression.ZipFile]::CreateFromDirectory($stageRoot, $zipPath, [System.IO.Compression.CompressionLevel]::Optimal, $false); Remove-Item $stageRoot -Recurse -Force; $exeHash = (Get-FileHash -Algorithm SHA256 (Join-Path $repo 'HDMI_CEC_Studio.exe')).Hash.ToLower(); $zipHash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower(); Write-Host '[EXE SHA256]' $exeHash; Write-Host '[ZIP SHA256]' $zipHash; $vj = Join-Path $repo 'version.json'; if (Test-Path $vj) { $utf8 = New-Object System.Text.UTF8Encoding($false); $txt = [System.IO.File]::ReadAllText($vj, $utf8); if ($txt -match 'windows_package_kind[^\r\n]*zip') { $updated = [regex]::Replace($txt, '(\x22windows_sha256\x22\s*:\s*\x22)[0-9a-fA-F]*(\x22)', ('${1}' + $zipHash + '${2}')); [System.IO.File]::WriteAllText($vj, $updated, $utf8); Write-Host '[OK] Synced version.json windows_sha256 to ZIP hash.' } }"
        if errorlevel 1 (
            echo [ERROR] Failed to package HDMI_CEC_Studio_Windows_x64.zip.
            exit /b 1
        )
    ) else (
        echo [ERROR] Failed to copy hdmi_cec_studio.exe. File may still be running or locked.
        exit /b 1
    )
) else (
    echo [ERROR] Cargo build failed with code %BUILD_EC%.
    exit /b 1
)
