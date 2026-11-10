use crate::hardware::state::{safe_lock, AppState};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

static HUD_CREATE_LOCK: Mutex<()> = Mutex::new(());
static OSK_CREATE_LOCK: Mutex<()> = Mutex::new(());

/// Ленивое создание (или получение) окна HUD оверлея.
/// Возвращает (окно, true_если_создано_сейчас).
pub fn get_or_create_hud_window(app: &AppHandle) -> Option<(WebviewWindow, bool)> {
    let _guard = HUD_CREATE_LOCK.lock().ok();
    if let Some(w) = app.get_webview_window("hud") {
        return Some((w, false));
    }
    match WebviewWindowBuilder::new(app, "hud", WebviewUrl::App("hud.html".into()))
        .title("CEC HUD Overlay")
        .inner_size(480.0, 170.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .resizable(false)
        .shadow(false)
        .build()
    {
        Ok(w) => Some((w, true)),
        Err(_) => app.get_webview_window("hud").map(|w| (w, false)),
    }
}

/// Ленивое создание (или получение) окна виртуальной экранной клавиатуры (OSK).
pub fn get_or_create_osk_window(app: &AppHandle) -> Option<(WebviewWindow, bool)> {
    let _guard = OSK_CREATE_LOCK.lock().ok();
    if let Some(w) = app.get_webview_window("osk") {
        return Some((w, false));
    }
    match WebviewWindowBuilder::new(app, "osk", WebviewUrl::App("osk.html".into()))
        .title("Smart On-Screen Keyboard")
        .inner_size(1040.0, 380.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .resizable(false)
        .shadow(false)
        .build()
    {
        Ok(w) => Some((w, true)),
        Err(_) => app.get_webview_window("osk").map(|w| (w, false)),
    }
}

/// Отправляет HUD событие и гарантирует отображение окна HUD (создавая его лениво при необходимости).
pub fn emit_and_show_hud(
    app: &AppHandle,
    show_payload: Option<serde_json::Value>,
    sector_payload: Option<serde_json::Value>,
) {
    if let Some(st) = app.try_state::<AppState>() {
        let mut pending = safe_lock(&st.pending_hud);
        let now = Instant::now();
        if let Some(ref p) = show_payload {
            pending.last_show = Some((now, p.clone()));
        }
        if let Some(ref s) = sector_payload {
            pending.last_sector = Some((now, s.clone()));
        }
    }

    if let Some(ref p) = show_payload {
        let _ = app.emit("show-hud", p);
    }
    if let Some(ref s) = sector_payload {
        let _ = app.emit("mouse-sector-jump", s);
    }

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.show();
        let _ = hud.unminimize();
        crate::platform::setup_hud_window(&hud);
    } else {
        // Создаем окно HUD в асинхронной задаче: при готовности hud.html заберет состояние через get_initial_hud_state
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = tauri::async_runtime::spawn_blocking(move || {
                let _ = get_or_create_hud_window(&app_clone);
            })
            .await;
        });
    }
}

/// Безопасно скрывает HUD окно, если оно существует (не создавая его зря).
pub fn hide_hud_if_exists(app: &AppHandle) {
    if let Some(st) = app.try_state::<AppState>() {
        let mut pending = safe_lock(&st.pending_hud);
        pending.last_show = None;
    }
    if let Some(hud) = app.get_webview_window("hud") {
        crate::platform::hide_hud_window_os(&hud);
    }
}
