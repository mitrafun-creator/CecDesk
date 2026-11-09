use crate::config::find_scripts_dir;
use crate::platform;
use crate::types::CustomScriptInfo;
use std::fs;

pub fn list_custom_scripts_internal() -> Vec<CustomScriptInfo> {
    let dir = find_scripts_dir();
    let mut scripts = Vec::new();
    let allowed_exts = ["bat", "cmd", "ps1", "vbs", "exe", "lnk", "url", "sh"];

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if allowed_exts.contains(&ext_lower.as_str()) {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let filename = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();
                            scripts.push(CustomScriptInfo {
                                id: format!("SCRIPT:{}", stem),
                                name: stem.to_string(),
                                filename,
                                ext: ext_lower,
                            });
                        }
                    }
                }
            }
        }
    }
    scripts.sort_by_key(|a| a.name.to_lowercase());
    scripts
}

pub fn open_scripts_folder_internal() -> Result<(), String> {
    let dir = find_scripts_dir();
    platform::open_folder(&dir)
}

pub fn run_custom_script(identifier: &str) -> String {
    let name = if let Some(stripped) = identifier.strip_prefix("SCRIPT:") {
        stripped
    } else {
        identifier
    };

    // Защита от Path Traversal: запрещены разделители путей и относительные ссылки
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return format!("Недопустимое имя сценария '{}': запрещены разделители путей", name);
    }

    let dir = find_scripts_dir();
    let allowed_exts = ["bat", "cmd", "ps1", "vbs", "exe", "lnk", "url", "sh"];

    let mut target_path = None;
    let mut found_ext = String::new();

    for ext in &allowed_exts {
        let p = dir.join(format!("{}.{}", name, ext));
        if p.exists() {
            target_path = Some(p);
            found_ext = ext.to_string();
            break;
        }
    }

    if target_path.is_none() {
        let p = dir.join(name);
        if p.exists() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                found_ext = ext.to_lowercase();
            }
            target_path = Some(p);
        }
    }

    let Some(path) = target_path else {
        return format!("Скрипт '{}' не найден в папке scripts", name);
    };

    // Проверка канонического пути: файл обязан лежать строго внутри каталога scripts
    if let (Ok(can_target), Ok(can_dir)) = (path.canonicalize(), dir.canonicalize()) {
        if !can_target.starts_with(&can_dir) {
            return format!("Доступ запрещен: сценарий '{}' находится за пределами scripts", name);
        }
    }

    let dir_clone = dir.clone();
    let name_clone = name.to_string();

    std::thread::spawn(move || {
        let _ = platform::run_script(&path, &found_ext, &dir_clone);
    });

    format!("Сценарий '{}' запущен", name_clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_blocked() {
        let res1 = run_custom_script("../secret");
        assert!(res1.contains("Недопустимое имя сценария"));

        let res2 = run_custom_script("..\\boot.ini");
        assert!(res2.contains("Недопустимое имя сценария"));

        let res3 = run_custom_script("subdir/../../evil");
        assert!(res3.contains("Недопустимое имя сценария"));
    }
}
