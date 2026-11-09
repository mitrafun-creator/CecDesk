use crate::platform;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub fn type_unicode_char_internal(text: &str) {
    platform::type_unicode_char(text);
}

pub fn type_special_key_internal(key: &str) {
    platform::type_special_key(key);
}

pub fn toggle_osk_helper(
    app: &AppHandle,
    osk_visible: &Arc<AtomicBool>,
    force_show: Option<bool>,
) -> bool {
    let current = osk_visible.load(Ordering::SeqCst);
    let target_state = force_show.unwrap_or(!current);

    if let Some(osk_win) = app.get_webview_window("osk") {
        if target_state {
            // Если система использует собственную нативную клавиатуру (например, SteamOS / Steam Big Picture: steam://open/keyboard),
            // platform::setup_osk_window открывает её и возвращает false, чтобы не показывать дублирующее веб-окно OSK.
            let ok = platform::setup_osk_window(&osk_win);
            if ok {
                osk_visible.store(true, Ordering::SeqCst);
                let _ = app.emit("osk-state-changed", true);
                let _ = osk_win.emit("osk-state-changed", true);
                return true;
            }
            osk_visible.store(false, Ordering::SeqCst);
            false
        } else {
            platform::hide_osk_window(&osk_win);
            osk_visible.store(false, Ordering::SeqCst);
            let _ = app.emit("osk-state-changed", false);
            let _ = osk_win.emit("osk-state-changed", false);
            false
        }
    } else {
        false
    }
}
