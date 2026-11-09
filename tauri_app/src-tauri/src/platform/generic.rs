use crate::types::EdidInfoDto;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use tauri::WebviewWindow;

pub fn create_hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    Command::new(program)
}

pub fn open_folder(_path: &Path) -> Result<(), String> {
    Ok(())
}

pub fn run_script(_path: &Path, _ext: &str, _dir: &Path) -> Result<(), String> {
    Ok(())
}

pub fn check_single_instance_or_focus(_window_title: &str) -> bool {
    true
}

pub fn put_pc_to_sleep() -> String {
    "Unsupported platform".to_string()
}

pub fn hibernate_pc() -> String {
    "Unsupported platform".to_string()
}

pub fn shutdown_pc() -> String {
    "Unsupported platform".to_string()
}

pub fn move_mouse_cursor(_dx: i32, _dy: i32) {}

pub fn trigger_mouse_click(_is_left: bool) {}

pub fn trigger_mouse_scroll(_up: bool) {}

pub fn trigger_double_click() {}

pub fn jump_cursor_to_sector(_sector: u8) -> (i32, i32) {
    (0, 0)
}

pub fn trigger_combo_key(_mod_vk: u8, _vk: u8) {}

pub fn trigger_repeated_key(_vk: u8, _count: usize) {}

pub fn trigger_os_key(_action: &str) -> String {
    "—".to_string()
}

pub fn emergency_kill_foreground_task() -> String {
    "Unsupported platform".to_string()
}

pub fn toggle_steam_big_picture() -> String {
    "Unsupported platform".to_string()
}

#[allow(dead_code)]
pub fn exit_steam_big_picture_to_desktop() -> String {
    "Unsupported platform".to_string()
}

pub fn detect_edid_os() -> EdidInfoDto {
    EdidInfoDto {
        pnp_id: "GEN".to_string(),
        monitor_id: "GENERIC_HDMI".to_string(),
        physical_address: "1000".to_string(),
        hdmi_port_num: 1,
    }
}

pub fn switch_audio_endpoint(_target: &str) -> Result<String, String> {
    Ok("Generic audio endpoint".to_string())
}

pub fn list_audio_endpoints() -> Vec<String> {
    vec!["Default Device".to_string()]
}

pub fn switch_display_mode(_extend: bool) {}

pub fn setup_osk_window(osk_win: &WebviewWindow) -> bool {
    let _ = osk_win.show();
    true
}

pub fn hide_osk_window(osk_win: &WebviewWindow) {
    let _ = osk_win.hide();
}

pub fn type_unicode_char(_text: &str) {}

pub fn type_special_key(_key: &str) {}

pub fn setup_hud_window(_hud_win: &WebviewWindow) -> bool {
    let _ = _hud_win.show();
    true
}

pub fn hide_hud_window_os(_hud_win: &WebviewWindow) {
    let _ = _hud_win.hide();
}

pub fn set_gamescope_overlay_props(_win: &WebviewWindow, _is_overlay: bool) {}

pub fn set_autostart_os(_enabled: bool) -> Result<(), String> {
    Ok(())
}

pub fn get_autostart_os() -> bool {
    false
}
