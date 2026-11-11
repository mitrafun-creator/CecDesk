#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod cec_protocol;
mod commands;
mod config;
mod custom_scripts;
mod edid;
mod hardware;
mod input_simulator;
mod key_profiles;
mod logger;
mod osk;
mod platform;
mod telemetry;
mod types;
mod updater;
mod window_manager;

use config::{
    clear_session_active_marker, find_logs_dir, init_crash_recovery_on_startup,
    load_or_create_config, migrate_legacy_data_if_needed, rotate_session_logs,
};
use hardware::{
    safe_lock, spawn_mouse_revert_watchdog, spawn_usb_watchdog, AppState, PendingHudState,
};
use logger::{
    flush_logger, init_logger, log_debug, log_error, log_info, now_unix_secs,
    write_crash_report_sync,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

fn main() {
    let debug_enabled =
        std::env::args().any(|a| a == "--debug" || a == "-d") || std::env::var("CEC_DEBUG").is_ok();

    if !platform::check_single_instance_or_focus("CecDesk") {
        return;
    }

    migrate_legacy_data_if_needed();
    let logs_dir = find_logs_dir();
    let panic_logs_dir = logs_dir.clone();

    std::panic::set_hook(Box::new(move |info| {
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
        write_crash_report_sync(&panic_logs_dir, &panic_str, &msg);
    }));

    let ts = now_unix_secs();
    let session_log_path = logs_dir.join(format!("cec_session_{}.log", ts));
    let latest_log_path = logs_dir.join("cec_latest.log");
    init_logger(
        logs_dir.clone(),
        session_log_path.clone(),
        latest_log_path.clone(),
        debug_enabled,
    );
    log_debug("app", "Verbose debug logging active");
    log_info(
        "app",
        &format!(
            "Starting CecDesk v{} (debug={}, storage={})",
            updater::CURRENT_APP_VERSION,
            debug_enabled,
            config::data_storage_mode_label()
        ),
    );
    init_crash_recovery_on_startup();

    let logs_dir_rotate = logs_dir.clone();
    std::thread::spawn(move || {
        rotate_session_logs(&logs_dir_rotate, 15);
    });

    let is_minimized_arg = std::env::args().any(|a| a == "--minimized" || a == "-m");

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
    let initial_show_pending = Arc::new(AtomicBool::new(!is_minimized_arg));
    let skip_standby_on_exit = Arc::new(AtomicBool::new(false));
    let pending_hud_shared = Arc::new(Mutex::new(PendingHudState::default()));

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
        initial_show_pending: Arc::clone(&initial_show_pending),
        skip_standby_on_exit: Arc::clone(&skip_standby_on_exit),
        pending_hud: Arc::clone(&pending_hud_shared),
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
    let initial_show_setup = Arc::clone(&initial_show_pending);

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
            log_info("app", "Tauri setup() entered");
            let app_handle = app.handle().clone();

            if is_minimized_arg {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            } else {
                // Страховочный показ окна, если WebView задержит вызов show_main_window
                let app_fallback = app.handle().clone();
                let show_flag = Arc::clone(&initial_show_setup);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(1400));
                    if show_flag.swap(false, Ordering::SeqCst) {
                        if let Some(win) = app_fallback.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.unminimize();
                            let _ = win.set_focus();
                            crate::platform::set_gamescope_overlay_props(&win, false);
                        }
                    }
                });
            }

            // ==================== СИСТЕМНЫЙ ТРЕЙ (ФОНОВЫЙ РЕЖИМ + STEAM BIG PICTURE) ====================
            let show_i = MenuItem::with_id(app, "show", "Открыть CecDesk", true, None::<&str>)?;
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
            let initial_show_tray = Arc::clone(&initial_show_setup);
            let initial_show_click = Arc::clone(&initial_show_setup);
            let tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("CecDesk — Управление ПК и SteamOS пультом ТВ")
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            initial_show_tray.store(false, Ordering::SeqCst);
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
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let res = tauri::async_runtime::spawn_blocking(crate::platform::toggle_steam_big_picture)
                                    .await
                                    .unwrap_or_default();
                                let _ = app_clone.emit("cec-toast", res);
                            });
                        }
                        "steam_exit_desktop" => {
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let res = tauri::async_runtime::spawn_blocking(crate::platform::exit_steam_big_picture_to_desktop)
                                    .await
                                    .unwrap_or_default();
                                let _ = app_clone.emit("cec-toast", res);
                            });
                        }
                        "steam_osk" => {
                            let app_clone = app.clone();
                            let osk_vis = Arc::clone(&osk_visible_tray);
                            tauri::async_runtime::spawn(async move {
                                let _ = tauri::async_runtime::spawn_blocking(move || {
                                    crate::osk::toggle_osk_helper(&app_clone, &osk_vis, None);
                                })
                                .await;
                            });
                        }
                        "toggle_mouse" => {
                            let state_now = {
                                let mut is_m = safe_lock(&mouse_mode_tray);
                                *is_m = !*is_m;
                                *safe_lock(&last_mouse_act_tray) = Instant::now();
                                *is_m
                            };
                            let _ = app.emit("mouse-mode-changed", state_now);
                            let msg = if state_now {
                                "Режим мыши ВКЛ"
                            } else {
                                "Режим мыши ВЫКЛ"
                            };
                            let _ = app.emit("cec-toast", msg);
                            let payload = serde_json::json!({
                                "icon": "mouse",
                                "title": if state_now { "Режим мыши активен" } else { "Режим мыши выключен" },
                                "desc": if state_now { "Стрелки: Курсор • OK: Клик • 1-9: Секторы" } else { "Обычный режим пульта" },
                                "mouse_active": state_now
                            });
                            if state_now {
                                crate::window_manager::emit_and_show_hud(app, Some(payload), None);
                            } else {
                                let _ = app.emit("show-hud", &payload);
                                crate::window_manager::hide_hud_if_exists(app);
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |tray, event| {
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
                                initial_show_click.store(false, Ordering::SeqCst);
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
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_icon(icon.clone());
            }
            if let Err(e) = tray_builder.icon(icon).build(app) {
                log_error("tray", &format!("Tray icon build failed: {}", e));
            }

            // Фоновые сторожевые таймеры
            spawn_mouse_revert_watchdog(mouse_mode_bg, last_mouse_bg, config_bg, app_handle.clone());
            spawn_usb_watchdog(
                writer_watchdog,
                config_watchdog,
                port_watchdog,
                manual_disc_watchdog,
                app_handle,
            );

            log_info("app", "Setup completed successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::show_main_window,
            commands::get_initial_hud_state,
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
            commands::install_app_update,
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
            commands::open_browser_url,
            commands::create_support_bundle,
            commands::open_support_bundle_folder,
            commands::send_bug_report,
            commands::send_telemetry_heartbeat,
            commands::get_diagnostics_snapshot,
            commands::get_pending_crash_report,
            commands::dismiss_crash_report,
            commands::report_client_error,
            commands::build_github_issue_url
        ]);

    let app_built = builder
        .build(tauri::generate_context!())
        .map_err(|e| {
            log_error("app", &format!("Tauri builder failed: {}", e));
            flush_logger();
            e
        })
        .expect("Ошибка сборки Tauri контекста");

    let exit_released = Arc::new(AtomicBool::new(false));
    let exit_released_loop = Arc::clone(&exit_released);

    app_built.run(move |app_handle, event| match &event {
        tauri::RunEvent::Ready => log_info("app", "Application Ready"),
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            // Защита от самопроизвольного завершения фонового процесса при входе/выходе из Steam Big Picture,
            // перезапуске проводника или закрытии вспомогательных окон (OSK / HUD).
            // Программа завершается только при явном вызове app.exit(0) из меню трея (когда code == Some(0)).
            if code.is_none() {
                api.prevent_exit();
                return;
            }
            log_info("app", "Explicit exit requested - releasing CEC remote capture");
            release_device_on_exit(app_handle, &exit_released_loop);
        }
        tauri::RunEvent::Exit => {
            log_info("app", "RunEvent::Exit - finalizing session");
            release_device_on_exit(app_handle, &exit_released_loop);
        }
        _ => {}
    });
}

fn release_device_on_exit(app_handle: &tauri::AppHandle, exit_released: &AtomicBool) {
    if !exit_released.swap(true, Ordering::SeqCst) {
        if let Some(st) = app_handle.try_state::<AppState>() {
            st.manual_disconnect.store(true, Ordering::SeqCst);
            st.connection_id.fetch_add(1, Ordering::SeqCst);
            let skip_standby = st.skip_standby_on_exit.load(Ordering::SeqCst);
            let stby_on_exit = !skip_standby && safe_lock(&st.config).auto_standby_tv_on_exit;
            let mut guard = safe_lock(&st.serial_writer);
            if let Some(ref mut port) = *guard {
                use std::io::Write;
                let _ = port.write_all(b"REMOTE_OFF\n");
                if stby_on_exit {
                    let _ = port.write_all(b"OFF\n");
                }
                let _ = port.flush();
                std::thread::sleep(Duration::from_millis(180));
            }
            *guard = None;
        }
        clear_session_active_marker();
        flush_logger();
    }
}
