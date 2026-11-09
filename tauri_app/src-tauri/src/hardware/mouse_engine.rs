use crate::hardware::state::safe_lock;
use crate::input_simulator::move_mouse_cursor;
use crate::types::AppConfig;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Расчет нелинейного ускорения курсора мыши для 4K/FullHD экранов
pub fn handle_mouse_movement(
    key_code: u8,
    streak_lock: &mut Option<(u8, Instant, u32)>,
) -> String {
    let (dx, dy, dir_str) = match key_code {
        0x01 => (0, -1, "Курсор ↑"),
        0x02 => (0, 1, "Курсор ↓"),
        0x03 => (-1, 0, "Курсор ←"),
        0x04 => (1, 0, "Курсор →"),
        _ => (0, 0, ""),
    };

    let now = Instant::now();
    let count = if let Some((last_k, last_t, c)) = *streak_lock {
        if last_k == key_code && now.duration_since(last_t) < Duration::from_millis(650) {
            c + 1
        } else {
            1
        }
    } else {
        1
    };
    *streak_lock = Some((key_code, now, count));

    // Прогрессивная кривая разгона
    let step = if count <= 1 {
        4 // Микро-шаг для точного попадания по элементам UI
    } else if count == 2 {
        10 // Второй клик
    } else if count <= 4 {
        24 // Начало плавного движения
    } else if count <= 7 {
        55 // Уверенный разгон
    } else if count <= 11 {
        95 // Быстрый бег через экран
    } else {
        150 // Максимальная скорость для 4K телевизоров
    };

    move_mouse_cursor(dx * step, dy * step);
    format!("{} ({}px)", dir_str, step)
}

/// Фоновый поток автоматического сброса режима мыши при неактивности
pub fn spawn_mouse_revert_watchdog(
    mouse_mode_shared: Arc<Mutex<bool>>,
    last_mouse_act_shared: Arc<Mutex<Instant>>,
    config_shared: Arc<Mutex<AppConfig>>,
    app_handle: AppHandle,
) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(1000));
            let is_active = *safe_lock(&mouse_mode_shared);
            let should_revert = safe_lock(&config_shared).auto_revert_mouse;
            if is_active && should_revert {
                let elapsed = safe_lock(&last_mouse_act_shared).elapsed();
                if elapsed > Duration::from_secs(40) {
                    *safe_lock(&mouse_mode_shared) = false;
                    let _ = app_handle.emit("mouse-mode-changed", false);
                    let _ = app_handle.emit(
                        "cec-toast",
                        "Режим мыши автоматически отключен (тайм-аут 40 сек)",
                    );
                    if let Some(hud) = app_handle.get_webview_window("hud") {
                        crate::platform::hide_hud_window_os(&hud);
                    }
                }
            }
        }
    });
}
