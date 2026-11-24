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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OskToggleOutcome {
    WebOpened,
    NativeOpened,
    Closed,
    Failed,
}

pub fn toggle_osk_checked(
    app: &AppHandle,
    osk_visible: &Arc<AtomicBool>,
    force_show: Option<bool>,
) -> OskToggleOutcome {
    // Сверяем флаг с реальной видимостью веб-окна OSK, чтобы исключить залипание флага
    let mut current = osk_visible.load(Ordering::SeqCst);
    if current {
        let actually_visible = app
            .get_webview_window("osk")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if !actually_visible {
            osk_visible.store(false, Ordering::SeqCst);
            current = false;
        }
    }
    let target_state = force_show.unwrap_or(!current);

    if target_state {
        if platform::try_native_osk() {
            // Нативная клавиатура Steam управляется самой ОС/Steam и закрывается пользователем напрямую,
            // поэтому НЕ поднимаем флаг веб-OSK (иначе remote_dispatcher заблокирует остальные кнопки).
            osk_visible.store(false, Ordering::SeqCst);
            let _ = app.emit("osk-state-changed", false);
            return OskToggleOutcome::NativeOpened;
        }
        if let Some((osk_win, _)) = crate::window_manager::get_or_create_osk_window(app) {
            let ok = platform::setup_osk_window(&osk_win);
            let visible_now = ok && osk_win.is_visible().unwrap_or(false);
            if visible_now {
                osk_visible.store(true, Ordering::SeqCst);
                let _ = app.emit("osk-state-changed", true);
                let _ = osk_win.emit("osk-state-changed", true);
                return OskToggleOutcome::WebOpened;
            }
        }
        osk_visible.store(false, Ordering::SeqCst);
        let _ = app.emit("osk-state-changed", false);
        OskToggleOutcome::Failed
    } else {
        platform::try_hide_native_osk();
        if let Some(osk_win) = app.get_webview_window("osk") {
            platform::hide_osk_window(&osk_win);
            let _ = osk_win.emit("osk-state-changed", false);
        }
        osk_visible.store(false, Ordering::SeqCst);
        let _ = app.emit("osk-state-changed", false);
        OskToggleOutcome::Closed
    }
}

pub fn toggle_osk_helper(
    app: &AppHandle,
    osk_visible: &Arc<AtomicBool>,
    force_show: Option<bool>,
) -> bool {
    matches!(
        toggle_osk_checked(app, osk_visible, force_show),
        OskToggleOutcome::WebOpened
    )
}
