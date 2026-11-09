#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod cec_protocol;
mod commands;
mod config;
mod custom_scripts;
mod edid;
mod hardware;
mod input_simulator;
mod osk;
mod platform;
mod types;
mod updater;

use config::{
    find_logs_dir, load_or_create_config, rotate_session_logs,
};
use hardware::{
    safe_lock, spawn_mouse_revert_watchdog, spawn_usb_watchdog, AppState,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

static DEBUG_LOGGING: AtomicBool = AtomicBool::new(false);

fn is_debug_enabled() -> bool {
    DEBUG_LOGGING.load(Ordering::Relaxed)
}

fn log_step(s: &str) {
    if !is_debug_enabled() {
        return;
    }
    let log_file = find_logs_dir().join("trace.log");
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let _ = writeln!(
            f,
            "[{}] {}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            s
        );
    }
}

fn main() {
    if std::env::args().any(|a| a == "--debug" || a == "-d") || std::env::var("CEC_DEBUG").is_ok() {
        DEBUG_LOGGING.store(true, Ordering::Relaxed);
    }
    log_step("1. main() started");
    if !platform::check_single_instance_or_focus("HDMI-CEC Studio") {
        log_step("EXIT: check_single_instance_or_focus returned false (another instance is already running)");
        return;
    }
    log_step("2. Single instance passed");

    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let msg = format!(
            "PANIC: {}\nLocation: {}\nBacktrace:\n{:?}",
            payload,
            location,
            std::backtrace::Backtrace::capture()
        );
        let panic_str = format!("PANIC OCCURRED: {} at {}", payload, location);
        let log_file = find_logs_dir().join("trace.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_file) {
            let _ = writeln!(
                f,
                "[{}] {}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                panic_str
            );
        }
        let _ = fs::write(find_logs_dir().join("panic.txt"), &msg);
    }));

    let logs_dir = find_logs_dir();
    rotate_session_logs(&logs_dir, 15);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let session_log_path = logs_dir.join(format!("cec_session_{}.log", ts));
    let latest_log_path = logs_dir.join("cec_latest.log");
    let _ = fs::write(
        &latest_log_path,
        format!("\u{FEFF}=== HDMI-CEC STUDIO LOG STARTED (ts: {}) ===\n", ts),
    );
    let _ = fs::write(
        &session_log_path,
        format!("\u{FEFF}=== HDMI-CEC STUDIO LOG STARTED (ts: {}) ===\n", ts),
    );

    let initial_cfg = load_or_create_config();
    let writer_shared = Arc::new(Mutex::new(None));
    let config_shared = Arc::new(Mutex::new(initial_cfg));
    let port_name_shared = Arc::new(Mutex::new(String::new()));
    let conn_id_shared = Arc::new(AtomicU64::new(0));
    let manual_disc_shared = Arc::new(AtomicBool::new(false));
    let mouse_mode_shared = Arc::new(Mutex::new(false));
    let last_mouse_act_shared = Arc::new(Mutex::new(Instant::now()));
    let mouse_streak_shared = Arc::new(Mutex::new(None));
    let back_hold_shared = Arc::new(Mutex::new(None));
    let last_seek_shared = Arc::new(Mutex::new(None));
    let osk_visible_shared = Arc::new(AtomicBool::new(false));
    let calibration_mode_shared = Arc::new(AtomicBool::new(false));

    let state = AppState {
        serial_writer: Arc::clone(&writer_shared),
        config: Arc::clone(&config_shared),
        active_port_name: Arc::clone(&port_name_shared),
        connection_id: Arc::clone(&conn_id_shared),
        manual_disconnect: Arc::clone(&manual_disc_shared),
        session_log_path: session_log_path.clone(),
        latest_log_path: latest_log_path.clone(),
        mouse_mode_active: Arc::clone(&mouse_mode_shared),
        last_mouse_activity: Arc::clone(&last_mouse_act_shared),
        mouse_move_streak: Arc::clone(&mouse_streak_shared),
        back_hold_start: Arc::clone(&back_hold_shared),
        last_seek_press: Arc::clone(&last_seek_shared),
        osk_visible: Arc::clone(&osk_visible_shared),
        calibration_mode: Arc::clone(&calibration_mode_shared),
    };

    let mouse_mode_tray = Arc::clone(&mouse_mode_shared);
    let last_mouse_act_tray = Arc::clone(&last_mouse_act_shared);
    let mouse_mode_bg = Arc::clone(&mouse_mode_shared);
    let last_mouse_bg = Arc::clone(&last_mouse_act_shared);
    let config_bg = Arc::clone(&config_shared);
    let writer_watchdog = Arc::clone(&writer_shared);
    let config_watchdog = Arc::clone(&config_shared);
    let port_watchdog = Arc::clone(&port_name_shared);
    let manual_disc_watchdog = Arc::clone(&manual_disc_shared);

    let builder = tauri::Builder::default()
        .manage(state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            log_step("3. setup() callback entered");
            let app_handle = app.handle().clone();

            let is_minimized_arg = std::env::args().any(|a| a == "--minimized" || a == "-m");
            if let Some(win) = app.get_webview_window("main") {
                if !is_minimized_arg {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                } else {
                    let _ = win.hide();
                }
            }

            if let Some(hud) = app.get_webview_window("hud") {
                let _ = hud.hide();
            }

            // ==================== СИСТЕМНЫЙ ТРЕЙ (ФОНОВЫЙ РЕЖИМ + STEAM BIG PICTURE) ====================
            log_step("3.1 building tray items");
            let show_i = MenuItem::with_id(app, "show", "Открыть Перехват пульта", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Свернуть в трей", true, None::<&str>)?;
            let mouse_i = MenuItem::with_id(
                app,
                "toggle_mouse",
                "Режим мыши (Вкл / Выкл)",
                true,
                None::<&str>,
            )?;
            let steam_bp_i = MenuItem::with_id(
                app,
                "steam_bp",
                "Steam Big Picture (Открыть / Закрыть)",
                true,
                None::<&str>,
            )?;
            let steam_exit_i = MenuItem::with_id(
                app,
                "steam_exit_desktop",
                "Выход из Big Picture на рабочий стол",
                true,
                None::<&str>,
            )?;
            let steam_osk_i = MenuItem::with_id(
                app,
                "steam_osk",
                "Экранная клавиатура (Steam / OSK)",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Выход из программы", true, None::<&str>)?;
            let tray_menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &hide_i,
                    &mouse_i,
                    &steam_bp_i,
                    &steam_exit_i,
                    &steam_osk_i,
                    &quit_i,
                ],
            )?;

            let osk_visible_tray = Arc::clone(&osk_visible_shared);
            log_step("3.2 building tray icon");
            let tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Перехват пульта — Управление ПК и SteamOS пультом ТВ")
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                                crate::platform::set_gamescope_overlay_props(&win, false);
                            }
                        }
                        "hide" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                        "steam_bp" => {
                            let res = crate::platform::toggle_steam_big_picture();
                            let _ = app.emit("cec-toast", res);
                        }
                        "steam_exit_desktop" => {
                            let res = crate::platform::exit_steam_big_picture_to_desktop();
                            let _ = app.emit("cec-toast", res);
                        }
                        "steam_osk" => {
                            crate::osk::toggle_osk_helper(app, &osk_visible_tray, None);
                        }
                        "toggle_mouse" => {
                            let mut is_m = safe_lock(&mouse_mode_tray);
                            *is_m = !*is_m;
                            *safe_lock(&last_mouse_act_tray) = Instant::now();
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
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.unminimize();
                                let _ = win.set_focus();
                            }
                        }
                    }
                });

            let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
                tauri::include_image!("icons/icon.png")
            });
            if let Err(e) = tray_builder.icon(icon).build(app) {
                log_step(&format!("TRAY BUILD ERROR: {}", e));
            }

            // Фоновые сторожевые таймеры
            log_step("3.3 spawning watchdogs");
            spawn_mouse_revert_watchdog(mouse_mode_bg, last_mouse_bg, config_bg, app_handle.clone());
            spawn_usb_watchdog(
                writer_watchdog,
                config_watchdog,
                port_watchdog,
                manual_disc_watchdog,
                app_handle,
            );

            log_step("4. setup() completed successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_serial_ports,
            commands::list_audio_endpoints,
            commands::set_audio_device,
            commands::detect_tv_edid,
            commands::load_tv_profiles,
            commands::save_tv_profile,
            commands::get_app_config,
            commands::save_app_config,
            commands::get_log_paths,
            commands::append_client_log,
            commands::open_logs_folder,
            commands::list_custom_scripts,
            commands::open_scripts_folder,
            commands::connect_esp32,
            commands::disconnect_esp32,
            commands::send_cec_command,
            commands::set_remote_passthrough,
            commands::set_calibration_mode,
            commands::trigger_os_key,
            commands::toggle_osk,
            commands::type_unicode_char,
            commands::type_special_key,
            commands::check_app_updates,
            commands::sync_online_tv_profiles,
            commands::check_firmware_updates,
            commands::flash_module_firmware,
            commands::pick_firmware_file,
            commands::get_autostart,
            commands::set_autostart,
            commands::get_mouse_mode,
            commands::show_hud_window,
            commands::hide_hud_window,
            commands::app_win_start_dragging,
            commands::app_win_minimize,
            commands::app_win_toggle_maximize,
            commands::app_win_hide_to_tray,
            commands::set_window_title,
            commands::get_app_version,
            commands::open_browser_url
        ]);

    log_step("2.3 Before .build()");
    let app_built = builder
        .build(tauri::generate_context!())
        .map_err(|e| {
            log_step(&format!("BUILD FAILED: {}", e));
            e
        })
        .expect("Ошибка сборки Tauri контекста");
    log_step("2.4 After .build()");

    app_built.run(move |app_handle, event| match &event {
        tauri::RunEvent::Ready => log_step("5. RunEvent::Ready"),
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            // Защита от самопроизвольного завершения фонового процесса при входе/выходе из Steam Big Picture,
            // перезапуске проводника или закрытии вспомогательных окон (OSK / HUD).
            // Программа завершается только при явном вызове app.exit(0) из меню трея (когда code == Some(0)).
            if code.is_none() {
                api.prevent_exit();
                return;
            }
            log_step("6. RunEvent::ExitRequested(explicit) - releasing CEC remote capture");
            if let Some(st) = app_handle.try_state::<AppState>() {
                let stby_on_exit = safe_lock(&st.config).auto_standby_tv_on_exit;
                let mut guard = safe_lock(&st.serial_writer);
                if let Some(ref mut port) = *guard {
                    let _ = port.write_all(b"REMOTE_OFF\n");
                    if stby_on_exit {
                        let _ = port.write_all(b"OFF\n");
                    }
                    let _ = port.flush();
                }
                *guard = None;
            }
        }
        tauri::RunEvent::Exit => {
            log_step("6. RunEvent::Exit - releasing CEC remote capture");
            if let Some(st) = app_handle.try_state::<AppState>() {
                let stby_on_exit = safe_lock(&st.config).auto_standby_tv_on_exit;
                let mut guard = safe_lock(&st.serial_writer);
                if let Some(ref mut port) = *guard {
                    let _ = port.write_all(b"REMOTE_OFF\n");
                    if stby_on_exit {
                        let _ = port.write_all(b"OFF\n");
                    }
                    let _ = port.flush();
                }
                *guard = None;
            }
        }
        _ => {}
    });
    log_step("8. main() completed");
}
