use crate::config::{find_base_dir, find_profiles_dir, save_atomic};
use crate::platform::create_hidden_command;
use crate::types::TvProfile;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const CURRENT_APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/tv_profiles/catalog.json";
pub const DEFAULT_APP_VERSION_URL: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/version.json";
pub const LATEST_BUNDLED_FW_VERSION: &str = "1.1.0";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppVersionInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: String,
    pub download_url: String,
    pub published_at: String,
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

/// Кроссплатформенное семантическое сравнение версий (SemVer: "1.2.3" vs "1.2.4")
pub fn compare_semver(v1: &str, v2: &str) -> Ordering {
    let parse_parts = |s: &str| -> (u32, u32, u32) {
        let clean = s.trim().trim_start_matches('v').trim_start_matches('V');
        let mut parts = clean.split('.').filter_map(|p| p.parse::<u32>().ok());
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    };
    let p1 = parse_parts(v1);
    let p2 = parse_parts(v2);
    p1.cmp(&p2)
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

/// Генерация локального манифеста каталога профилей для отправки в онлайн-репозиторий
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

/// Онлайн-синхронизация каталога профилей телевизоров
pub fn sync_tv_profiles_online() -> ProfileSyncResult {
    let prof_dir = find_profiles_dir();
    let _ = fs::create_dir_all(&prof_dir);

    let _ = generate_local_catalog_manifest(&prof_dir);

    let manifest_txt = match fetch_url_text(DEFAULT_CATALOG_URL, 6) {
        Ok(t) => t,
        Err(e) => {
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
        let target_file = prof_dir.join(format!("{}.json", item.id));
        let needs_fetch = !target_file.exists();

        if needs_fetch {
            if let Ok(profile_json) = fetch_url_text(&item.url, 5) {
                if serde_json::from_str::<TvProfile>(&profile_json).is_ok()
                    && save_atomic(&target_file, &profile_json).is_ok()
                {
                    updated += 1;
                }
            }
        }
    }

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

/// Проверка наличия новых версий приложения HDMI-CEC Studio
pub fn check_app_updates() -> AppVersionInfo {
    let current_ver = CURRENT_APP_VERSION.to_string();

    if let Ok(txt) = fetch_url_text(DEFAULT_APP_VERSION_URL, 5) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
            let latest_ver = val["version"].as_str().unwrap_or("1.0.0").to_string();
            let has_update = compare_semver(&latest_ver, &current_ver) == Ordering::Greater;
            let release_notes = val["notes"]
                .as_str()
                .unwrap_or("Улучшения стабильности и новые функции.")
                .to_string();
            let download_url = val["download_url"]
                .as_str()
                .unwrap_or("https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases")
                .to_string();
            let published_at = val["published_at"].as_str().unwrap_or("").to_string();

            return AppVersionInfo {
                current_version: current_ver,
                latest_version: latest_ver,
                has_update,
                release_notes,
                download_url,
                published_at,
            };
        }
    }

    AppVersionInfo {
        current_version: current_ver.clone(),
        latest_version: current_ver,
        has_update: false,
        release_notes: "Установлена актуальная версия приложения.".to_string(),
        download_url: "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases".to_string(),
        published_at: "2026-11-09".to_string(),
    }
}

/// Поиск директории со встроенными образами прошивок (bin/firmware)
pub fn find_firmware_dir() -> PathBuf {
    let base = find_base_dir();
    let bin_fw = base.join("bin").join("firmware");
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
    let _ = fs::create_dir_all(&bin_fw);
    bin_fw
}

pub fn find_bundled_rp2040_uf2() -> Option<PathBuf> {
    let p1 = find_firmware_dir().join("rp2040_zero_firmware.uf2");
    if p1.exists() {
        return Some(p1);
    }
    let p2 = find_base_dir()
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
    let p2 = find_base_dir()
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
}
