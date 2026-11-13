use super::*;

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

    fs::write(&target_uf2, &uf2_bytes).map_err(|e| {
        format!(
            "Ошибка записи прошивки на накопитель {}: {}",
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
        .timeout(Duration::from_millis(1500))
        .open()
        .map_err(|e| format!("Не удалось открыть порт {}: {}", port_name, e))?;
    let _ = port.write_data_terminal_ready(true);
    let _ = port.clear(serialport::ClearBuffer::Input);
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
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        return Err("Устройство отключилось во время OTA-прошивки (EOF)".to_string());
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.contains(expected) {
                            return Ok(());
                        }
                        if trimmed.starts_with("ERR:") {
                            return Err(format!("Контроллер вернул ошибку: {}", trimmed));
                        }
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        continue;
                    }
                    Err(e) => {
                        return Err(format!("Ошибка чтения порта во время OTA: {}", e));
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

