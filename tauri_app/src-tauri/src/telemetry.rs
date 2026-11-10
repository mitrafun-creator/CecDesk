use crate::audio::list_audio_endpoints_internal;
use crate::config::{
    data_storage_mode_label, find_logs_dir, find_reports_dir, get_pending_crash_report_state,
};
use crate::edid::detect_edid_os;
use crate::hardware::{safe_lock, AppState};
use crate::logger::{
    get_dropped_log_count, get_error_count, get_recent_errors, get_warn_count, log_error,
    log_info, log_warn, now_iso8601, now_unix_secs, redact_sensitive, redact_url_query,
};
use crate::platform::create_hidden_command;
use crate::types::AppConfig;
use crate::updater::{detect_controller_type_by_port, CURRENT_APP_VERSION};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

const MAX_LOG_TAIL_BYTES: usize = 512 * 1024;
const MAX_KEPT_BUNDLES: usize = 10;
pub const GITHUB_REPO_PREFIX: &str = "https://github.com/mitrafun-creator/HDMI_CEC_Emulator";
pub const GITHUB_RAW_PREFIX: &str =
    "https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/";
pub const GITHUB_NEW_ISSUE_URL: &str =
    "https://github.com/mitrafun-creator/HDMI_CEC_Emulator/issues/new";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiagnosticsSnapshot {
    pub generated_at: String,
    pub app_version: String,
    pub controller_type: String,
    pub active_port: String,
    pub os_platform: String,
    pub os_arch: String,
    pub display_session: String,
    pub install_id: String,
    pub storage_mode: String,
    pub update_channel: String,
    pub telemetry_opt_in: bool,
    pub serial_ports_count: usize,
    pub serial_ports_summary: Vec<String>,
    pub edid_pnp_id: String,
    pub edid_monitor_id: String,
    pub edid_physical_address: String,
    pub edid_hdmi_port: u8,
    pub audio_endpoints_count: usize,
    pub active_profile_id: String,
    pub session_error_count: u64,
    pub session_warn_count: u64,
    pub dropped_log_count: u64,
    pub dirty_shutdown_detected: bool,
    pub has_pending_panic: bool,
    pub recent_errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_note: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SupportBundleResult {
    pub zip_path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub included_files: Vec<String>,
    pub summary_text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BugReportResult {
    pub sent: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TelemetryResult {
    pub sent: bool,
    pub message: String,
}

/// Проверяет внешний URL для безопасного открытия в системном браузере (`https://` или `http://`).
pub fn validate_external_browser_url(raw: &str) -> Result<String, String> {
    let url = raw.trim();
    if url.is_empty() || url.len() > 8000 {
        return Err("Недопустимая длина URL".to_string());
    }
    if url
        .chars()
        .any(|c| c.is_control() || matches!(c, ' ' | '\t' | '"' | '\'' | '<' | '>' | '`' | '\\'))
    {
        return Err("URL содержит недопустимые символы или пробелы".to_string());
    }
    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return Err("Разрешены только HTTP/HTTPS ссылки".to_string());
    };
    let host = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || !host.contains('.') && !host.eq_ignore_ascii_case("localhost") {
        return Err("URL не содержит корректного домена".to_string());
    }
    Ok(url.to_string())
}

/// Строгая валидация HTTPS endpoint для телеметрии и отправки баг-репортов.
pub fn validate_https_webhook_url(raw: &str) -> Result<String, String> {
    let url = raw.trim();
    if url.is_empty() {
        return Err("Webhook URL не настроен".to_string());
    }
    if url.len() > 2048 {
        return Err("Webhook URL слишком длинный".to_string());
    }
    if url.chars().any(|c| {
        c.is_control()
            || matches!(
                c,
                ' ' | '\t' | '"' | '\'' | '<' | '>' | '`' | '\\' | ';' | '|' | '\r' | '\n'
            )
    }) {
        return Err("Webhook URL содержит запрещенные символы".to_string());
    }
    let Some(after_scheme) = url.strip_prefix("https://") else {
        return Err("Endpoint телеметрии/отчетов обязан использовать HTTPS".to_string());
    };
    let host = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || !host.contains('.') {
        return Err("Webhook URL не содержит валидного домена".to_string());
    }
    Ok(url.to_string())
}

/// Проверяет, что удаленный URL обновления/каталога ведет строго на официальный репозиторий.
pub fn validate_trusted_repo_url(raw: &str) -> Result<String, String> {
    let safe = validate_external_browser_url(raw)?;
    if safe.starts_with(GITHUB_REPO_PREFIX) || safe.starts_with(GITHUB_RAW_PREFIX) {
        Ok(safe)
    } else {
        Err("Недопустимый источник обновления: разрешен только официальный репозиторий GitHub".to_string())
    }
}

/// Проверяет и нормализует ID профиля ТВ (защита от Path Traversal).
pub fn sanitize_profile_id(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 80 {
        return Err("Некорректный ID профиля ТВ (допустимо 1..80 символов)".to_string());
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(trimmed.to_string())
    } else {
        Err(
            "ID профиля ТВ может содержать только латинские буквы, цифры, '_' и '-'".to_string(),
        )
    }
}

fn detect_display_session() -> String {
    #[cfg(target_os = "windows")]
    {
        "Windows Desktop".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if std::env::var("GAMESCOPE_WAYLAND_DISPLAY").is_ok() {
            return "Gamescope / SteamOS Gaming Mode".to_string();
        }
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return "Linux Wayland".to_string();
        }
        if std::env::var("DISPLAY").is_ok() {
            return "Linux X11".to_string();
        }
        "Linux Headless".to_string()
    }
}

pub fn build_diagnostics_snapshot(state: &AppState, user_note: Option<String>) -> DiagnosticsSnapshot {
    let cfg = safe_lock(&state.config).clone();
    let active_port = safe_lock(&state.active_port_name).clone();
    let crash = get_pending_crash_report_state();
    let edid = detect_edid_os();
    let audio_count = list_audio_endpoints_internal().len();

    let mut ports_summary = Vec::new();
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            if p.port_name.eq_ignore_ascii_case("COM1") {
                continue;
            }
            let desc = match &p.port_type {
                serialport::SerialPortType::UsbPort(u) => {
                    format!("{} (USB {:04X}:{:04X})", p.port_name, u.vid, u.pid)
                }
                _ => p.port_name.clone(),
            };
            ports_summary.push(redact_sensitive(&desc));
        }
    }

    let controller_type = if active_port.is_empty() {
        "Не подключен".to_string()
    } else {
        detect_controller_type_by_port(&active_port)
    };

    let clean_note = user_note.and_then(|n| {
        let t = redact_sensitive(n.trim());
        if t.is_empty() {
            None
        } else {
            Some(t.chars().take(1200).collect())
        }
    });

    DiagnosticsSnapshot {
        generated_at: now_iso8601(),
        app_version: CURRENT_APP_VERSION.to_string(),
        controller_type,
        active_port: if active_port.is_empty() {
            "none".to_string()
        } else {
            active_port
        },
        os_platform: std::env::consts::OS.to_string(),
        os_arch: std::env::consts::ARCH.to_string(),
        display_session: detect_display_session(),
        install_id: cfg.install_id,
        storage_mode: data_storage_mode_label().to_string(),
        update_channel: cfg.update_channel,
        telemetry_opt_in: cfg.telemetry_opt_in,
        serial_ports_count: ports_summary.len(),
        serial_ports_summary: ports_summary,
        edid_pnp_id: edid.pnp_id,
        edid_monitor_id: redact_sensitive(&edid.monitor_id),
        edid_physical_address: edid.physical_address,
        edid_hdmi_port: edid.hdmi_port_num,
        audio_endpoints_count: audio_count,
        active_profile_id: cfg.active_profile_id,
        session_error_count: get_error_count(),
        session_warn_count: get_warn_count(),
        dropped_log_count: get_dropped_log_count(),
        dirty_shutdown_detected: crash.dirty_shutdown,
        has_pending_panic: crash.has_panic_file,
        recent_errors: get_recent_errors(),
        user_note: clean_note,
    }
}

fn sanitize_json_value(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::String(s) => {
            *s = redact_sensitive(s);
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sanitize_json_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if k == "telemetry_endpoint" {
                    if let serde_json::Value::String(ref url) = v {
                        *v = serde_json::Value::String(redact_url_query(url));
                        continue;
                    }
                }
                sanitize_json_value(v);
            }
        }
        _ => {}
    }
}

pub fn sanitize_config_for_export(cfg: &AppConfig) -> serde_json::Value {
    let mut val = serde_json::to_value(cfg).unwrap_or_else(|_| serde_json::json!({}));
    sanitize_json_value(&mut val);
    val
}

fn read_redacted_log_tail(path: &Path, max_bytes: usize) -> Option<Vec<u8>> {
    let bytes = fs::read(path).ok()?;
    let slice = if bytes.len() > max_bytes {
        let mut start = bytes.len() - max_bytes;
        while start < bytes.len() && (bytes[start] & 0b1100_0000) == 0b1000_0000 {
            start += 1;
        }
        &bytes[start..]
    } else {
        &bytes[..]
    };
    let text = String::from_utf8_lossy(slice);
    let redacted = redact_sensitive(&text);
    Some(redacted.into_bytes())
}

fn unix_secs_to_dos_datetime(secs: u64) -> (u16, u16) {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let h = rem / 3_600;
    let m = (rem % 3_600) / 60;
    let s = (rem % 60) / 2;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u16;
    let mo = (if mp < 10 { mp + 3 } else { mp - 9 }) as u16;
    let yr = (y + if mo <= 2 { 1 } else { 0 }).clamp(1980, 2107) as u16;

    let dos_time = ((h as u16) << 11) | ((m as u16) << 5) | (s as u16);
    let dos_date = ((yr - 1980) << 9) | (mo << 5) | d;
    (dos_time, dos_date)
}

/// Чистый Rust ZIP архиватор (DEFLATE + CRC32 + UTF-8 flag) без внешних зависимостей.
pub fn build_zip_archive(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    let (dos_time, dos_date) = unix_secs_to_dos_datetime(now_unix_secs());
    let mut out = Vec::new();
    let mut central_dir = Vec::new();
    let entry_count: u16 = entries
        .len()
        .try_into()
        .map_err(|_| "Слишком много файлов в архиве".to_string())?;

    for (name, raw_data) in entries {
        let name_bytes = name.as_bytes();
        let name_len: u16 = name_bytes
            .len()
            .try_into()
            .map_err(|_| format!("Слишком длинное имя файла в архиве: {}", name))?;

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(raw_data);
        let crc = hasher.finalize();

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(raw_data)
            .map_err(|e| format!("Ошибка сжатия {}: {}", name, e))?;
        let compressed = encoder
            .finish()
            .map_err(|e| format!("Ошибка финализации сжатия {}: {}", name, e))?;

        let uncomp_len: u32 = raw_data
            .len()
            .try_into()
            .map_err(|_| format!("Файл {} слишком велик для ZIP", name))?;
        let comp_len: u32 = compressed
            .len()
            .try_into()
            .map_err(|_| format!("Сжатый файл {} слишком велик для ZIP", name))?;

        let local_offset: u32 = out
            .len()
            .try_into()
            .map_err(|_| "Размер ZIP архива превысил 4 ГБ".to_string())?;

        // Local file header (30 bytes)
        out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 filename flag
        out.extend_from_slice(&8u16.to_le_bytes()); // method 8 = DEFLATE
        out.extend_from_slice(&dos_time.to_le_bytes());
        out.extend_from_slice(&dos_date.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&comp_len.to_le_bytes());
        out.extend_from_slice(&uncomp_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&compressed);

        // Central directory header (46 bytes)
        central_dir.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
        central_dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central_dir.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central_dir.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
        central_dir.extend_from_slice(&8u16.to_le_bytes()); // DEFLATE
        central_dir.extend_from_slice(&dos_time.to_le_bytes());
        central_dir.extend_from_slice(&dos_date.to_le_bytes());
        central_dir.extend_from_slice(&crc.to_le_bytes());
        central_dir.extend_from_slice(&comp_len.to_le_bytes());
        central_dir.extend_from_slice(&uncomp_len.to_le_bytes());
        central_dir.extend_from_slice(&name_len.to_le_bytes());
        central_dir.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central_dir.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central_dir.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central_dir.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central_dir.extend_from_slice(&0x0000_0020u32.to_le_bytes()); // external attrs
        central_dir.extend_from_slice(&local_offset.to_le_bytes());
        central_dir.extend_from_slice(name_bytes);
    }

    let cd_offset: u32 = out
        .len()
        .try_into()
        .map_err(|_| "Размер ZIP архива превысил 4 ГБ".to_string())?;
    let cd_size: u32 = central_dir
        .len()
        .try_into()
        .map_err(|_| "Центральный каталог ZIP превысил 4 ГБ".to_string())?;

    out.extend_from_slice(&central_dir);

    // End of Central Directory (22 bytes)
    out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // disk number
    out.extend_from_slice(&0u16.to_le_bytes()); // start disk
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len

    Ok(out)
}

fn find_latest_session_log(logs_dir: &Path, current_session: &Path) -> Option<PathBuf> {
    if current_session.is_file() {
        return Some(current_session.to_path_buf());
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("cec_session_") && name.ends_with(".log") {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    if best.as_ref().map(|b| mtime > b.0).unwrap_or(true) {
                        best = Some((mtime, p));
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

fn prune_old_support_bundles(reports_dir: &Path, max_keep: usize) {
    if let Ok(entries) = fs::read_dir(reports_dir) {
        let mut zips = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("hdmi_cec_support_") && name.ends_with(".zip") {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    zips.push((p, mtime));
                }
            }
        }
        if zips.len() > max_keep {
            zips.sort_by_key(|a| std::cmp::Reverse(a.1));
            for (old, _) in zips.into_iter().skip(max_keep) {
                let _ = fs::remove_file(old);
            }
        }
    }
}

pub fn create_support_bundle_internal(
    state: &AppState,
    user_note: Option<String>,
) -> Result<SupportBundleResult, String> {
    let snapshot = build_diagnostics_snapshot(state, user_note);
    let diag_bytes = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| e.to_string())?
        .into_bytes();

    let cfg_val = sanitize_config_for_export(&safe_lock(&state.config).clone());
    let cfg_bytes = serde_json::to_string_pretty(&cfg_val)
        .map_err(|e| e.to_string())?
        .into_bytes();

    let logs_dir = find_logs_dir();
    let latest_log_bytes = read_redacted_log_tail(&state.latest_log_path, MAX_LOG_TAIL_BYTES);
    let session_log_bytes = find_latest_session_log(&logs_dir, &state.session_log_path)
        .and_then(|p| read_redacted_log_tail(&p, MAX_LOG_TAIL_BYTES));
    let trace_bytes = read_redacted_log_tail(&logs_dir.join("trace.log"), MAX_LOG_TAIL_BYTES);
    let trace_prev_bytes =
        read_redacted_log_tail(&logs_dir.join("trace.prev.log"), MAX_LOG_TAIL_BYTES);
    let panic_bytes = read_redacted_log_tail(&logs_dir.join("panic.txt"), MAX_LOG_TAIL_BYTES)
        .or_else(|| read_redacted_log_tail(&logs_dir.join("panic_last.txt"), MAX_LOG_TAIL_BYTES));

    let mut owned_entries: Vec<(&str, &[u8])> = vec![
        ("diagnostics.json", &diag_bytes),
        ("cec_studio_config.json", &cfg_bytes),
    ];
    if let Some(ref b) = latest_log_bytes {
        owned_entries.push(("logs/cec_latest.log", b));
    }
    if let Some(ref b) = session_log_bytes {
        owned_entries.push(("logs/cec_session_latest.log", b));
    }
    if let Some(ref b) = trace_bytes {
        owned_entries.push(("logs/trace.log", b));
    }
    if let Some(ref b) = trace_prev_bytes {
        owned_entries.push(("logs/trace.prev.log", b));
    }
    if let Some(ref b) = panic_bytes {
        owned_entries.push(("logs/panic.txt", b));
    }

    let zip_bytes = build_zip_archive(&owned_entries)?;
    let reports_dir = find_reports_dir();
    let filename = format!("hdmi_cec_support_{}.zip", now_unix_secs());
    let zip_path = reports_dir.join(&filename);
    let tmp_path = zip_path.with_extension("zip.tmp");
    fs::write(&tmp_path, &zip_bytes)
        .map_err(|e| format!("Не удалось записать архив отчета: {}", e))?;
    if fs::rename(&tmp_path, &zip_path).is_err() {
        fs::write(&zip_path, &zip_bytes)
            .map_err(|e| format!("Не удалось сохранить архив отчета: {}", e))?;
        let _ = fs::remove_file(&tmp_path);
    }

    prune_old_support_bundles(&reports_dir, MAX_KEPT_BUNDLES);

    let included_files: Vec<String> = owned_entries.iter().map(|(n, _)| (*n).to_string()).collect();
    let summary_text = format!(
        "App: {} | OS: {} ({}) | Session: {} | Controller: {} ({}) | EDID: {} PA={} | Errors: {} | Warns: {}",
        snapshot.app_version,
        snapshot.os_platform,
        snapshot.os_arch,
        snapshot.display_session,
        snapshot.controller_type,
        snapshot.active_port,
        snapshot.edid_pnp_id,
        snapshot.edid_physical_address,
        snapshot.session_error_count,
        snapshot.session_warn_count
    );

    log_info(
        "support",
        &format!(
            "Created support bundle {} ({} bytes, {} files)",
            filename,
            zip_bytes.len(),
            included_files.len()
        ),
    );

    Ok(SupportBundleResult {
        zip_path: zip_path.to_string_lossy().to_string(),
        filename,
        size_bytes: zip_bytes.len() as u64,
        included_files,
        summary_text,
    })
}

fn post_json_webhook(endpoint: &str, payload_json: &str) -> Result<String, String> {
    let valid_url = validate_https_webhook_url(endpoint)?;
    let mut child = create_hidden_command("curl")
        .args([
            "-sS",
            "--fail-with-body",
            "--max-time",
            "10",
            "-H",
            "Content-Type: application/json",
            "-H",
            "X-HDMI-CEC-Studio-Schema: 1.0",
            "-X",
            "POST",
            "--data-binary",
            "@-",
            &valid_url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Не удалось запустить curl для отправки отчета: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload_json.as_bytes())
            .map_err(|e| format!("Ошибка передачи данных в curl: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Ошибка ожидания ответа сервера: {}", e))?;

    if output.status.success() {
        Ok("Отправлено".to_string())
    } else {
        let stderr_txt = redact_sensitive(&String::from_utf8_lossy(&output.stderr));
        let stdout_txt = redact_sensitive(&String::from_utf8_lossy(&output.stdout));
        let detail = if !stderr_txt.trim().is_empty() {
            stderr_txt.trim().to_string()
        } else {
            stdout_txt.trim().to_string()
        };
        Err(format!(
            "Сервер вернул ошибку ({}): {}",
            output.status,
            if detail.is_empty() { "unknown" } else { &detail }
        ))
    }
}

pub fn send_bug_report_internal(
    state: &AppState,
    user_note: Option<String>,
) -> Result<BugReportResult, String> {
    let endpoint = safe_lock(&state.config).telemetry_endpoint.clone();
    let valid_endpoint = validate_https_webhook_url(&endpoint).map_err(|_| {
        "HTTPS endpoint для баг-репортов не указан. Создайте .zip отчет локально и отправьте его через GitHub Issue или укажите свой HTTPS webhook в настройках.".to_string()
    })?;

    let snapshot = build_diagnostics_snapshot(state, user_note);
    let recent_lines: Vec<String> = crate::logger::get_recent_lines()
        .into_iter()
        .rev()
        .take(50)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let payload = serde_json::json!({
        "kind": "bug_report",
        "schema_version": "1.0",
        "snapshot": snapshot,
        "recent_logs": recent_lines
    });
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    match post_json_webhook(&valid_endpoint, &json) {
        Ok(_) => {
            log_info("telemetry", "Bug report delivered to configured HTTPS webhook");
            Ok(BugReportResult {
                sent: true,
                message: "Баг-репорт отправлен на сервер сбора.".to_string(),
            })
        }
        Err(e) => {
            log_error("telemetry", &format!("Bug report webhook delivery failed: {}", e));
            Err(e)
        }
    }
}

pub fn send_telemetry_heartbeat_internal(
    state: &AppState,
    trigger: &str,
) -> Result<TelemetryResult, String> {
    let cfg = safe_lock(&state.config).clone();
    if !cfg.telemetry_opt_in {
        return Ok(TelemetryResult {
            sent: false,
            message: "Телеметрия отключена пользователем.".to_string(),
        });
    }
    if cfg.telemetry_endpoint.trim().is_empty() {
        return Ok(TelemetryResult {
            sent: false,
            message: "HTTPS endpoint телеметрии не задан.".to_string(),
        });
    }

    let valid_endpoint = validate_https_webhook_url(&cfg.telemetry_endpoint)?;
    let snapshot = build_diagnostics_snapshot(state, None);
    let payload = serde_json::json!({
        "kind": "telemetry_heartbeat",
        "schema_version": "1.0",
        "trigger": trigger,
        "timestamp": snapshot.generated_at,
        "install_id": snapshot.install_id,
        "app_version": snapshot.app_version,
        "update_channel": snapshot.update_channel,
        "os": snapshot.os_platform,
        "arch": snapshot.os_arch,
        "display_session": snapshot.display_session,
        "storage_mode": snapshot.storage_mode,
        "controller_type": snapshot.controller_type,
        "edid_pnp_id": snapshot.edid_pnp_id,
        "session_error_count": snapshot.session_error_count,
        "session_warn_count": snapshot.session_warn_count,
        "dirty_shutdown_detected": snapshot.dirty_shutdown_detected,
        "has_pending_panic": snapshot.has_pending_panic
    });
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    match post_json_webhook(&valid_endpoint, &json) {
        Ok(_) => {
            log_info("telemetry", &format!("Sent anonymous heartbeat ({})", trigger));
            Ok(TelemetryResult {
                sent: true,
                message: "Анонимная телеметрия отправлена.".to_string(),
            })
        }
        Err(e) => {
            log_warn("telemetry", &format!("Telemetry heartbeat failed: {}", e));
            Err(e)
        }
    }
}

fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 32);
    for b in input.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

pub fn build_github_issue_url_internal(
    state: &AppState,
    user_note: Option<String>,
) -> Result<String, String> {
    let snapshot = build_diagnostics_snapshot(state, user_note.clone());
    let title = format!(
        "[Beta {}] Bug report ({}, {})",
        snapshot.app_version, snapshot.os_platform, snapshot.controller_type
    );
    let note_str = snapshot
        .user_note
        .clone()
        .unwrap_or_else(|| "(Опишите проблему здесь)".to_string());
    let errors_preview = if snapshot.recent_errors.is_empty() {
        "none".to_string()
    } else {
        snapshot
            .recent_errors
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    };

    let body = format!(
        "### Описание проблемы\n{}\n\n### Диагностика\n- Версия: {}\n- ОС: {} ({}) / {}\n- Режим хранения: {}\n- Контроллер: {} ({})\n- ТВ EDID: {} / PA {}\n- Ошибки сессии: {}, Предупреждения: {}, Аварийный прошлый выход: {}\n\n### Последние ошибки\n```\n{}\n```\n\n> Прикрепите сформированный архив `hdmi_cec_support_*.zip` из папки `reports/`.",
        note_str,
        snapshot.app_version,
        snapshot.os_platform,
        snapshot.os_arch,
        snapshot.display_session,
        snapshot.storage_mode,
        snapshot.controller_type,
        snapshot.active_port,
        snapshot.edid_pnp_id,
        snapshot.edid_physical_address,
        snapshot.session_error_count,
        snapshot.session_warn_count,
        snapshot.dirty_shutdown_detected || snapshot.has_pending_panic,
        errors_preview
    );

    let full = format!(
        "{}?title={}&body={}",
        GITHUB_NEW_ISSUE_URL,
        percent_encode_query(&title),
        percent_encode_query(&body)
    );
    validate_external_browser_url(&full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    #[test]
    fn test_zip_builder_valid_structure_and_deflate_roundtrip() {
        let diag = br#"{"app_version":"1.1.0-beta.1","ok":true}"#;
        let log = "Строка лога UTF-8 с русским текстом\nSecond line\n".as_bytes();
        let archive = build_zip_archive(&[
            ("diagnostics.json", diag),
            ("logs/cec_latest.log", log),
        ])
        .expect("zip build must succeed");

        assert_eq!(&archive[0..4], b"PK\x03\x04");
        let eocd_pos = archive.len() - 22;
        assert_eq!(&archive[eocd_pos..eocd_pos + 4], b"PK\x05\x06");

        let count = u16::from_le_bytes([archive[eocd_pos + 8], archive[eocd_pos + 9]]);
        assert_eq!(count, 2);

        // Проверим первый Local File Header и распаковку DEFLATE
        let crc_read = u32::from_le_bytes([archive[14], archive[15], archive[16], archive[17]]);
        let comp_size = u32::from_le_bytes([archive[18], archive[19], archive[20], archive[21]]) as usize;
        let name_len = u16::from_le_bytes([archive[26], archive[27]]) as usize;
        let extra_len = u16::from_le_bytes([archive[28], archive[29]]) as usize;

        let mut expected_hasher = crc32fast::Hasher::new();
        expected_hasher.update(diag);
        assert_eq!(crc_read, expected_hasher.finalize());

        let data_start = 30 + name_len + extra_len;
        let compressed = &archive[data_start..data_start + comp_size];
        let mut dec = DeflateDecoder::new(compressed);
        let mut unpacked = Vec::new();
        dec.read_to_end(&mut unpacked).unwrap();
        assert_eq!(unpacked.as_slice(), diag);
    }

    #[test]
    fn test_url_and_profile_id_validation() {
        assert!(validate_external_browser_url("https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases").is_ok());
        assert!(validate_external_browser_url("https://github.com/mitrafun-creator/HDMI_CEC_Emulator/issues/new?title=Test&body=Hello%20World").is_ok());
        assert!(validate_external_browser_url("javascript:alert(1)").is_err());
        assert!(validate_external_browser_url("file:///etc/passwd").is_err());
        assert!(validate_external_browser_url("https://example.com/a\"b").is_err());

        assert!(validate_https_webhook_url("https://telemetry.example.com/v1/ingest").is_ok());
        assert!(validate_https_webhook_url("http://telemetry.example.com/v1/ingest").is_err());
        assert!(validate_https_webhook_url("https://example.com/;calc.exe").is_err());

        assert!(validate_trusted_repo_url("https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/version.json").is_ok());
        assert!(validate_trusted_repo_url("https://evil.example.com/version.json").is_err());

        assert_eq!(sanitize_profile_id("lg_simplink-2026").unwrap(), "lg_simplink-2026");
        assert!(sanitize_profile_id("../evil").is_err());
        assert!(sanitize_profile_id("foo/bar").is_err());
        assert!(sanitize_profile_id("bad.json").is_err());
    }
}
