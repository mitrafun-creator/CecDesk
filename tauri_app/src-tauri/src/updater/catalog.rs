use super::*;

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
                                    "https://raw.githubusercontent.com/mitrafun-creator/CecDesk/main/tv_profiles/{}",
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

