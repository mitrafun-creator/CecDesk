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
    "https://raw.githubusercontent.com/mitrafun-creator/CecDesk/main/tv_profiles/catalog.json";
pub const DEFAULT_APP_VERSION_URL: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/CecDesk/main/version.json";
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
        .args([
            "-fsSL",
            "--proto",
            "=https",
            "--max-time",
            &timeout_secs.to_string(),
            url,
        ])
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

mod archive;
mod catalog;
mod firmware_flasher;
mod manifest;
mod self_update;

pub use archive::*;
pub use catalog::*;
pub use firmware_flasher::*;
pub use manifest::*;
pub use self_update::*;

#[cfg(test)]
mod tests;
