use crate::audio::{list_audio_endpoints_internal, switch_audio_endpoint};
use crate::config::{
    find_config_path, find_logs_dir, find_profiles_dir, save_atomic, write_log_to_disk,
};
use crate::custom_scripts::{
    list_custom_scripts_internal, open_scripts_folder_internal,
};
use crate::edid::detect_edid_os;
use crate::hardware::{safe_lock, spawn_serial_reader_thread, AppState};
use crate::input_simulator::{create_hidden_command, trigger_os_key_internal};
use crate::osk::{
    toggle_osk_helper, type_special_key_internal, type_unicode_char_internal,
};
use crate::types::{AppConfig, CustomScriptInfo, EdidInfoDto, PortInfoDto, TvProfile};
use std::fs;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn list_serial_ports() -> Vec<PortInfoDto> {
    let mut out = Vec::new();
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            let port_upper = p.port_name.to_uppercase();
            // Полностью скрываем материнский системный порт COM1 (ACPI\PNP0501) и фантомные порты /dev/ttyS* в Linux/SteamOS
            if port_upper == "COM1" || port_upper.starts_with("/DEV/TTYS") {
                continue;
            }

            let mut matched_vid = String::new();
            let mut is_usb = port_upper.starts_with("/DEV/TTYACM") || port_upper.starts_with("/DEV/TTYUSB");
            let desc = match &p.port_type {
                serialport::SerialPortType::UsbPort(usb) => {
                    is_usb = true;
                    matched_vid = format!("{:04X}", usb.vid).to_uppercase();
                    format!(
                        "USB {:04X}:{:04X} ({})",
                        usb.vid,
                        usb.pid,
                        usb.serial_number.clone().unwrap_or_default()
                    )
                }
                _ => format!("USB Serial ({})", p.port_name),
            };

            let mut is_pref = desc.to_uppercase().contains("E8:3D:C1:F2:DE:08")
                || (matched_vid == "303A" && !desc.to_uppercase().contains("D0:CF:13:07:AC:C0"))
                || desc.to_uppercase().contains("CEC-EMU")
                || matched_vid == "2E8A";

            #[cfg(target_os = "windows")]
            {
                if matched_vid.is_empty() {
                    // Проверяем реестр Windows на принадлежность к USB-устройствам: ESP32 (303A) или RP2040 (2E8A)
                    use winreg::enums::HKEY_LOCAL_MACHINE;
                    use winreg::RegKey;
                    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
                    if let Ok(usb_key) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Enum\USB") {
                        for sub_name in usb_key.enum_keys().flatten() {
                            let sub_u = sub_name.to_uppercase();
                            if let Ok(dev_k) = usb_key.open_subkey(&sub_name) {
                                for inst in dev_k.enum_keys().flatten() {
                                    let ppath = format!(r"{}\Device Parameters", inst);
                                    if let Ok(pk) = dev_k.open_subkey(&ppath) {
                                        if let Ok(pn) = pk.get_value::<String, _>("PortName") {
                                            if pn.to_uppercase() == port_upper {
                                                is_usb = true;
                                                if sub_u.contains("2E8A") {
                                                    matched_vid = "2E8A".to_string();
                                                    is_pref = true;
                                                } else if sub_u.contains("303A")
                                                    || sub_u.contains("E8:3D:C1:F2:DE:08")
                                                {
                                                    matched_vid = "303A".to_string();
                                                    is_pref = true;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if !matched_vid.is_empty() {
                                break;
                            }
                        }
                    }
                    if !is_pref && port_upper == "COM10" {
                        matched_vid = "2E8A".to_string();
                        is_pref = true;
                        is_usb = true;
                    } else if !is_pref && port_upper == "COM6" {
                        matched_vid = "303A".to_string();
                        is_pref = true;
                        is_usb = true;
                    }
                }
                // Отфильтровываем фантомные не-USB порты (ACPI, Bluetooth-модемы и пр.)
                if !is_usb && !is_pref {
                    continue;
                }
            }

            #[cfg(target_os = "linux")]
            {
                if !is_usb && !is_pref {
                    continue;
                }
            }

            let final_desc = if is_pref {
                if matched_vid == "2E8A" || desc.contains("2E8A") || desc.contains("RP2040") || port_upper == "COM10" {
                    format!("CEC-EMU RP2040-Zero ({})", p.port_name)
                } else {
                    format!("CEC-EMU ESP32-S3 ({})", p.port_name)
                }
            } else {
                desc
            };

            out.push(PortInfoDto {
                name: p.port_name,
                desc: final_desc,
                is_preferred: is_pref,
            });
        }
    }
    // Сортируем: сначала предпочтительные порты, затем по имени порта
    out.sort_by(|a, b| b.is_preferred.cmp(&a.is_preferred).then(a.name.cmp(&b.name)));
    out
}

#[tauri::command]
pub fn list_audio_endpoints() -> Vec<String> {
    list_audio_endpoints_internal()
}

#[tauri::command]
pub fn set_audio_device(target: String) -> Result<String, String> {
    switch_audio_endpoint(&target)
}

#[tauri::command]
pub fn detect_tv_edid() -> EdidInfoDto {
    detect_edid_os()
}

#[tauri::command]
pub fn load_tv_profiles() -> Vec<TvProfile> {
    let dir = find_profiles_dir();
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(txt) = fs::read_to_string(entry.path()) {
                    if let Ok(prof) = serde_json::from_str::<TvProfile>(&txt) {
                        list.push(prof);
                    }
                }
            }
        }
    }
    list.sort_by(|a, b| a.brand.cmp(&b.brand).then(a.name.cmp(&b.name)));
    list
}

#[tauri::command]
pub fn save_tv_profile(profile: TvProfile) -> Result<String, String> {
    let dir = find_profiles_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.json", profile.id));
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    save_atomic(&path, &json)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_app_config(state: State<'_, AppState>) -> AppConfig {
    safe_lock(&state.config).clone()
}

#[tauri::command]
pub fn save_app_config(new_cfg: AppConfig, state: State<'_, AppState>) -> Result<String, String> {
    let path = find_config_path();
    let json = serde_json::to_string_pretty(&new_cfg).map_err(|e| e.to_string())?;
    save_atomic(&path, &json)?;

    let prev_cfg = safe_lock(&state.config).clone();
    let remote_changed = prev_cfg.remote_to_os_enabled != new_cfg.remote_to_os_enabled;
    let wake_changed = prev_cfg.wake_pc_on_tv_on != new_cfg.wake_pc_on_tv_on;
    let stby_changed = prev_cfg.auto_standby_tv_on_exit != new_cfg.auto_standby_tv_on_exit;
    let polite_changed = prev_cfg.non_intrusive_cec != new_cfg.non_intrusive_cec;
    let pin_changed = prev_cfg.opto_pwr_pin != new_cfg.opto_pwr_pin && new_cfg.opto_pwr_pin > 0;

    let is_remote_enabled = new_cfg.remote_to_os_enabled;
    let wake_pc_on_tv = new_cfg.wake_pc_on_tv_on;
    let tv_off_on_pc_off = new_cfg.auto_standby_tv_on_exit;
    let non_intrusive = new_cfg.non_intrusive_cec;
    let opto_pin = new_cfg.opto_pwr_pin;
    *safe_lock(&state.config) = new_cfg;

    // Отправляем в порт только те аппаратные параметры, которые действительно изменились
    if remote_changed || wake_changed || stby_changed || polite_changed || pin_changed {
        if let Ok(mut guard) = state.serial_writer.lock() {
            if let Some(ref mut port) = *guard {
                if polite_changed {
                    let polite_cmd = if non_intrusive {
                        b"POLITE 1\n" as &[u8]
                    } else {
                        b"POLITE 0\n"
                    };
                    let _ = port.write_all(polite_cmd);
                }
                if remote_changed {
                    if is_remote_enabled {
                        let _ = port.write_all(b"REMOTE_ON\n");
                    } else {
                        let _ = port.write_all(b"REMOTE_OFF\n");
                    }
                }
                if wake_changed {
                    let wake_cmd = if wake_pc_on_tv {
                        b"PC_WAKE_ON_TV 1\n" as &[u8]
                    } else {
                        b"PC_WAKE_ON_TV 0\n"
                    };
                    let _ = port.write_all(wake_cmd);
                }
                if stby_changed {
                    let stby_cmd = if tv_off_on_pc_off {
                        b"PC_OFF_TV_STANDBY 1\n" as &[u8]
                    } else {
                        b"PC_OFF_TV_STANDBY 0\n"
                    };
                    let _ = port.write_all(stby_cmd);
                }
                if pin_changed {
                    let pin_cmd = format!("PC_PWR_PIN {}\n", opto_pin);
                    let _ = port.write_all(pin_cmd.as_bytes());
                }
            }
        }
    }

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_log_paths(state: State<'_, AppState>) -> serde_json::Value {
    serde_json::json!({
        "session_log": state.session_log_path.to_string_lossy().to_string(),
        "latest_log": state.latest_log_path.to_string_lossy().to_string(),
        "logs_dir": find_logs_dir().to_string_lossy().to_string()
    })
}

#[tauri::command]
pub fn append_client_log(line: String, state: State<'_, AppState>) {
    write_log_to_disk(&state.session_log_path, &state.latest_log_path, &line);
}

#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    let dir = find_logs_dir();
    #[cfg(target_os = "windows")]
    {
        create_hidden_command("explorer")
            .arg(dir.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(dir.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_custom_scripts() -> Vec<CustomScriptInfo> {
    list_custom_scripts_internal()
}

#[tauri::command]
pub fn open_scripts_folder() -> Result<(), String> {
    open_scripts_folder_internal()
}

#[tauri::command]
pub fn connect_esp32(
    port_name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.manual_disconnect.store(false, Ordering::SeqCst);
    let conn_id = state.connection_id.fetch_add(1, Ordering::SeqCst) + 1;

    let had_prev = {
        let mut guard = safe_lock(&state.serial_writer);
        let was_some = guard.is_some();
        *guard = None;
        was_some
    };

    // Даем предыдущему потоку чтения 80 мс на выход из read_line() и освобождение дескриптора COM-порта
    if had_prev {
        std::thread::sleep(Duration::from_millis(80));
    }

    let mut port = serialport::new(&port_name, 115200)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| format!("Не удалось открыть {}: {}", port_name, e))?;

    let _ = port.write_data_terminal_ready(true);
    let _ = port.write_request_to_send(false);

    let reader_port = port.try_clone().map_err(|e| e.to_string())?;
    {
        let mut guard = safe_lock(&state.serial_writer);
        *guard = Some(port);
        let mut p_name = safe_lock(&state.active_port_name);
        *p_name = port_name.clone();
    }

    let cfg_snapshot = safe_lock(&state.config).clone();
    if let Ok(mut guard) = state.serial_writer.lock() {
        if let Some(ref mut p) = *guard {
            if cfg_snapshot.non_intrusive_cec {
                let _ = p.write_all(b"POLITE 1\n");
            } else {
                let _ = p.write_all(b"POLITE 0\n");
            }
            if !cfg_snapshot.osd_name.trim().is_empty() {
                let name_cmd = format!("NAME {}\n", cfg_snapshot.osd_name.trim());
                let _ = p.write_all(name_cmd.as_bytes());
            }
            if !cfg_snapshot.pa_override.trim().is_empty() {
                let pa_cmd = format!("PA {}\n", cfg_snapshot.pa_override.trim());
                let _ = p.write_all(pa_cmd.as_bytes());
            }
            if cfg_snapshot.remote_to_os_enabled {
                let _ = p.write_all(b"REMOTE_ON\n");
            } else {
                let _ = p.write_all(b"REMOTE_OFF\n");
            }
            if cfg_snapshot.wake_pc_on_tv_on {
                let _ = p.write_all(b"PC_WAKE_ON_TV 1\n");
            } else {
                let _ = p.write_all(b"PC_WAKE_ON_TV 0\n");
            }
            if cfg_snapshot.auto_standby_tv_on_exit {
                let _ = p.write_all(b"PC_OFF_TV_STANDBY 1\n");
            } else {
                let _ = p.write_all(b"PC_OFF_TV_STANDBY 0\n");
            }
            if cfg_snapshot.opto_pwr_pin > 0 {
                let pin_cmd = format!("PC_PWR_PIN {}\n", cfg_snapshot.opto_pwr_pin);
                let _ = p.write_all(pin_cmd.as_bytes());
            }
            let _ = p.write_all(b"INFO\n");
            let _ = p.write_all(b"STATE\n");
        }
    }

    write_log_to_disk(
        &state.session_log_path,
        &state.latest_log_path,
        &format!("=== ПОДКЛЮЧЕНО К ПОРТУ {} ===", port_name),
    );

    spawn_serial_reader_thread(reader_port, app, &state, conn_id);

    Ok(format!("Подключено к {}", port_name))
}

#[tauri::command]
pub fn disconnect_esp32(state: State<'_, AppState>) -> String {
    state.manual_disconnect.store(true, Ordering::SeqCst);
    state.connection_id.fetch_add(1, Ordering::SeqCst);
    let mut guard = safe_lock(&state.serial_writer);
    if let Some(ref mut port) = *guard {
        let _ = port.write_all(b"REMOTE_OFF\n");
        let _ = port.flush();
    }
    *guard = None;
    "Отключено".to_string()
}

#[tauri::command]
pub fn send_cec_command(
    cmd: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = safe_lock(&state.serial_writer);
    if let Some(ref mut port) = *guard {
        let trimmed = cmd.trim();
        write_log_to_disk(
            &state.session_log_path,
            &state.latest_log_path,
            &format!("CMD_TX: {}", trimmed),
        );
        let payload = format!("{}\n", trimmed);
        if let Err(e) = port.write_all(payload.as_bytes()) {
            *guard = None;
            let _ = app.emit("cec-disconnected", ());
            return Err(format!("Порт закрыт или отключен: {}", e));
        }
        Ok(())
    } else {
        Err("Порт ESP32-S3 не открыт!".to_string())
    }
}

#[tauri::command]
pub fn set_remote_passthrough(enabled: bool, state: State<'_, AppState>) {
    {
        let mut cfg = safe_lock(&state.config);
        cfg.remote_to_os_enabled = enabled;
        if let Ok(json) = serde_json::to_string_pretty(&*cfg) {
            let _ = save_atomic(&find_config_path(), &json);
        }
    }
    if let Ok(mut guard) = state.serial_writer.lock() {
        if let Some(ref mut port) = *guard {
            if enabled {
                let _ = port.write_all(b"REMOTE_ON\n");
            } else {
                let _ = port.write_all(b"REMOTE_OFF\n");
            }
        }
    }
}

#[tauri::command]
pub fn set_calibration_mode(enabled: bool, state: State<'_, AppState>) {
    state.calibration_mode.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub fn get_mouse_mode(state: State<'_, AppState>) -> bool {
    *safe_lock(&state.mouse_mode_active)
}

#[tauri::command]
pub fn trigger_os_key(action: String, app: AppHandle, state: State<'_, AppState>) -> String {
    if action == "SHOW_KEYBOARD" {
        toggle_osk_helper(&app, &state.osk_visible, None);
        let is_open = state.osk_visible.load(Ordering::SeqCst);
        return if is_open {
            "Клавиатура ОТКРЫТА".to_string()
        } else {
            "Клавиатура СКРЫТА".to_string()
        };
    }
    if action == "MOUSE_TOGGLE" {
        let mut is_m = safe_lock(&state.mouse_mode_active);
        *is_m = !*is_m;
        *safe_lock(&state.last_mouse_activity) = Instant::now();
        *safe_lock(&state.mouse_move_streak) = None;
        let state_now = *is_m;
        let _ = app.emit("mouse-mode-changed", state_now);
        let msg = if state_now {
            "Режим мыши ВКЛ"
        } else {
            "Режим мыши ВЫКЛ"
        };
        let _ = app.emit("cec-toast", msg);
        let _ = app.emit(
            "show-hud",
            serde_json::json!({
                "icon": "mouse",
                "title": if state_now { "Режим мыши активен" } else { "Режим мыши выключен" },
                "desc": if state_now { "Стрелки: Курсор • OK: Клик • 1-9: Секторы" } else { "Обычный режим пульта" },
                "mouse_active": state_now
            }),
        );
        if let Some(hud) = app.get_webview_window("hud") {
            if state_now {
                let _ = hud.show();
                let _ = hud.unminimize();
                crate::platform::setup_hud_window(&hud);
            } else {
                crate::platform::hide_hud_window_os(&hud);
            }
        }
        return msg.to_string();
    }
    if action == "OPTO_POWER" {
        if let Ok(mut guard) = state.serial_writer.lock() {
            if let Some(ref mut port) = *guard {
                let _ = port.write_all(b"PC_PWR_PULSE\n");
            }
        }
        return "Импульс кнопки Power (Оптопара 350 мс)".to_string();
    }
    trigger_os_key_internal(&action)
}

#[tauri::command]
pub fn toggle_osk(force_show: Option<bool>, app: AppHandle, state: State<'_, AppState>) -> bool {
    toggle_osk_helper(&app, &state.osk_visible, force_show)
}

#[tauri::command]
pub fn type_unicode_char(ch: String) {
    type_unicode_char_internal(&ch);
}

#[tauri::command]
pub fn type_special_key(key: String) {
    type_special_key_internal(&key);
}

#[tauri::command]
pub fn check_app_updates() -> crate::updater::AppVersionInfo {
    crate::updater::check_app_updates()
}

#[tauri::command]
pub fn sync_online_tv_profiles() -> crate::updater::ProfileSyncResult {
    crate::updater::sync_tv_profiles_online()
}

#[tauri::command]
pub fn check_firmware_updates(
    current_fw_ver: String,
    port_name: Option<String>,
    state: State<'_, AppState>,
) -> crate::updater::FirmwareVersionInfo {
    let active_port = port_name.unwrap_or_else(|| safe_lock(&state.active_port_name).clone());
    crate::updater::check_firmware_update(&current_fw_ver, &active_port)
}

#[tauri::command]
pub fn flash_module_firmware(
    port_name: Option<String>,
    custom_file_path: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let target_port = port_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| safe_lock(&state.active_port_name).clone());

    // Освобождаем COM-порт и временно приостанавливаем авто-переподключение сторожевого таймера
    state.manual_disconnect.store(true, Ordering::SeqCst);
    state.connection_id.fetch_add(1, Ordering::SeqCst);
    {
        let mut guard = safe_lock(&state.serial_writer);
        *guard = None;
    }
    std::thread::sleep(Duration::from_millis(160));

    write_log_to_disk(
        &state.session_log_path,
        &state.latest_log_path,
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
    state.manual_disconnect.store(false, Ordering::SeqCst);

    match &res {
        Ok(msg) => {
            write_log_to_disk(
                &state.session_log_path,
                &state.latest_log_path,
                &format!("=== ПРОШИВКА УСПЕШНО ОБНОВЛЕНА: {} ===", msg),
            );
        }
        Err(err) => {
            write_log_to_disk(
                &state.session_log_path,
                &state.latest_log_path,
                &format!("=== ОШИБКА ОБНОВЛЕНИЯ ПРОШИВКИ: {} ===", err),
            );
        }
    }

    res
}

#[tauri::command]
pub fn pick_firmware_file() -> Option<String> {
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
}

#[tauri::command]
pub fn get_autostart() -> bool {
    crate::platform::get_autostart_os()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    crate::platform::set_autostart_os(enabled)
}

#[tauri::command]
pub fn show_hud_window(app: AppHandle) -> Result<(), String> {
    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.show();
        let _ = hud.unminimize();
        crate::platform::setup_hud_window(&hud);
    }
    Ok(())
}

#[tauri::command]
pub fn hide_hud_window(app: AppHandle) -> Result<(), String> {
    if let Some(hud) = app.get_webview_window("hud") {
        crate::platform::hide_hud_window_os(&hud);
    }
    Ok(())
}

#[tauri::command]
pub fn set_window_title(app: AppHandle, title: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_title(&title);
    }
    Ok(())
}

#[tauri::command]
pub fn get_app_version() -> String {
    crate::updater::CURRENT_APP_VERSION.to_string()
}

#[tauri::command]
pub fn open_browser_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
