use crate::audio::{list_audio_endpoints_internal, switch_audio_endpoint};
use crate::config::{
    find_bundled_profiles_dir, find_config_path, find_user_profiles_dir, save_atomic,
    write_log_to_disk,
};
use crate::edid::detect_edid_os;
use crate::hardware::{safe_lock, spawn_serial_reader_thread, AppState};
use crate::input_simulator::trigger_os_key_internal;
use crate::logger::{log_info, log_warn};
use crate::osk::{toggle_osk_helper, type_special_key_internal, type_unicode_char_internal};
use crate::telemetry::sanitize_profile_id;
use crate::types::{EdidInfoDto, PortInfoDto, TvProfile};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};


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

/// Классификация и фильтрация последовательного порта (COM / ttyACM) для выбора донгла CecDesk
pub(crate) fn classify_port_info(
    port_name: &str,
    usb_ids: Option<(u16, u16, &str)>,
    usb_reg_hint: Option<&str>,
    filter_non_usb: bool,
) -> Option<PortInfoDto> {
    let port_upper = port_name.to_uppercase();
    // Полностью скрываем материнский системный порт COM1 (ACPI\PNP0501) и фантомные порты /dev/ttyS* в Linux/SteamOS
    if port_upper == "COM1" || port_upper.starts_with("/DEV/TTYS") {
        return None;
    }

    let mut matched_vid = String::new();
    let mut is_usb = port_upper.starts_with("/DEV/TTYACM") || port_upper.starts_with("/DEV/TTYUSB");
    let desc = match usb_ids {
        Some((vid, pid, serial)) => {
            is_usb = true;
            matched_vid = format!("{:04X}", vid).to_uppercase();
            format!("USB {:04X}:{:04X} ({})", vid, pid, serial)
        }
        None => format!("USB Serial ({})", port_name),
    };

    let desc_u = desc.to_uppercase();
    let mut is_pref = desc_u.contains("E8:3D:C1:F2:DE:08")
        || (matched_vid == "303A" && !desc_u.contains("D0:CF:13:07:AC:C0"))
        || desc_u.contains("CECDESK")
        || desc_u.contains("CEC-EMU")
        || matched_vid == "2E8A";

    if matched_vid.is_empty() {
        if let Some(sub_u) = usb_reg_hint {
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

    if filter_non_usb && !is_usb && !is_pref {
        return None;
    }

    let final_desc = if is_pref {
        if matched_vid == "2E8A" || desc_u.contains("2E8A") || desc_u.contains("RP2040") || port_upper == "COM10" {
            format!("CecDesk Dongle RP2040-Zero ({})", port_name)
        } else {
            format!("CecDesk Dongle ESP32-S3 ({})", port_name)
        }
    } else {
        desc
    };

    Some(PortInfoDto {
        name: port_name.to_string(),
        desc: final_desc,
        is_preferred: is_pref,
    })
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
                let (vid, pid, serial) = match &p.port_type {
                    serialport::SerialPortType::UsbPort(usb) => {
                        (usb.vid, usb.pid, usb.serial_number.clone().unwrap_or_default())
                    }
                    _ => (0, 0, String::new()),
                };
                let usb_ids = matches!(p.port_type, serialport::SerialPortType::UsbPort(_))
                    .then_some((vid, pid, serial.as_str()));

                #[cfg(target_os = "windows")]
                let usb_reg_hint = if usb_ids.is_none() {
                    let reg_map = usb_reg_map.get_or_insert_with(scan_windows_usb_serial_map);
                    reg_map.get(&port_upper).map(|s| s.as_str())
                } else {
                    None
                };
                #[cfg(not(target_os = "windows"))]
                let usb_reg_hint: Option<&str> = None;

                let filter_non_usb = cfg!(any(target_os = "windows", target_os = "linux"));
                if let Some(info) = classify_port_info(&p.port_name, usb_ids, usb_reg_hint, filter_non_usb) {
                    out.push(info);
                }
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

        // Даем предыдущему потоку чтения >100 мс на выход из read_until() и освобождение дескриптора COM-порта
        if had_prev {
            std::thread::sleep(Duration::from_millis(130));
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
            return Ok(if is_open {
                "Клавиатура ОТКРЫТА".to_string()
            } else {
                "Клавиатура СКРЫТА".to_string()
            });
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
            return Ok(msg.to_string());
        }
        if action == "OPTO_POWER" {
            let mut guard = st.serial_writer.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut port) = *guard {
                port.write_all(b"PC_PWR_PULSE\n")
                    .map_err(|e| format!("Ошибка отправки PC_PWR_PULSE: {}", e))?;
                let _ = port.flush();
                return Ok("Импульс кнопки Power (Оптопара 350 мс)".to_string());
            }
            return Err("Адаптер HDMI-CEC не подключен".to_string());
        }
        if crate::key_profiles::is_profile_switch_action(&action) {
            return Ok(crate::key_profiles::execute_profile_switch_action(&action, &app, &st.config));
        }
        Ok(trigger_os_key_internal(&action))
    })
    .await
    .map_err(|e| e.to_string())?
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

#[cfg(test)]
mod tests {
    use super::classify_port_info;

    #[test]
    fn test_classify_port_rp2040_and_esp32() {
        let rp = classify_port_info("COM7", Some((0x2E8A, 0x000A, "123456")), None, true)
            .expect("RP2040 USB port should be detected");
        assert!(rp.is_preferred);
        assert!(rp.desc.contains("CecDesk Dongle RP2040-Zero"));

        let esp = classify_port_info("COM8", Some((0x303A, 0x1001, "CECDESK01")), None, true)
            .expect("ESP32-S3 USB port should be detected");
        assert!(esp.is_preferred);
        assert!(esp.desc.contains("CecDesk Dongle ESP32-S3"));
    }

    #[test]
    fn test_classify_port_legacy_serial_and_system_filter() {
        let legacy = classify_port_info("/dev/ttyACM0", Some((0x1234, 0x5678, "CEC-EMU-01")), None, true)
            .expect("Legacy CEC-EMU serial tag should remain compatible");
        assert!(legacy.is_preferred);

        assert!(classify_port_info("COM1", None, None, true).is_none());
        assert!(classify_port_info("/dev/ttyS0", None, None, true).is_none());
        assert!(classify_port_info("COM4", None, None, true).is_none());
    }
}

