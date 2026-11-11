use super::*;

pub(super) fn download_url_to_file(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Не удалось создать каталог загрузки: {}", e))?;
    }
    let _ = fs::remove_file(dest);
    let dest_str = dest.to_string_lossy().to_string();
    let output = create_hidden_command("curl")
        .args([
            "-f",
            "-s",
            "-S",
            "-L",
            "--proto",
            "=https",
            "--max-time",
            "180",
            "-o",
            &dest_str,
            url,
        ])
        .output()
        .map_err(|e| format!("Не удалось запустить curl для загрузки обновления: {}", e))?;

    if output.status.success() && dest.is_file() {
        Ok(())
    } else {
        let _ = fs::remove_file(dest);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("Ошибка загрузки обновления (код {}).", output.status)
        } else {
            format!("Ошибка загрузки обновления: {}", stderr)
        })
    }
}

#[cfg(target_os = "windows")]
pub(super) fn emit_app_update_progress(app: &AppHandle, percent: u8, stage: &str, message: &str) {
    let _ = app.emit(
        "app-update-progress",
        AppUpdateProgress {
            percent,
            stage: stage.to_string(),
            message: message.to_string(),
        },
    );
}

#[cfg(target_os = "windows")]
pub(super) fn cleanup_stale_update_dirs(temp_root: &Path) {
    if let Ok(entries) = fs::read_dir(temp_root) {
        let now = std::time::SystemTime::now();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("cecdesk_update_") || name.starts_with("hdmi_cec_update_") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if now.duration_since(modified).unwrap_or_default()
                            > Duration::from_secs(3600)
                        {
                            let _ = fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) const WINDOWS_APPLY_UPDATE_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$oldPid = [int]$env:CEC_UPD_PID
$payloadDir = $env:CEC_UPD_PAYLOAD
$targetDir = $env:CEC_UPD_TARGET
$exeName = $env:CEC_UPD_EXE
$stageRoot = $env:CEC_UPD_ROOT
$logPath = Join-Path $stageRoot 'apply_update.log'

function Write-UpdLog([string]$msg) {
    $ts = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
    Add-Content -LiteralPath $logPath -Value "[$ts] $msg" -Encoding UTF8 -ErrorAction SilentlyContinue
}

Write-UpdLog "Updater started: pid=$oldPid exe=$exeName target=$targetDir"

for ($i = 0; $i -lt 130; $i++) {
    $proc = Get-Process -Id $oldPid -ErrorAction SilentlyContinue
    if (-not $proc) { break }
    Start-Sleep -Milliseconds 150
}
Start-Sleep -Milliseconds 350

if (Get-Process -Id $oldPid -ErrorAction SilentlyContinue) {
    Write-UpdLog "Timed out waiting for process PID $oldPid to exit."
    exit 1
}

$targetExe = Join-Path $targetDir $exeName
$backupExe = Join-Path $targetDir ($exeName + '.bak')
if (Test-Path -LiteralPath $targetExe) {
    Copy-Item -LiteralPath $targetExe -Destination $backupExe -Force
}

$copyOk = $false
try {
    $files = Get-ChildItem -LiteralPath $payloadDir -Recurse -File
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($payloadDir.Length).TrimStart('\', '/')
        $dest = Join-Path $targetDir $rel
        $destParent = Split-Path -Path $dest -Parent
        if (-not (Test-Path -LiteralPath $destParent)) {
            New-Item -ItemType Directory -Path $destParent -Force | Out-Null
        }
        $copied = $false
        for ($try = 0; $try -lt 15; $try++) {
            try {
                Copy-Item -LiteralPath $f.FullName -Destination $dest -Force
                $copied = $true
                break
            } catch {
                Start-Sleep -Milliseconds 250
            }
        }
        if (-not $copied) {
            throw "Failed copying $rel after retries"
        }
    }
    $copyOk = $true
    Write-UpdLog "Files copied successfully."
} catch {
    Write-UpdLog ("Update error: " + $_.Exception.Message)
    if (Test-Path -LiteralPath $backupExe) {
        Copy-Item -LiteralPath $backupExe -Destination $targetExe -Force -ErrorAction SilentlyContinue
        Write-UpdLog "Restored backup executable."
    }
}

if ($copyOk -and (Test-Path -LiteralPath $backupExe)) {
    Remove-Item -LiteralPath $backupExe -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $targetExe) {
    Start-Process -FilePath $targetExe -WorkingDirectory $targetDir
    Write-UpdLog "Relaunched $targetExe"
}

if ($copyOk) {
    Remove-Item -LiteralPath $payloadDir -Recurse -Force -ErrorAction SilentlyContinue
    $dlDir = Join-Path $stageRoot 'download'
    Remove-Item -LiteralPath $dlDir -Recurse -Force -ErrorAction SilentlyContinue
}
"#;

#[cfg(target_os = "windows")]
fn install_app_update_windows(app: &AppHandle, state: &AppState) -> Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Не удалось определить путь к запущенной программе: {}", e))?;
    let target_dir = exe_path
        .parent()
        .ok_or_else(|| "Не удалось определить рабочий каталог приложения.".to_string())?
        .to_path_buf();
    let exe_name = exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("CecDesk.exe")
        .to_string();

    // Проверяем права на запись в папку с CecDesk.exe ДО скачивания обновления
    let probe_path = target_dir.join(format!(".cecdesk_write_probe_{}.tmp", std::process::id()));
    fs::write(&probe_path, b"ok").map_err(|e| {
        format!(
            "Нет прав на обновление файлов в каталоге {} ({}). Переместите программу в папку с правами записи или запустите от имени администратора.",
            target_dir.display(),
            e
        )
    })?;
    let _ = fs::remove_file(&probe_path);

    emit_app_update_progress(app, 8, "manifest", "Проверка манифеста релиза на GitHub...");
    let manifest_txt = fetch_url_text(DEFAULT_APP_VERSION_URL, 10)?;
    let info = parse_version_manifest(&manifest_txt, CURRENT_APP_VERSION, true)?;
    if !info.has_update {
        return Err("Уже установлена актуальная версия приложения.".to_string());
    }
    let (pkg_url, expected_sha, pkg_kind) = match (
        info.package_url.clone(),
        info.package_sha256.clone(),
        info.package_kind.clone(),
    ) {
        (Some(u), Some(s), Some(k)) => (u, s, k),
        _ => {
            return Err(
                "Для этой версии в version.json не указан прямой пакет Windows с SHA-256. Скачайте обновление со страницы релизов."
                    .to_string(),
            );
        }
    };

    let temp_root = std::env::temp_dir();
    cleanup_stale_update_dirs(&temp_root);

    let stage_root = temp_root.join(format!("cecdesk_update_{}", std::process::id()));
    let dl_dir = stage_root.join("download");
    let payload_dir = stage_root.join("payload");
    let _ = fs::remove_dir_all(&stage_root);
    fs::create_dir_all(&dl_dir)
        .and_then(|_| fs::create_dir_all(&payload_dir))
        .map_err(|e| format!("Не удалось подготовить временную папку обновления: {}", e))?;

    let download_file_name = if pkg_kind == "zip" {
        "update_package.zip"
    } else {
        exe_name.as_str()
    };
    let dl_path = dl_dir.join(download_file_name);

    emit_app_update_progress(
        app,
        22,
        "downloading",
        &format!("Скачивание v{} с GitHub...", info.latest_version),
    );
    if let Err(e) = download_url_to_file(&pkg_url, &dl_path) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(e);
    }

    emit_app_update_progress(
        app,
        62,
        "verifying",
        "Проверка контрольной суммы SHA-256...",
    );
    let actual_sha = match compute_file_sha256_hex(&dl_path) {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_dir_all(&stage_root);
            return Err(e);
        }
    };
    if actual_sha != expected_sha {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!(
            "Контрольная сумма SHA-256 не совпала! Ожидалось {}..., получено {}...",
            &expected_sha[..12],
            &actual_sha[..12]
        ));
    }

    emit_app_update_progress(app, 78, "staging", "Распаковка и проверка файлов обновления...");
    let staged_exe = payload_dir.join(&exe_name);
    if pkg_kind == "zip" {
        if let Err(e) = extract_safe_update_zip(&dl_path, &payload_dir, &exe_name) {
            let _ = fs::remove_dir_all(&stage_root);
            return Err(e);
        }
    } else if let Err(e) = fs::copy(&dl_path, &staged_exe) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Ошибка подготовки файла обновления: {}", e));
    }

    if let Err(e) = verify_windows_pe_binary(&staged_exe) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(e);
    }

    let script_path = stage_root.join("apply_update.ps1");
    if let Err(e) = fs::write(&script_path, WINDOWS_APPLY_UPDATE_SCRIPT) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Не удалось создать скрипт установки обновления: {}", e));
    }

    emit_app_update_progress(
        app,
        94,
        "restarting",
        "Освобождение контроллера и перезапуск в новую версию...",
    );
    log_info(
        "updater",
        &format!(
            "Self-update staged cleanly for v{} (kind={}, sha256={}...); restarting",
            info.latest_version,
            pkg_kind,
            &expected_sha[..12]
        ),
    );

    // Не выключаем ТВ командой OFF при плановом перезапуске ради обновления (но REMOTE_OFF отправится штатно)
    state.skip_standby_on_exit.store(true, AtomicOrdering::SeqCst);

    let spawn_res = create_hidden_command("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.to_string_lossy(),
        ])
        .env("CEC_UPD_PID", std::process::id().to_string())
        .env("CEC_UPD_PAYLOAD", payload_dir.to_string_lossy().as_ref())
        .env("CEC_UPD_TARGET", target_dir.to_string_lossy().as_ref())
        .env("CEC_UPD_EXE", &exe_name)
        .env("CEC_UPD_ROOT", stage_root.to_string_lossy().as_ref())
        .spawn();

    if let Err(e) = spawn_res {
        state.skip_standby_on_exit.store(false, AtomicOrdering::SeqCst);
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Не удалось запустить процесс замены файла: {}", e));
    }

    emit_app_update_progress(app, 100, "done", "Перезапуск...");
    let app_exit = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(160));
        app_exit.exit(0);
    });

    Ok(format!("Перезапуск для установки v{}...", info.latest_version))
}

/// Загрузка, проверка SHA-256 и установка обновления (на Windows в 1 клик поверх текущего каталога)
pub fn install_app_update_internal(app: &AppHandle, state: &AppState) -> Result<String, String> {
    if APP_UPDATE_IN_PROGRESS.swap(true, AtomicOrdering::SeqCst) {
        return Err("Обновление приложения уже выполняется.".to_string());
    }

    struct ResetGuard;
    impl Drop for ResetGuard {
        fn drop(&mut self) {
            APP_UPDATE_IN_PROGRESS.store(false, AtomicOrdering::SeqCst);
        }
    }
    let _guard = ResetGuard;

    #[cfg(target_os = "windows")]
    {
        install_app_update_windows(app, state)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state);
        Err("Автообновление в один клик поддерживается только в Windows. Используйте страницу релизов GitHub или install_steamos.sh.".to_string())
    }
}
