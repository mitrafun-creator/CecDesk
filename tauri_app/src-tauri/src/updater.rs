use crate::config::{
    find_bundled_profiles_dir, find_resource_dir, find_user_profiles_dir, save_atomic,
};
use crate::hardware::AppState;
use crate::logger::{log_info, log_warn};
use crate::platform::create_hidden_command;
use crate::telemetry::{sanitize_profile_id, validate_trusted_repo_url};
use crate::types::TvProfile;
#[cfg(any(target_os = "windows", test))]
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
#[cfg(any(target_os = "windows", test))]
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
#[cfg(any(target_os = "windows", test))]
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const CURRENT_APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/tv_profiles/catalog.json";
pub const DEFAULT_APP_VERSION_URL: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/version.json";
pub const LATEST_BUNDLED_FW_VERSION: &str = "1.1.0";

static APP_UPDATE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppVersionInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
    pub package_url: Option<String>,
    pub package_sha256: Option<String>,
    pub package_kind: Option<String>,
    pub can_self_update: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppUpdateProgress {
    pub percent: u8,
    pub stage: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileManifestItem {
    pub id: String,
    pub brand: String,
    pub name: String,
    pub cec_system_name: String,
    pub version: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileCatalogManifest {
    pub catalog_version: String,
    pub updated_at: String,
    pub profiles: Vec<ProfileManifestItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileSyncResult {
    pub success: bool,
    pub updated_count: usize,
    pub total_remote: usize,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirmwareVersionInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub changelog: String,
    pub controller_type: String,
    pub rp2040_uf2_available: bool,
    pub esp32_bin_available: bool,
    pub uf2_drive_mounted: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirmwareFlashProgress {
    pub percent: u8,
    pub stage: String,
    pub message: String,
}

/// Семантическое сравнение версий SemVer 2.0 (`major.minor.patch[-prerelease][+build]`).
pub fn compare_semver(v1: &str, v2: &str) -> Ordering {
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum PreIdent {
        Num(u64),
        Alpha(String),
    }

    impl Ord for PreIdent {
        fn cmp(&self, other: &Self) -> Ordering {
            match (self, other) {
                (PreIdent::Num(a), PreIdent::Num(b)) => a.cmp(b),
                (PreIdent::Num(_), PreIdent::Alpha(_)) => Ordering::Less,
                (PreIdent::Alpha(_), PreIdent::Num(_)) => Ordering::Greater,
                (PreIdent::Alpha(a), PreIdent::Alpha(b)) => a.cmp(b),
            }
        }
    }

    impl PartialOrd for PreIdent {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    fn parse_semver(raw: &str) -> ((u32, u32, u32), Option<Vec<PreIdent>>) {
        let clean = raw.trim().trim_start_matches('v').trim_start_matches('V');
        let no_build = clean.split('+').next().unwrap_or("");
        let (core_str, pre_str) = match no_build.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (no_build, None),
        };
        let mut core_it = core_str.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
        let core = (
            core_it.next().unwrap_or(0),
            core_it.next().unwrap_or(0),
            core_it.next().unwrap_or(0),
        );
        let pre = pre_str.filter(|p| !p.is_empty()).map(|p| {
            p.split('.')
                .map(|seg| match seg.parse::<u64>() {
                    Ok(n) => PreIdent::Num(n),
                    Err(_) => PreIdent::Alpha(seg.to_string()),
                })
                .collect::<Vec<_>>()
        });
        (core, pre)
    }

    let (c1, pre1) = parse_semver(v1);
    let (c2, pre2) = parse_semver(v2);
    match c1.cmp(&c2) {
        Ordering::Equal => match (pre1, pre2) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => {
                for (ia, ib) in a.iter().zip(b.iter()) {
                    let ord = ia.cmp(ib);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                a.len().cmp(&b.len())
            }
        },
        non_eq => non_eq,
    }
}

/// Легковесный кроссплатформенный загрузчик через системный curl (без тяжелых внешних TLS библиотек)
pub fn fetch_url_text(url: &str, timeout_secs: u32) -> Result<String, String> {
    let output = create_hidden_command("curl")
        .args(["-s", "-L", "--max-time", &timeout_secs.to_string(), url])
        .output()
        .map_err(|e| format!("Не удалось выполнить запрос через curl: {}", e))?;

    if output.status.success() {
        let txt = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if txt.is_empty() {
            Err("Пустой ответ от сервера".to_string())
        } else {
            Ok(txt)
        }
    } else {
        Err(format!("Ошибка сети (HTTP status: {})", output.status))
    }
}

/// Генерация локального манифеста каталога профилей (используется вручную при подготовке релиза).
#[allow(dead_code)]
pub fn generate_local_catalog_manifest(
    profiles_dir: &Path,
) -> Result<ProfileCatalogManifest, String> {
    let mut items = Vec::new();
    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name == "catalog.json" {
                        continue;
                    }
                    if let Ok(txt) = fs::read_to_string(&path) {
                        if let Ok(p) = serde_json::from_str::<TvProfile>(&txt) {
                            items.push(ProfileManifestItem {
                                id: p.id.clone(),
                                brand: p.brand,
                                name: p.name,
                                cec_system_name: p.cec_system_name,
                                version: "1.0.0".to_string(),
                                url: format!(
                                    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/tv_profiles/{}",
                                    file_name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    items.sort_by(|a, b| a.brand.cmp(&b.brand).then(a.name.cmp(&b.name)));
    let manifest = ProfileCatalogManifest {
        catalog_version: "1.0.0".to_string(),
        updated_at: "2026-10-26".to_string(),
        profiles: items,
    };

    let cat_path = profiles_dir.join("catalog.json");
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = save_atomic(&cat_path, &json);
    }

    Ok(manifest)
}

/// Онлайн-синхронизация каталога профилей телевизоров (загружает новые профили в data_dir/tv_profiles).
pub fn sync_tv_profiles_online() -> ProfileSyncResult {
    let user_prof_dir = find_user_profiles_dir();
    let bundled_prof_dir = find_bundled_profiles_dir();

    let manifest_txt = match fetch_url_text(DEFAULT_CATALOG_URL, 6) {
        Ok(t) => t,
        Err(e) => {
            log_warn("updater", &format!("TV profiles catalog fetch failed: {}", e));
            return ProfileSyncResult {
                success: false,
                updated_count: 0,
                total_remote: 0,
                message: format!("Каталог недоступен офлайн: {}", e),
            };
        }
    };

    let manifest: ProfileCatalogManifest = match serde_json::from_str(&manifest_txt) {
        Ok(m) => m,
        Err(e) => {
            log_warn("updater", &format!("TV catalog JSON parse error: {}", e));
            return ProfileSyncResult {
                success: false,
                updated_count: 0,
                total_remote: 0,
                message: format!("Ошибка формата манифеста: {}", e),
            };
        }
    };

    let mut updated = 0;
    for item in &manifest.profiles {
        let Ok(safe_id) = sanitize_profile_id(&item.id) else {
            log_warn(
                "updater",
                &format!("Rejected unsafe profile id from remote catalog: {}", item.id),
            );
            continue;
        };
        let Ok(valid_url) = validate_trusted_repo_url(&item.url) else {
            log_warn(
                "updater",
                &format!("Rejected untrusted profile URL for id={}", safe_id),
            );
            continue;
        };

        let target_file = user_prof_dir.join(format!("{}.json", safe_id));
        let bundled_file = bundled_prof_dir.join(format!("{}.json", safe_id));
        let needs_fetch = !target_file.exists() && !bundled_file.exists();

        if needs_fetch {
            match fetch_url_text(&valid_url, 5) {
                Ok(profile_json) => {
                    if serde_json::from_str::<TvProfile>(&profile_json).is_ok()
                        && save_atomic(&target_file, &profile_json).is_ok()
                    {
                        updated += 1;
                    }
                }
                Err(e) => {
                    log_warn(
                        "updater",
                        &format!("Failed to download TV profile {}: {}", safe_id, e),
                    );
                }
            }
        }
    }

    log_info(
        "updater",
        &format!(
            "TV catalog sync finished: downloaded={}, total_remote={}",
            updated,
            manifest.profiles.len()
        ),
    );

    ProfileSyncResult {
        success: true,
        updated_count: updated,
        total_remote: manifest.profiles.len(),
        message: format!(
            "Каталог синхронизирован. Загружено новых: {}, всего в базе: {}",
            updated,
            manifest.profiles.len()
        ),
    }
}

fn normalize_sha256_hex(raw: &str) -> Option<String> {
    let clean = raw.trim().to_ascii_lowercase();
    if clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(clean)
    } else {
        None
    }
}

fn infer_package_kind(explicit_kind: Option<&str>, url: &str) -> Option<String> {
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
        "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases".to_string();
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

/// Проверка наличия новых версий приложения HDMI-CEC Studio
pub fn check_app_updates() -> AppVersionInfo {
    let current_ver = CURRENT_APP_VERSION.to_string();
    let fallback_release_url =
        "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases".to_string();

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

#[cfg(test)]
pub fn compute_sha256_hex(data: &[u8]) -> String {
    compute_sha256_from_reader(std::io::Cursor::new(data)).unwrap_or_default()
}

#[cfg(any(target_os = "windows", test))]
pub fn compute_file_sha256_hex(path: &Path) -> Result<String, String> {
    let file =
        fs::File::open(path).map_err(|e| format!("Не удалось открыть файл для SHA-256: {}", e))?;
    compute_sha256_from_reader(BufReader::new(file))
}

#[cfg(any(target_os = "windows", test))]
fn compute_sha256_from_reader<R: Read>(mut reader: R) -> Result<String, String> {
    const H_INIT: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4,
        0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
        0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f,
        0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
        0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc,
        0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
        0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116,
        0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
        0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7,
        0xc671_78f2,
    ];

    fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let j = i * 4;
            *word = u32::from_be_bytes([block[j], block[j + 1], block[j + 2], block[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut state = H_INIT;
    let mut block = [0u8; 64];
    let mut block_len = 0usize;
    let mut total_bytes = 0u64;
    let mut io_buf = [0u8; 64 * 1024];

    loop {
        let n = reader
            .read(&mut io_buf)
            .map_err(|e| format!("Ошибка чтения при расчете SHA-256: {}", e))?;
        if n == 0 {
            break;
        }
        total_bytes = total_bytes
            .checked_add(n as u64)
            .ok_or_else(|| "Файл слишком велик для SHA-256".to_string())?;
        let mut idx = 0usize;
        while idx < n {
            let take = (64 - block_len).min(n - idx);
            block[block_len..block_len + take].copy_from_slice(&io_buf[idx..idx + take]);
            block_len += take;
            idx += take;
            if block_len == 64 {
                compress(&mut state, &block);
                block_len = 0;
            }
        }
    }

    block[block_len] = 0x80;
    block_len += 1;
    if block_len > 56 {
        block[block_len..64].fill(0);
        compress(&mut state, &block);
        block_len = 0;
    }
    block[block_len..56].fill(0);
    let total_bits = total_bytes.wrapping_mul(8);
    block[56..64].copy_from_slice(&total_bits.to_be_bytes());
    compress(&mut state, &block);

    let mut out = String::with_capacity(64);
    for word in state {
        use std::fmt::Write as FmtWrite;
        let _ = write!(&mut out, "{:08x}", word);
    }
    Ok(out)
}

#[cfg(any(target_os = "windows", test))]
fn sanitize_zip_rel_segments(raw_name: &str) -> Result<Option<Vec<String>>, String> {
    if raw_name.contains('\0') {
        return Err("Архив содержит недопустимый нулевой байт в имени файла".to_string());
    }
    let normalized = raw_name.replace('\\', "/");
    if normalized.ends_with('/') {
        return Ok(None);
    }
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.contains(':') || normalized.starts_with('/') {
        return Err(format!("Заблокирован небезопасный путь в архиве: {}", raw_name));
    }

    let mut parts = Vec::new();
    for seg in trimmed.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.chars().any(char::is_control) {
            return Err(format!("Заблокирован выход за пределы каталога (Zip-Slip): {}", raw_name));
        }
        parts.push(seg.to_string());
    }
    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts))
    }
}

#[cfg(any(target_os = "windows", test))]
fn normalize_allowed_update_rel_path(segments: &[String], exe_name: &str) -> Option<PathBuf> {
    if segments.is_empty() {
        return None;
    }
    if segments.len() == 1 {
        let f = &segments[0];
        if f.eq_ignore_ascii_case("hdmi_cec_studio.exe") || f.eq_ignore_ascii_case(exe_name) {
            return Some(PathBuf::from(exe_name));
        }
        if f.eq_ignore_ascii_case("WebView2Loader.dll")
            || f.eq_ignore_ascii_case("README.md")
            || f.eq_ignore_ascii_case("version.json")
        {
            return Some(PathBuf::from(f));
        }
        return None;
    }
    let top_lower = segments[0].to_ascii_lowercase();
    if top_lower == "bin" || top_lower == "tv_profiles" {
        let mut p = PathBuf::from(&top_lower);
        for seg in &segments[1..] {
            p.push(seg);
        }
        return Some(p);
    }
    None
}

/// Безопасная распаковка ZIP-пакета обновления без внешних зависимостей (CRC-32 + защита от Zip-Slip).
#[cfg(any(target_os = "windows", test))]
pub fn extract_safe_update_zip(
    zip_path: &Path,
    stage_dir: &Path,
    exe_name: &str,
) -> Result<Vec<PathBuf>, String> {
    const MAX_ENTRY_BYTES: usize = 150 * 1024 * 1024;
    const MAX_TOTAL_BYTES: usize = 250 * 1024 * 1024;

    let bytes =
        fs::read(zip_path).map_err(|e| format!("Не удалось прочитать скачанный архив: {}", e))?;
    if bytes.len() < 22 {
        return Err("Архив обновления поврежден (слишком малый размер).".to_string());
    }

    let min_eocd = bytes.len().saturating_sub(65_557);
    let mut eocd_off = None;
    let mut pos = bytes.len() - 22;
    loop {
        if bytes[pos..pos + 4] == [0x50, 0x4B, 0x05, 0x06] {
            let comment_len = u16::from_le_bytes([bytes[pos + 20], bytes[pos + 21]]) as usize;
            if pos + 22 + comment_len == bytes.len() {
                eocd_off = Some(pos);
                break;
            }
        }
        if pos <= min_eocd {
            break;
        }
        pos -= 1;
    }
    let eocd = eocd_off.ok_or_else(|| "Файл не является корректным ZIP-архивом (нет EOCD).".to_string())?;

    let total_entries = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
    let cd_size = u32::from_le_bytes([bytes[eocd + 12], bytes[eocd + 13], bytes[eocd + 14], bytes[eocd + 15]]) as usize;
    let cd_offset = u32::from_le_bytes([bytes[eocd + 16], bytes[eocd + 17], bytes[eocd + 18], bytes[eocd + 19]]) as usize;

    let cd_end = cd_offset
        .checked_add(cd_size)
        .filter(|&end| end <= eocd)
        .ok_or_else(|| "Повреждена таблица оглавления ZIP-архива.".to_string())?;

    struct ParsedEntry {
        segments: Vec<String>,
        method: u16,
        crc32: u32,
        comp_size: usize,
        uncomp_size: usize,
        local_hdr_offset: usize,
    }

    let mut entries = Vec::with_capacity(total_entries.min(4096));
    let mut cursor = cd_offset;
    for _ in 0..total_entries {
        if cursor + 46 > cd_end || bytes[cursor..cursor + 4] != [0x50, 0x4B, 0x01, 0x02] {
            return Err("Поврежден заголовок файла в ZIP-архиве.".to_string());
        }
        let method = u16::from_le_bytes([bytes[cursor + 10], bytes[cursor + 11]]);
        let crc32 = u32::from_le_bytes([
            bytes[cursor + 16],
            bytes[cursor + 17],
            bytes[cursor + 18],
            bytes[cursor + 19],
        ]);
        let comp_size = u32::from_le_bytes([
            bytes[cursor + 20],
            bytes[cursor + 21],
            bytes[cursor + 22],
            bytes[cursor + 23],
        ]) as usize;
        let uncomp_size = u32::from_le_bytes([
            bytes[cursor + 24],
            bytes[cursor + 25],
            bytes[cursor + 26],
            bytes[cursor + 27],
        ]) as usize;
        let name_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[cursor + 30], bytes[cursor + 31]]) as usize;
        let comment_len = u16::from_le_bytes([bytes[cursor + 32], bytes[cursor + 33]]) as usize;
        let local_hdr_offset = u32::from_le_bytes([
            bytes[cursor + 42],
            bytes[cursor + 43],
            bytes[cursor + 44],
            bytes[cursor + 45],
        ]) as usize;

        let name_start = cursor + 46;
        let name_end = name_start + name_len;
        let next_cursor = name_end + extra_len + comment_len;
        if next_cursor > cd_end {
            return Err("Некорректные границы имени файла в ZIP-архиве.".to_string());
        }

        let raw_name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();
        if let Some(segments) = sanitize_zip_rel_segments(&raw_name)? {
            if uncomp_size > MAX_ENTRY_BYTES {
                return Err(format!("Файл в архиве превышает допустимый лимит: {}", raw_name));
            }
            entries.push(ParsedEntry {
                segments,
                method,
                crc32,
                comp_size,
                uncomp_size,
                local_hdr_offset,
            });
        }
        cursor = next_cursor;
    }

    let first_root = entries.first().and_then(|e| {
        if e.segments.len() >= 2 {
            Some(e.segments[0].clone())
        } else {
            None
        }
    });
    let strip_wrapper = first_root.as_ref().is_some_and(|root| {
        entries
            .iter()
            .all(|e| e.segments.len() >= 2 && e.segments[0] == *root)
            && entries.iter().any(|e| {
                e.segments.len() == 2
                    && (e.segments[1].eq_ignore_ascii_case("hdmi_cec_studio.exe")
                        || e.segments[1].eq_ignore_ascii_case(exe_name))
            })
    });

    fs::create_dir_all(stage_dir)
        .map_err(|e| format!("Не удалось создать папку распаковки: {}", e))?;

    let mut total_unpacked = 0usize;
    let mut written_paths = Vec::new();
    let mut seen_rel = HashSet::new();

    for entry in &entries {
        let seg_slice: &[String] = if strip_wrapper {
            &entry.segments[1..]
        } else {
            &entry.segments[..]
        };
        let Some(rel_path) = normalize_allowed_update_rel_path(seg_slice, exe_name) else {
            continue;
        };
        if !seen_rel.insert(rel_path.clone()) {
            return Err(format!(
                "Дублирующийся файл в архиве обновления: {}",
                rel_path.display()
            ));
        }

        total_unpacked = total_unpacked
            .checked_add(entry.uncomp_size)
            .filter(|&tot| tot <= MAX_TOTAL_BYTES)
            .ok_or_else(|| "Суммарный размер распакованного обновления превышает лимит.".to_string())?;

        let lh = entry.local_hdr_offset;
        if lh + 30 > cd_offset || bytes[lh..lh + 4] != [0x50, 0x4B, 0x03, 0x04] {
            return Err("Поврежден локальный заголовок ZIP-записи.".to_string());
        }
        let l_name_len = u16::from_le_bytes([bytes[lh + 26], bytes[lh + 27]]) as usize;
        let l_extra_len = u16::from_le_bytes([bytes[lh + 28], bytes[lh + 29]]) as usize;
        let data_start = lh + 30 + l_name_len + l_extra_len;
        let data_end = data_start
            .checked_add(entry.comp_size)
            .filter(|&end| end <= cd_offset)
            .ok_or_else(|| "Некорректный размер сжатых данных в ZIP-архиве.".to_string())?;
        let comp_slice = &bytes[data_start..data_end];

        let unpacked = match entry.method {
            0 => {
                if comp_slice.len() != entry.uncomp_size {
                    return Err("Несовпадение размера stored-файла в ZIP-архиве.".to_string());
                }
                comp_slice.to_vec()
            }
            8 => {
                let mut dec = DeflateDecoder::new(comp_slice).take((entry.uncomp_size as u64) + 1);
                let mut buf = Vec::with_capacity(entry.uncomp_size.min(8 * 1024 * 1024));
                dec.read_to_end(&mut buf)
                    .map_err(|e| format!("Ошибка распаковки DEFLATE в архиве: {}", e))?;
                if buf.len() != entry.uncomp_size {
                    return Err("Фактический размер файла в ZIP не совпал с заголовком.".to_string());
                }
                buf
            }
            other => {
                return Err(format!(
                    "Неподдерживаемый метод сжатия ZIP ({}). Требуется Store или Deflate.",
                    other
                ));
            }
        };

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&unpacked);
        if hasher.finalize() != entry.crc32 {
            return Err(format!(
                "Ошибка контрольной суммы CRC-32 при распаковке {}",
                rel_path.display()
            ));
        }

        let dest = stage_dir.join(&rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать подкаталог {}: {}", parent.display(), e))?;
        }
        fs::write(&dest, &unpacked)
            .map_err(|e| format!("Ошибка записи распакованного файла {}: {}", dest.display(), e))?;
        written_paths.push(rel_path);
    }

    if !written_paths.iter().any(|p| p == Path::new(exe_name)) {
        return Err(format!("В архиве обновления не найден исполняемый файл {}.", exe_name));
    }

    Ok(written_paths)
}

#[cfg(any(target_os = "windows", test))]
fn verify_windows_pe_binary(path: &Path) -> Result<(), String> {
    let meta = fs::metadata(path)
        .map_err(|e| format!("Не найден подготовленный бинарник обновления: {}", e))?;
    if meta.len() < 1_000_000 {
        return Err(format!(
            "Скачанный исполняемый файл слишком мал ({} байт) и не похож на полный билд.",
            meta.len()
        ));
    }
    let mut f = fs::File::open(path)
        .map_err(|e| format!("Не удалось проверить заголовок исполняемого файла: {}", e))?;
    let mut magic = [0u8; 2];
    f.read_exact(&mut magic)
        .map_err(|e| format!("Ошибка чтения заголовка PE: {}", e))?;
    if &magic != b"MZ" {
        return Err("Скачанный файл не является корректным Windows PE (.exe) модулем.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn download_url_to_file(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Не удалось создать каталог загрузки: {}", e))?;
    }
    let _ = fs::remove_file(dest);
    let dest_str = dest.to_string_lossy().to_string();
    let output = create_hidden_command("curl")
        .args([
            "-f",
            "-s",
            "-S",
            "-L",
            "--proto",
            "=https",
            "--max-time",
            "180",
            "-o",
            &dest_str,
            url,
        ])
        .output()
        .map_err(|e| format!("Не удалось запустить curl для загрузки обновления: {}", e))?;

    if output.status.success() && dest.is_file() {
        Ok(())
    } else {
        let _ = fs::remove_file(dest);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("Ошибка загрузки обновления (код {}).", output.status)
        } else {
            format!("Ошибка загрузки обновления: {}", stderr)
        })
    }
}

#[cfg(target_os = "windows")]
fn emit_app_update_progress(app: &AppHandle, percent: u8, stage: &str, message: &str) {
    let _ = app.emit(
        "app-update-progress",
        AppUpdateProgress {
            percent,
            stage: stage.to_string(),
            message: message.to_string(),
        },
    );
}

#[cfg(target_os = "windows")]
fn cleanup_stale_update_dirs(temp_root: &Path) {
    if let Ok(entries) = fs::read_dir(temp_root) {
        let now = std::time::SystemTime::now();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("hdmi_cec_update_") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if now.duration_since(modified).unwrap_or_default()
                            > Duration::from_secs(3600)
                        {
                            let _ = fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
const WINDOWS_APPLY_UPDATE_SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'
$oldPid = [int]$env:CEC_UPD_PID
$payloadDir = $env:CEC_UPD_PAYLOAD
$targetDir = $env:CEC_UPD_TARGET
$exeName = $env:CEC_UPD_EXE
$stageRoot = $env:CEC_UPD_ROOT
$logPath = Join-Path $stageRoot 'apply_update.log'

function Write-UpdLog([string]$msg) {
    $ts = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
    Add-Content -LiteralPath $logPath -Value "[$ts] $msg" -Encoding UTF8 -ErrorAction SilentlyContinue
}

Write-UpdLog "Updater started: pid=$oldPid exe=$exeName target=$targetDir"

for ($i = 0; $i -lt 130; $i++) {
    $proc = Get-Process -Id $oldPid -ErrorAction SilentlyContinue
    if (-not $proc) { break }
    Start-Sleep -Milliseconds 150
}
Start-Sleep -Milliseconds 350

if (Get-Process -Id $oldPid -ErrorAction SilentlyContinue) {
    Write-UpdLog "Timed out waiting for process PID $oldPid to exit."
    exit 1
}

$targetExe = Join-Path $targetDir $exeName
$backupExe = Join-Path $targetDir ($exeName + '.bak')
if (Test-Path -LiteralPath $targetExe) {
    Copy-Item -LiteralPath $targetExe -Destination $backupExe -Force
}

$copyOk = $false
try {
    $files = Get-ChildItem -LiteralPath $payloadDir -Recurse -File
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($payloadDir.Length).TrimStart('\', '/')
        $dest = Join-Path $targetDir $rel
        $destParent = Split-Path -Path $dest -Parent
        if (-not (Test-Path -LiteralPath $destParent)) {
            New-Item -ItemType Directory -Path $destParent -Force | Out-Null
        }
        $copied = $false
        for ($try = 0; $try -lt 15; $try++) {
            try {
                Copy-Item -LiteralPath $f.FullName -Destination $dest -Force
                $copied = $true
                break
            } catch {
                Start-Sleep -Milliseconds 250
            }
        }
        if (-not $copied) {
            throw "Failed copying $rel after retries"
        }
    }
    $copyOk = $true
    Write-UpdLog "Files copied successfully."
} catch {
    Write-UpdLog ("Update error: " + $_.Exception.Message)
    if (Test-Path -LiteralPath $backupExe) {
        Copy-Item -LiteralPath $backupExe -Destination $targetExe -Force -ErrorAction SilentlyContinue
        Write-UpdLog "Restored backup executable."
    }
}

if ($copyOk -and (Test-Path -LiteralPath $backupExe)) {
    Remove-Item -LiteralPath $backupExe -Force -ErrorAction SilentlyContinue
}

if (Test-Path -LiteralPath $targetExe) {
    Start-Process -FilePath $targetExe -WorkingDirectory $targetDir
    Write-UpdLog "Relaunched $targetExe"
}

if ($copyOk) {
    Remove-Item -LiteralPath $payloadDir -Recurse -Force -ErrorAction SilentlyContinue
    $dlDir = Join-Path $stageRoot 'download'
    Remove-Item -LiteralPath $dlDir -Recurse -Force -ErrorAction SilentlyContinue
}
"#;

#[cfg(target_os = "windows")]
fn install_app_update_windows(app: &AppHandle, state: &AppState) -> Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Не удалось определить путь к запущенной программе: {}", e))?;
    let target_dir = exe_path
        .parent()
        .ok_or_else(|| "Не удалось определить рабочий каталог приложения.".to_string())?
        .to_path_buf();
    let exe_name = exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("HDMI_CEC_Studio.exe")
        .to_string();

    // Проверяем права на запись в папку с HDMI_CEC_Studio.exe ДО скачивания обновления
    let probe_path = target_dir.join(format!(".hdmi_cec_write_probe_{}.tmp", std::process::id()));
    fs::write(&probe_path, b"ok").map_err(|e| {
        format!(
            "Нет прав на обновление файлов в каталоге {} ({}). Переместите программу в папку с правами записи или запустите от имени администратора.",
            target_dir.display(),
            e
        )
    })?;
    let _ = fs::remove_file(&probe_path);

    emit_app_update_progress(app, 8, "manifest", "Проверка манифеста релиза на GitHub...");
    let manifest_txt = fetch_url_text(DEFAULT_APP_VERSION_URL, 10)?;
    let info = parse_version_manifest(&manifest_txt, CURRENT_APP_VERSION, true)?;
    if !info.has_update {
        return Err("Уже установлена актуальная версия приложения.".to_string());
    }
    let (pkg_url, expected_sha, pkg_kind) = match (
        info.package_url.clone(),
        info.package_sha256.clone(),
        info.package_kind.clone(),
    ) {
        (Some(u), Some(s), Some(k)) => (u, s, k),
        _ => {
            return Err(
                "Для этой версии в version.json не указан прямой пакет Windows с SHA-256. Скачайте обновление со страницы релизов."
                    .to_string(),
            );
        }
    };

    let temp_root = std::env::temp_dir();
    cleanup_stale_update_dirs(&temp_root);

    let stage_root = temp_root.join(format!("hdmi_cec_update_{}", std::process::id()));
    let dl_dir = stage_root.join("download");
    let payload_dir = stage_root.join("payload");
    let _ = fs::remove_dir_all(&stage_root);
    fs::create_dir_all(&dl_dir)
        .and_then(|_| fs::create_dir_all(&payload_dir))
        .map_err(|e| format!("Не удалось подготовить временную папку обновления: {}", e))?;

    let download_file_name = if pkg_kind == "zip" {
        "update_package.zip"
    } else {
        exe_name.as_str()
    };
    let dl_path = dl_dir.join(download_file_name);

    emit_app_update_progress(
        app,
        22,
        "downloading",
        &format!("Скачивание v{} с GitHub...", info.latest_version),
    );
    if let Err(e) = download_url_to_file(&pkg_url, &dl_path) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(e);
    }

    emit_app_update_progress(
        app,
        62,
        "verifying",
        "Проверка контрольной суммы SHA-256...",
    );
    let actual_sha = match compute_file_sha256_hex(&dl_path) {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_dir_all(&stage_root);
            return Err(e);
        }
    };
    if actual_sha != expected_sha {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!(
            "Контрольная сумма SHA-256 не совпала! Ожидалось {}..., получено {}...",
            &expected_sha[..12],
            &actual_sha[..12]
        ));
    }

    emit_app_update_progress(app, 78, "staging", "Распаковка и проверка файлов обновления...");
    let staged_exe = payload_dir.join(&exe_name);
    if pkg_kind == "zip" {
        if let Err(e) = extract_safe_update_zip(&dl_path, &payload_dir, &exe_name) {
            let _ = fs::remove_dir_all(&stage_root);
            return Err(e);
        }
    } else if let Err(e) = fs::copy(&dl_path, &staged_exe) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Ошибка подготовки файла обновления: {}", e));
    }

    if let Err(e) = verify_windows_pe_binary(&staged_exe) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(e);
    }

    let script_path = stage_root.join("apply_update.ps1");
    if let Err(e) = fs::write(&script_path, WINDOWS_APPLY_UPDATE_SCRIPT) {
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Не удалось создать скрипт установки обновления: {}", e));
    }

    emit_app_update_progress(
        app,
        94,
        "restarting",
        "Освобождение контроллера и перезапуск в новую версию...",
    );
    log_info(
        "updater",
        &format!(
            "Self-update staged cleanly for v{} (kind={}, sha256={}...); restarting",
            info.latest_version,
            pkg_kind,
            &expected_sha[..12]
        ),
    );

    // Не выключаем ТВ командой OFF при плановом перезапуске ради обновления (но REMOTE_OFF отправится штатно)
    state.skip_standby_on_exit.store(true, AtomicOrdering::SeqCst);

    let spawn_res = create_hidden_command("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.to_string_lossy(),
        ])
        .env("CEC_UPD_PID", std::process::id().to_string())
        .env("CEC_UPD_PAYLOAD", payload_dir.to_string_lossy().as_ref())
        .env("CEC_UPD_TARGET", target_dir.to_string_lossy().as_ref())
        .env("CEC_UPD_EXE", &exe_name)
        .env("CEC_UPD_ROOT", stage_root.to_string_lossy().as_ref())
        .spawn();

    if let Err(e) = spawn_res {
        state.skip_standby_on_exit.store(false, AtomicOrdering::SeqCst);
        let _ = fs::remove_dir_all(&stage_root);
        return Err(format!("Не удалось запустить процесс замены файла: {}", e));
    }

    emit_app_update_progress(app, 100, "done", "Перезапуск...");
    let app_exit = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(160));
        app_exit.exit(0);
    });

    Ok(format!("Перезапуск для установки v{}...", info.latest_version))
}

/// Загрузка, проверка SHA-256 и установка обновления (на Windows в 1 клик поверх текущего каталога)
pub fn install_app_update_internal(app: &AppHandle, state: &AppState) -> Result<String, String> {
    if APP_UPDATE_IN_PROGRESS.swap(true, AtomicOrdering::SeqCst) {
        return Err("Обновление приложения уже выполняется.".to_string());
    }

    struct ResetGuard;
    impl Drop for ResetGuard {
        fn drop(&mut self) {
            APP_UPDATE_IN_PROGRESS.store(false, AtomicOrdering::SeqCst);
        }
    }
    let _guard = ResetGuard;

    #[cfg(target_os = "windows")]
    {
        install_app_update_windows(app, state)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state);
        Err("Автообновление в один клик поддерживается только в Windows. Используйте страницу релизов GitHub или install_steamos.sh.".to_string())
    }
}

/// Поиск директории со встроенными образами прошивок (bin/firmware)
pub fn find_firmware_dir() -> PathBuf {
    let res_dir = find_resource_dir();
    let bin_fw = res_dir.join("bin").join("firmware");
    if bin_fw.exists() {
        return bin_fw;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate1 = exe_dir.join("firmware");
            if candidate1.exists() {
                return candidate1;
            }
            let candidate2 = exe_dir.join("bin").join("firmware");
            if candidate2.exists() {
                return candidate2;
            }
        }
    }
    bin_fw
}

pub fn find_bundled_rp2040_uf2() -> Option<PathBuf> {
    let p1 = find_firmware_dir().join("rp2040_zero_firmware.uf2");
    if p1.exists() {
        return Some(p1);
    }
    let p2 = find_resource_dir()
        .join("firmware")
        .join(".pio")
        .join("build")
        .join("rp2040-zero")
        .join("firmware.uf2");
    if p2.exists() {
        return Some(p2);
    }
    None
}

pub fn find_bundled_esp32_bin() -> Option<PathBuf> {
    let p1 = find_firmware_dir().join("esp32_s3_firmware.bin");
    if p1.exists() {
        return Some(p1);
    }
    let p2 = find_resource_dir()
        .join("firmware")
        .join(".pio")
        .join("build")
        .join("esp32-s3-devkitc-1")
        .join("firmware.bin");
    if p2.exists() {
        return Some(p2);
    }
    None
}

/// Поиск смонтированного UF2-накопителя RPI-RP2 (проверка INFO_UF2.TXT на дисках D..Z в Windows и /run/media, /media, /mnt в Linux/SteamOS)
pub fn detect_rp2040_uf2_drive() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        for letter in b'D'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let info_path = Path::new(&drive).join("INFO_UF2.TXT");
            if info_path.exists() {
                return Some(drive);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let direct_roots = [
            "/run/media/deck/RPI-RP2",
            "/media/deck/RPI-RP2",
            "/run/media/RPI-RP2",
            "/media/RPI-RP2",
            "/mnt/RPI-RP2",
        ];
        for r in direct_roots {
            let p = Path::new(r);
            if p.join("INFO_UF2.TXT").exists() {
                return Some(r.to_string());
            }
        }
        for base in ["/run/media", "/media", "/mnt"] {
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.flatten() {
                    let sub = entry.path();
                    if sub.join("INFO_UF2.TXT").exists() {
                        return Some(sub.to_string_lossy().to_string());
                    }
                    let rpi = sub.join("RPI-RP2");
                    if rpi.join("INFO_UF2.TXT").exists() {
                        return Some(rpi.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}

/// Определение типа контроллера по имени порта и USB VID/PID
pub fn detect_controller_type_by_port(port_name: &str) -> String {
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            if p.port_name.eq_ignore_ascii_case(port_name) {
                if let serialport::SerialPortType::UsbPort(usb) = p.port_type {
                    if usb.vid == 0x2E8A {
                        return "RP2040-Zero".to_string();
                    }
                    if usb.vid == 0x303A || usb.vid == 0x10C4 || usb.vid == 0x1A86 {
                        return "ESP32-S3".to_string();
                    }
                }
            }
        }
    }
    if detect_rp2040_uf2_drive().is_some() {
        return "RP2040-Zero (Загрузчик UF2)".to_string();
    }
    "Автоопределение".to_string()
}

/// Проверка версии прошивки и готовности встроенного программатора
pub fn check_firmware_update(detected_fw_ver: &str, port_name: &str) -> FirmwareVersionInfo {
    let cur = if detected_fw_ver.is_empty() {
        "1.1.0"
    } else {
        detected_fw_ver
    };
    let update_available = compare_semver(LATEST_BUNDLED_FW_VERSION, cur) == Ordering::Greater;
    let ctrl_type = detect_controller_type_by_port(port_name);

    FirmwareVersionInfo {
        current_version: cur.to_string(),
        latest_version: LATEST_BUNDLED_FW_VERSION.to_string(),
        update_available,
        changelog: if update_available {
            "Доступна прошивка v1.1.0: поддержка оптопары питания ПК (GP1/GPIO5), авто-пробуждение и встроенный загрузчик.".to_string()
        } else {
            "Установлена актуальная прошивка v1.1.0 (поддержка оптопары ПК, сценариев сна и встроенного обновления).".to_string()
        },
        controller_type: ctrl_type,
        rp2040_uf2_available: find_bundled_rp2040_uf2().is_some(),
        esp32_bin_available: find_bundled_esp32_bin().is_some(),
        uf2_drive_mounted: detect_rp2040_uf2_drive(),
    }
}

fn emit_flash_progress(app: &AppHandle, percent: u8, stage: &str, message: &str) {
    let _ = app.emit(
        "fw-flash-progress",
        FirmwareFlashProgress {
            percent,
            stage: stage.to_string(),
            message: message.to_string(),
        },
    );
}

/// Нативная прошивка RP2040-Zero через UF2 загрузчик (без стороннего ПО и драйверов Zadig)
fn flash_rp2040_uf2(port_name: &str, uf2_path: &Path, app: &AppHandle) -> Result<String, String> {
    if !uf2_path.exists() {
        return Err(format!("Файл прошивки UF2 не найден: {}", uf2_path.display()));
    }
    let uf2_bytes = fs::read(uf2_path)
        .map_err(|e| format!("Не удалось прочитать файл UF2: {}", e))?;
    if uf2_bytes.len() < 512 || &uf2_bytes[0..4] != b"UF2\n" {
        return Err("Выбранный файл не является корректным образом прошивки UF2 (RP2040).".to_string());
    }

    emit_flash_progress(
        app,
        15,
        "bootloader",
        "Перевод RP2040-Zero в режим загрузчика UF2...",
    );

    if detect_rp2040_uf2_drive().is_none() && !port_name.is_empty() {
        // 1. Отправка программной команды BOOTLOADER (вызывает reset_usb_boot(0, 0))
        if let Ok(mut port) = serialport::new(port_name, 115_200)
            .timeout(Duration::from_millis(400))
            .open()
        {
            let _ = port.write_data_terminal_ready(true);
            let _ = port.write_all(b"BOOTLOADER\n");
            let _ = port.flush();
            std::thread::sleep(Duration::from_millis(120));
        }
        std::thread::sleep(Duration::from_millis(250));

        // 2. Если диск еще не появился — выполняем стандартный аппаратный сброс 1200 бод DTR
        if detect_rp2040_uf2_drive().is_none() {
            if let Ok(mut port) = serialport::new(port_name, 1200)
                .timeout(Duration::from_millis(300))
                .open()
            {
                let _ = port.write_data_terminal_ready(false);
                std::thread::sleep(Duration::from_millis(150));
            }
            #[cfg(target_os = "windows")]
            {
                let _ = create_hidden_command("mode")
                    .args([port_name, "BAUD=1200", "PARITY=N", "DATA=8"])
                    .output();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = create_hidden_command("stty")
                    .args(["-F", port_name, "1200"])
                    .output();
            }
        }
    }

    emit_flash_progress(
        app,
        35,
        "waiting_drive",
        "Ожидание монтирования накопителя RPI-RP2...",
    );

    let mut found_drive = None;
    let start_wait = Instant::now();
    while start_wait.elapsed() < Duration::from_millis(7500) {
        if let Some(d) = detect_rp2040_uf2_drive() {
            found_drive = Some(d);
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    let Some(drive) = found_drive else {
        return Err(
            "Не удалось обнаружить накопитель RPI-RP2. Зажмите кнопку BOOT на плате RP2040-Zero, переподключите USB-кабель и нажмите кнопку обновления еще раз.".to_string(),
        );
    };

    let target_uf2 = Path::new(&drive).join("firmware.uf2");
    emit_flash_progress(
        app,
        65,
        "writing",
        &format!(
            "Запись прошивки ({} КБ) на накопитель {}...",
            uf2_bytes.len() / 1024,
            drive
        ),
    );

    fs::copy(uf2_path, &target_uf2).map_err(|e| {
        format!(
            "Ошибка копирования прошивки на накопитель {}: {}",
            drive, e
        )
    })?;

    emit_flash_progress(
        app,
        90,
        "rebooting",
        "Перезагрузка контроллера RP2040-Zero...",
    );
    std::thread::sleep(Duration::from_millis(2500));

    emit_flash_progress(
        app,
        100,
        "done",
        "Прошивка RP2040-Zero успешно обновлена!",
    );
    Ok(format!(
        "Прошивка RP2040-Zero успешно установлена через накопитель {} ({} КБ)",
        drive,
        uf2_bytes.len() / 1024
    ))
}

/// Нативная прошивка ESP32-S3 по Serial OTA протоколу (без стороннего ПО и Python/esptool)
fn flash_esp32_serial_ota(
    port_name: &str,
    bin_path: &Path,
    app: &AppHandle,
) -> Result<String, String> {
    if port_name.is_empty() {
        return Err("Не выбран COM-порт контроллера ESP32-S3.".to_string());
    }
    if !bin_path.exists() {
        return Err(format!("Файл прошивки BIN не найден: {}", bin_path.display()));
    }
    let fw_bytes = fs::read(bin_path)
        .map_err(|e| format!("Не удалось прочитать файл прошивки BIN: {}", e))?;
    if fw_bytes.len() < 1024 || fw_bytes[0] != 0xE9 {
        return Err(
            "Выбранный файл не является корректным образом прошивки ESP32 (.bin magic 0xE9)."
                .to_string(),
        );
    }

    emit_flash_progress(
        app,
        10,
        "connecting",
        &format!("Открытие порта {} для Serial OTA...", port_name),
    );

    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(4000))
        .open()
        .map_err(|e| format!("Не удалось открыть порт {}: {}", port_name, e))?;
    let _ = port.write_data_terminal_ready(true);
    std::thread::sleep(Duration::from_millis(120));

    let read_port = port
        .try_clone()
        .map_err(|e| format!("Ошибка клонирования дескриптора порта: {}", e))?;
    let mut reader = BufReader::new(read_port);

    let wait_for_reply =
        |reader: &mut BufReader<Box<dyn serialport::SerialPort>>, expected: &str, timeout_ms: u64| -> Result<(), String> {
            let start = Instant::now();
            let mut line = String::new();
            while start.elapsed() < Duration::from_millis(timeout_ms) {
                line.clear();
                if let Ok(n) = reader.read_line(&mut line) {
                    if n > 0 {
                        let trimmed = line.trim();
                        if trimmed.contains(expected) {
                            return Ok(());
                        }
                        if trimmed.starts_with("ERR:") {
                            return Err(format!("Контроллер вернул ошибку: {}", trimmed));
                        }
                    }
                }
            }
            Err(format!("Таймаут ожидания ответа '{}' от контроллера", expected))
        };

    emit_flash_progress(
        app,
        18,
        "ota_begin",
        &format!(
            "Инициализация раздела OTA ({} КБ)...",
            fw_bytes.len() / 1024
        ),
    );

    let begin_cmd = format!("OTA_BEGIN {}\n", fw_bytes.len());
    port.write_all(begin_cmd.as_bytes())
        .map_err(|e| format!("Ошибка отправки OTA_BEGIN: {}", e))?;
    let _ = port.flush();

    wait_for_reply(&mut reader, "OK:OTA_BEGIN", 5000)?;

    let chunk_size = 192usize;
    let total_len = fw_bytes.len();
    let mut sent = 0usize;
    let mut chunk_idx = 0usize;

    for chunk in fw_bytes.chunks(chunk_size) {
        let mut hex_str = String::with_capacity(chunk.len() * 2 + 12);
        hex_str.push_str("OTA_CHUNK ");
        for b in chunk {
            use std::fmt::Write as FmtWrite;
            let _ = write!(&mut hex_str, "{:02X}", b);
        }
        hex_str.push('\n');

        port.write_all(hex_str.as_bytes())
            .map_err(|e| format!("Ошибка передачи блока OTA: {}", e))?;

        wait_for_reply(&mut reader, "OK:OTA_CHUNK", 3000)?;

        sent += chunk.len();
        chunk_idx += 1;

        if chunk_idx.is_multiple_of(24) || sent == total_len {
            let pct = 20 + ((sent as f64 / total_len as f64) * 72.0) as u8;
            emit_flash_progress(
                app,
                pct.min(92),
                "writing",
                &format!(
                    "Запись прошивки ESP32-S3: {} / {} КБ ({}%)",
                    sent / 1024,
                    total_len / 1024,
                    ((sent * 100) / total_len)
                ),
            );
        }
    }

    emit_flash_progress(
        app,
        95,
        "finalizing",
        "Проверка контрольной суммы и активация прошивки...",
    );

    port.write_all(b"OTA_END\n")
        .map_err(|e| format!("Ошибка отправки OTA_END: {}", e))?;
    let _ = port.flush();
    wait_for_reply(&mut reader, "OK:OTA_END", 4000)?;

    drop(reader);
    drop(port);

    emit_flash_progress(
        app,
        98,
        "rebooting",
        "Перезагрузка контроллера ESP32-S3...",
    );
    std::thread::sleep(Duration::from_millis(2200));

    emit_flash_progress(
        app,
        100,
        "done",
        "Прошивка ESP32-S3 успешно обновлена!",
    );

    Ok(format!(
        "Прошивка ESP32-S3 успешно обновлена по Serial OTA ({} КБ)",
        total_len / 1024
    ))
}

/// Универсальная точка входа для обновления прошивки модуля (RP2040-Zero или ESP32-S3)
pub fn flash_controller_firmware_internal(
    port_name: &str,
    custom_file_path: Option<String>,
    app: &AppHandle,
) -> Result<String, String> {
    if let Some(ref custom) = custom_file_path {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "uf2" {
                return flash_rp2040_uf2(port_name, &p, app);
            } else if ext == "bin" {
                return flash_esp32_serial_ota(port_name, &p, app);
            } else {
                return Err(
                    "Поддерживаются только файлы прошивок .uf2 (RP2040-Zero) и .bin (ESP32-S3)."
                        .to_string(),
                );
            }
        }
    }

    let ctrl_type = detect_controller_type_by_port(port_name);
    if ctrl_type.contains("RP2040") || detect_rp2040_uf2_drive().is_some() {
        let uf2_path = find_bundled_rp2040_uf2().ok_or_else(|| {
            "Встроенный образ rp2040_zero_firmware.uf2 не найден в папке bin/firmware.".to_string()
        })?;
        flash_rp2040_uf2(port_name, &uf2_path, app)
    } else {
        let bin_path = find_bundled_esp32_bin().ok_or_else(|| {
            "Встроенный образ esp32_s3_firmware.bin не найден в папке bin/firmware.".to_string()
        })?;
        flash_esp32_serial_ota(port_name, &bin_path, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_semver() {
        assert_eq!(compare_semver("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_semver("1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_semver("1.0.0", "1.1.0"), Ordering::Less);
        assert_eq!(compare_semver("v2.0.0", "1.9.9"), Ordering::Greater);
        assert_eq!(compare_semver("1.0.0", "v1.0.0"), Ordering::Equal);
    }

    #[test]
    fn test_compare_semver_prerelease() {
        assert_eq!(compare_semver("1.1.0", "1.1.0-beta.1"), Ordering::Greater);
        assert_eq!(compare_semver("1.1.0-beta.1", "1.1.0"), Ordering::Less);
        assert_eq!(
            compare_semver("1.1.0-beta.2", "1.1.0-beta.1"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-beta.10", "1.1.0-beta.2"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-rc.1", "1.1.0-beta.9"),
            Ordering::Greater
        );
        assert_eq!(
            compare_semver("1.1.0-beta.1+build.42", "1.1.0-beta.1"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_sha256_nist_vectors() {
        assert_eq!(
            compute_sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            compute_sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn test_parse_version_manifest_windows_self_update() {
        let valid_sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let manifest = serde_json::json!({
            "version": "1.2.0",
            "published_at": "2026-11-11",
            "notes": "Self update test",
            "download_url": "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases",
            "windows_package_url": "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases/download/v1.2.0/HDMI_CEC_Studio_Windows_x64.zip",
            "windows_sha256": valid_sha.to_uppercase()
        })
        .to_string();

        let win_info = parse_version_manifest(&manifest, "1.1.0-beta.1", true).unwrap();
        assert!(win_info.has_update);
        assert!(win_info.can_self_update);
        assert_eq!(win_info.package_kind.as_deref(), Some("zip"));
        assert_eq!(win_info.package_sha256.as_deref(), Some(valid_sha));

        let linux_info = parse_version_manifest(&manifest, "1.1.0-beta.1", false).unwrap();
        assert!(linux_info.has_update);
        assert!(!linux_info.can_self_update);

        let same_ver_info = parse_version_manifest(&manifest, "1.2.0", true).unwrap();
        assert!(!same_ver_info.has_update);
        assert!(!same_ver_info.can_self_update);

        let bad_host = serde_json::json!({
            "version": "1.2.0",
            "windows": {
                "url": "https://evil.example.com/update.exe",
                "sha256": valid_sha
            }
        })
        .to_string();
        let bad_info = parse_version_manifest(&bad_host, "1.1.0-beta.1", true).unwrap();
        assert!(!bad_info.can_self_update);
        assert!(bad_info.package_url.is_none());

        let missing_sha = serde_json::json!({
            "version": "1.2.0",
            "windows_package_url": "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/raw/main/HDMI_CEC_Studio.exe"
        })
        .to_string();
        let no_sha_info = parse_version_manifest(&missing_sha, "1.1.0-beta.1", true).unwrap();
        assert!(!no_sha_info.can_self_update);
    }

    #[test]
    fn test_extract_safe_update_zip_and_zip_slip_blocked() {
        let root = std::env::temp_dir().join(format!("hdmi_cec_zip_upd_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut fake_exe = vec![b'M', b'Z'];
        fake_exe.resize(1_050_000, 0x41);
        let fw_bytes = b"UF2\nPAYLOAD";
        let prof_bytes = br#"{"id":"lg_test"}"#;

        let zip_bytes = crate::telemetry::build_zip_archive(&[
            ("HDMI_CEC_Studio_Win/HDMI_CEC_Studio.exe", &fake_exe),
            ("HDMI_CEC_Studio_Win/bin/firmware/rp2040_zero_firmware.uf2", fw_bytes),
            ("HDMI_CEC_Studio_Win/tv_profiles/lg_test.json", prof_bytes),
            ("HDMI_CEC_Studio_Win/extra/ignored.txt", b"ignore"),
        ])
        .unwrap();

        let zip_path = root.join("ok.zip");
        fs::write(&zip_path, &zip_bytes).unwrap();
        let digest = compute_file_sha256_hex(&zip_path).unwrap();
        assert_eq!(digest, compute_sha256_hex(&zip_bytes));

        let stage = root.join("stage");
        let extracted = extract_safe_update_zip(&zip_path, &stage, "HDMI_CEC_Studio.exe").unwrap();
        assert_eq!(extracted.len(), 3);
        assert!(verify_windows_pe_binary(&stage.join("HDMI_CEC_Studio.exe")).is_ok());
        assert_eq!(
            fs::read(stage.join("bin/firmware/rp2040_zero_firmware.uf2")).unwrap(),
            fw_bytes
        );
        assert_eq!(
            fs::read(stage.join("tv_profiles/lg_test.json")).unwrap(),
            prof_bytes
        );
        assert!(!stage.join("extra/ignored.txt").exists());

        let evil_zip = crate::telemetry::build_zip_archive(&[
            ("../../evil.exe", &fake_exe),
        ])
        .unwrap();
        let evil_path = root.join("evil.zip");
        fs::write(&evil_path, &evil_zip).unwrap();
        assert!(extract_safe_update_zip(&evil_path, &root.join("stage_evil"), "HDMI_CEC_Studio.exe").is_err());

        let not_pe = root.join("not_pe.exe");
        fs::write(&not_pe, vec![0x00u8; 1_050_000]).unwrap();
        assert!(verify_windows_pe_binary(&not_pe).is_err());

        let _ = fs::remove_dir_all(&root);
    }
}
