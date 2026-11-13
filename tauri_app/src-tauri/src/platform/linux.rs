use crate::edid::parse_edid_bytes;
use crate::logger::log_warn;
use crate::types::EdidInfoDto;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::WebviewWindow;

static STEAM_BP_ACTIVE: AtomicBool = AtomicBool::new(false);
static INSTANCE_LOCK_FILE: OnceLock<Mutex<fs::File>> = OnceLock::new();

pub fn create_hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    Command::new(program)
}

fn has_command(cmd: &str) -> bool {
    use std::collections::HashMap;
    static CMD_CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CMD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(&cached) = guard.get(cmd) {
            return cached;
        }
    }
    let found = Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", cmd)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(cmd.to_string(), found);
    }
    found
}

/// Проверяет, запущена ли сессия внутри композитора SteamOS Gaming Mode (Gamescope)
fn is_gamescope_session() -> bool {
    if env::var("GAMESCOPE_WAYLAND_DISPLAY").is_ok() {
        return true;
    }
    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        let dl = desktop.to_lowercase();
        if dl.contains("gamescope") || dl.contains("steamos") {
            return true;
        }
    }
    Command::new("pgrep")
        .args(["-x", "gamescope"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Проверяет, запущен ли клиент Steam в системе
fn is_steam_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "steam"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Проверяет, находится ли сейчас окно Steam Big Picture / GamepadUI в активном фокусе
fn is_steam_big_picture_focused() -> bool {
    if is_gamescope_session() {
        return true;
    }
    if let Ok(out) = Command::new("sh")
        .args([
            "-c",
            "W=$(xdotool getwindowfocus 2>/dev/null); \
             if [ -n \"$W\" ]; then xprop -id \"$W\" WM_NAME WM_CLASS _NET_WM_STATE 2>/dev/null; fi",
        ])
        .output()
    {
        let txt = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if txt.contains("big picture")
            || txt.contains("steambigpicture")
            || (txt.contains("steam") && txt.contains("_net_wm_state_fullscreen"))
        {
            return true;
        }
    }
    STEAM_BP_ACTIVE.load(Ordering::Relaxed)
}

/// Открывает URI через клиент Steam в фоне без привязки к текущему терминалу
fn run_steam_uri(uri: &str) -> bool {
    if has_command("steam")
        && Command::new("steam")
            .arg(uri)
            .spawn()
            .is_ok()
    {
        return true;
    }
    Command::new("xdg-open").arg(uri).spawn().is_ok()
}

pub fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path.as_os_str())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn run_script(path: &Path, ext: &str, dir: &Path) -> Result<(), String> {
    let mut cmd = if ext.eq_ignore_ascii_case("sh") || ext.is_empty() {
        let mut c = Command::new("sh");
        c.arg(path.as_os_str());
        c
    } else {
        Command::new(path.as_os_str())
    };
    cmd.current_dir(dir)
        .spawn()
        .map_err(|e| {
            log_warn("scripts", &format!("Failed launching script {:?}: {}", path.file_name(), e));
            e.to_string()
        })?;
    Ok(())
}

pub fn check_single_instance_or_focus(_window_title: &str) -> bool {
    let lock_path = if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("cecdesk.lock")
    } else if let Ok(home) = env::var("HOME") {
        let dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("cecdesk");
        let _ = fs::create_dir_all(&dir);
        dir.join("app.lock")
    } else {
        let uid = unsafe { libc::geteuid() };
        PathBuf::from(format!("/tmp/cecdesk-{}.lock", uid))
    };

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
    {
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            if matches!(err.raw_os_error(), Some(libc::EWOULDBLOCK) | Some(libc::EAGAIN)) {
                let minimized = env::args().any(|a| a == "--minimized");
                if !minimized {
                    let _ = Command::new("xdotool")
                        .args(["search", "--name", "CecDesk", "windowactivate"])
                        .spawn();
                }
                return false;
            }
            log_warn("instance", &format!("flock unexpected error: {}", err));
            return true;
        }
        let _ = file.set_len(0);
        let _ = writeln!(file, "{}", std::process::id());
        let _ = INSTANCE_LOCK_FILE.set(Mutex::new(file));
    }
    true
}

pub fn put_pc_to_sleep() -> String {
    let _ = Command::new("systemctl").arg("suspend").spawn();
    "ПК переведен в режим сна (SteamOS / Linux)".to_string()
}

pub fn hibernate_pc() -> String {
    let _ = Command::new("systemctl").arg("hibernate").spawn();
    "ПК переведен в гибернацию (SteamOS / Linux)".to_string()
}

pub fn shutdown_pc() -> String {
    let _ = Command::new("systemctl").arg("poweroff").spawn();
    "Выключение ПК (SteamOS / Linux)".to_string()
}

pub fn move_mouse_cursor(dx: i32, dy: i32) {
    if has_command("ydotool") {
        let _ = Command::new("ydotool")
            .args(["mousemove", "--", &dx.to_string(), &dy.to_string()])
            .spawn();
    } else {
        let _ = Command::new("xdotool")
            .args(["mousemove_relative", "--", &dx.to_string(), &dy.to_string()])
            .spawn();
    }
}

pub fn trigger_mouse_click(is_left: bool) {
    if has_command("ydotool") {
        // ydotool click: 0xC0 = Left click (down+up), 0xC1 = Right click (down+up)
        let code = if is_left { "0xC0" } else { "0xC1" };
        let _ = Command::new("ydotool").args(["click", code]).spawn();
    } else {
        let btn = if is_left { "1" } else { "3" };
        let _ = Command::new("xdotool").args(["click", btn]).spawn();
    }
}

pub fn trigger_mouse_scroll(up: bool) {
    if has_command("ydotool") {
        let wheel = if up { "1" } else { "-1" };
        let _ = Command::new("ydotool")
            .args(["mousemove", "--wheel", "--", "0", wheel])
            .spawn();
    } else {
        let btn = if up { "4" } else { "5" };
        let _ = Command::new("xdotool").args(["click", btn]).spawn();
    }
}

pub fn trigger_double_click() {
    if has_command("ydotool") {
        let _ = Command::new("ydotool")
            .args(["click", "--repeat", "2", "--next-delay", "50", "0xC0"])
            .spawn();
    } else {
        let _ = Command::new("xdotool")
            .args(["click", "--repeat", "2", "--delay", "50", "1"])
            .spawn();
    }
}

pub fn jump_cursor_to_sector(sector: u8) -> (i32, i32) {
    let (pct_x, pct_y) = match sector {
        1 => (12, 14),
        2 => (50, 14),
        3 => (88, 14),
        4 => (12, 50),
        5 => (50, 50),
        6 => (88, 50),
        7 => (12, 86),
        8 => (50, 86),
        9 => (88, 86),
        _ => (50, 50),
    };
    let _ = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "W=$(xdotool getdisplaygeometry 2>/dev/null | awk '{{print $1}}'); \
             H=$(xdotool getdisplaygeometry 2>/dev/null | awk '{{print $2}}'); \
             W=${{W:-1920}}; H=${{H:-1080}}; \
             X=$((W * {} / 100)); Y=$((H * {} / 100)); \
             if command -v ydotool >/dev/null 2>&1; then ydotool mousemove --absolute -x $X -y $Y 2>/dev/null || xdotool mousemove $X $Y; \
             else xdotool mousemove $X $Y; fi",
            pct_x, pct_y
        ))
        .spawn();
    (pct_x, pct_y)
}

/// Отправка одиночной клавиши через ydotool (Linux evdev keycode) или xdotool (X11 keysym)
fn send_linux_key(evdev_code: Option<u16>, xdo_key: &str) {
    if let Some(code) = evdev_code {
        if has_command("ydotool") {
            let down = format!("{}:1", code);
            let up = format!("{}:0", code);
            if Command::new("ydotool")
                .args(["key", &down, &up])
                .spawn()
                .is_ok()
            {
                return;
            }
        }
    }
    let _ = Command::new("xdotool").args(["key", xdo_key]).spawn();
}

pub fn trigger_combo_key(mod_vk: u8, vk: u8) {
    // Сопоставление модификаторов (Windows VK -> Linux evdev / X11)
    let (mod_evdev, mod_name): (u16, &str) = match mod_vk {
        0x11 => (29, "ctrl"),   // KEY_LEFTCTRL
        0x12 => (56, "alt"),    // KEY_LEFTALT
        0x10 => (42, "shift"),  // KEY_LEFTSHIFT
        0x5B => (125, "super"), // KEY_LEFTMETA
        _ => (29, "ctrl"),
    };
    let (key_evdev, key_name): (u16, &str) = match vk {
        0x43 => (46, "c"),       // KEY_C
        0x56 => (47, "v"),       // KEY_V
        0x41 => (30, "a"),       // KEY_A
        0x44 => (32, "d"),       // KEY_D
        0x09 => (15, "Tab"),     // KEY_TAB
        0x73 => (62, "F4"),      // KEY_F4
        0x25 => (105, "Left"),   // KEY_LEFT
        0x27 => (106, "Right"),  // KEY_RIGHT
        0x26 => (103, "Up"),     // KEY_UP
        0x28 => (108, "Down"),   // KEY_DOWN
        _ => return,
    };
    if has_command("ydotool") {
        let m_dn = format!("{}:1", mod_evdev);
        let k_dn = format!("{}:1", key_evdev);
        let k_up = format!("{}:0", key_evdev);
        let m_up = format!("{}:0", mod_evdev);
        if Command::new("ydotool")
            .args(["key", &m_dn, &k_dn, &k_up, &m_up])
            .spawn()
            .is_ok()
        {
            return;
        }
    }
    let combo = format!("{}+{}", mod_name, key_name);
    let _ = Command::new("xdotool").args(["key", &combo]).spawn();
}

pub fn trigger_repeated_key(vk: u8, count: usize) {
    let (evdev, xdo_key) = match vk {
        0x25 => (Some(105u16), "Left"),
        0x27 => (Some(106u16), "Right"),
        0x26 => (Some(103u16), "Up"),
        0x28 => (Some(108u16), "Down"),
        _ => (None, "Right"),
    };
    for i in 0..count {
        send_linux_key(evdev, xdo_key);
        if i + 1 < count {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
}

pub fn trigger_os_key(action: &str) -> String {
    if action == "NONE" {
        return "—".to_string();
    }
    if action == "MOUSE_LCLICK" {
        trigger_mouse_click(true);
        return "ЛКМ (Клик)".to_string();
    }
    if action == "MOUSE_RCLICK" {
        trigger_mouse_click(false);
        return "ПКМ (Меню)".to_string();
    }
    if action == "STEAM_BIG_PICTURE" {
        return toggle_steam_big_picture();
    }
    if action == "STEAM_EXIT_TO_DESKTOP" {
        return exit_steam_big_picture_to_desktop();
    }
    if action == "STEAM_KEYBOARD" {
        let _ = run_steam_uri("steam://open/keyboard");
        return "SteamOS: Нативная экранная клавиатура Steam".to_string();
    }
    if action.starts_with("SCRIPT:") {
        return crate::custom_scripts::run_custom_script(action);
    }
    if action == "SHOW_KEYBOARD" {
        return "Экранная клавиатура".to_string();
    }
    if action == "WIN_D" {
        // Если мы внутри Big Picture — сворачиваем его на рабочий стол без убийства Steam
        if is_steam_big_picture_focused() {
            return exit_steam_big_picture_to_desktop();
        }
        trigger_combo_key(0x5B, 0x44); // Super + D
        return "Рабочий стол (Super+D)".to_string();
    }
    if action == "ALT_TAB" {
        trigger_combo_key(0x12, 0x09); // Alt + Tab
        return "Переключение задач (Alt+Tab)".to_string();
    }
    if action == "PC_SLEEP" {
        return put_pc_to_sleep();
    }
    if action == "PC_HIBERNATE" {
        return hibernate_pc();
    }
    if action == "PC_SHUTDOWN" {
        return shutdown_pc();
    }
    if action == "OPTO_POWER" {
        return "Импульс кнопки Power (Оптопара)".to_string();
    }

    let (evdev, xdo_key, desc): (Option<u16>, &str, &str) = match action {
        "LEFT" => (Some(105), "Left", "Влево"),
        "UP" => (Some(103), "Up", "Вверх"),
        "RIGHT" => (Some(106), "Right", "Вправо"),
        "DOWN" => (Some(108), "Down", "Вниз"),
        "ENTER" => (Some(28), "Return", "OK"),
        "ESC" => (Some(1), "Escape", "Esc"),
        "BACKSPACE" => (Some(14), "BackSpace", "Назад"),
        "BROWSER_BACK" => (Some(158), "XF86Back", "Назад в браузере"),
        "SPACE" => (Some(57), "space", "Пауза"),
        "VOL_MUTE" => (Some(113), "XF86AudioMute", "Mute"),
        "VOL_DOWN" => (Some(114), "XF86AudioLowerVolume", "Громкость −"),
        "VOL_UP" => (Some(115), "XF86AudioRaiseVolume", "Громкость +"),
        "MEDIA_NEXT" => (Some(163), "XF86AudioNext", "След. трек"),
        "MEDIA_PREV" => (Some(165), "XF86AudioPrev", "Пред. трек"),
        "MEDIA_STOP" => (Some(166), "XF86AudioStop", "Стоп"),
        "MEDIA_PLAY" => (Some(164), "XF86AudioPlay", "Play / Pause"),
        "PGUP" => (Some(104), "Prior", "PgUp"),
        "PGDN" => (Some(109), "Next", "PgDn"),
        "F11" => (Some(87), "F11", "Полный экран"),
        "TAB" => (Some(15), "Tab", "Tab"),
        "NUM_0" => (Some(11), "0", "0"),
        "NUM_1" => (Some(2), "1", "1"),
        "NUM_2" => (Some(3), "2", "2"),
        "NUM_3" => (Some(4), "3", "3"),
        "NUM_4" => (Some(5), "4", "4"),
        "NUM_5" => (Some(6), "5", "5"),
        "NUM_6" => (Some(7), "6", "6"),
        "NUM_7" => (Some(8), "7", "7"),
        "NUM_8" => (Some(9), "8", "8"),
        "NUM_9" => (Some(10), "9", "9"),
        _ => return "—".to_string(),
    };
    send_linux_key(evdev, xdo_key);
    desc.to_string()
}

pub fn emergency_kill_foreground_task() -> String {
    // Защищаем процессы самой программы, композитора SteamOS (gamescope/kwin/plasmashell) и клиента Steam от аварийного убийства
    let _ = Command::new("sh")
        .args([
            "-c",
            "PID=$(xdotool getwindowpid $(xdotool getwindowfocus) 2>/dev/null); \
             if [ -n \"$PID\" ]; then \
               COMM=$(cat /proc/$PID/comm 2>/dev/null | tr '[:upper:]' '[:lower:]'); \
               case \"$COMM\" in \
                 *cecdesk*|*hdmi_cec*|*steam*|*gamescope*|*kwin*|*plasmashell*|*systemd*|*xwayland*) exit 0 ;; \
                 *) kill -9 \"$PID\" 2>/dev/null ;; \
               esac; \
             fi",
        ])
        .spawn();
    "Зависшее приложение принудительно закрыто".to_string()
}

/// Выход из режима Steam Big Picture / Gaming Mode обратно на рабочий стол без вылета фоновой службы CecDesk
pub fn exit_steam_big_picture_to_desktop() -> String {
    STEAM_BP_ACTIVE.store(false, Ordering::Relaxed);

    // 1. Если мы в полноценной сессии SteamOS Gaming Mode (Gamescope) и есть утилита переключения сессии на рабочий стол KDE Plasma
    if is_gamescope_session() && has_command("steamos-session-select") {
        let _ = Command::new("steamos-session-select")
            .arg("plasma")
            .spawn();
        return "SteamOS: Выход из Gaming Mode на рабочий стол (Plasma)".to_string();
    }

    // 2. В Desktop Mode (или обычном Linux) штатно закрываем оверлей Big Picture и возвращаем окно Steam на рабочий стол
    let _ = run_steam_uri("steam://close/bigpicture");
    let _ = run_steam_uri("steam://open/main");
    "Steam: Выход из Big Picture на рабочий стол".to_string()
}

pub fn toggle_steam_big_picture() -> String {
    // Если сейчас открыт режим Big Picture / Gaming Mode — закрываем его и выходим на рабочий стол
    if is_steam_big_picture_focused() {
        return exit_steam_big_picture_to_desktop();
    }

    // Если мы на рабочем столе SteamOS и пользователь хочет вернуться в Gaming Mode (при отсутствии запущенного десктопного Big Picture)
    STEAM_BP_ACTIVE.store(true, Ordering::Relaxed);
    if !run_steam_uri("steam://open/bigpicture") {
        let _ = Command::new("steam").arg("-gamepadui").spawn();
    }
    "Steam: Открытие Big Picture / Gamepad UI".to_string()
}

pub fn detect_edid_os() -> EdidInfoDto {
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains("HDMI") || name.contains("-DP-") {
                let edid_path = entry.path().join("edid");
                if let Ok(bytes) = fs::read(&edid_path) {
                    if let Some(dto) = parse_edid_bytes(&bytes, &name) {
                        return dto;
                    }
                }
            }
        }
    }
    EdidInfoDto {
        pnp_id: "UNK".to_string(),
        monitor_id: "STEAMOS_HDMI".to_string(),
        physical_address: "1000".to_string(),
        hdmi_port_num: 1,
    }
}

pub fn switch_audio_endpoint(target: &str) -> Result<String, String> {
    // 1. Переключение через pactl (PipeWire-Pulse на SteamOS и Linux) + перенос всех активных потоков
    if let Ok(out) = Command::new("pactl").args(["list", "short", "sinks"]).output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let q = target.to_lowercase();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && line.to_lowercase().contains(&q) {
                let sink_id = parts[0];
                let sink_name = parts[1];
                let _ = Command::new("pactl")
                    .args(["set-default-sink", sink_name])
                    .status();
                if let Ok(inputs) = Command::new("pactl")
                    .args(["list", "short", "sink-inputs"])
                    .output()
                {
                    let in_str = String::from_utf8_lossy(&inputs.stdout);
                    for in_line in in_str.lines() {
                        if let Some(input_id) = in_line.split_whitespace().next() {
                            let _ = Command::new("pactl")
                                .args(["move-sink-input", input_id, sink_id])
                                .status();
                        }
                    }
                }
                return Ok(format!("Звук переключен на {}", sink_name));
            }
        }
    }
    // 2. Fallback через wpctl (WirePlumber / PipeWire)
    let _ = Command::new("wpctl").args(["set-default", target]).spawn();
    Ok(format!("Переключен на {}", target))
}

pub fn list_audio_endpoints() -> Vec<String> {
    let mut sinks = Vec::new();
    if let Ok(out) = Command::new("pactl").args(["list", "short", "sinks"]).output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                sinks.push(parts[1].to_string());
            }
        }
    }
    if sinks.is_empty() {
        sinks.push("alsa_output.hdmi-stereo".to_string());
        sinks.push("Default Audio Sink".to_string());
    }
    sinks
}

pub fn switch_display_mode(_extend: bool) {}

/// Прописывает X11-атомы оверлея Gamescope (`STEAM_OVERLAY`, `STEAM_NOTIFICATION`, `STEAM_INPUT_FOCUS`),
/// чтобы окна программы и HUD-уведомления отображались поверх Steam Big Picture / Gaming Mode.
pub fn set_gamescope_overlay_props(win: &WebviewWindow, is_hud_or_osk: bool) {
    let title = win.title().unwrap_or_else(|_| "CecDesk".to_string());
    let script = if is_hud_or_osk {
        "WID=$(xdotool search --name \"$1\" 2>/dev/null | head -n 1); \
         if [ -n \"$WID\" ]; then \
           xprop -id \"$WID\" -f STEAM_NOTIFICATION 32c -set STEAM_NOTIFICATION 1 2>/dev/null; \
           xprop -id \"$WID\" -f STEAM_OVERLAY 32c -set STEAM_OVERLAY 1 2>/dev/null; \
         fi"
    } else {
        "WID=$(xdotool search --name \"$1\" 2>/dev/null | head -n 1); \
         if [ -n \"$WID\" ]; then \
           xprop -id \"$WID\" -f STEAM_GAME 32c -set STEAM_GAME 769 2>/dev/null; \
           xprop -id \"$WID\" -f STEAM_OVERLAY 32c -set STEAM_OVERLAY 1 2>/dev/null; \
           xprop -id \"$WID\" -f STEAM_INPUT_FOCUS 32c -set STEAM_INPUT_FOCUS 1 2>/dev/null; \
         fi"
    };
    if let Err(e) = Command::new("sh")
        .args(["-c", script, "cec_xprop", &title])
        .spawn()
    {
        log_warn("gamescope", &format!("Failed applying xprop atoms: {}", e));
    }
}

pub fn try_native_osk() -> bool {
    if is_gamescope_session() || is_steam_running() {
        return run_steam_uri("steam://open/keyboard");
    }
    false
}

pub fn try_hide_native_osk() {
    if is_gamescope_session() || is_steam_running() {
        let _ = run_steam_uri("steam://close/keyboard");
    }
}

pub fn setup_osk_window(osk_win: &WebviewWindow) -> bool {
    // В SteamOS / Steam Big Picture есть собственная нативная экранная клавиатура с полной поддержкой пульта/геймпада.
    // Если запущен Steam или активна сессия Gamescope — вызываем нативную клавиатуру Steam (`steam://open/keyboard`)
    // и возвращаем false, чтобы не перекрывать экран второй клавиатурой.
    if is_gamescope_session() || is_steam_running() {
        if run_steam_uri("steam://open/keyboard") {
            return false;
        }
    }
    let _ = osk_win.show();
    let _ = osk_win.set_always_on_top(true);
    set_gamescope_overlay_props(osk_win, true);
    true
}

pub fn hide_osk_window(osk_win: &WebviewWindow) {
    if is_gamescope_session() || is_steam_running() {
        let _ = run_steam_uri("steam://close/keyboard");
    }
    let _ = osk_win.hide();
}

pub fn type_unicode_char(text: &str) {
    if has_command("wtype") {
        let _ = Command::new("wtype").arg(text).spawn();
    } else {
        let _ = Command::new("xdotool").args(["type", "--", text]).spawn();
    }
}

pub fn type_special_key(key: &str) {
    let (evdev, xdo_key) = match key {
        "BACKSPACE" => (Some(14u16), "BackSpace"),
        "ENTER" => (Some(28u16), "Return"),
        "SPACE" => (Some(57u16), "space"),
        "TAB" => (Some(15u16), "Tab"),
        "ESC" => (Some(1u16), "Escape"),
        "LEFT" => (Some(105u16), "Left"),
        "UP" => (Some(103u16), "Up"),
        "RIGHT" => (Some(106u16), "Right"),
        "DOWN" => (Some(108u16), "Down"),
        _ => return,
    };
    send_linux_key(evdev, xdo_key);
}

pub fn setup_hud_window(hud_win: &WebviewWindow) -> bool {
    let _ = hud_win.show();
    let _ = hud_win.set_always_on_top(true);
    set_gamescope_overlay_props(hud_win, true);
    true
}

pub fn hide_hud_window_os(hud_win: &WebviewWindow) {
    let _ = hud_win.hide();
}

fn autostart_desktop_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    Some(
        Path::new(&home)
            .join(".config")
            .join("autostart")
            .join("cecdesk.desktop"),
    )
}

fn systemd_user_service_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    Some(
        Path::new(&home)
            .join(".config")
            .join("systemd")
            .join("user")
            .join("cecdesk.service"),
    )
}

pub fn set_autostart_os(enabled: bool) -> Result<(), String> {
    let exe_path = env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    if let Some(desktop_file) = autostart_desktop_path() {
        if let Some(parent) = desktop_file.parent() {
            let _ = fs::remove_file(parent.join("hdmi-cec-studio.desktop"));
        }
        if enabled {
            if let Some(parent) = desktop_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=CecDesk\n\
                 Comment=Управление ПК и SteamOS пультом телевизора по HDMI-CEC\n\
                 Exec=\"{}\" --minimized\n\
                 Terminal=false\n\
                 Categories=Utility;HardwareSettings;\n\
                 X-GNOME-Autostart-enabled=true\n",
                exe_path
            );
            fs::write(&desktop_file, content).map_err(|e| e.to_string())?;
        } else if desktop_file.exists() {
            let _ = fs::remove_file(&desktop_file);
        }
    }

    // Создается независимый пользовательский сервис systemd (WantedBy=default.target, KillMode=process),
    // который НЕ завершается при переключении между Steam Big Picture (Gaming Mode) и рабочим столом (Desktop Mode).
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "hdmi-cec-studio.service"])
        .status();
    if let Some(service_file) = systemd_user_service_path() {
        if let Some(parent) = service_file.parent() {
            let _ = fs::remove_file(parent.join("hdmi-cec-studio.service"));
        }
        if enabled {
            if let Some(parent) = service_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let unit = format!(
                "[Unit]\n\
                 Description=CecDesk Background Daemon (SteamOS Gaming & Desktop Mode)\n\
                 After=pipewire.service\n\n\
                 [Service]\n\
                 Type=simple\n\
                 Environment=DISPLAY=:0\n\
                 Environment=GDK_BACKEND=x11\n\
                 ExecStart=\"{}\" --minimized\n\
                 KillMode=process\n\
                 Restart=always\n\
                 RestartSec=2\n\n\
                 [Install]\n\
                 WantedBy=default.target\n",
                exe_path
            );
            if fs::write(&service_file, unit).is_ok() {
                let _ = Command::new("systemctl")
                    .args(["--user", "daemon-reload"])
                    .status();
                let _ = Command::new("systemctl")
                    .args(["--user", "enable", "--now", "cecdesk.service"])
                    .status();
            }
        } else {
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "cecdesk.service"])
                .status();
            if service_file.exists() {
                let _ = fs::remove_file(&service_file);
            }
        }
    }

    Ok(())
}

pub fn get_autostart_os() -> bool {
    if let Some(p) = autostart_desktop_path() {
        if p.exists() {
            return true;
        }
    }
    if let Some(s) = systemd_user_service_path() {
        if s.exists() {
            return true;
        }
    }
    false
}
