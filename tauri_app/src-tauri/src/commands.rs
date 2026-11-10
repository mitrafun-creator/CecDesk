use crate::audio::{list_audio_endpoints_internal, switch_audio_endpoint};
use crate::config::{
    dismiss_crash_report_state, find_bundled_profiles_dir, find_config_path, find_logs_dir,
    find_reports_dir, find_user_profiles_dir, generate_install_id, get_pending_crash_report_state,
    save_atomic, write_log_to_disk, PendingCrashReport,
};
use crate::custom_scripts::{
    list_custom_scripts_internal, open_scripts_folder_internal,
};
use crate::edid::detect_edid_os;
use crate::hardware::{safe_lock, spawn_serial_reader_thread, AppState};
use crate::input_simulator::trigger_os_key_internal;
use crate::logger::{log_error, log_frontend_error, log_info, log_warn};
use crate::osk::{
    toggle_osk_helper, type_special_key_internal, type_unicode_char_internal,
};
use crate::telemetry::{
    build_diagnostics_snapshot, build_github_issue_url_internal, create_support_bundle_internal,
    sanitize_profile_id, send_bug_report_internal, send_telemetry_heartbeat_internal,
    validate_external_browser_url, validate_https_webhook_url, BugReportResult,
    DiagnosticsSnapshot, SupportBundleResult, TelemetryResult,
};
use crate::types::{AppConfig, CustomScriptInfo, EdidInfoDto, PortInfoDto, TvProfile};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn show_main_window(app: AppHandle, state: State<'_, AppState>) {
    if state.initial_show_pending.swap(false, Ordering::SeqCst) {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
            crate::platform::set_gamescope_overlay_props(&win, false);
        }
    }
}

#[tauri::command]
pub fn get_initial_hud_state(state: State<'_, AppState>) -> serde_json::Value {
    let mouse_active = *safe_lock(&state.mouse_mode_active);
    let mut pending = safe_lock(&state.pending_hud);
    let now = Instant::now();
    let show = pending.last_show.take().and_then(|(t, v)| {
        if now.duration_since(t).as_millis() < 3500 {
            Some(v)
        } else {
            None
        }
    });
    let sector = pending.last_sector.take().and_then(|(t, v)| {
        if now.duration_since(t).as_millis() < 3500 {
            Some(v)
        } else {
            None
        }
    });
    serde_json::json!({
        "mouse_active": mouse_active,
        "last_show": show,
        "last_sector": sector
    })
}

#[cfg(target_os = "windows")]
fn scan_windows_usb_serial_map() -> std::collections::HashMap<String, String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let mut map = std::collections::HashMap::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(usb_key) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Enum\USB") {
        for sub_name in usb_key.enum_keys().flatten() {
            let sub_u = sub_name.to_uppercase();
            if let Ok(dev_k) = usb_key.open_subkey(&sub_name) {
                for inst in dev_k.enum_keys().flatten() {
                    let ppath = format!(r"{}\Device Parameters", inst);
                    if let Ok(pk) = dev_k.open_subkey(&ppath) {
                        if let Ok(pn) = pk.get_value::<String, _>("PortName") {
                            map.insert(pn.to_uppercase(), sub_u.clone());
                        }
                    }
                }
            }
        }
    }
    map
}

#[tauri::command]
pub async fn list_serial_ports() -> Vec<PortInfoDto> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut out = Vec::new();
        if let Ok(ports) = serialport::available_ports() {
            #[cfg(target_os = "windows")]
            let mut usb_reg_map: Option<std::collections::HashMap<String, String>> = None;

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
                        // Единоразово кешируем карту USB-портов из реестра вместо повторного обхода на каждый порт
                        let reg_map = usb_reg_map.get_or_insert_with(scan_windows_usb_serial_map);
                        if let Some(sub_u) = reg_map.get(&port_upper) {
                            is_usb = true;
                            if sub_u.contains("2E8A") {
                                matched_vid = "2E8A".to_string();
                                is_pref = true;
                            } else if sub_u.contains("303A") || sub_u.contains("E8:3D:C1:F2:DE:08") {
                                matched_vid = "303A".to_string();
                                is_pref = true;
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
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn list_audio_endpoints() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(list_audio_endpoints_internal)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn set_audio_device(target: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || switch_audio_endpoint(&target))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn detect_tv_edid() -> EdidInfoDto {
    tauri::async_runtime::spawn_blocking(detect_edid_os)
        .await
        .unwrap_or_else(|_| detect_edid_os())
}

fn read_profiles_from_dir(dir: &std::path::Path, map: &mut HashMap<String, TvProfile>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if path.file_name().and_then(|s| s.to_str()) == Some("catalog.json") {
                    continue;
                }
                match fs::read_to_string(&path) {
                    Ok(txt) => match serde_json::from_str::<TvProfile>(&txt) {
                        Ok(prof) => {
                            map.insert(prof.id.clone(), prof);
                        }
                        Err(e) => {
                            log_warn(
                                "profiles",
                                &format!("Skipped invalid TV profile JSON {:?}: {}", path.file_name(), e),
                            );
                        }
                    },
                    Err(e) => {
                        log_warn(
                            "profiles",
                            &format!("Failed to read TV profile {:?}: {}", path.file_name(), e),
                        );
                    }
                }
            }
        }
    }
}

#[tauri::command]
pub async fn load_tv_profiles() -> Vec<TvProfile> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut merged: HashMap<String, TvProfile> = HashMap::new();
        read_profiles_from_dir(&find_bundled_profiles_dir(), &mut merged);
        read_profiles_from_dir(&find_user_profiles_dir(), &mut merged);
        let mut list: Vec<TvProfile> = merged.into_values().collect();
        list.sort_by(|a, b| a.brand.cmp(&b.brand).then(a.name.cmp(&b.name)));
        list
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn save_tv_profile(mut profile: TvProfile) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let safe_id = sanitize_profile_id(&profile.id)?;
        profile.id = safe_id.clone();
        let dir = find_user_profiles_dir();
        let path = dir.join(format!("{}.json", safe_id));
        let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
        save_atomic(&path, &json)?;
        log_info("profiles", &format!("Saved TV profile '{}'", safe_id));
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_app_config(state: State<'_, AppState>) -> AppConfig {
    safe_lock(&state.config).clone()
}

#[tauri::command]
pub async fn save_app_config(
    mut new_cfg: AppConfig,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !new_cfg.telemetry_endpoint.trim().is_empty() {
            new_cfg.telemetry_endpoint = validate_https_webhook_url(&new_cfg.telemetry_endpoint)?;
        } else {
            new_cfg.telemetry_endpoint.clear();
        }

        let prev_cfg = safe_lock(&st.config).clone();
        if new_cfg.install_id.trim().is_empty() {
            new_cfg.install_id = if !prev_cfg.install_id.trim().is_empty() {
                prev_cfg.install_id.clone()
            } else {
                generate_install_id()
            };
        }

        let path = find_config_path();
        let json = serde_json::to_string_pretty(&new_cfg).map_err(|e| e.to_string())?;
        save_atomic(&path, &json)?;

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
        *safe_lock(&st.config) = new_cfg;

        // Отправляем в порт только те аппаратные параметры, которые действительно изменились
        if remote_changed || wake_changed || stby_changed || polite_changed || pin_changed {
            if let Ok(mut guard) = st.serial_writer.lock() {
                if let Some(ref mut port) = *guard {
                    if polite_changed {
                        let polite_cmd = if non_intrusive {
                            b"POLITE 1\n" as &[u8]
                        } else {
                            b"POLITE 0\n"
                        };
                        if let Err(e) = port.write_all(polite_cmd) {
                            log_warn("serial", &format!("Failed writing POLITE setting: {}", e));
                        }
                    }
                    if remote_changed {
                        let cmd = if is_remote_enabled {
                            b"REMOTE_ON\n" as &[u8]
                        } else {
                            b"REMOTE_OFF\n"
                        };
                        if let Err(e) = port.write_all(cmd) {
                            log_warn("serial", &format!("Failed writing REMOTE toggle: {}", e));
                        }
                    }
                    if wake_changed {
                        let wake_cmd = if wake_pc_on_tv {
                            b"PC_WAKE_ON_TV 1\n" as &[u8]
                        } else {
                            b"PC_WAKE_ON_TV 0\n"
                        };
                        if let Err(e) = port.write_all(wake_cmd) {
                            log_warn("serial", &format!("Failed writing PC_WAKE_ON_TV: {}", e));
                        }
                    }
                    if stby_changed {
                        let stby_cmd = if tv_off_on_pc_off {
                            b"PC_OFF_TV_STANDBY 1\n" as &[u8]
                        } else {
                            b"PC_OFF_TV_STANDBY 0\n"
                        };
                        if let Err(e) = port.write_all(stby_cmd) {
                            log_warn("serial", &format!("Failed writing PC_OFF_TV_STANDBY: {}", e));
                        }
                    }
                    if pin_changed {
                        let pin_cmd = format!("PC_PWR_PIN {}\n", opto_pin);
                        if let Err(e) = port.write_all(pin_cmd.as_bytes()) {
                            log_warn("serial", &format!("Failed writing PC_PWR_PIN: {}", e));
                        }
                    }
                    let _ = port.flush();
                }
            }
        }

        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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
pub async fn append_client_log(line: String, state: State<'_, AppState>) -> Result<(), String> {
    let sess = state.session_log_path.clone();
    let lat = state.latest_log_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        write_log_to_disk(&sess, &lat, &line);
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_logs_folder() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = find_logs_dir();
        crate::platform::open_folder(&dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_custom_scripts() -> Vec<CustomScriptInfo> {
    tauri::async_runtime::spawn_blocking(list_custom_scripts_internal)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn open_scripts_folder() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(open_scripts_folder_internal)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn connect_esp32(
    port_name: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        st.manual_disconnect.store(false, Ordering::SeqCst);
        let conn_id = st.connection_id.fetch_add(1, Ordering::SeqCst) + 1;

        let had_prev = {
            let mut guard = safe_lock(&st.serial_writer);
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
            let mut guard = safe_lock(&st.serial_writer);
            *guard = Some(port);
            let mut p_name = safe_lock(&st.active_port_name);
            *p_name = port_name.clone();
        }

        let cfg_snapshot = safe_lock(&st.config).clone();
        if let Ok(mut guard) = st.serial_writer.lock() {
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
                let _ = p.flush();
            }
        }

        write_log_to_disk(
            &st.session_log_path,
            &st.latest_log_path,
            &format!("=== ПОДКЛЮЧЕНО К ПОРТУ {} ===", port_name),
        );

        spawn_serial_reader_thread(reader_port, app, &st, conn_id);

        Ok(format!("Подключено к {}", port_name))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn disconnect_esp32(state: State<'_, AppState>) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        st.manual_disconnect.store(true, Ordering::SeqCst);
        st.connection_id.fetch_add(1, Ordering::SeqCst);
        let mut guard = safe_lock(&st.serial_writer);
        if let Some(ref mut port) = *guard {
            let _ = port.write_all(b"REMOTE_OFF\n");
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(180));
        }
        *guard = None;
        "Отключено".to_string()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_cec_command(
    cmd: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = safe_lock(&st.serial_writer);
        if let Some(ref mut port) = *guard {
            let trimmed = cmd.trim();
            write_log_to_disk(
                &st.session_log_path,
                &st.latest_log_path,
                &format!("CMD_TX: {}", trimmed),
            );
            let payload = format!("{}\n", trimmed);
            if let Err(e) = port.write_all(payload.as_bytes()) {
                *guard = None;
                let _ = app.emit("cec-disconnected", ());
                return Err(format!("Порт закрыт или отключен: {}", e));
            }
            let _ = port.flush();
            Ok(())
        } else {
            Err("Порт ESP32-S3 не открыт!".to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_remote_passthrough(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        {
            let mut cfg = safe_lock(&st.config);
            cfg.remote_to_os_enabled = enabled;
            if let Ok(json) = serde_json::to_string_pretty(&*cfg) {
                let _ = save_atomic(&find_config_path(), &json);
            }
        }
        if let Ok(mut guard) = st.serial_writer.lock() {
            if let Some(ref mut port) = *guard {
                if enabled {
                    let _ = port.write_all(b"REMOTE_ON\n");
                } else {
                    let _ = port.write_all(b"REMOTE_OFF\n");
                }
                let _ = port.flush();
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
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
pub async fn trigger_os_key(
    action: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if action == "SHOW_KEYBOARD" {
            toggle_osk_helper(&app, &st.osk_visible, None);
            let is_open = st.osk_visible.load(Ordering::SeqCst);
            return if is_open {
                "Клавиатура ОТКРЫТА".to_string()
            } else {
                "Клавиатура СКРЫТА".to_string()
            };
        }
        if action == "MOUSE_TOGGLE" {
            let mut is_m = safe_lock(&st.mouse_mode_active);
            *is_m = !*is_m;
            *safe_lock(&st.last_mouse_activity) = Instant::now();
            *safe_lock(&st.mouse_move_streak) = None;
            let state_now = *is_m;
            drop(is_m);
            let _ = app.emit("mouse-mode-changed", state_now);
            let msg = if state_now {
                "Режим мыши ВКЛ"
            } else {
                "Режим мыши ВЫКЛ"
            };
            let _ = app.emit("cec-toast", msg);
            let hud_payload = serde_json::json!({
                "icon": "mouse",
                "title": if state_now { "Режим мыши активен" } else { "Режим мыши выключен" },
                "desc": if state_now { "Стрелки: Курсор • OK: Клик • 1-9: Секторы" } else { "Обычный режим пульта" },
                "mouse_active": state_now
            });
            if state_now {
                crate::window_manager::emit_and_show_hud(&app, Some(hud_payload), None);
            } else {
                let _ = app.emit("show-hud", &hud_payload);
                crate::window_manager::hide_hud_if_exists(&app);
            }
            return msg.to_string();
        }
        if action == "OPTO_POWER" {
            if let Ok(mut guard) = st.serial_writer.lock() {
                if let Some(ref mut port) = *guard {
                    let _ = port.write_all(b"PC_PWR_PULSE\n");
                }
            }
            return "Импульс кнопки Power (Оптопара 350 мс)".to_string();
        }
        trigger_os_key_internal(&action)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_osk(
    force_show: Option<bool>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        toggle_osk_helper(&app, &st.osk_visible, force_show)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn type_unicode_char(ch: String) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        type_unicode_char_internal(&ch);
    })
    .await;
}

#[tauri::command]
pub async fn type_special_key(key: String) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        type_special_key_internal(&key);
    })
    .await;
}

#[tauri::command]
pub async fn check_app_updates() -> crate::updater::AppVersionInfo {
    tauri::async_runtime::spawn_blocking(crate::updater::check_app_updates)
        .await
        .unwrap_or_else(|_| crate::updater::check_app_updates())
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

#[tauri::command]
pub async fn get_autostart() -> bool {
    tauri::async_runtime::spawn_blocking(crate::platform::get_autostart_os)
        .await
        .unwrap_or(false)
}

#[tauri::command]
pub async fn set_autostart(enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::platform::set_autostart_os(enabled))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn show_hud_window(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some((hud, _)) = crate::window_manager::get_or_create_hud_window(&app) {
            let _ = hud.show();
            let _ = hud.unminimize();
            crate::platform::setup_hud_window(&hud);
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_hud_window(app: AppHandle) -> Result<(), String> {
    crate::window_manager::hide_hud_if_exists(&app);
    Ok(())
}

#[tauri::command]
pub fn app_win_start_dragging(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.start_dragging();
    }
    Ok(())
}

#[tauri::command]
pub fn app_win_minimize(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
    Ok(())
}

#[tauri::command]
pub fn app_win_toggle_maximize(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(is_max) = w.is_maximized() {
            if is_max {
                let _ = w.unmaximize();
            } else {
                let _ = w.maximize();
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn app_win_hide_to_tray(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
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
    let valid_url = validate_external_browser_url(&url).map_err(|e| {
        log_warn("security", &format!("Blocked unsafe browser URL: {}", e));
        e
    })?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&valid_url)
            .spawn()
            .map_err(|e| {
                log_error("platform", &format!("Failed opening URL via rundll32: {}", e));
                e.to_string()
            })?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&valid_url)
            .spawn()
            .map_err(|e| {
                log_error("platform", &format!("Failed opening URL via xdg-open: {}", e));
                e.to_string()
            })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn create_support_bundle(
    user_note: Option<String>,
    state: State<'_, AppState>,
) -> Result<SupportBundleResult, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || create_support_bundle_internal(&st, user_note))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_support_bundle_folder() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = find_reports_dir();
        crate::platform::open_folder(&dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn send_bug_report(
    user_note: Option<String>,
    state: State<'_, AppState>,
) -> Result<BugReportResult, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || send_bug_report_internal(&st, user_note))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn send_telemetry_heartbeat(
    trigger: Option<String>,
    state: State<'_, AppState>,
) -> Result<TelemetryResult, String> {
    let st = state.inner().clone();
    let trig = trigger.unwrap_or_else(|| "startup".to_string());
    tauri::async_runtime::spawn_blocking(move || send_telemetry_heartbeat_internal(&st, &trig))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_diagnostics_snapshot(
    state: State<'_, AppState>,
) -> Result<DiagnosticsSnapshot, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || build_diagnostics_snapshot(&st, None))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pending_crash_report() -> PendingCrashReport {
    get_pending_crash_report_state()
}

#[tauri::command]
pub fn dismiss_crash_report() -> Result<(), String> {
    dismiss_crash_report_state()
}

#[tauri::command]
pub fn report_client_error(source: String, message: String, stack: Option<String>) {
    log_frontend_error(&source, &message, stack.as_deref());
}

#[tauri::command]
pub async fn build_github_issue_url(
    user_note: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let st = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || build_github_issue_url_internal(&st, user_note))
        .await
        .map_err(|e| e.to_string())?
}
