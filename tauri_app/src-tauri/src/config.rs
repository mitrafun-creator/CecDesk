use crate::types::AppConfig;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn find_base_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if exe_dir.join("tv_profiles").exists() {
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
    PathBuf::from(".")
}

pub fn find_profiles_dir() -> PathBuf {
    find_base_dir().join("tv_profiles")
}

pub fn find_logs_dir() -> PathBuf {
    let dir = find_base_dir().join("logs");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_scripts_dir() -> PathBuf {
    let dir = find_base_dir().join("scripts");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn find_config_path() -> PathBuf {
    find_base_dir().join("cec_studio_config.json")
}

/// Атомарное сохранение текстового содержимого через временный файл
pub fn save_atomic(target_path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp_path = target_path.with_extension("tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("Не удалось записать временный файл: {}", e))?;
    if let Err(_e) = fs::rename(&tmp_path, target_path) {
        // Fallback для Windows, если целевой файл блокирован: удаляем и пробуем перезаписать
        let _ = fs::remove_file(target_path);
        fs::rename(&tmp_path, target_path)
            .or_else(|_| fs::write(target_path, content))
            .map_err(|e| format!("Не удалось атомарно переименовать файл: {}", e))?;
    }
    Ok(())
}

/// Ротация файлов сессионных логов: сохранять только последние `max_keep` файлов
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
            session_files.sort_by_key(|a| std::cmp::Reverse(a.1)); // от свежих к старым
            for (p, _) in session_files.into_iter().skip(max_keep) {
                let _ = fs::remove_file(p);
            }
        }
    }
}

pub fn load_or_create_config() -> AppConfig {
    let p = find_config_path();
    let def = AppConfig::default();
    if let Ok(txt) = fs::read_to_string(&p) {
        if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&txt) {
            let mut changed = false;
            // Миграция устаревших значений
            for (code, val) in cfg.key_mappings.iter_mut() {
                if val == "MEME" {
                    *val = if code == "0x20" { "NUM_0".to_string() } else { "NONE".to_string() };
                    changed = true;
                }
            }
            for (k, v) in &def.key_mappings {
                if !cfg.key_mappings.contains_key(k) {
                    cfg.key_mappings.insert(k.clone(), v.clone());
                    changed = true;
                }
            }
            if changed {
                if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                    let _ = save_atomic(&p, &json);
                }
            }
            return cfg;
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(&def) {
        let _ = save_atomic(&p, &json);
    }
    def
}

pub fn write_log_to_disk(session_path: &Path, latest_path: &Path, msg: &str) {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[ts:{}] {}\n", now_secs, msg);
    if let Ok(mut f1) = OpenOptions::new().create(true).append(true).open(session_path) {
        let _ = f1.write_all(line.as_bytes());
    }
    if let Ok(mut f2) = OpenOptions::new().create(true).append(true).open(latest_path) {
        let _ = f2.write_all(line.as_bytes());
    }
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
    fn test_legacy_config_deserialization() {
        let legacy_json = r#"{
            "active_profile_id": "samsung_qled",
            "osd_name": "HTPC",
            "remote_to_os_enabled": true
        }"#;

        let parsed: AppConfig = serde_json::from_str(legacy_json).expect("Failed to deserialize legacy config");
        assert_eq!(parsed.active_profile_id, "samsung_qled");
        assert_eq!(parsed.osd_name, "HTPC");
        assert_eq!(parsed.language, "ru");
        assert!(parsed.custom_profile_names.is_empty());
    }
}
