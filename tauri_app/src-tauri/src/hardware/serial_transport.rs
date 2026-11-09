use crate::audio::{switch_audio_endpoint, switch_display_mode};
use crate::config::write_log_to_disk;
use crate::hardware::remote_dispatcher::{
    dispatch_deck_control, dispatch_user_control_pressed, dispatch_user_control_released,
};
use crate::hardware::state::{safe_lock, AppState};
use serialport::SerialPort;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Фоновый поток непрерывного чтения данных из последовательного порта ESP32-S3 / RP2040
pub fn spawn_serial_reader_thread(
    reader_port: Box<dyn SerialPort>,
    app: AppHandle,
    state: &AppState,
    conn_id: u64,
) {
    let cfg_arc = Arc::clone(&state.config);
    let writer_ref = Arc::clone(&state.serial_writer);
    let conn_id_ref = Arc::clone(&state.connection_id);
    let mouse_mode_ref = Arc::clone(&state.mouse_mode_active);
    let last_mouse_act_ref = Arc::clone(&state.last_mouse_activity);
    let mouse_streak_ref = Arc::clone(&state.mouse_move_streak);
    let back_hold_ref = Arc::clone(&state.back_hold_start);
    let last_seek_ref = Arc::clone(&state.last_seek_press);
    let osk_visible_ref = Arc::clone(&state.osk_visible);
    let calib_mode_ref = Arc::clone(&state.calibration_mode);
    let sess_log = state.session_log_path.clone();
    let lat_log = state.latest_log_path.clone();

    thread::spawn(move || {
        let mut buf_reader = BufReader::new(reader_port);
        let mut raw_buf: Vec<u8> = Vec::with_capacity(256);
        let mut last_heartbeat = String::new();
        let mut last_pc_off_trigger: Option<Instant> = None;
        let mut was_tv_on = false;
        loop {
            if conn_id_ref.load(Ordering::SeqCst) != conn_id {
                break;
            }
            raw_buf.clear();
            match buf_reader.read_until(b'\n', &mut raw_buf) {
                Ok(0) => break,
                Ok(_) => {
                    if conn_id_ref.load(Ordering::SeqCst) != conn_id {
                        break;
                    }
                    let line = String::from_utf8_lossy(&raw_buf);
                    let trimmed = line.trim().to_string();
                    if !trimmed.is_empty() {
                        // Дедупликация периодических heartbeat строк (OK:LINE_...) для защиты от забивания логов
                        let is_heartbeat = trimmed.starts_with("OK:LINE_");
                        if !is_heartbeat || trimmed != last_heartbeat {
                            write_log_to_disk(&sess_log, &lat_log, &format!("ESP32: {}", trimmed));
                            if is_heartbeat {
                                last_heartbeat = trimmed.clone();
                            }
                        }
                        let _ = app.emit("cec-rx-line", &trimmed);

                        // Проверка аппаратного статуса линии CEC
                        if trimmed.starts_with("OK:LINE_") {
                            let is_high = trimmed.contains("LINE_HIGH");
                            let tv_conn = trimmed.contains("EXT_PULLUP_YES");
                            let _ = app.emit(
                                "cec-hardware-status",
                                serde_json::json!({
                                    "line_high": is_high,
                                    "tv_connected": tv_conn,
                                    "raw": &trimmed
                                }),
                            );
                        }

                        // Проверка статуса питания телевизора (0x90 Power Status или 0x36 Standby)
                        let is_power_rep = trimmed.contains(": 90 ");
                        let is_standby_cmd = trimmed.contains(": 36");

                        if trimmed.starts_with("RX ") && (is_power_rep || is_standby_cmd) {
                            let cfg = safe_lock(&cfg_arc).clone();
                            if is_power_rep && trimmed.contains(": 90 00") {
                                // ТВ ВКЛЮЧЕН
                                was_tv_on = true;
                                if cfg.auto_audio_switch && !cfg.tv_audio_device.is_empty() {
                                    let _ = switch_audio_endpoint(&cfg.tv_audio_device);
                                }
                                if cfg.display_disable_on_sleep {
                                    switch_display_mode(true);
                                }
                            } else if (is_power_rep
                                && (trimmed.contains(": 90 01") || trimmed.contains(": 90 03")))
                                || is_standby_cmd
                            {
                                let transitioned_to_off = is_standby_cmd || was_tv_on;
                                was_tv_on = false;
                                // ТВ В СПЯЩЕМ РЕЖИМЕ (по команде Standby или статусу 0x90 01/03)
                                if cfg.auto_audio_switch && !cfg.fallback_audio_device.is_empty() {
                                    let _ = switch_audio_endpoint(&cfg.fallback_audio_device);
                                }
                                if cfg.display_disable_on_sleep {
                                    switch_display_mode(false);
                                }
                                // Сценарий усыпления или выключения ПК при выключении ТВ
                                if cfg.power_off_pc_on_tv_off && transitioned_to_off {
                                    let can_trigger = match last_pc_off_trigger {
                                        Some(t) => t.elapsed() > Duration::from_secs(15),
                                        None => true,
                                    };
                                    if can_trigger {
                                        last_pc_off_trigger = Some(Instant::now());
                                        let act = cfg.pc_off_action.to_uppercase();
                                        let msg = match act.as_str() {
                                            "SHUTDOWN" => crate::platform::shutdown_pc(),
                                            "HIBERNATE" => crate::platform::hibernate_pc(),
                                            "OPTO_PULSE" => {
                                                if let Ok(mut g) = writer_ref.lock() {
                                                    if let Some(ref mut p) = *g {
                                                        let _ = p.write_all(b"PC_PWR_PULSE\n");
                                                    }
                                                }
                                                "Импульс кнопки Power (Оптопара 350 мс)".to_string()
                                            }
                                            _ => crate::platform::put_pc_to_sleep(),
                                        };
                                        write_log_to_disk(
                                            &sess_log,
                                            &lat_log,
                                            &format!("TV_OFF_PC_ACTION ({}): {}", act, msg),
                                        );
                                        let _ = app.emit("cec-toast", msg);
                                    }
                                }
                            }
                        }

                        // Обработка входящих CEC пакетов
                        if let Some(rx_body) = trimmed.strip_prefix("RX ") {
                            let cfg = safe_lock(&cfg_arc).clone();
                            if cfg.remote_to_os_enabled {
                                if let Some((_hdr, hex_part)) = rx_body.split_once(':') {
                                    let bytes: Vec<u8> = hex_part
                                        .split_whitespace()
                                        .filter_map(|b| u8::from_str_radix(b, 16).ok())
                                        .collect();

                                    if bytes.len() >= 2 && bytes[0] == 0x44 {
                                        let key_hex = format!("0x{:02X}", bytes[1]);
                                        if cfg.key_mappings.get(&key_hex).map(|s| s.as_str())
                                            == Some("OPTO_POWER")
                                        {
                                            if let Ok(mut g) = writer_ref.lock() {
                                                if let Some(ref mut p) = *g {
                                                    let _ = p.write_all(b"PC_PWR_PULSE\n");
                                                }
                                            }
                                        }
                                        // 0x44: User Control Pressed
                                        dispatch_user_control_pressed(
                                            bytes[1],
                                            &app,
                                            &cfg,
                                            &calib_mode_ref,
                                            &osk_visible_ref,
                                            &mouse_mode_ref,
                                            &last_mouse_act_ref,
                                            &mouse_streak_ref,
                                            &back_hold_ref,
                                            &last_seek_ref,
                                            &sess_log,
                                            &lat_log,
                                        );
                                    } else if !bytes.is_empty() && bytes[0] == 0x45 {
                                        // 0x45: User Control Released
                                        dispatch_user_control_released(
                                            &app,
                                            &back_hold_ref,
                                            &last_seek_ref,
                                            &mouse_streak_ref,
                                            &calib_mode_ref,
                                        );
                                    } else if !bytes.is_empty()
                                        && (bytes[0] == 0x41 || bytes[0] == 0x42)
                                    {
                                        // 0x41 / 0x42: Deck Play / Control
                                        dispatch_deck_control(bytes[0], &app);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if conn_id_ref.load(Ordering::SeqCst) != conn_id
                        || safe_lock(&writer_ref).is_none()
                    {
                        break;
                    }
                    continue;
                }
                Err(_) => break,
            }
        }
        // Сбрасываем активное подключение ТОЛЬКО если этот поток всё ещё является текущим (не был заменен новым портом!)
        if conn_id_ref.load(Ordering::SeqCst) == conn_id {
            {
                let mut guard = safe_lock(&writer_ref);
                *guard = None;
            }
            write_log_to_disk(&sess_log, &lat_log, "=== ПОРТ ОТКЛЮЧЕН ===");
            let _ = app.emit("cec-disconnected", ());
        }
    });
}
