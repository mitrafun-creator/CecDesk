use crate::cec_protocol::cec_key_name;
use crate::config::write_log_to_disk;
use crate::hardware::mouse_engine::handle_mouse_movement;
use crate::hardware::state::safe_lock;
use crate::input_simulator::{
    emergency_kill_foreground_task, jump_cursor_to_sector, trigger_combo_key,
    trigger_double_click, trigger_mouse_click, trigger_mouse_scroll,
    trigger_os_key_internal, trigger_repeated_key,
};
use crate::osk::{toggle_osk_helper, type_unicode_char_internal};
use crate::types::AppConfig;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub type KeyStreakRef = Arc<Mutex<Option<(u8, Instant, u32)>>>;

/// Обработка нажатия клавиши пульта (CEC Opcode 0x44: User Control Pressed)
#[allow(clippy::too_many_arguments)]
pub fn dispatch_user_control_pressed(
    key_code: u8,
    app: &AppHandle,
    cfg: &AppConfig,
    calib_mode_ref: &Arc<AtomicBool>,
    osk_visible_ref: &Arc<AtomicBool>,
    mouse_mode_ref: &Arc<Mutex<bool>>,
    last_mouse_act_ref: &Arc<Mutex<Instant>>,
    mouse_streak_ref: &KeyStreakRef,
    back_hold_ref: &Arc<Mutex<Option<Instant>>>,
    last_seek_ref: &KeyStreakRef,
    sess_log: &Path,
    lat_log: &Path,
) {
    let key_hex = format!("0x{:02X}", key_code);
    let btn_name = cec_key_name(key_code);

    // 1. В режиме калибровки/сборки пульта - ТОЛЬКО транслируем код для захвата конструктором,
    // полностью блокируя любые действия в Windows
    if calib_mode_ref.load(Ordering::SeqCst) {
        write_log_to_disk(
            sess_log,
            lat_log,
            &format!("CALIBRATION_CAPTURE: {} ({})", key_hex, btn_name),
        );
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": "Режим калибровки"
            }),
        );
        return;
    }

    let os_act = cfg
        .key_mappings
        .get(&key_hex)
        .cloned()
        .unwrap_or_else(|| "NONE".to_string());

    if os_act == "NONE" {
        write_log_to_disk(
            sess_log,
            lat_log,
            &format!("REMOTE_UNMAPPED: {} ({})", key_hex, btn_name),
        );
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": "Не назначено"
            }),
        );
        return;
    }

    // 2. Обработка экранной клавиатуры (OSK)
    let is_osk_open = osk_visible_ref.load(Ordering::SeqCst);
    if is_osk_open {
        if key_code == 0x0D
            || key_code == 0x0A
            || key_code == 0x0B
            || os_act == "BACKSPACE"
            || os_act == "ESC"
            || os_act == "BROWSER_BACK"
            || os_act == "SHOW_KEYBOARD"
        {
            toggle_osk_helper(app, osk_visible_ref, Some(false));
            write_log_to_disk(
                sess_log,
                lat_log,
                &format!("OSK_NAV: {} ({}) -> Закрыть клавиатуру", key_hex, btn_name),
            );
            let _ = app.emit(
                "cec-remote-key",
                serde_json::json!({
                    "code": key_hex,
                    "button": btn_name,
                    "os_action": "OSK: Закрыть клавиатуру"
                }),
            );
            return;
        }

        let (osk_cmd, osk_desc): (&str, &str) = match key_code {
            0x01 => ("UP", "Вверх ↑"),
            0x02 => ("DOWN", "Вниз ↓"),
            0x03 => ("LEFT", "Влево ←"),
            0x04 => ("RIGHT", "Вправо →"),
            0x00 => ("SELECT", "Выбор"),
            0x72 => ("LANG", "Смена языка (Красная)"),
            0x73 => ("SPACE", "Пробел (Зеленая)"),
            0x74 => ("BACKSPACE", "Стереть (Желтая)"),
            0x71 => ("ENTER", "Ввод Enter (Синяя)"),
            _ => ("", ""),
        };
        if !osk_cmd.is_empty() {
            if let Some(osk_w) = app.get_webview_window("osk") {
                let _ = osk_w.emit("osk-nav", osk_cmd);
            }
            write_log_to_disk(
                sess_log,
                lat_log,
                &format!("OSK_NAV: {} ({}) -> {}", key_hex, btn_name, osk_desc),
            );
            let _ = app.emit(
                "cec-remote-key",
                serde_json::json!({
                    "code": key_hex,
                    "button": btn_name,
                    "os_action": format!("OSK: {}", osk_desc)
                }),
            );
            return;
        }

        // Прямой ввод цифр 0-9 при открытой экранной клавиатуре
        if (0x20..=0x29).contains(&key_code) && os_act.starts_with("NUM_") {
            let digit = ((b'0' + (key_code - 0x20)) as char).to_string();
            type_unicode_char_internal(&digit);
            write_log_to_disk(
                sess_log,
                lat_log,
                &format!("OSK_INPUT: {} ({}) -> Ввод цифры '{}'", key_hex, btn_name, digit),
            );
            let _ = app.emit(
                "cec-remote-key",
                serde_json::json!({
                    "code": key_hex,
                    "button": btn_name,
                    "os_action": format!("Печать: {}", digit)
                }),
            );
            return;
        }
    }

    if os_act == "SHOW_KEYBOARD" {
        toggle_osk_helper(app, osk_visible_ref, None);
        let is_now_open = osk_visible_ref.load(Ordering::SeqCst);
        let desc = if is_now_open {
            "Клавиатура ОТКРЫТА"
        } else {
            "Клавиатура СКРЫТА"
        };
        write_log_to_disk(sess_log, lat_log, &format!("OSK_TOGGLE: {}", desc));
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": desc
            }),
        );
        return;
    }

    // 3. Аварийное снятие зависшего окна (удержание кнопки Back 0x0D > 3.8 сек)
    if key_code == 0x0D && cfg.emergency_kill_enabled {
        let mut hold_lock = safe_lock(back_hold_ref);
        if let Some(start_t) = *hold_lock {
            if start_t.elapsed() >= Duration::from_millis(3800) {
                *hold_lock = None;
                let res = emergency_kill_foreground_task();
                write_log_to_disk(sess_log, lat_log, &format!("EMERGENCY_KILL: {}", res));
                let _ = app.emit("cec-toast", format!("Аварийное закрытие: {}", res));
                crate::window_manager::emit_and_show_hud(
                    app,
                    Some(serde_json::json!({
                        "icon": "warning",
                        "title": "Аварийное закрытие",
                        "desc": format!("Завершено: {}", res)
                    })),
                    None,
                );
                return;
            }
        } else {
            *hold_lock = Some(Instant::now());
        }
    }

    // 4. Саундбар eARC: игнорируем регулировку громкости ПК, если саундбар включен
    if cfg.soundbar_passthrough && (key_code == 0x41 || key_code == 0x42 || key_code == 0x43) {
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": "eARC Акустика"
            }),
        );
        return;
    }

    let mut is_mouse_mode = *safe_lock(mouse_mode_ref);

    // 5. Переключение режима мыши
    if os_act == "MOUSE_TOGGLE" {
        is_mouse_mode = !is_mouse_mode;
        *safe_lock(mouse_mode_ref) = is_mouse_mode;
        *safe_lock(last_mouse_act_ref) = Instant::now();
        *safe_lock(mouse_streak_ref) = None;
        let mode_str = if is_mouse_mode {
            "Режим мыши ВКЛ"
        } else {
            "Режим мыши ВЫКЛ"
        };
        let _ = app.emit("mouse-mode-changed", is_mouse_mode);
        let hud_payload = serde_json::json!({
            "icon": "mouse",
            "title": if is_mouse_mode { "Режим мыши активен" } else { "Режим мыши выключен" },
            "desc": if is_mouse_mode { "Стрелки: Курсор • OK: Клик • 1-9: Секторы" } else { "Обычный режим пульта" },
            "mouse_active": is_mouse_mode
        });
        if is_mouse_mode {
            crate::window_manager::emit_and_show_hud(app, Some(hud_payload), None);
        } else {
            let _ = app.emit("show-hud", &hud_payload);
            crate::window_manager::hide_hud_if_exists(app);
        }
        write_log_to_disk(sess_log, lat_log, &format!("MOUSE_MODE_TOGGLE: {}", mode_str));
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": mode_str
            }),
        );
    } else if is_mouse_mode {
        // Управление курсором и действиями в режиме мыши
        *safe_lock(last_mouse_act_ref) = Instant::now();
        let action_desc: String = match key_code {
            // Навигация курсором со ступенчатым ускорением
            0x01..=0x04 => {
                let mut streak_lock = safe_lock(mouse_streak_ref);
                handle_mouse_movement(key_code, &mut streak_lock)
            }
            // Клик левой кнопкой мыши (ЛКМ)
            0x00 | 0x4C => {
                trigger_mouse_click(true);
                "ЛКМ (Клик)".to_string()
            }
            // Контекстное меню (ПКМ)
            0x09 | 0x0A | 0x0B | 0x4B => {
                trigger_mouse_click(false);
                "ПКМ (Меню)".to_string()
            }
            // Двойной клик ЛКМ
            0x44 => {
                trigger_double_click();
                "2x Клик (ЛКМ)".to_string()
            }
            // Прокрутка колесом мыши (Канал + / Канал -)
            0x30 => {
                trigger_mouse_scroll(true);
                "Скролл вверх".to_string()
            }
            0x31 => {
                trigger_mouse_scroll(false);
                "Скролл вниз".to_string()
            }
            // Навигация в браузере (Назад / Вперед)
            0x0D | 0x48 => {
                trigger_combo_key(0x12, 0x25); // Alt + Left
                "Назад в браузере".to_string()
            }
            0x49 => {
                trigger_combo_key(0x12, 0x27); // Alt + Right
                "Вперед в браузере".to_string()
            }
            // Полезные быстрые макросы на цветных кнопках
            0x72 => {
                trigger_combo_key(0x11, 0x43); // Ctrl + C
                "Копировать (Ctrl+C)".to_string()
            }
            0x73 => {
                trigger_combo_key(0x11, 0x56); // Ctrl + V
                "Вставить (Ctrl+V)".to_string()
            }
            0x74 => {
                trigger_combo_key(0x11, 0x41); // Ctrl + A
                "Выделить всё (Ctrl+A)".to_string()
            }
            0x71 => {
                trigger_combo_key(0x5B, 0x44); // Win + D
                "Рабочий стол (Win+D)".to_string()
            }
            // Адаптивные прыжки по секторам экрана (Цифры 0-9)
            0x20..=0x29 => {
                let sector = if key_code == 0x20 { 0 } else { key_code - 0x20 };
                let (x, y) = jump_cursor_to_sector(sector);
                crate::window_manager::emit_and_show_hud(
                    app,
                    None,
                    Some(serde_json::json!({
                        "sector": sector,
                        "x": x,
                        "y": y
                    })),
                );
                if sector == 0 || sector == 5 {
                    format!("Прыжок в центр (Сектор {})", sector)
                } else {
                    format!("Прыжок в сектор {}", sector)
                }
            }
            // Все прочие кнопки, не относящиеся к мыши, БЛОКИРУЮТСЯ
            _ => "Заблокировано (Режим мыши)".to_string(),
        };
        write_log_to_disk(
            sess_log,
            lat_log,
            &format!("MOUSE_MODE_ACT: {} ({}) -> {}", key_hex, btn_name, action_desc),
        );
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": action_desc
            }),
        );
    } else {
        // 6. Умная перемотка с ускорением (Hold-to-Accelerate Seek) — срабатывает только когда клавиша назначена на LEFT или RIGHT
        if cfg.hold_seek_accelerate && (os_act == "LEFT" || os_act == "RIGHT") {
            let mut seek_lock = safe_lock(last_seek_ref);
            let now = Instant::now();
            let is_left = os_act == "LEFT";
            let vk = if is_left { 0x25 } else { 0x27 }; // Left / Right
            let (cnt, desc) = if let Some((k, t, c)) = *seek_lock {
                if k == key_code && now.duration_since(t) < Duration::from_millis(600) {
                    let new_c = c + 1;
                    if new_c > 6 {
                        trigger_repeated_key(vk, 6);
                        (new_c, "Ускорение >> (30 сек)")
                    } else if new_c > 2 {
                        trigger_repeated_key(vk, 3);
                        (new_c, "Ускорение > (15 сек)")
                    } else {
                        trigger_repeated_key(vk, 1);
                        (new_c, "Шаг 5 сек")
                    }
                } else {
                    trigger_repeated_key(vk, 1);
                    (1, "Шаг 5 сек")
                }
            } else {
                trigger_repeated_key(vk, 1);
                (1, "Шаг 5 сек")
            };
            *seek_lock = Some((key_code, now, cnt));
            let _ = app.emit(
                "cec-remote-key",
                serde_json::json!({
                    "code": key_hex,
                    "button": btn_name,
                    "os_action": desc
                }),
            );
            return;
        }

        // 7. Стандартное выполнение действия ОС
        let os_desc = trigger_os_key_internal(&os_act);
        if cfg.hud_enabled && (os_act.starts_with("VOL_") || os_act.starts_with("MEDIA_")) {
            let (icon, title) = match os_act.as_str() {
                "VOL_UP" => ("volume-up", "Громкость +"),
                "VOL_DOWN" => ("volume-down", "Громкость −"),
                "VOL_MUTE" => ("volume-mute", "Без звука"),
                "MEDIA_PLAY" => ("play-pause", "Play / Pause"),
                "MEDIA_STOP" => ("stop", "Стоп"),
                "MEDIA_NEXT" => ("next", "След. трек"),
                "MEDIA_PREV" => ("prev", "Пред. трек"),
                _ => ("command", os_desc.as_str()),
            };
            crate::window_manager::emit_and_show_hud(
                app,
                Some(serde_json::json!({
                    "icon": icon,
                    "title": title,
                    "desc": btn_name
                })),
                None,
            );
        }
        write_log_to_disk(
            sess_log,
            lat_log,
            &format!("REMOTE_PASS: {} ({}) -> {}", key_hex, btn_name, os_desc),
        );
        let _ = app.emit(
            "cec-remote-key",
            serde_json::json!({
                "code": key_hex,
                "button": btn_name,
                "os_action": os_desc
            }),
        );
    }
}

/// Обработка отпускания клавиши пульта (Opcode 0x45: User Control Released)
pub fn dispatch_user_control_released(
    app: &AppHandle,
    back_hold_ref: &Arc<Mutex<Option<Instant>>>,
    last_seek_ref: &KeyStreakRef,
    mouse_streak_ref: &KeyStreakRef,
    _calib_mode_ref: &Arc<AtomicBool>,
) {
    *safe_lock(back_hold_ref) = None;
    *safe_lock(last_seek_ref) = None;
    *safe_lock(mouse_streak_ref) = None;
    let _ = app.emit("cec-remote-release", ());
}

/// Обработка команд плеера (Opcode 0x41 / 0x42: Deck Play / Control)
pub fn dispatch_deck_control(opcode: u8, app: &AppHandle) {
    let os_desc = trigger_os_key_internal("MEDIA_PLAY");
    let _ = app.emit(
        "cec-remote-key",
        serde_json::json!({
            "code": format!("0x{:02X}", opcode),
            "button": "Deck Play/Control",
            "os_action": os_desc
        }),
    );
}
