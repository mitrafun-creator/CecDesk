use crate::hardware::state::safe_lock;
use crate::logger::log_warn;
use crate::types::AppConfig;
use serialport::SerialPort;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Аппаратный сторожевой таймер (USB Auto-Healing & Bus Keep-Alive)
pub fn spawn_usb_watchdog(
    writer_shared: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    config_shared: Arc<Mutex<AppConfig>>,
    active_port_shared: Arc<Mutex<String>>,
    manual_disconnect_shared: Arc<AtomicBool>,
    app_handle: AppHandle,
) {
    thread::spawn(move || {
        let mut state_ping_tick = 0u32;
        loop {
            thread::sleep(Duration::from_millis(1900));
            let is_open = safe_lock(&writer_shared).is_some();
            let auto_heal = safe_lock(&config_shared).auto_healing_usb;
            let is_manual_disc = manual_disconnect_shared.load(Ordering::Relaxed);
            if !is_open && auto_heal && !is_manual_disc {
                let preferred_last = safe_lock(&active_port_shared).clone();
                if let Ok(ports) = serialport::available_ports() {
                    if !preferred_last.is_empty()
                        && ports.iter().any(|p| p.port_name.eq_ignore_ascii_case(&preferred_last))
                    {
                        let _ = app_handle.emit("cec-auto-reconnecting", &preferred_last);
                        continue;
                    }
                    for p in ports {
                        if p.port_name.to_uppercase() == "COM1" {
                            continue;
                        }
                        let is_match = match &p.port_type {
                            serialport::SerialPortType::UsbPort(usb) => {
                                let s = usb.serial_number.as_deref().unwrap_or("").to_uppercase();
                                usb.vid == 0x303A
                                    || usb.vid == 0x2E8A
                                    || s.contains("CECDESK")
                                    || s.contains("CEC-EMU")
                            }
                            _ => false,
                        };
                        if is_match {
                            let _ = app_handle.emit("cec-auto-reconnecting", &p.port_name);
                            break;
                        }
                    }
                }
            } else if is_open {
                state_ping_tick += 1;
                if state_ping_tick >= 4 {
                    state_ping_tick = 0;
                    if let Ok(mut guard) = writer_shared.lock() {
                        if let Some(ref mut p) = *guard {
                            if let Err(e) = p.write_all(b"STATE\n") {
                                log_warn("watchdog", &format!("Keep-alive STATE write failed: {}", e));
                                *guard = None;
                                let _ = app_handle.emit("cec-disconnected", ());
                            }
                        }
                    }
                }
            }
        }
    });
}
