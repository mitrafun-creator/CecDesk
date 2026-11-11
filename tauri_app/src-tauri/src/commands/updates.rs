use crate::config::write_log_to_disk;
use crate::hardware::{safe_lock, AppState};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, State};


#[tauri::command]
pub async fn check_app_updates() -> crate::updater::AppVersionInfo {
    tauri::async_runtime::spawn_blocking(crate::updater::check_app_updates)
        .await
        .unwrap_or_else(|_| crate::updater::check_app_updates())
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::updater::install_app_update_internal(&app, &st))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sync_online_tv_profiles() -> crate::updater::ProfileSyncResult {
    tauri::async_runtime::spawn_blocking(crate::updater::sync_tv_profiles_online)
        .await
        .unwrap_or_else(|e| crate::updater::ProfileSyncResult {
            success: false,
            updated_count: 0,
            total_remote: 0,
            message: e.to_string(),
        })
}

#[tauri::command]
pub async fn check_firmware_updates(
    current_fw_ver: String,
    port_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::updater::FirmwareVersionInfo, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let active_port = port_name.unwrap_or_else(|| safe_lock(&st.active_port_name).clone());
        crate::updater::check_firmware_update(&current_fw_ver, &active_port)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn flash_module_firmware(
    port_name: Option<String>,
    custom_file_path: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target_port = port_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| safe_lock(&st.active_port_name).clone());

        // Освобождаем COM-порт и временно приостанавливаем авто-переподключение сторожевого таймера
        st.manual_disconnect.store(true, Ordering::SeqCst);
        st.connection_id.fetch_add(1, Ordering::SeqCst);
        {
            let mut guard = safe_lock(&st.serial_writer);
            *guard = None;
        }
        std::thread::sleep(Duration::from_millis(160));

        write_log_to_disk(
            &st.session_log_path,
            &st.latest_log_path,
            &format!(
                "=== ЗАПУСК ВСТРОЕННОГО ОБНОВЛЕНИЯ ПРОШИВКИ (порт: {}, файл: {:?}) ===",
                target_port, custom_file_path
            ),
        );

        let res = crate::updater::flash_controller_firmware_internal(
            &target_port,
            custom_file_path,
            &app,
        );

        // Возобновляем работу сторожевого таймера после завершения прошивки
        st.manual_disconnect.store(false, Ordering::SeqCst);

        match &res {
            Ok(msg) => {
                write_log_to_disk(
                    &st.session_log_path,
                    &st.latest_log_path,
                    &format!("=== ПРОШИВКА УСПЕШНО ОБНОВЛЕНА: {} ===", msg),
                );
            }
            Err(err) => {
                write_log_to_disk(
                    &st.session_log_path,
                    &st.latest_log_path,
                    &format!("=== ОШИБКА ОБНОВЛЕНИЯ ПРОШИВКИ: {} ===", err),
                );
            }
        }

        res
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn pick_firmware_file() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "windows")]
        {
            let ps_script = "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Filter = 'Прошивки контроллеров (*.uf2;*.bin)|*.uf2;*.bin|RP2040-Zero UF2 (*.uf2)|*.uf2|ESP32-S3 BIN (*.bin)|*.bin'; $f.Title = 'Выберите файл прошивки модуля (.uf2 или .bin)'; if ($f.ShowDialog() -eq 'OK') { Write-Output $f.FileName }";
            if let Ok(out) = crate::platform::create_hidden_command("powershell")
                .args(["-NoProfile", "-Sta", "-Command", ps_script])
                .output()
            {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            // SteamOS KDE Plasma (kdialog) с резервным вызовом zenity
            if let Ok(out) = std::process::Command::new("kdialog")
                .args([
                    "--title",
                    "Выберите файл прошивки модуля (.uf2 или .bin)",
                    "--getopenfilename",
                    ".",
                    "*.uf2 *.bin | Прошивки контроллеров (*.uf2, *.bin)",
                ])
                .output()
            {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            if let Ok(out) = std::process::Command::new("zenity")
                .args([
                    "--file-selection",
                    "--title=Выберите файл прошивки модуля (.uf2 или .bin)",
                    "--file-filter=*.uf2 *.bin",
                ])
                .output()
            {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
        }
        None
    })
    .await
    .unwrap_or(None)
}

