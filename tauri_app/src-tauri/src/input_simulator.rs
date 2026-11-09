use crate::platform;
use std::ffi::OsStr;
use std::process::Command;

pub fn create_hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    platform::create_hidden_command(program)
}

pub fn move_mouse_cursor(dx: i32, dy: i32) {
    platform::move_mouse_cursor(dx, dy);
}

pub fn trigger_mouse_click(is_left: bool) {
    platform::trigger_mouse_click(is_left);
}

pub fn trigger_mouse_scroll(up: bool) {
    platform::trigger_mouse_scroll(up);
}

pub fn trigger_double_click() {
    platform::trigger_double_click();
}

pub fn jump_cursor_to_sector(sector: u8) -> (i32, i32) {
    platform::jump_cursor_to_sector(sector)
}

#[allow(dead_code)]
pub fn trigger_combo_key(mod_vk: u8, vk: u8) {
    platform::trigger_combo_key(mod_vk, vk);
}

pub fn trigger_repeated_key(vk: u8, count: usize) {
    platform::trigger_repeated_key(vk, count);
}

pub fn emergency_kill_foreground_task() -> String {
    platform::emergency_kill_foreground_task()
}

#[allow(dead_code)]
pub fn toggle_steam_big_picture() -> String {
    platform::toggle_steam_big_picture()
}

pub fn trigger_os_key_internal(action: &str) -> String {
    platform::trigger_os_key(action)
}
