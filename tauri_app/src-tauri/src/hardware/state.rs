use crate::types::AppConfig;
use serialport::SerialPort;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Буфер последних HUD-уведомлений для лениво создаваемого окна HUD
#[derive(Clone, Default)]
pub struct PendingHudState {
    pub last_show: Option<(Instant, serde_json::Value)>,
    pub last_sector: Option<(Instant, serde_json::Value)>,
}

/// Глобальное состояние приложения и аппаратных интерфейсов
#[derive(Clone)]
pub struct AppState {
    pub serial_writer: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub active_port_name: Arc<Mutex<String>>,
    pub connection_id: Arc<AtomicU64>,
    pub manual_disconnect: Arc<AtomicBool>,
    pub session_log_path: PathBuf,
    pub latest_log_path: PathBuf,
    pub mouse_mode_active: Arc<Mutex<bool>>,
    pub last_mouse_activity: Arc<Mutex<Instant>>,
    pub mouse_move_streak: Arc<Mutex<Option<(u8, Instant, u32)>>>,
    pub back_hold_start: Arc<Mutex<Option<Instant>>>,
    pub last_seek_press: Arc<Mutex<Option<(u8, Instant, u32)>>>,
    pub osk_visible: Arc<AtomicBool>,
    pub calibration_mode: Arc<AtomicBool>,
    pub initial_show_pending: Arc<AtomicBool>,
    pub pending_hud: Arc<Mutex<PendingHudState>>,
}

/// Безопасный захват Mutex с автоматическим восстановлением после отравления (poison resilience)
pub fn safe_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
