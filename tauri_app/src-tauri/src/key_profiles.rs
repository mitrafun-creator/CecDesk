use crate::config::{find_config_path, save_atomic};
use crate::hardware::state::safe_lock;
use crate::types::AppConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const KEY_PROFILE_ORDER: [&str; 6] = [
    "balanced",
    "cinema",
    "steam",
    "desktop",
    "numpad",
    "custom",
];

fn build_mappings(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// Возвращает встроенную таблицу назначений клавиш для стандартного профиля (`None` для `custom` или неизвестного ID).
pub fn preset_mappings(profile_id: &str) -> Option<HashMap<String, String>> {
    const TAIL_DEFAULT: [(&str, &str); 14] = [
        ("0x48", "LEFT"),
        ("0x49", "RIGHT"),
        ("0x4B", "MEDIA_NEXT"),
        ("0x4C", "MEDIA_PREV"),
        ("0x30", "PGUP"),
        ("0x31", "PGDN"),
        ("0x20", "NUM_0"),
        ("0x21", "NUM_1"),
        ("0x22", "NUM_2"),
        ("0x23", "NUM_3"),
        ("0x24", "NUM_4"),
        ("0x25", "NUM_5"),
        ("0x26", "NUM_6"),
        ("0x27", "NUM_7"),
    ];
    const NUM_8_9: [(&str, &str); 2] = [("0x28", "NUM_8"), ("0x29", "NUM_9")];

    let head: &[(&str, &str)] = match profile_id {
        "balanced" => &[
            ("0x00", "ENTER"),
            ("0x01", "UP"),
            ("0x02", "DOWN"),
            ("0x03", "LEFT"),
            ("0x04", "RIGHT"),
            ("0x0D", "ESC"),
            ("0x09", "STEAM_BIG_PICTURE"),
            ("0x0A", "SHOW_KEYBOARD"),
            ("0x71", "MOUSE_TOGGLE"),
            ("0x72", "STEAM_BIG_PICTURE"),
            ("0x73", "VOL_MUTE"),
            ("0x74", "SPACE"),
            ("0x41", "VOL_UP"),
            ("0x42", "VOL_DOWN"),
            ("0x43", "VOL_MUTE"),
            ("0x44", "MEDIA_PLAY"),
            ("0x45", "MEDIA_STOP"),
            ("0x46", "MEDIA_PLAY"),
        ],
        "cinema" => &[
            ("0x00", "SPACE"),
            ("0x01", "VOL_UP"),
            ("0x02", "VOL_DOWN"),
            ("0x03", "LEFT"),
            ("0x04", "RIGHT"),
            ("0x0D", "ESC"),
            ("0x09", "F11"),
            ("0x0A", "SHOW_KEYBOARD"),
            ("0x71", "MOUSE_TOGGLE"),
            ("0x72", "NONE"),
            ("0x73", "VOL_MUTE"),
            ("0x74", "SPACE"),
            ("0x41", "VOL_UP"),
            ("0x42", "VOL_DOWN"),
            ("0x43", "VOL_MUTE"),
            ("0x44", "MEDIA_PLAY"),
            ("0x45", "MEDIA_STOP"),
            ("0x46", "MEDIA_PLAY"),
        ],
        "steam" => &[
            ("0x00", "ENTER"),
            ("0x01", "UP"),
            ("0x02", "DOWN"),
            ("0x03", "LEFT"),
            ("0x04", "RIGHT"),
            ("0x0D", "ESC"),
            ("0x09", "STEAM_BIG_PICTURE"),
            ("0x0A", "ESC"),
            ("0x71", "MOUSE_TOGGLE"),
            ("0x72", "STEAM_BIG_PICTURE"),
            ("0x73", "ALT_TAB"),
            ("0x74", "WIN_D"),
            ("0x41", "VOL_UP"),
            ("0x42", "VOL_DOWN"),
            ("0x43", "VOL_MUTE"),
            ("0x44", "ENTER"),
            ("0x45", "ESC"),
            ("0x46", "SPACE"),
            ("0x48", "LEFT"),
            ("0x49", "RIGHT"),
            ("0x4B", "TAB"),
            ("0x4C", "BROWSER_BACK"),
            ("0x30", "PGUP"),
            ("0x31", "PGDN"),
            ("0x20", "NUM_0"),
            ("0x21", "NUM_1"),
            ("0x22", "NUM_2"),
            ("0x23", "NUM_3"),
            ("0x24", "NUM_4"),
            ("0x25", "NUM_5"),
            ("0x26", "NUM_6"),
            ("0x27", "NUM_7"),
            ("0x28", "NUM_8"),
            ("0x29", "NUM_9"),
        ],
        "desktop" => &[
            ("0x00", "ENTER"),
            ("0x01", "PGUP"),
            ("0x02", "PGDN"),
            ("0x03", "BROWSER_BACK"),
            ("0x04", "TAB"),
            ("0x0D", "BACKSPACE"),
            ("0x09", "WIN_D"),
            ("0x0A", "SHOW_KEYBOARD"),
            ("0x71", "MOUSE_TOGGLE"),
            ("0x72", "ALT_TAB"),
            ("0x73", "TAB"),
            ("0x74", "SHOW_KEYBOARD"),
            ("0x41", "VOL_UP"),
            ("0x42", "VOL_DOWN"),
            ("0x43", "VOL_MUTE"),
            ("0x44", "SPACE"),
            ("0x45", "ESC"),
            ("0x46", "SPACE"),
            ("0x48", "LEFT"),
            ("0x49", "RIGHT"),
            ("0x4B", "TAB"),
            ("0x4C", "BROWSER_BACK"),
            ("0x30", "PGUP"),
            ("0x31", "PGDN"),
            ("0x20", "NUM_0"),
            ("0x21", "NUM_1"),
            ("0x22", "NUM_2"),
            ("0x23", "NUM_3"),
            ("0x24", "NUM_4"),
            ("0x25", "NUM_5"),
            ("0x26", "NUM_6"),
            ("0x27", "NUM_7"),
            ("0x28", "NUM_8"),
            ("0x29", "NUM_9"),
        ],
        "numpad" => &[
            ("0x00", "ENTER"),
            ("0x01", "UP"),
            ("0x02", "DOWN"),
            ("0x03", "LEFT"),
            ("0x04", "RIGHT"),
            ("0x0D", "BACKSPACE"),
            ("0x09", "SHOW_KEYBOARD"),
            ("0x0A", "ESC"),
            ("0x71", "MOUSE_TOGGLE"),
            ("0x72", "STEAM_BIG_PICTURE"),
            ("0x73", "VOL_MUTE"),
            ("0x74", "SPACE"),
            ("0x41", "VOL_UP"),
            ("0x42", "VOL_DOWN"),
            ("0x43", "VOL_MUTE"),
            ("0x44", "MEDIA_PLAY"),
            ("0x45", "MEDIA_STOP"),
            ("0x46", "MEDIA_PLAY"),
        ],
        _ => return None,
    };

    let mut map = build_mappings(head);
    if matches!(profile_id, "balanced" | "cinema" | "numpad") {
        for (k, v) in TAIL_DEFAULT.iter().chain(NUM_8_9.iter()) {
            map.insert((*k).to_string(), (*v).to_string());
        }
    }
    Some(map)
}

pub fn is_profile_switch_action(action: &str) -> bool {
    action == "PROFILE_NEXT"
        || action == "PROFILE_PREV"
        || action.starts_with("PROFILE_SWITCH:")
}

pub fn resolve_target_profile(current_id: &str, action: &str) -> Option<&'static str> {
    if action == "PROFILE_NEXT" {
        let pos = KEY_PROFILE_ORDER
            .iter()
            .position(|&id| id == current_id)
            .unwrap_or(0);
        return Some(KEY_PROFILE_ORDER[(pos + 1) % KEY_PROFILE_ORDER.len()]);
    }
    if action == "PROFILE_PREV" {
        let pos = KEY_PROFILE_ORDER
            .iter()
            .position(|&id| id == current_id)
            .unwrap_or(0);
        let prev = if pos == 0 {
            KEY_PROFILE_ORDER.len() - 1
        } else {
            pos - 1
        };
        return Some(KEY_PROFILE_ORDER[prev]);
    }
    if let Some(target) = action.strip_prefix("PROFILE_SWITCH:") {
        return KEY_PROFILE_ORDER.iter().copied().find(|&id| id == target);
    }
    None
}

pub fn profile_display_name(cfg: &AppConfig, profile_id: &str) -> String {
    match profile_id {
        "balanced" => "Универсальный".to_string(),
        "cinema" => "Кино".to_string(),
        "steam" => "Игры".to_string(),
        "desktop" => "Браузер".to_string(),
        "numpad" => "Цифры".to_string(),
        "custom" => {
            let trimmed = cfg.custom_key_profile_name.trim();
            if trimmed.is_empty() {
                "Пользовательский".to_string()
            } else {
                trimmed.to_string()
            }
        }
        other => other.to_string(),
    }
}

pub fn profile_icon(profile_id: &str) -> &'static str {
    match profile_id {
        "balanced" => "command",
        "cinema" => "play-pause",
        "steam" => "gamepad",
        "desktop" => "browser",
        "numpad" => "numpad",
        "custom" => "spark",
        _ => "command",
    }
}

/// Переключает активный профиль кнопок в конфигурации и возвращает `(название, иконка)`.
pub fn apply_profile_switch(cfg: &mut AppConfig, target_id: &str) -> (String, &'static str) {
    if cfg.active_key_profile == "custom" && target_id != "custom" {
        cfg.custom_key_mappings = cfg.key_mappings.clone();
    }

    if target_id == "custom" {
        if cfg.custom_key_mappings.is_empty() {
            cfg.custom_key_mappings = if cfg.key_mappings.is_empty() {
                preset_mappings("balanced").unwrap_or_default()
            } else {
                cfg.key_mappings.clone()
            };
        }
        cfg.key_mappings = cfg.custom_key_mappings.clone();
    } else if let Some(preset) = preset_mappings(target_id) {
        cfg.key_mappings = preset;
    }

    cfg.active_key_profile = target_id.to_string();
    (profile_display_name(cfg, target_id), profile_icon(target_id))
}

/// Выполняет действие смены профиля с сохранением конфига и оповещением UI/HUD.
pub fn execute_profile_switch_action(
    action: &str,
    app: &AppHandle,
    cfg_arc: &Arc<Mutex<AppConfig>>,
) -> String {
    let (target_id, snapshot, hud_enabled, display_name, icon) = {
        let mut guard = safe_lock(cfg_arc);
        let Some(target) = resolve_target_profile(&guard.active_key_profile, action) else {
            return format!("Неизвестный профиль: {}", action);
        };
        let (name, ico) = apply_profile_switch(&mut guard, target);
        (target, guard.clone(), guard.hud_enabled, name, ico)
    };

    if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
        let _ = save_atomic(&find_config_path(), &json);
    }

    let _ = app.emit("key-profile-changed", &snapshot);
    if hud_enabled {
        crate::window_manager::emit_and_show_hud(
            app,
            Some(serde_json::json!({
                "icon": icon,
                "title": "Профиль пульта",
                "desc": display_name
            })),
            None,
        );
    }
    let _ = target_id;
    format!("Профиль: {}", display_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_target_profile_cycle_and_direct() {
        assert_eq!(resolve_target_profile("balanced", "PROFILE_NEXT"), Some("cinema"));
        assert_eq!(resolve_target_profile("custom", "PROFILE_NEXT"), Some("balanced"));
        assert_eq!(resolve_target_profile("balanced", "PROFILE_PREV"), Some("custom"));
        assert_eq!(resolve_target_profile("cinema", "PROFILE_PREV"), Some("balanced"));
        assert_eq!(resolve_target_profile("balanced", "PROFILE_SWITCH:steam"), Some("steam"));
        assert_eq!(resolve_target_profile("steam", "PROFILE_SWITCH:custom"), Some("custom"));
        assert_eq!(resolve_target_profile("balanced", "PROFILE_SWITCH:unknown"), None);
    }

    #[test]
    fn test_apply_profile_switch_preserves_custom_name_and_mappings() {
        let mut cfg = AppConfig::default();
        cfg.custom_key_profile_name = "Kodi HTPC".to_string();
        cfg.custom_key_mappings.insert("0x72".to_string(), "PROFILE_NEXT".to_string());

        let (name, ico) = apply_profile_switch(&mut cfg, "custom");
        assert_eq!(name, "Kodi HTPC");
        assert_eq!(ico, "spark");
        assert_eq!(cfg.active_key_profile, "custom");
        assert_eq!(cfg.key_mappings.get("0x72").map(String::as_str), Some("PROFILE_NEXT"));

        let (cinema_name, _) = apply_profile_switch(&mut cfg, "cinema");
        assert_eq!(cinema_name, "Кино");
        assert_eq!(cfg.key_mappings.get("0x00").map(String::as_str), Some("SPACE"));

        let (back_name, _) = apply_profile_switch(&mut cfg, "custom");
        assert_eq!(back_name, "Kodi HTPC");
        assert_eq!(cfg.key_mappings.get("0x72").map(String::as_str), Some("PROFILE_NEXT"));
    }

    #[test]
    fn test_all_builtin_presets_exist() {
        for id in ["balanced", "cinema", "steam", "desktop", "numpad"] {
            let map = preset_mappings(id).expect("builtin preset must exist");
            assert!(map.contains_key("0x00"));
        }
        assert!(preset_mappings("custom").is_none());
    }
}
