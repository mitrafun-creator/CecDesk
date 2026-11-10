use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_TRACE_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SESSION_LOG_BYTES: u64 = 4 * 1024 * 1024;
const RECENT_ERRORS_CAP: usize = 80;
const RECENT_LINES_CAP: usize = 160;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }
}

enum LogCmd {
    WriteTrace {
        path: PathBuf,
        line: String,
    },
    WriteSession {
        session_path: PathBuf,
        latest_path: PathBuf,
        trace_path: Option<PathBuf>,
        line: String,
    },
    Flush(Arc<(Mutex<bool>, Condvar)>),
}

struct LoggerRuntime {
    tx: SyncSender<LogCmd>,
    trace_path: PathBuf,
}

static LOGGER: OnceLock<LoggerRuntime> = OnceLock::new();
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);
static WARN_COUNT: AtomicU64 = AtomicU64::new(0);
static DROPPED_COUNT: AtomicU64 = AtomicU64::new(0);
static RECENT_ERRORS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static RECENT_LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Конвертация Unix epoch (сек, мс) в ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SS.mmmZ`) по алгоритму Говарда Хиннанта.
pub fn format_utc_iso8601(secs: u64, millis: u16) -> String {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let h = rem / 3_600;
    let m = (rem % 3_600) / 60;
    let s = rem % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = y + if mo <= 2 { 1 } else { 0 };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        yr,
        mo,
        d,
        h,
        m,
        s,
        millis.min(999)
    )
}

pub fn now_iso8601() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_utc_iso8601(dur.as_secs(), dur.subsec_millis() as u16)
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Маскирует персональные домашние каталоги, имена пользователей ОС и секреты в URL/заголовках.
pub fn redact_sensitive(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut text = input.to_string();

    for var_name in ["USERPROFILE", "HOME"] {
        if let Ok(val) = std::env::var(var_name) {
            let trimmed = val.trim();
            if trimmed.len() >= 4 {
                text = text.replace(trimmed, "<USER_HOME>");
                let fwd = trimmed.replace('\\', "/");
                if fwd != trimmed {
                    text = text.replace(&fwd, "<USER_HOME>");
                }
            }
        }
    }

    text = mask_user_segments(&text);
    text = mask_auth_secrets(&text);
    text
}

fn mask_user_segments(input: &str) -> String {
    let lower = input.to_lowercase();
    let patterns: &[(&str, &str)] = &[
        (":\\users\\", ":\\Users\\"),
        (":/users/", ":/Users/"),
        ("/var/home/", "/var/home/"),
        ("/home/", "/home/"),
        ("/users/", "/Users/"),
    ];

    let mut out = String::with_capacity(input.len());
    let mut idx = 0usize;
    let len = input.len();

    while idx < len {
        let mut matched: Option<(&str, &str)> = None;
        for &(pat_lower, pat_repl) in patterns {
            if lower[idx..].starts_with(pat_lower) {
                if (pat_lower == ":\\users\\" || pat_lower == ":/users/")
                    && idx > 0
                    && input.as_bytes()[idx - 1].is_ascii_alphabetic()
                {
                    matched = Some((pat_lower, pat_repl));
                    break;
                } else if pat_lower.starts_with('/') {
                    let prev_ok = idx == 0
                        || matches!(
                            input.as_bytes()[idx - 1],
                            b' ' | b'"' | b'\'' | b'(' | b'[' | b'=' | b':' | b'\n' | b'\r' | b'\t'
                        );
                    if prev_ok {
                        matched = Some((pat_lower, pat_repl));
                        break;
                    }
                }
            }
        }

        if let Some((pat_lower, pat_repl)) = matched {
            out.push_str(pat_repl);
            idx += pat_lower.len();
            let rest = &input[idx..];
            let seg_len = rest
                .find(['\\', '/', '"', '\'', '\n', '\r', '\t', ')', ']'])
                .unwrap_or(rest.len());
            if seg_len > 0 {
                let seg = &rest[..seg_len];
                if seg.eq_ignore_ascii_case("<USER>") || seg.eq_ignore_ascii_case("Public") {
                    out.push_str(seg);
                } else {
                    out.push_str("<USER>");
                }
                idx += seg_len;
            }
        } else if let Some(ch) = input[idx..].chars().next() {
            out.push(ch);
            idx += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

fn mask_auth_secrets(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let lower = input.to_lowercase();
    let secret_keys = [
        "token=",
        "key=",
        "api_key=",
        "apikey=",
        "secret=",
        "sig=",
        "auth=",
        "password=",
        "webhook_secret=",
    ];

    let mut i = 0usize;
    while i < input.len() {
        if lower[i..].starts_with("bearer ") {
            let prev_ok = i == 0 || !input.as_bytes()[i - 1].is_ascii_alphanumeric();
            if prev_ok {
                out.push_str("Bearer [REDACTED]");
                i += "bearer ".len();
                let tok_len = input[i..]
                    .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '&' | ')' | ']'))
                    .unwrap_or(input.len() - i);
                i += tok_len;
                continue;
            }
        }

        let mut matched_key: Option<&str> = None;
        for &k in &secret_keys {
            if lower[i..].starts_with(k) {
                let prev_ok = i == 0
                    || matches!(
                        input.as_bytes()[i - 1],
                        b'?' | b'&' | b' ' | b'"' | b'\'' | b';'
                    );
                if prev_ok {
                    matched_key = Some(k);
                    break;
                }
            }
        }

        if let Some(k) = matched_key {
            out.push_str(&input[i..i + k.len()]);
            out.push_str("[REDACTED]");
            i += k.len();
            let val_len = input[i..]
                .find(|c: char| c.is_whitespace() || matches!(c, '&' | '"' | '\'' | ')' | ']'))
                .unwrap_or(input.len() - i);
            i += val_len;
        } else if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// Убирает query-параметры URL (используется для безопасной сериализации webhook endpoint в бандлах).
pub fn redact_url_query(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match trimmed.find('?') {
        Some(pos) => format!("{}?[REDACTED]", &trimmed[..pos]),
        None => trimmed.to_string(),
    }
}

fn rotate_if_oversized(path: &Path, max_bytes: u64) {
    if max_bytes == 0 {
        return;
    }
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() >= max_bytes {
            let prev = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("log");
                path.with_file_name(format!("{}.prev.{}", stem, ext))
            } else {
                path.with_extension("prev")
            };
            let _ = fs::remove_file(&prev);
            if fs::rename(path, &prev).is_err() {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn append_line_capped(path: &Path, line: &str, max_bytes: u64) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    rotate_if_oversized(path, max_bytes);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

pub fn init_logger(logs_dir: PathBuf, session_log: PathBuf, latest_log: PathBuf, debug: bool) {
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
    let _ = fs::create_dir_all(&logs_dir);
    let trace_path = logs_dir.join("trace.log");

    let header = format!(
        "\u{FEFF}=== HDMI-CEC STUDIO LOG STARTED ({}) ===\n",
        now_iso8601()
    );
    let _ = fs::write(&latest_log, &header);
    let _ = fs::write(&session_log, &header);

    LOGGER.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<LogCmd>(1024);
        let trace_clone = trace_path.clone();
        let _ = std::thread::Builder::new()
            .name("cec-log-writer".to_string())
            .spawn(move || {
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        LogCmd::WriteTrace { path, line } => {
                            append_line_capped(&path, &line, MAX_TRACE_LOG_BYTES);
                        }
                        LogCmd::WriteSession {
                            session_path,
                            latest_path,
                            trace_path,
                            line,
                        } => {
                            append_line_capped(&session_path, &line, MAX_SESSION_LOG_BYTES);
                            append_line_capped(&latest_path, &line, MAX_SESSION_LOG_BYTES);
                            if let Some(tp) = trace_path {
                                append_line_capped(&tp, &line, MAX_TRACE_LOG_BYTES);
                            }
                        }
                        LogCmd::Flush(pair) => {
                            let (lock, cvar) = &*pair;
                            if let Ok(mut done) = lock.lock() {
                                *done = true;
                                cvar.notify_all();
                            }
                        }
                    }
                }
            });
        LoggerRuntime {
            tx,
            trace_path: trace_clone,
        }
    });
}

fn push_bounded(buf: &Mutex<VecDeque<String>>, line: String, cap: usize) {
    let mut g = match buf.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    while g.len() >= cap {
        g.pop_front();
    }
    g.push_back(line);
}

pub fn log_event(level: LogLevel, component: &str, msg: &str) {
    if level >= LogLevel::Debug && !is_debug_enabled() {
        return;
    }

    let safe_msg = redact_sensitive(msg);
    let formatted = format!("[{}] [{}] [{}] {}", now_iso8601(), level.as_str(), component, safe_msg);

    match level {
        LogLevel::Error => {
            ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
            push_bounded(&RECENT_ERRORS, formatted.clone(), RECENT_ERRORS_CAP);
        }
        LogLevel::Warn => {
            WARN_COUNT.fetch_add(1, Ordering::Relaxed);
            push_bounded(&RECENT_ERRORS, formatted.clone(), RECENT_ERRORS_CAP);
        }
        _ => {}
    }
    push_bounded(&RECENT_LINES, formatted.clone(), RECENT_LINES_CAP);

    let line_with_nl = format!("{}\n", formatted);
    if let Some(rt) = LOGGER.get() {
        if rt
            .tx
            .try_send(LogCmd::WriteTrace {
                path: rt.trace_path.clone(),
                line: line_with_nl,
            })
            .is_err()
        {
            DROPPED_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn log_error(component: &str, msg: &str) {
    log_event(LogLevel::Error, component, msg);
}

pub fn log_warn(component: &str, msg: &str) {
    log_event(LogLevel::Warn, component, msg);
}

pub fn log_info(component: &str, msg: &str) {
    log_event(LogLevel::Info, component, msg);
}

pub fn log_debug(component: &str, msg: &str) {
    log_event(LogLevel::Debug, component, msg);
}

pub fn log_session(session_path: &Path, latest_path: &Path, msg: &str) {
    let safe_msg = redact_sensitive(msg);
    let upper = safe_msg.to_uppercase();
    let level = if upper.contains("ОШИБКА") || upper.contains("ERROR") || upper.contains("FAILED") {
        ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
        LogLevel::Error
    } else if upper.contains("WARN") || upper.contains("ТАЙМАУТ") || upper.contains("TIMEOUT") {
        WARN_COUNT.fetch_add(1, Ordering::Relaxed);
        LogLevel::Warn
    } else {
        LogLevel::Info
    };

    let formatted = format!("[{}] [{}] {}", now_iso8601(), level.as_str(), safe_msg);
    if level <= LogLevel::Warn {
        push_bounded(&RECENT_ERRORS, formatted.clone(), RECENT_ERRORS_CAP);
    }
    push_bounded(&RECENT_LINES, formatted.clone(), RECENT_LINES_CAP);

    let line_with_nl = format!("{}\n", formatted);
    if let Some(rt) = LOGGER.get() {
        let trace_opt = if level <= LogLevel::Warn {
            Some(rt.trace_path.clone())
        } else {
            None
        };
        if rt
            .tx
            .try_send(LogCmd::WriteSession {
                session_path: session_path.to_path_buf(),
                latest_path: latest_path.to_path_buf(),
                trace_path: trace_opt,
                line: line_with_nl,
            })
            .is_err()
        {
            DROPPED_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        append_line_capped(session_path, &line_with_nl, MAX_SESSION_LOG_BYTES);
        append_line_capped(latest_path, &line_with_nl, MAX_SESSION_LOG_BYTES);
    }
}

pub fn log_frontend_error(source: &str, message: &str, stack: Option<&str>) {
    let clean_source = source.trim();
    let clean_msg = redact_sensitive(message.trim());
    let detail = match stack {
        Some(st) if !st.trim().is_empty() => {
            let trimmed = st.trim();
            let shortened: String = trimmed.chars().take(600).collect();
            format!("{} | stack: {}", clean_msg, redact_sensitive(&shortened))
        }
        _ => clean_msg,
    };
    log_event(
        LogLevel::Error,
        &format!("frontend:{}", if clean_source.is_empty() { "ui" } else { clean_source }),
        &detail,
    );
}

pub fn write_crash_report_sync(logs_dir: &Path, panic_summary: &str, full_report: &str) {
    let _ = fs::create_dir_all(logs_dir);
    let safe_summary = redact_sensitive(panic_summary);
    let safe_full = redact_sensitive(full_report);
    let trace_line = format!("[{}] [ERROR] [panic] {}\n", now_iso8601(), safe_summary);
    append_line_capped(&logs_dir.join("trace.log"), &trace_line, MAX_TRACE_LOG_BYTES);
    let _ = fs::write(
        logs_dir.join("panic.txt"),
        format!("Timestamp: {}\n{}\n", now_iso8601(), safe_full),
    );
}

pub fn flush_logger() {
    if let Some(rt) = LOGGER.get() {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        if rt.tx.try_send(LogCmd::Flush(Arc::clone(&pair))).is_ok() {
            let (lock, cvar) = &*pair;
            if let Ok(guard) = lock.lock() {
                if !*guard {
                    let _ = cvar.wait_timeout(guard, Duration::from_millis(250));
                }
            }
        }
    }
}

pub fn get_error_count() -> u64 {
    ERROR_COUNT.load(Ordering::Relaxed)
}

pub fn get_warn_count() -> u64 {
    WARN_COUNT.load(Ordering::Relaxed)
}

pub fn get_dropped_log_count() -> u64 {
    DROPPED_COUNT.load(Ordering::Relaxed)
}

pub fn get_recent_errors() -> Vec<String> {
    let g = match RECENT_ERRORS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    g.iter().cloned().collect()
}

pub fn get_recent_lines() -> Vec<String> {
    let g = match RECENT_LINES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    g.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utc_iso8601_formatting() {
        assert_eq!(format_utc_iso8601(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_utc_iso8601(1_700_000_000, 42),
            "2023-11-14T22:13:20.042Z"
        );
    }

    #[test]
    fn test_redact_windows_and_unix_paths() {
        let win = r#"Failed to open C:\Users\AliceSmith\AppData\Local\HDMI-CEC Studio\cfg.json"#;
        let red_w = redact_sensitive(win);
        assert!(!red_w.contains("AliceSmith"));
        assert!(red_w.contains("C:\\Users\\<USER>\\AppData\\Local"));

        let win_cyr = r#"Path C:\Users\Дмитрий\Desktop\Tools\config.json"#;
        let red_c = redact_sensitive(win_cyr);
        assert!(!red_c.contains("Дмитрий"));
        assert!(red_c.contains("<USER>"));

        let nix = "Reading /home/deck/.local/share/hdmi-cec-studio/trace.log";
        let red_n = redact_sensitive(nix);
        assert!(!red_n.contains("/deck/"));
        assert!(red_n.contains("/home/<USER>/.local"));
    }

    #[test]
    fn test_redact_tokens_and_queries() {
        let msg = "Webhook failed https://example.com/hook?token=secret123&env=beta with Bearer abc.def.ghi";
        let red = redact_sensitive(msg);
        assert!(!red.contains("secret123"));
        assert!(!red.contains("abc.def.ghi"));
        assert!(red.contains("token=[REDACTED]"));
        assert!(red.contains("Bearer [REDACTED]"));

        assert_eq!(
            redact_url_query("https://example.com/ingest?token=xyz"),
            "https://example.com/ingest?[REDACTED]"
        );
    }

    #[test]
    fn test_log_rotation_on_size_cap() {
        let dir = std::env::temp_dir().join(format!(
            "hdmi_cec_log_rot_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.log");
        let prev = dir.join("test.prev.log");

        append_line_capped(&file, &"A".repeat(80), 64);
        assert!(file.exists());
        assert!(!prev.exists());

        append_line_capped(&file, &"B".repeat(20), 64);
        assert!(prev.exists());
        assert!(file.exists());
        assert!(fs::read_to_string(&file).unwrap().starts_with('B'));

        let _ = fs::remove_dir_all(&dir);
    }
}
