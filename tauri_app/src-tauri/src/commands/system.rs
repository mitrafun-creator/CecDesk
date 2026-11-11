use crate::config::{
    dismiss_crash_report_state, find_config_path, find_logs_dir, find_reports_dir,
    generate_install_id, get_pending_crash_report_state, save_atomic, write_log_to_disk,
    PendingCrashReport,
};
use crate::custom_scripts::{list_custom_scripts_internal, open_scripts_folder_internal};
use crate::hardware::{safe_lock, AppState};
use crate::logger::{log_error, log_frontend_error, log_warn};
use crate::telemetry::{
    build_diagnostics_snapshot, build_github_issue_url_internal, create_support_bundle_internal,
    send_bug_report_internal, send_telemetry_heartbeat_internal, validate_external_browser_url,
    validate_https_webhook_url, BugReportResult, DiagnosticsSnapshot, SupportBundleResult,
    TelemetryResult,
};
use crate::types::{AppConfig, CustomScriptInfo};
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tauri::{AppHandle, Manager, State};


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
