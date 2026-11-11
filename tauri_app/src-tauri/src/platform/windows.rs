use crate::config::find_resource_dir;
use crate::edid::parse_edid_bytes;
use crate::logger::log_warn;
use crate::types::EdidInfoDto;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;
use tauri::WebviewWindow;

pub static PREV_OSK_WNDPROC: AtomicIsize = AtomicIsize::new(0);
pub static TARGET_WINDOW_HWND: AtomicIsize = AtomicIsize::new(0);

// ==================== ПРОЦЕССЫ И КОМАНДЫ ====================

pub fn create_hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(program);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

pub fn open_folder(path: &Path) -> Result<(), String> {
    create_hidden_command("explorer")
        .arg(path.as_os_str())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn run_script(path: &Path, ext: &str, dir: &Path) -> Result<(), String> {
    let script_str = path.to_string_lossy().to_string();
    let mut cmd = match ext {
        "bat" | "cmd" => {
            let mut c = create_hidden_command("cmd");
            c.args(["/c", &script_str]);
            c
        }
        "ps1" => {
            let mut c = create_hidden_command("powershell");
            c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &script_str]);
            c
        }
        "vbs" => {
            let mut c = create_hidden_command("wscript");
            c.args(["//nologo", &script_str]);
            c
        }
        "exe" => create_hidden_command(&script_str),
        _ => {
            let mut c = create_hidden_command("cmd");
            c.args(["/c", "start", "", &script_str]);
            c
        }
    };
    cmd.current_dir(dir);
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

static INSTANCE_MUTEX: AtomicIsize = AtomicIsize::new(0);

pub fn check_single_instance_or_focus(window_title_prefix: &str) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, BOOL, ERROR_ALREADY_EXISTS, HWND, LPARAM,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindow, SendMessageTimeoutW,
        SetForegroundWindow, ShowWindow, SMTO_ABORTIFHUNG, SW_RESTORE, SW_SHOW, WM_NULL,
    };

    extern "system" {
        fn CreateMutexW(
            lp_attributes: *const core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;
    }

    let mutex_name: Vec<u16> = "Local\\HDMI_CEC_Studio_SingleInstance_Mutex\0"
        .encode_utf16()
        .collect();
    let is_minimized = std::env::args().any(|a| a == "--minimized" || a == "-m");

    unsafe {
        let mut handle = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if handle.is_null() {
            return true;
        }

        if GetLastError() != ERROR_ALREADY_EXISTS {
            // Первый запуск: не тратим время на taskkill и sleep
            INSTANCE_MUTEX.store(handle as isize, Ordering::SeqCst);
            return true;
        }

        if is_minimized {
            CloseHandle(handle);
            return false;
        }

        let my_pid = std::process::id();
        struct EnumData {
            my_pid: u32,
            prefix: String,
            found_hwnd: HWND,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam as *mut EnumData);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 || pid == data.my_pid {
                return 1;
            }
            let mut buffer = [0u16; 512];
            let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            if len > 0 {
                let title = String::from_utf16_lossy(&buffer[..len as usize]);
                if title.starts_with(&data.prefix)
                    || title.contains("HDMI-CEC Studio")
                    || title.contains("Перехват пульта")
                {
                    data.found_hwnd = hwnd;
                    return 0; // stop enumeration
                }
            }
            1 // continue
        }

        let mut data = EnumData {
            my_pid,
            prefix: window_title_prefix.to_string(),
            found_hwnd: std::ptr::null_mut(),
        };

        EnumWindows(Some(enum_proc), &mut data as *mut EnumData as LPARAM);

        // Если окно существующего экземпляра живо и отвечает на сообщения — сразу выводим его на передний план
        if !data.found_hwnd.is_null() && IsWindow(data.found_hwnd) != 0 {
            let mut res: usize = 0;
            let alive = SendMessageTimeoutW(
                data.found_hwnd,
                WM_NULL,
                0,
                0,
                SMTO_ABORTIFHUNG,
                250,
                &mut res,
            );
            if alive != 0 {
                ShowWindow(data.found_hwnd, SW_SHOW);
                ShowWindow(data.found_hwnd, SW_RESTORE);
                SetForegroundWindow(data.found_hwnd);
                CloseHandle(handle);
                return false;
            }
        }

        // Только если процесс завис или остался зомби без окна — принудительно завершаем старую копию
        CloseHandle(handle);
        let filter = format!("PID ne {}", my_pid);
        let _ = create_hidden_command("taskkill")
            .args(["/F", "/IM", "HDMI_CEC_Studio.exe", "/FI", &filter])
            .output();
        std::thread::sleep(Duration::from_millis(120));

        handle = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if !handle.is_null() {
            INSTANCE_MUTEX.store(handle as isize, Ordering::SeqCst);
        }
    }
    true
}

// ==================== УПРАВЛЕНИЕ ПИТАНИЕМ ПК (СОН / ГИБЕРНАЦИЯ / ВЫКЛЮЧЕНИЕ) ====================

pub fn put_pc_to_sleep() -> String {
    let _ = create_hidden_command("rundll32.exe")
        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
        .spawn();
    "ПК переведен в режим сна".to_string()
}

pub fn hibernate_pc() -> String {
    let _ = create_hidden_command("shutdown")
        .args(["/h"])
        .spawn();
    "ПК переведен в гибернацию".to_string()
}

pub fn shutdown_pc() -> String {
    let _ = create_hidden_command("shutdown")
        .args(["/s", "/t", "3"])
        .spawn();
    "Выключение ПК через 3 сек".to_string()
}

// ==================== МЫШЬ И КЛАВИАТУРА ====================

pub fn move_mouse_cursor(dx: i32, dy: i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetCursorPos(pt.x + dx, pt.y + dy);
    }
}

pub fn trigger_mouse_click(is_left: bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP,
    };
    unsafe {
        if is_left {
            mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(15));
            mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        } else {
            mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(15));
            mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
        }
    }
}

pub fn trigger_mouse_scroll(up: bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{mouse_event, MOUSEEVENTF_WHEEL};
    unsafe {
        let delta: i32 = if up { 120 } else { -120 };
        mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta, 0);
    }
}

pub fn trigger_double_click() {
    trigger_mouse_click(true);
    std::thread::sleep(Duration::from_millis(60));
    trigger_mouse_click(true);
}

pub fn jump_cursor_to_sector(sector: u8) -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut left = 0i32;
        let mut top = 0i32;
        let mut width = 1920i32;
        let mut height = 1080i32;
        if !hmon.is_null() {
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hmon, &mut mi) != 0 {
                left = mi.rcWork.left;
                top = mi.rcWork.top;
                width = (mi.rcWork.right - mi.rcWork.left).max(640);
                height = (mi.rcWork.bottom - mi.rcWork.top).max(480);
            }
        }

        // Адаптивная 3x3 сетка с комфортными отступами от краев (12%, 50%, 88% по X и 14%, 50%, 86% по Y)
        // 1: Верх-Лево,   2: Верх-Центр,   3: Верх-Право
        // 4: Центр-Лево,  5: Центр экрана, 6: Центр-Право
        // 7: Низ-Лево,    8: Низ-Центр,    9: Низ-Право (в углу, но не впритык к краю)
        // 0: Возврат точно в центр
        let (pct_x, pct_y) = match sector {
            1 => (0.12f32, 0.14f32),
            2 => (0.50f32, 0.14f32),
            3 => (0.88f32, 0.14f32),
            4 => (0.12f32, 0.50f32),
            5 => (0.50f32, 0.50f32),
            6 => (0.88f32, 0.50f32),
            7 => (0.12f32, 0.86f32),
            8 => (0.50f32, 0.86f32),
            9 => (0.88f32, 0.86f32),
            _ => (0.50f32, 0.50f32),
        };
        let target_x = left + (width as f32 * pct_x) as i32;
        let target_y = top + (height as f32 * pct_y) as i32;
        SetCursorPos(target_x, target_y);
        (target_x, target_y)
    }
}

pub fn trigger_combo_key(mod_vk: u8, vk: u8) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
    unsafe {
        keybd_event(mod_vk, 0, 0, 0);
        std::thread::sleep(Duration::from_millis(10));
        keybd_event(vk, 0, 0, 0);
        std::thread::sleep(Duration::from_millis(15));
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
        std::thread::sleep(Duration::from_millis(10));
        keybd_event(mod_vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn trigger_repeated_key(vk: u8, count: usize) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
    unsafe {
        for i in 0..count {
            keybd_event(vk, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(12));
            keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
            if i + 1 < count {
                std::thread::sleep(Duration::from_millis(18));
            }
        }
    }
}

pub fn trigger_os_key(action: &str) -> String {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
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
        let _ = create_hidden_command("cmd")
            .args(["/c", "start", "", "steam://open/keyboard"])
            .spawn();
        return "Steam: Нативная экранная клавиатура".to_string();
    }
    if action.starts_with("SCRIPT:") {
        return crate::custom_scripts::run_custom_script(action);
    }
    if action == "SHOW_KEYBOARD" {
        return "Экранная клавиатура".to_string();
    }
    if action == "WIN_D" {
        trigger_combo_key(0x5B, 0x44); // Win + D
        return "Рабочий стол (Win+D)".to_string();
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
    let (vk, desc): (u8, &str) = match action {
        "LEFT" => (0x25, "Влево"),
        "UP" => (0x26, "Вверх"),
        "RIGHT" => (0x27, "Вправо"),
        "DOWN" => (0x28, "Вниз"),
        "ENTER" => (0x0D, "OK"),
        "ESC" => (0x1B, "Esc"),
        "BACKSPACE" => (0x08, "Назад"),
        "BROWSER_BACK" => (0xA6, "Назад в браузере"),
        "SPACE" => (0x20, "Пауза"),
        "VOL_MUTE" => (0xAD, "Mute"),
        "VOL_DOWN" => (0xAE, "Громкость −"),
        "VOL_UP" => (0xAF, "Громкость +"),
        "MEDIA_NEXT" => (0xB0, "След. трек"),
        "MEDIA_PREV" => (0xB1, "Пред. трек"),
        "MEDIA_STOP" => (0xB2, "Стоп"),
        "MEDIA_PLAY" => (0xB3, "Play / Pause"),
        "PGUP" => (0x21, "PgUp"),
        "PGDN" => (0x22, "PgDn"),
        "F11" => (0x7A, "Полный экран"),
        "TAB" => (0x09, "Tab"),
        "NUM_0" => (0x30, "0"),
        "NUM_1" => (0x31, "1"),
        "NUM_2" => (0x32, "2"),
        "NUM_3" => (0x33, "3"),
        "NUM_4" => (0x34, "4"),
        "NUM_5" => (0x35, "5"),
        "NUM_6" => (0x36, "6"),
        "NUM_7" => (0x37, "7"),
        "NUM_8" => (0x38, "8"),
        "NUM_9" => (0x39, "9"),
        _ => return "—".to_string(),
    };
    unsafe {
        keybd_event(vk, 0, 0, 0);
        std::thread::sleep(Duration::from_millis(15));
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
    desc.to_string()
}

pub fn emergency_kill_foreground_task() -> String {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let my_pid = std::process::id();
            if pid > 4 && pid != my_pid {
                let hquery = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if !hquery.is_null() {
                    let mut buf = [0u16; 512];
                    let mut len = buf.len() as u32;
                    let ok = QueryFullProcessImageNameW(hquery, 0, buf.as_mut_ptr(), &mut len);
                    CloseHandle(hquery);
                    if ok != 0 {
                        let proc_name =
                            String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
                        if proc_name.ends_with("explorer.exe")
                            || proc_name.ends_with("dwm.exe")
                            || proc_name.ends_with("taskhostw.exe")
                            || proc_name.ends_with("csrss.exe")
                            || proc_name.ends_with("lsass.exe")
                            || proc_name.ends_with("services.exe")
                            || proc_name.ends_with("hdmi_cec_studio.exe")
                        {
                            return "Защищенный процесс Windows (пропуск)".to_string();
                        }
                    }
                }

                let hproc = OpenProcess(PROCESS_TERMINATE, 0, pid);
                if !hproc.is_null() {
                    TerminateProcess(hproc, 1);
                    CloseHandle(hproc);
                    return format!("Зависший процесс PID {} закрыт", pid);
                }
            }
        }
    }
    "Не удалось закрыть процесс".to_string()
}

pub fn toggle_steam_big_picture() -> String {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid > 0 {
                let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if !handle.is_null() {
                    let mut buf = [0u16; 512];
                    let mut len = buf.len() as u32;
                    let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
                    CloseHandle(handle);
                    if ok != 0 {
                        let exe_name =
                            String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
                        if exe_name.ends_with("steam.exe")
                            || exe_name.ends_with("steamwebhelper.exe")
                        {
                            let _ = create_hidden_command("cmd")
                                .args(["/c", "start", "", "steam://close/bigpicture"])
                                .spawn();
                            let _ = create_hidden_command("cmd")
                                .args(["/c", "start", "", "steam://open/main"])
                                .spawn();
                            return "Steam: Выход из Big Picture на рабочий стол".to_string();
                        }
                    }
                }
            }
        }
    }

    let _ = create_hidden_command("cmd")
        .args(["/c", "start", "", "steam://open/bigpicture"])
        .spawn();
    "Steam Big Picture".to_string()
}

pub fn exit_steam_big_picture_to_desktop() -> String {
    let _ = create_hidden_command("cmd")
        .args(["/c", "start", "", "steam://close/bigpicture"])
        .spawn();
    let _ = create_hidden_command("cmd")
        .args(["/c", "start", "", "steam://open/main"])
        .spawn();
    "Steam: Выход из Big Picture на рабочий стол".to_string()
}

// ==================== EDID & ДИСПЛЕЙ & ЗВУК ====================

fn collect_active_display_ids() -> Vec<String> {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};

    const DISPLAY_DEVICE_ACTIVE: u32 = 0x0000_0001;
    const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x0000_0001;

    fn utf16_z_to_upper(buf: &[u16]) -> String {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len]).to_ascii_uppercase()
    }

    let mut ids = Vec::new();
    let mut i = 0u32;
    loop {
        let mut adapter: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        adapter.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        if unsafe { EnumDisplayDevicesW(std::ptr::null(), i, &mut adapter, 0) } == 0 {
            break;
        }
        i += 1;
        if (adapter.StateFlags & DISPLAY_DEVICE_ACTIVE) == 0 {
            continue;
        }

        let mut j = 0u32;
        loop {
            let mut mon: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
            mon.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if unsafe {
                EnumDisplayDevicesW(
                    adapter.DeviceName.as_ptr(),
                    j,
                    &mut mon,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
            } == 0
            {
                break;
            }
            j += 1;
            if (mon.StateFlags & DISPLAY_DEVICE_ACTIVE) != 0 {
                let dev_id = utf16_z_to_upper(&mon.DeviceID);
                if !dev_id.is_empty() {
                    ids.push(dev_id);
                }
            }
        }
    }
    ids
}

pub fn detect_edid_os() -> EdidInfoDto {
    use winreg::enums::*;
    use winreg::RegKey;

    let active_ids = collect_active_display_ids();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let display_path = r"SYSTEM\CurrentControlSet\Enum\DISPLAY";
    let mut active_found: Vec<EdidInfoDto> = Vec::new();
    let mut fallback_found: Vec<EdidInfoDto> = Vec::new();

    if let Ok(display_key) = hklm.open_subkey(display_path) {
        for mon_id in display_key.enum_keys().flatten() {
            if let Ok(mon_key) = display_key.open_subkey(&mon_id) {
                for inst_id in mon_key.enum_keys().flatten() {
                    let param_path = format!("{}\\Device Parameters", inst_id);
                    if let Ok(param_key) = mon_key.open_subkey(&param_path) {
                        if let Ok(reg_val) = param_key.get_raw_value("EDID") {
                            if let Some(dto) = parse_edid_bytes(&reg_val.bytes, &mon_id) {
                                let needle = format!(
                                    "#{}#{}#",
                                    mon_id.to_ascii_uppercase(),
                                    inst_id.to_ascii_uppercase()
                                );
                                if active_ids.iter().any(|id| id.contains(&needle)) {
                                    active_found.push(dto);
                                } else {
                                    fallback_found.push(dto);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(best_active) = active_found
        .iter()
        .find(|d| d.physical_address != "0000" && d.hdmi_port_num > 0)
    {
        return best_active.clone();
    }
    if let Some(first_active) = active_found.into_iter().next() {
        return first_active;
    }
    if let Some(first_fallback) = fallback_found.into_iter().next() {
        return first_fallback;
    }
    EdidInfoDto {
        pnp_id: "UNK".to_string(),
        monitor_id: "DEFAULT_HDMI1".to_string(),
        physical_address: "1000".to_string(),
        hdmi_port_num: 1,
    }
}

fn switch_audio_endpoint_in_dir(resource_dir: &Path, target: &str) -> Result<String, String> {
    let tool_path = resource_dir.join("bin").join("AudioTool.exe");
    if !tool_path.exists() {
        log_warn(
            "audio",
            &format!(
                "bin/AudioTool.exe not found in resource directory: {}",
                resource_dir.display()
            ),
        );
        return Err(
            "Не найдена утилита bin/AudioTool.exe рядом с приложением. Автопереключение звука недоступно."
                .to_string(),
        );
    }

    let output = create_hidden_command(&tool_path)
        .args(["set", target])
        .output()
        .map_err(|e| {
            log_warn("audio", &format!("AudioTool.exe invocation failed: {}", e));
            e.to_string()
        })?;
    let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        let err_txt = String::from_utf8_lossy(&output.stderr).trim().to_string();
        log_warn(
            "audio",
            &format!("AudioTool.exe exited {}: {}", output.status, err_txt),
        );
        return Err(if !err_txt.is_empty() {
            err_txt
        } else if !res.is_empty() {
            res
        } else {
            format!("AudioTool.exe завершилась с ошибкой ({})", output.status)
        });
    }
    Ok(if res.is_empty() {
        format!("Переключен на {}", target)
    } else {
        res
    })
}

pub fn switch_audio_endpoint(target: &str) -> Result<String, String> {
    switch_audio_endpoint_in_dir(&find_resource_dir(), target)
}

pub fn list_audio_endpoints() -> Vec<String> {
    let tool_path = find_resource_dir().join("bin").join("AudioTool.exe");
    if tool_path.exists() {
        match create_hidden_command(&tool_path).arg("list").output() {
            Ok(output) => {
                let txt = String::from_utf8_lossy(&output.stdout);
                return txt
                    .lines()
                    .filter_map(|l| {
                        let parts: Vec<&str> = l.split('\t').collect();
                        if parts.len() >= 2 {
                            Some(parts[1].trim().to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
            }
            Err(e) => {
                log_warn("audio", &format!("Failed listing audio endpoints via AudioTool.exe: {}", e));
            }
        }
    }
    Vec::new()
}

pub fn switch_display_mode(extend: bool) {
    let arg = if extend { "/extend" } else { "/internal" };
    let _ = create_hidden_command("DisplaySwitch.exe")
        .arg(arg)
        .spawn();
}

// ==================== OSK (ЭКРАННАЯ КЛАВИАТУРА) ====================

unsafe extern "system" fn osk_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CallWindowProcW, DefWindowProcW};
    const WM_MOUSEACTIVATE: u32 = 0x0021;
    const MA_NOACTIVATE: isize = 3;

    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE;
    }

    let prev = PREV_OSK_WNDPROC.load(Ordering::SeqCst);
    if prev != 0 {
        CallWindowProcW(
            std::mem::transmute::<
                isize,
                Option<unsafe extern "system" fn(*mut std::ffi::c_void, u32, usize, isize) -> isize>,
            >(prev),
            hwnd,
            msg,
            wparam,
            lparam,
        )
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

pub fn ensure_target_focus() {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };
    extern "system" {
        fn AttachThreadInput(idattach: u32, idattachto: u32, fattach: i32) -> i32;
    }

    let target = TARGET_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if !target.is_null() {
        unsafe {
            if IsWindow(target) == 0 {
                TARGET_WINDOW_HWND.store(0, Ordering::SeqCst);
                return;
            }
            let cur_fg = GetForegroundWindow();
            if cur_fg != target {
                let cur_thread = GetCurrentThreadId();
                let fg_thread = GetWindowThreadProcessId(cur_fg, std::ptr::null_mut());
                let target_thread = GetWindowThreadProcessId(target, std::ptr::null_mut());
                if target_thread > 0 {
                    AttachThreadInput(cur_thread, target_thread, 1);
                    AttachThreadInput(cur_thread, fg_thread, 1);
                    SetForegroundWindow(target);
                    AttachThreadInput(cur_thread, target_thread, 0);
                    AttachThreadInput(cur_thread, fg_thread, 0);
                }
            }
        }
    }
}

pub fn try_native_osk() -> bool {
    false
}

pub fn try_hide_native_osk() {}

pub fn setup_osk_window(osk_win: &WebviewWindow) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, GWL_WNDPROC, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    };

    let osk_w = 1040i32;
    let osk_h = 380i32;
    let prev_fg = unsafe { GetForegroundWindow() };

    let mut pos_x = (1920 - osk_w) / 2;
    let mut pos_y = 1080 - osk_h - 28;
    unsafe {
        let hmon = MonitorFromWindow(
            if !prev_fg.is_null() {
                prev_fg
            } else {
                std::ptr::null_mut()
            },
            MONITOR_DEFAULTTONEAREST,
        );
        if !hmon.is_null() {
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hmon, &mut mi) != 0 {
                let work_w = mi.rcWork.right - mi.rcWork.left;
                pos_x = mi.rcWork.left + (work_w - osk_w) / 2;
                pos_y = mi.rcWork.bottom - osk_h - 28;
            }
        }
    }

    if let Ok(raw_hwnd) = osk_win.hwnd() {
        let hwnd = raw_hwnd.0 as HWND;
        if prev_fg != hwnd && !prev_fg.is_null() {
            TARGET_WINDOW_HWND.store(prev_fg as isize, Ordering::SeqCst);
        }

        let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let new_style = ex_style
            | (WS_EX_NOACTIVATE as isize)
            | (WS_EX_TOPMOST as isize)
            | (WS_EX_TOOLWINDOW as isize);
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

            if PREV_OSK_WNDPROC.load(Ordering::SeqCst) == 0 {
                let old_proc =
                    SetWindowLongPtrW(hwnd, GWL_WNDPROC, osk_subclass_proc as *const () as isize);
                PREV_OSK_WNDPROC.store(old_proc, Ordering::SeqCst);
            }

            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                pos_x,
                pos_y,
                osk_w,
                osk_h,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
        }
        return true;
    }
    false
}

pub fn hide_osk_window(osk_win: &WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_NOTOPMOST, SWP_HIDEWINDOW, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    };
    if let Ok(raw_hwnd) = osk_win.hwnd() {
        let hwnd = raw_hwnd.0 as HWND;
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_HIDEWINDOW,
            );
        }
    }
    TARGET_WINDOW_HWND.store(0, Ordering::SeqCst);
}

pub fn type_unicode_char(text: &str) {
    ensure_target_focus();
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };
    for ch in text.chars() {
        let mut utf16_buf = [0u16; 2];
        let encoded = ch.encode_utf16(&mut utf16_buf);
        for &mut code in encoded {
            let mut inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: 0,
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: 0,
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            unsafe {
                SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        }
    }
}

pub fn type_special_key(key: &str) {
    ensure_target_focus();
    let vk: u8 = match key {
        "BACKSPACE" => 0x08,
        "ENTER" => 0x0D,
        "SPACE" => 0x20,
        "TAB" => 0x09,
        "ESC" => 0x1B,
        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,
        _ => return,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
    unsafe {
        keybd_event(vk, 0, 0, 0);
        std::thread::sleep(Duration::from_millis(15));
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn setup_hud_window(hud_win: &WebviewWindow) -> bool {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    };

    let _ = hud_win.show();
    let _ = hud_win.unminimize();

    let hud_w = 480i32;
    let hud_h = 170i32;

    // Сначала пробуем монитор по текущему положению курсора, иначе берем Primary (ТВ 1920x1080)
    let mut pos_x = (1920 - hud_w) / 2;
    let mut pos_y = 36i32;
    unsafe {
        let mut pt: POINT = std::mem::zeroed();
        let has_cursor = GetCursorPos(&mut pt) != 0;
        let hmon = if has_cursor {
            MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY)
        } else {
            let prev_fg = GetForegroundWindow();
            if !prev_fg.is_null() {
                MonitorFromWindow(prev_fg, MONITOR_DEFAULTTOPRIMARY)
            } else {
                std::ptr::null_mut()
            }
        };

        if !hmon.is_null() {
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hmon, &mut mi) != 0 {
                let work_w = mi.rcWork.right - mi.rcWork.left;
                pos_x = mi.rcWork.left + (work_w - hud_w) / 2;
                pos_y = mi.rcWork.top + 36;
            }
        }
    }

    if let Ok(raw_hwnd) = hud_win.hwnd() {
        let hwnd = raw_hwnd.0 as HWND;
        let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        // Обязательно СОХРАНЯЕМ WS_EX_LAYERED для корректного рендеринга прозрачности WebView2
        // и добавляем WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_EX_TOOLWINDOW и WS_EX_TRANSPARENT (0x20)
        let new_style = ex_style
            | (WS_EX_LAYERED as isize)
            | (WS_EX_NOACTIVATE as isize)
            | (WS_EX_TOPMOST as isize)
            | (WS_EX_TOOLWINDOW as isize)
            | 0x00000020_isize /* WS_EX_TRANSPARENT (click-through) */;
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                pos_x,
                pos_y,
                hud_w,
                hud_h,
                SWP_SHOWWINDOW | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        return true;
    }
    false
}

pub fn hide_hud_window_os(hud_win: &WebviewWindow) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_NOTOPMOST, SWP_HIDEWINDOW, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    };
    if let Ok(raw_hwnd) = hud_win.hwnd() {
        let hwnd = raw_hwnd.0 as HWND;
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_HIDEWINDOW,
            );
        }
    }
    let _ = hud_win.hide();
}

pub fn set_gamescope_overlay_props(_win: &WebviewWindow, _is_overlay: bool) {}

pub fn set_autostart_os(enabled: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_SET_VALUE | KEY_QUERY_VALUE,
        )
        .map_err(|e| e.to_string())?;

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let val_str = format!("\"{}\" --minimized", exe_path.to_string_lossy());

    if enabled {
        run_key
            .set_value("HDMI_CEC_Studio", &val_str)
            .map_err(|e| e.to_string())?;
    } else {
        let _ = run_key.delete_value("HDMI_CEC_Studio");
    }
    Ok(())
}

pub fn get_autostart_os() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
        run_key.get_value::<String, _>("HDMI_CEC_Studio").is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_audio_endpoint_missing_binary_returns_err() {
        let tmp = std::env::temp_dir().join(format!("hdmi_cec_audio_missing_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let res = switch_audio_endpoint_in_dir(&tmp, "Speakers");
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
