/**
 * hardware_status.js - Управление статусом адаптера, линией CEC и умным питанием ТВ
 */

let lastHwStatus = { usbConnected: false, tvConnected: false, lineHigh: false, port: "" };
let isTvPowerOn = false;
let lineLowDebounceTimer = null;
let confirmedLineLow = false;

/**
 * Единая кнопка включения/выключения ТВ в зависимости от реального статуса
 */
function updateTvPowerButtonUi(isOn) {
  isTvPowerOn = !!isOn;
  const btn = document.getElementById("btn-tv-power");
  const lbl = document.getElementById("lbl-tv-power");
  const ico = document.getElementById("ico-tv-power");
  if (!btn || !lbl || !ico) return;

  if (isTvPowerOn) {
    btn.className = "apple-btn power-toggle-btn is-on";
    lbl.textContent = typeof t === "function" ? t("remote.turn_off_tv") : "Выключить ТВ";
    ico.innerHTML = '<path d="M18.36 6.64a9 9 0 1 1-12.73 0"></path><line x1="12" y1="2" x2="12" y2="12"></line>';
    const desc = document.getElementById("min-status-desc");
    if (desc) desc.textContent = typeof t === "function" ? t("remote.connection_active") : "Телевизор включен";
  } else {
    btn.className = "apple-btn power-toggle-btn is-off";
    lbl.textContent = typeof t === "function" ? t("remote.turn_on_tv") : "Включить ТВ";
    ico.innerHTML = '<path d="M18.36 6.64a9 9 0 1 1-12.73 0"></path><line x1="12" y1="2" x2="12" y2="12"></line>';
    const desc = document.getElementById("min-status-desc");
    if (desc) desc.textContent = typeof t === "function" ? t("remote.status_offline") : "В режиме сна";
  }
}

/**
 * Интеллектуальное переключение питания
 */
async function toggleTvPower() {
  if (isTvPowerOn) {
    addLog("[Питание] Выключение экрана телевизора...", "log-tx");
    await standbyTv();
    updateTvPowerButtonUi(false);
  } else {
    addLog("[Питание] Включение экрана телевизора...", "log-tx");
    await wakeAndSelectTv();
    updateTvPowerButtonUi(true);
  }
}

/**
 * Лаконичное обновление аппаратного статуса без лишней детализации
 */
function updateHardwareStatusUi(usbState, tvState, remoteState) {
  if (usbState) {
    if (usbState.connected !== undefined) lastHwStatus.usbConnected = !!usbState.connected;
    if (usbState.port) lastHwStatus.port = usbState.port;
  }

  if (tvState && tvState.checked) {
    if (tvState.lineHigh) {
      if (lineLowDebounceTimer) {
        clearTimeout(lineLowDebounceTimer);
        lineLowDebounceTimer = null;
      }
      confirmedLineLow = false;
      lastHwStatus.tvConnected = !!tvState.tvConnected;
      lastHwStatus.lineHigh = true;
    } else {
      // Защита от мигания при отправке фреймов: линия физически в нуле во время передачи байта
      if (!confirmedLineLow && !lineLowDebounceTimer) {
        lineLowDebounceTimer = setTimeout(() => {
          confirmedLineLow = true;
          lastHwStatus.lineHigh = false;
          lastHwStatus.tvConnected = false;
          updateHardwareStatusUi(null, { checked: true, lineHigh: false, tvConnected: false });
        }, 1200);
      }
    }
  }

  const curPort = lastHwStatus.port || selectedPort || "COM6";

  // 1. USB адаптер
  const usbBadge = document.getElementById("hw-usb-badge");

  if (lastHwStatus.usbConnected) {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(true);
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-online";
      const connStr = typeof t === "function" ? t("remote.status_connected") : "Подключен";
      usbBadge.textContent = `${connStr} (${curPort})`;
    }
  } else if (usbState && usbState.reconnecting) {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(false);
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-warning";
      usbBadge.textContent = typeof t === "function" ? t("status.reconnecting") : "Поиск...";
    }
  } else {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(false);
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-error";
      usbBadge.textContent = typeof t === "function" ? t("remote.status_disconnected") : "Отключен";
    }
  }

  // 2. Линия CEC / ТВ
  const cecBadge = document.getElementById("hw-cec-badge");

  if (!lastHwStatus.usbConnected) {
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-neutral";
      cecBadge.textContent = typeof t === "function" ? t("status.line_no_power") : "Нет питания";
    }
  } else if (lastHwStatus.lineHigh) {
    if (lastHwStatus.tvConnected) {
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-online";
        cecBadge.textContent = typeof t === "function" ? t("remote.status_online") : "3.3V ТВ в сети";
      }
    } else {
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-warning";
        cecBadge.textContent = typeof t === "function" ? t("status.line_free_badge") : "Шина свободна";
      }
    }
  } else if (confirmedLineLow) {
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-error";
      cecBadge.textContent = typeof t === "function" ? t("status.line_err_badge") : "0V / Ошибка";
    }
  } else {
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-warning";
      cecBadge.textContent = typeof t === "function" ? t("remote.syncing") : "Опрос шины...";
    }
  }

  // 3. Мост пульта
  const remBadge = document.getElementById("hw-remote-badge");
  const chkRemote = document.getElementById("chk-remote-os");
  const isPass = remoteState !== undefined ? remoteState : (chkRemote ? chkRemote.checked : true);

  if (isPass) {
    if (remBadge) {
      remBadge.className = "hw-badge badge-online";
      remBadge.textContent = typeof t === "function" ? t("remote.status_enabled") : "Включен";
    }
  } else {
    if (remBadge) {
      remBadge.className = "hw-badge badge-neutral";
      remBadge.textContent = typeof t === "function" ? t("remote.status_disabled") : "Выключен";
    }
  }

  // Обновление главного верхнего индикатора статуса
  if (!lastHwStatus.usbConnected) {
    setHumanStatus(
      "offline",
      typeof t === "function" ? t("status.usb_disconnected") : "USB: Отключен",
      typeof t === "function" ? t("status.usb_disconnected_desc") : "Подключите адаптер к компьютеру"
    );
  } else if (lastHwStatus.lineHigh && lastHwStatus.tvConnected) {
    setHumanStatus(
      "online",
      typeof t === "function" ? t("status.ready_online") : "Готово • ТВ в сети",
      typeof t === "function" ? t("status.ready_online_desc") : "Связь с телевизором активна"
    );
  } else if (lastHwStatus.lineHigh && !lastHwStatus.tvConnected) {
    setHumanStatus(
      "standby",
      typeof t === "function" ? t("status.tv_no_response") : "ТВ не отвечает",
      typeof t === "function" ? t("status.tv_no_response_desc") : "Проверьте HDMI кабель или питание ТВ"
    );
  } else if (confirmedLineLow) {
    setHumanStatus(
      "offline",
      typeof t === "function" ? t("status.line_error") : "Линия CEC 0V",
      typeof t === "function" ? t("status.line_error_desc") : "Короткое замыкание или ТВ выключен из розетки"
    );
  } else {
    setHumanStatus(
      "standby",
      typeof t === "function" ? t("remote.syncing") : "Опрос шины...",
      typeof t === "function" ? t("remote.syncing") : "Проверка связи с телевизором..."
    );
  }

  // Управление размытием и модальным окном раздела «Пульт» при отсутствии связи с ПК или ТВ
  const isTvOnline = lastHwStatus.usbConnected && lastHwStatus.lineHigh && lastHwStatus.tvConnected;
  updateRemoteOfflineOverlay(isTvOnline, !!(usbState && usbState.reconnecting), curPort);

  if (typeof updateDynamicWindowTitle === "function") {
    updateDynamicWindowTitle();
  }
}

/**
 * Показ/скрытие модального перекрытия раздела «Пульт», если нет связи с модулем или ТВ не отвечает
 */
function updateRemoteOfflineOverlay(isTvOnline, isReconnecting, curPort) {
  const stage = document.getElementById("remote-stage");
  const overlay = document.getElementById("remote-offline-overlay");
  const reasonEl = document.getElementById("remote-offline-reason");
  if (!stage || !overlay) return;

  if (isTvOnline) {
    stage.classList.remove("is-blurred");
    overlay.classList.remove("visible");
  } else {
    stage.classList.add("is-blurred");
    overlay.classList.add("visible");
    if (reasonEl) {
      if (!lastHwStatus.usbConnected) {
        if (isReconnecting) {
          reasonEl.textContent = typeof t === "function"
            ? t("remote.offline_reason_reconnecting", "Поиск и переподключение к USB-модулю...")
            : "Поиск и переподключение к USB-модулю...";
        } else {
          reasonEl.textContent = typeof t === "function"
            ? t("remote.offline_reason_no_usb", "Нет связи с ПК: модуль отключен или порт не отвечает")
            : "Нет связи с ПК: модуль отключен или порт не отвечает";
        }
      } else if (confirmedLineLow) {
        reasonEl.textContent = typeof t === "function"
          ? t("remote.offline_reason_line_low", "Линия HDMI-CEC 0V: проверьте кабель или питание ТВ")
          : "Линия HDMI-CEC 0V: проверьте кабель или питание ТВ";
      } else {
        const baseMsg = typeof t === "function"
          ? t("remote.offline_reason_tv_no_resp", "Модуль подключен, но ТВ или пульт не отвечает")
          : "Модуль подключен, но ТВ или пульт не отвечает";
        reasonEl.textContent = curPort ? `${baseMsg} (${curPort})` : baseMsg;
      }
    }
  }
}

/**
 * Кнопка «Обновить» внутри модального окна отсутствия связи с телевизором
 */
async function refreshRemoteConnectionModal() {
  const ico = document.getElementById("ico-remote-offline-refresh");
  const btn = document.getElementById("btn-remote-offline-refresh");
  const reasonEl = document.getElementById("remote-offline-reason");
  if (ico) {
    ico.classList.add("spinning");
    setTimeout(() => ico.classList.remove("spinning"), 650);
  }
  if (btn) {
    btn.disabled = true;
    setTimeout(() => { btn.disabled = false; }, 650);
  }
  if (reasonEl) {
    reasonEl.textContent = typeof t === "function"
      ? t("remote.offline_reason_checking", "Опрос модуля и проверка связи с телевизором...")
      : "Опрос модуля и проверка связи с телевизором...";
  }

  if (typeof refreshPorts === "function") {
    await refreshPorts();
  }

  if (!isConnected || (selectedPort && typeof connectedPort !== "undefined" && selectedPort !== connectedPort)) {
    if (typeof connectDeviceNow === "function") {
      await connectDeviceNow(true);
    }
  } else {
    await checkHardwareLineNow();
    setTimeout(() => sendCmd("RAW 00 8F"), 220);
  }
}

/**
 * Ручной опрос состояния шины CEC
 */
async function checkHardwareLineNow() {
  const btn = document.getElementById("btn-refresh-hw");
  if (btn) {
    btn.classList.add("spinning");
    setTimeout(() => btn.classList.remove("spinning"), 500);
  }
  const cecBadge = document.getElementById("hw-cec-badge");
  if (cecBadge) {
    cecBadge.className = "hw-badge badge-warning";
    cecBadge.textContent = "Замер...";
  }
  await sendCmd("STATE");
}

/**
 * Установка читаемого пользователем глобального статуса
 */
function setHumanStatus(stateType, topLabel, detailText) {
  const led = document.getElementById("top-led");
  if (led) led.className = "status-led " + stateType;
  const topText = document.getElementById("top-status-text");
  if (topText) topText.textContent = topLabel;
  if (detailText) {
    const desc = document.getElementById("min-status-desc");
    if (desc) desc.textContent = detailText;
  }
}

/**
 * Разбор входящей строки протокола CEC и обновление индикаторов UI
 */
function decodeCecLine(line) {
  if (line.includes("Logical address assigned: 4") || line.startsWith("OK:READY_LA_4") || line.startsWith("OK:AUTO_ANNOUNCED")) {
    setHumanStatus("online", "Готово", "Телевизор обнаружен и готов");
    setTimeout(() => sendCmd("RAW 00 8F"), 200);
    return "Канал синхронизирован";
  }

  if (line.includes("Transmit failed, no acknowledge")) {
    return "Проверка шины";
  }

  if (line.startsWith("OK:LINE_")) {
    const isHigh = line.includes("LINE_HIGH");
    const hasPull = line.includes("EXT_PULLUP_YES");
    updateHardwareStatusUi(
      { connected: isConnected, port: selectedPort },
      { checked: true, tvConnected: hasPull, lineHigh: isHigh },
      document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
    );
    return isHigh && hasPull
      ? "Замер: 3.3V (ТВ в сети, контакт надежен)"
      : (isHigh ? "Замер: Шина свободна (3.3V ESP32, ТВ не обнаружен)" : "Замер: КЗ на шине CEC (0V)");
  }

  if (line.startsWith("OK:REMOTE_CAPTURE_ON")) {
    setHumanStatus("online", "Готово", "Пульт подключен");
    return "Перехват пульта включен";
  }

  if (!(line.startsWith("RX ") || line.startsWith("TX "))) return "";
  if (line.startsWith("RX ")) {
    // Любой входящий пакет от телевизора подтверждает живую связь по шине HDMI-CEC
    lastHwStatus.tvConnected = true;
    lastHwStatus.lineHigh = true;
    confirmedLineLow = false;
    updateHardwareStatusUi(
      { connected: isConnected, port: selectedPort },
      { checked: true, tvConnected: true, lineHigh: true },
      document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
    );
  }
  const parts = line.substring(3).split(":");
  if (parts.length < 2) return "";
  const bytes = parts[1].trim().split(/\s+/).filter(Boolean).map(x => parseInt(x, 16));
  if (bytes.length === 0) return "Опрос";

  const op = bytes[0];
  const name = CEC_OPCODES[op] || `0x${op.toString(16).toUpperCase()}`;

  // Распознавание нажатий кнопок в режиме калибровки прямо из потока RX
  if (op === 0x44 && bytes.length >= 2 && typeof isCalibratingRemote !== "undefined" && isCalibratingRemote) {
    const kHex = "0x" + bytes[1].toString(16).toUpperCase().padStart(2, "0");
    handleCalibrationKeyCapture(kHex);
  }

  // Статус питания (0x90)
  if (op === 0x90 && bytes.length >= 2) {
    const p = POWER_MAP[bytes[1]] || "";
    if (bytes[1] === 0) {
      setHumanStatus("online", "Готово", "Телевизор включен");
      updateTvPowerButtonUi(true);
    } else {
      setHumanStatus("standby", "Настройка", "В режиме сна");
      updateTvPowerButtonUi(false);
    }
    return `${name} (${p})`;
  }

  // Включение экрана со стороны ТВ
  if (op === 0x04) {
    updateTvPowerButtonUi(true);
  } else if (op === 0x36) {
    updateTvPowerButtonUi(false);
  }

  return name;
}
