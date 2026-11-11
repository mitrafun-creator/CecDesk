use super::*;

pub(super) fn normalize_sha256_hex(raw: &str) -> Option<String> {
    let clean = raw.trim().to_ascii_lowercase();
    if clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(clean)
    } else {
        None
    }
}

pub(super) fn infer_package_kind(explicit_kind: Option<&str>, url: &str) -> Option<String> {
    if let Some(k) = explicit_kind {
        let lower = k.trim().to_ascii_lowercase();
        if lower == "zip" || lower == "exe" {
            return Some(lower);
        }
        return None;
    }
    let path_only = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path_only.ends_with(".zip") {
        Some("zip".to_string())
    } else if path_only.ends_with(".exe") {
        Some("exe".to_string())
    } else {
        None
    }
}

/// Разбор и валидация манифеста `version.json` с поддержкой self-update пакета для Windows.
pub fn parse_version_manifest(
    txt: &str,
    current_ver: &str,
    is_windows: bool,
) -> Result<AppVersionInfo, String> {
    let val: serde_json::Value =
        serde_json::from_str(txt).map_err(|e| format!("Ошибка формата version.json: {}", e))?;

    let fallback_release_url =
        "https://github.com/mitrafun-creator/CecDesk/releases".to_string();
    let latest_ver = val["version"].as_str().unwrap_or(current_ver).to_string();
    let has_update = compare_semver(&latest_ver, current_ver) == Ordering::Greater;
    let release_notes = val["notes"]
        .as_str()
        .unwrap_or("Улучшения стабильности и новые функции.")
        .to_string();
    let raw_download_url = val["download_url"]
        .as_str()
        .unwrap_or(&fallback_release_url);
    let download_url = match validate_trusted_repo_url(raw_download_url) {
        Ok(u) => u,
        Err(e) => {
            log_warn(
                "updater",
                &format!("Ignored untrusted download_url in version.json: {}", e),
            );
            fallback_release_url
        }
    };
    let published_at = val["published_at"].as_str().unwrap_or("").to_string();

    let raw_win_url = val["windows"]["url"]
        .as_str()
        .or_else(|| val["windows_package_url"].as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let raw_win_sha = val["windows"]["sha256"]
        .as_str()
        .or_else(|| val["windows_sha256"].as_str());
    let raw_win_kind = val["windows"]["kind"]
        .as_str()
        .or_else(|| val["windows_package_kind"].as_str());

    let mut package_url = None;
    let mut package_sha256 = None;
    let mut package_kind = None;

    if let Some(u) = raw_win_url {
        match validate_trusted_repo_url(u) {
            Ok(valid_u) => {
                if let (Some(sha), Some(kind)) = (
                    raw_win_sha.and_then(normalize_sha256_hex),
                    infer_package_kind(raw_win_kind, &valid_u),
                ) {
                    package_url = Some(valid_u);
                    package_sha256 = Some(sha);
                    package_kind = Some(kind);
                } else {
                    log_warn(
                        "updater",
                        "Windows update URL present in version.json, but valid sha256/kind missing; self-update disabled",
                    );
                }
            }
            Err(e) => {
                log_warn(
                    "updater",
                    &format!("Ignored untrusted windows_package_url in version.json: {}", e),
                );
            }
        }
    }

    let can_self_update = is_windows
        && has_update
        && package_url.is_some()
        && package_sha256.is_some()
        && package_kind.is_some();

    Ok(AppVersionInfo {
        current_version: current_ver.to_string(),
        latest_version: latest_ver,
        has_update,
        release_notes,
        download_url,
        published_at,
        package_url,
        package_sha256,
        package_kind,
        can_self_update,
    })
}

/// Проверка наличия новых версий приложения CecDesk
pub fn check_app_updates() -> AppVersionInfo {
    let current_ver = CURRENT_APP_VERSION.to_string();
    let fallback_release_url =
        "https://github.com/mitrafun-creator/CecDesk/releases".to_string();

    if let Ok(txt) = fetch_url_text(DEFAULT_APP_VERSION_URL, 5) {
        if let Ok(info) = parse_version_manifest(&txt, &current_ver, cfg!(target_os = "windows")) {
            return info;
        }
    }

    AppVersionInfo {
        current_version: current_ver.clone(),
        latest_version: current_ver,
        has_update: false,
        release_notes: "Установлена актуальная версия приложения.".to_string(),
        download_url: fallback_release_url,
        published_at: "2026-11-10".to_string(),
        package_url: None,
        package_sha256: None,
        package_kind: None,
        can_self_update: false,
    }
}
