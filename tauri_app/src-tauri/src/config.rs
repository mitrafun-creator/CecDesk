use crate::logger::{log_info, log_session, log_warn, now_iso8601, redact_sensitive};
use crate::types::AppConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PendingCrashReport {
    pub has_issue: bool,
    pub has_panic_file: bool,
    pub dirty_shutdown: bool,
    pub panic_summary: String,
    pub detected_at: String,
}

static MIGRATION_DONE: OnceLock<()> = OnceLock::new();
static CRASH_STATE: OnceLock<Mutex<PendingCrashReport>> = OnceLock::new();

/// Находит каталог со встроенными ресурсами (tv_profiles/, bin/AudioTool.exe, bin/firmware/).
pub fn find_resource_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if exe_dir.join("tv_profiles").exists() || exe_dir.join("bin").exists() {
                return exe_dir.to_path_buf();
            }
            if let Some(parent) = exe_dir.parent() {
                if parent.join("tv_profiles").exists() {
                    return parent.to_path_buf();
                }
            }
        }
    }
    let candidates = [
        PathBuf::from("."),
        PathBuf::from(".."),
        PathBuf::from("../.."),
    ];
    for c in candidates {
        if c.join("tv_profiles").exists() {
            return c;
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

pub fn is_portable_mode() -> bool {
    if let Ok(val) = std::env::var("CEC_PORTABLE") {
        let v = val.trim();
        if v == "1" || v.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    if std::env::args().any(|a| a == "--portable") {
        return true;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if exe_dir.join("portable.txt").exists() || exe_dir.join(".portable").exists() {
                return true;
            }
        }
    }
    let res_dir = find_resource_dir();
    res_dir.join("portable.txt").exists() || res_dir.join(".portable").exists()
}

pub struct DataDirEnv<'a> {
    pub resource_dir: &'a Path,
    pub exe_dir: Option<&'a Path>,
    pub local_app_data: Option<&'a str>,
    pub app_data: Option<&'a str>,
    pub xdg_data_home: Option<&'a str>,
    pub home: Option<&'a str>,
    pub is_windows: bool,
}

pub fn resolve_data_dir_with_env(portable: bool, env: &DataDirEnv<'_>) -> PathBuf {
    if portable {
        if let Some(ed) = env.exe_dir {
            if ed.join("portable.txt").exists() || ed.join(".portable").exists() {
                return ed.to_path_buf();
            }
        }
        return env.resource_dir.to_path_buf();
    }

    if env.is_windows {
        if let Some(local) = env.local_app_data.filter(|s| !s.trim().is_empty()) {
            return PathBuf::from(local).join("CecDesk");
        }
        if let Some(roaming) = env.app_data.filter(|s| !s.trim().is_empty()) {
            return PathBuf::from(roaming).join("CecDesk");
        }
    } else {
        if let Some(xdg) = env.xdg_data_home.filter(|s| !s.trim().is_empty()) {
            return PathBuf::from(xdg).join("cecdesk");
        }
        if let Some(h) = env.home.filter(|s| !s.trim().is_empty()) {
            return PathBuf::from(h)
                .join(".local")
                .join("share")
                .join("cecdesk");
        }
    }

    env.resource_dir.to_path_buf()
}

/// Возвращает каталог записываемых пользовательских данных (%LOCALAPPDATA%\CecDesk или ~/.local/share/cecdesk, либо рядом с exe в portable-режиме).
pub fn find_data_dir() -> PathBuf {
    let resource_dir = find_resource_dir();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let local_app = std::env::var("LOCALAPPDATA").ok();
    let app_data = std::env::var("APPDATA").ok();
    let xdg_data = std::env::var("XDG_DATA_HOME").ok();
    let home = std::env::var("HOME").ok();

    let env = DataDirEnv {
        resource_dir: &resource_dir,
        exe_dir: exe_dir.as_deref(),
        local_app_data: local_app.as_deref(),
        app_data: app_data.as_deref(),
        xdg_data_home: xdg_data.as_deref(),
        home: home.as_deref(),
        is_windows: cfg!(target_os = "windows"),
    };
    let dir = resolve_data_dir_with_env(is_portable_mode(), &env);
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn data_storage_mode_label() -> &'static str {
    if is_portable_mode() {
        "Portable"
    } else if cfg!(target_os = "windows") {
        "AppData (Local)"
    } else {
        "XDG Data"
    }
}

/// Каталог встроенных read-only профилей ТВ.
pub fn find_bundled_profiles_dir() -> PathBuf {
    find_resource_dir().join("tv_profiles")
}

/// Каталог записываемых пользовательских / синхронизированных профилей ТВ.
pub fn find_user_profiles_dir() -> PathBuf {
    let dir = find_data_dir().join("tv_profiles");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_logs_dir() -> PathBuf {
    let dir = find_data_dir().join("logs");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_scripts_dir() -> PathBuf {
    let dir = find_data_dir().join("scripts");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_reports_dir() -> PathBuf {
    let dir = find_data_dir().join("reports");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_config_path() -> PathBuf {
    find_data_dir().join("cecdesk_config.json")
}

fn copy_scripts_if_missing(src_scripts: &Path, dst_scripts: &Path) -> usize {
    let mut copied = 0usize;
    if !src_scripts.is_dir() {
        return 0;
    }
    let _ = fs::create_dir_all(dst_scripts);
    if let Ok(entries) = fs::read_dir(src_scripts) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            if src_path.is_file() {
                if let Some(name) = src_path.file_name() {
                    if name == ".gitkeep" {
                        continue;
                    }
                    let dst_path = dst_scripts.join(name);
                    if !dst_path.exists() && fs::copy(&src_path, &dst_path).is_ok() {
                        copied += 1;
                    }
                }
            }
        }
    }
    copied
}

/// Однократная неразрушающая миграция legacy config/scripts из каталога установки в `data_dir`.
pub fn migrate_legacy_data_in_dirs(
    legacy_dir: &Path,
    extra_legacy_dir: Option<&Path>,
    data_dir: &Path,
) -> (bool, usize) {
    if legacy_dir == data_dir {
        return (false, 0);
    }
    let _ = fs::create_dir_all(data_dir);

    let mut config_migrated = false;
    let dst_cfg = data_dir.join("cecdesk_config.json");
    if !dst_cfg.exists() {
        let mut candidates = vec![
            legacy_dir.join("cecdesk_config.json"),
            legacy_dir.join("cec_studio_config.json"),
        ];
        if let Some(extra) = extra_legacy_dir {
            candidates.push(extra.join("cecdesk_config.json"));
            candidates.push(extra.join("cec_studio_config.json"));
        }
        for src_cfg in candidates {
            if src_cfg.is_file() && fs::copy(&src_cfg, &dst_cfg).is_ok() {
                config_migrated = true;
                break;
            }
        }
    }

    let mut scripts_copied = copy_scripts_if_missing(&legacy_dir.join("scripts"), &data_dir.join("scripts"));
    if let Some(extra) = extra_legacy_dir {
        scripts_copied += copy_scripts_if_missing(&extra.join("scripts"), &data_dir.join("scripts"));
    }

    (config_migrated, scripts_copied)
}

pub fn migrate_legacy_data_if_needed() {
    MIGRATION_DONE.get_or_init(|| {
        let resource_dir = find_resource_dir();
        let data_dir = find_data_dir();
        let mut extra_dirs: Vec<PathBuf> = Vec::new();
        if cfg!(target_os = "windows") {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                extra_dirs.push(PathBuf::from(local).join("HDMI-CEC Studio"));
            }
        } else {
            if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
                extra_dirs.push(PathBuf::from(xdg).join("hdmi-cec-studio"));
            }
            if let Ok(home) = std::env::var("HOME") {
                extra_dirs.push(PathBuf::from(&home).join(".local").join("share").join("hdmi-cec-studio"));
                extra_dirs.push(PathBuf::from(&home).join(".local").join("share").join("hdmi_cec_studio"));
            }
        }
        let (mut cfg_migrated, mut scripts_copied) =
            migrate_legacy_data_in_dirs(&resource_dir, None, &data_dir);
        for extra in &extra_dirs {
            let (c, s) = migrate_legacy_data_in_dirs(&resource_dir, Some(extra), &data_dir);
            cfg_migrated = cfg_migrated || c;
            scripts_copied += s;
        }
        if cfg_migrated || scripts_copied > 0 {
            log_info(
                "config",
                &format!(
                    "Migrated legacy user data into {} (config={}, scripts={})",
                    data_storage_mode_label(),
                    cfg_migrated,
                    scripts_copied
                ),
            );
        }
    });
}

fn fill_random_bytes(buf: &mut [u8]) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "bcrypt")]
        extern "system" {
            fn BCryptGenRandom(
                h_algorithm: *mut core::ffi::c_void,
                pb_buffer: *mut u8,
                cb_buffer: u32,
                dw_flags: u32,
            ) -> i32;
        }
        const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
        let status = unsafe {
            BCryptGenRandom(
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                buf.len() as u32,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
        };
        if status == 0 {
            return;
        }
    }

    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = fs::File::open("/dev/urandom") {
            if f.read_exact(buf).is_ok() {
                return;
            }
        }
    }

    // Fallback SplitMix64 entropy mixer
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ ((std::process::id() as u64) << 32)
        ^ (buf.as_ptr() as usize as u64);
    for chunk in buf.chunks_mut(8) {
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let bytes = z.to_le_bytes();
        for (dst, src) in chunk.iter_mut().zip(bytes.iter()) {
            *dst = *src;
        }
    }
}

/// Генерирует анонимный случайный UUID v4 без привязки к аппаратному обеспечению или имени пользователя.
pub fn generate_install_id() -> String {
    let mut b = [0u8; 16];
    fill_random_bytes(&mut b);
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// Атомарное сохранение текстового содержимого через уникальный временный файл.
pub fn save_atomic(target_path: &Path, content: &str) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TMP_SEQ: AtomicU64 = AtomicU64::new(1);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Не удалось создать каталог {}: {}", parent.display(), e))?;
    }
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config");
    let tmp_path = target_path.with_file_name(format!("{}.{}.{}.tmp", file_name, pid, seq));
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Не удалось записать временный файл: {}", e))?;
    if let Err(_e) = fs::rename(&tmp_path, target_path) {
        let _ = fs::remove_file(target_path);
        fs::rename(&tmp_path, target_path)
            .or_else(|_| fs::write(target_path, content))
            .map_err(|e| format!("Не удалось атомарно переименовать файл: {}", e))?;
    }
    let _ = fs::remove_file(&tmp_path);
    Ok(())
}

/// Ротация файлов сессионных логов: сохранять только последние `max_keep` файлов.
pub fn rotate_session_logs(logs_dir: &Path, max_keep: usize) {
    if let Ok(entries) = fs::read_dir(logs_dir) {
        let mut session_files: Vec<(PathBuf, SystemTime)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("cec_session_") && name.ends_with(".log") {
                    let mtime = entry.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH);
                    session_files.push((path, mtime));
                }
            }
        }
        if session_files.len() > max_keep {
            session_files.sort_by_key(|a| std::cmp::Reverse(a.1));
            for (p, _) in session_files.into_iter().skip(max_keep) {
                let _ = fs::remove_file(p);
            }
        }
    }
}

pub fn load_or_create_config() -> AppConfig {
    migrate_legacy_data_if_needed();
    let p = find_config_path();
    let mut def = AppConfig::default();

    if let Ok(txt) = fs::read_to_string(&p) {
        match serde_json::from_str::<AppConfig>(&txt) {
            Ok(mut cfg) => {
                let mut changed = false;
                for (code, val) in cfg.key_mappings.iter_mut() {
                    if val == "MEME" {
                        *val = if code == "0x20" {
                            "NUM_0".to_string()
                        } else {
                            "NONE".to_string()
                        };
                        changed = true;
                    }
                }
                for (k, v) in &def.key_mappings {
                    if !cfg.key_mappings.contains_key(k) {
                        cfg.key_mappings.insert(k.clone(), v.clone());
                        changed = true;
                    }
                }
                if cfg.install_id.trim().is_empty() {
                    cfg.install_id = generate_install_id();
                    changed = true;
                }
                if changed {
                    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                        if let Err(e) = save_atomic(&p, &json) {
                            log_warn("config", &format!("Failed to save migrated config: {}", e));
                        }
                    }
                }
                return cfg;
            }
            Err(e) => {
                log_warn(
                    "config",
                    &format!("Config JSON invalid, backing up and restoring defaults: {}", e),
                );
                let backup = p.with_extension("corrupted.json");
                let _ = fs::copy(&p, backup);
            }
        }
    }

    def.install_id = generate_install_id();
    if let Ok(json) = serde_json::to_string_pretty(&def) {
        if let Err(e) = save_atomic(&p, &json) {
            log_warn("config", &format!("Failed to create initial config: {}", e));
        }
    }
    def
}

pub fn write_log_to_disk(session_path: &Path, latest_path: &Path, msg: &str) {
    log_session(session_path, latest_path, msg);
}

fn session_active_path() -> PathBuf {
    find_data_dir().join("session.active")
}

/// Проверяет наличие `panic.txt` / незакрытой прошлой сессии `session.active` и выставляет маркер текущей сессии.
pub fn init_crash_recovery_on_startup() -> PendingCrashReport {
    let logs_dir = find_logs_dir();
    let panic_path = logs_dir.join("panic.txt");
    let sentinel = session_active_path();

    let has_panic = panic_path.is_file();
    let dirty_shutdown = sentinel.is_file();

    let panic_summary = if has_panic {
        fs::read_to_string(&panic_path)
            .map(|s| redact_sensitive(&s).chars().take(480).collect())
            .unwrap_or_else(|_| "Обнаружен отчёт о прошлом аварийном завершении (panic.txt)".to_string())
    } else {
        String::new()
    };

    let report = PendingCrashReport {
        has_issue: has_panic || dirty_shutdown,
        has_panic_file: has_panic,
        dirty_shutdown,
        panic_summary,
        detected_at: now_iso8601(),
    };

    let state_mutex = CRASH_STATE.get_or_init(|| Mutex::new(PendingCrashReport::default()));
    if let Ok(mut guard) = state_mutex.lock() {
        *guard = report.clone();
    }

    let marker_content = format!("pid={}\nstarted_at={}\n", std::process::id(), now_iso8601());
    let _ = fs::write(&sentinel, marker_content);

    if report.has_issue {
        log_warn(
            "recovery",
            &format!(
                "Detected previous abnormal exit (panic_file={}, dirty_shutdown={})",
                has_panic, dirty_shutdown
            ),
        );
    }

    report
}

pub fn get_pending_crash_report_state() -> PendingCrashReport {
    let state_mutex = CRASH_STATE.get_or_init(|| Mutex::new(PendingCrashReport::default()));
    match state_mutex.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    }
}

pub fn dismiss_crash_report_state() -> Result<(), String> {
    let state_mutex = CRASH_STATE.get_or_init(|| Mutex::new(PendingCrashReport::default()));
    if let Ok(mut g) = state_mutex.lock() {
        g.has_issue = false;
    }
    let logs_dir = find_logs_dir();
    let panic_path = logs_dir.join("panic.txt");
    if panic_path.is_file() {
        let ack_path = logs_dir.join("panic_last.txt");
        let _ = fs::remove_file(&ack_path);
        fs::rename(&panic_path, &ack_path).or_else(|_| {
            fs::copy(&panic_path, &ack_path).and_then(|_| fs::remove_file(&panic_path))
        })
        .map_err(|e| format!("Не удалось архивировать panic.txt: {}", e))?;
    }
    Ok(())
}

pub fn clear_session_active_marker() {
    let sentinel = session_active_path();
    let _ = fs::remove_file(sentinel);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meme_migration() {
        let mut cfg = AppConfig::default();
        cfg.key_mappings.insert("0x20".to_string(), "MEME".to_string());
        cfg.key_mappings.insert("0x72".to_string(), "MEME".to_string());

        for (code, val) in cfg.key_mappings.iter_mut() {
            if val == "MEME" {
                *val = if code == "0x20" { "NUM_0".to_string() } else { "NONE".to_string() };
            }
        }

        assert_eq!(cfg.key_mappings.get("0x20").unwrap(), "NUM_0");
        assert_eq!(cfg.key_mappings.get("0x72").unwrap(), "NONE");
    }

    #[test]
    fn test_legacy_config_deserialization_and_clean_defaults() {
        let legacy_json = r#"{
            "active_profile_id": "samsung_qled",
            "osd_name": "HTPC",
            "remote_to_os_enabled": true
        }"#;

        let parsed: AppConfig =
            serde_json::from_str(legacy_json).expect("Failed to deserialize legacy config");
        assert_eq!(parsed.active_profile_id, "samsung_qled");
        assert_eq!(parsed.osd_name, "HTPC");
        assert_eq!(parsed.language, "ru");
        assert!(parsed.custom_profile_names.is_empty());
        assert!(!parsed.telemetry_opt_in);
        assert!(!parsed.auto_audio_switch);
        assert!(parsed.tv_audio_device.is_empty());
        assert!(parsed.fallback_audio_device.is_empty());
        assert!(!parsed.emergency_kill_enabled);
    }

    #[test]
    fn test_resolve_data_dir_modes() {
        let res = Path::new("/opt/hdmi_cec_studio");
        let exe = Path::new("/opt/hdmi_cec_studio");

        assert_eq!(
            resolve_data_dir_with_env(
                false,
                &DataDirEnv {
                    resource_dir: Path::new("C:\\Program Files\\HDMI-CEC Studio"),
                    exe_dir: None,
                    local_app_data: Some("C:\\Users\\Test\\AppData\\Local"),
                    app_data: None,
                    xdg_data_home: None,
                    home: None,
                    is_windows: true,
                }
            ),
            PathBuf::from("C:\\Users\\Test\\AppData\\Local\\CecDesk")
        );

        assert_eq!(
            resolve_data_dir_with_env(
                false,
                &DataDirEnv {
                    resource_dir: res,
                    exe_dir: Some(exe),
                    local_app_data: None,
                    app_data: None,
                    xdg_data_home: Some("/home/deck/.data"),
                    home: Some("/home/deck"),
                    is_windows: false,
                }
            ),
            PathBuf::from("/home/deck/.data/cecdesk")
        );

        assert_eq!(
            resolve_data_dir_with_env(
                false,
                &DataDirEnv {
                    resource_dir: res,
                    exe_dir: Some(exe),
                    local_app_data: None,
                    app_data: None,
                    xdg_data_home: None,
                    home: Some("/home/deck"),
                    is_windows: false,
                }
            ),
            PathBuf::from("/home/deck/.local/share/cecdesk")
        );

        assert_eq!(
            resolve_data_dir_with_env(
                true,
                &DataDirEnv {
                    resource_dir: res,
                    exe_dir: Some(exe),
                    local_app_data: None,
                    app_data: None,
                    xdg_data_home: None,
                    home: None,
                    is_windows: true,
                }
            ),
            PathBuf::from("/opt/hdmi_cec_studio")
        );
    }

    #[test]
    fn test_migrate_legacy_data_non_destructive() {
        let root = std::env::temp_dir().join(format!("cecdesk_mig_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let legacy = root.join("install");
        let data = root.join("appdata");
        fs::create_dir_all(legacy.join("scripts")).unwrap();
        fs::write(legacy.join("cec_studio_config.json"), r#"{"osd_name":"LIVING_ROOM"}"#).unwrap();
        fs::write(legacy.join("scripts").join("kodi.bat"), "echo hi").unwrap();

        let (cfg_copied, scripts_copied) = migrate_legacy_data_in_dirs(&legacy, None, &data);
        assert!(cfg_copied);
        assert_eq!(scripts_copied, 1);
        assert!(data.join("cecdesk_config.json").exists());
        assert!(data.join("scripts").join("kodi.bat").exists());
        assert!(legacy.join("cec_studio_config.json").exists());

        let (cfg_again, scripts_again) = migrate_legacy_data_in_dirs(&legacy, None, &data);
        assert!(!cfg_again);
        assert_eq!(scripts_again, 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_generate_install_id_uuid_v4_format() {
        let id1 = generate_install_id();
        let id2 = generate_install_id();
        assert_eq!(id1.len(), 36);
        assert_eq!(&id1[14..15], "4");
        let variant = &id1[19..20];
        assert!(["8", "9", "a", "b"].contains(&variant));
        assert_ne!(id1, id2);
    }
}
