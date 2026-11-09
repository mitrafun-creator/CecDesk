/**
 * hardware_status.js - Управление статусом оборудования, шиной CEC и умным питанием ТВ
 */

let lastHwStatus = { usbConnected: false, tvConnected: false, lineHigh: false, port: "" };
let tvPowerState = "syncing"; // "syncing" | "on" | "off"
let lineLowDebounceTimer = null;
let confirmedLineLow = false;
let tvPowerPingWatchdog = null;
let lastTvPowerVerifiedTime = 0;

/**
 * Единая умная кнопка питания ТВ: Включить / Выключить / Определение состояния (серая с вращающимся спиннером)
 * @param {"syncing" | boolean | "on" | "off"} state
 */
function updateTvPowerButtonUi(state) {
  const btn = document.getElementById("btn-tv-power");
  const lbl = document.getElementById("lbl-tv-power");
  const ico = document.getElementById("ico-tv-power");
  const desc = document.getElementById("min-status-desc");

  if (state === "syncing" || state === null || state === undefined) {
    tvPowerState = "syncing";
    if (btn) {
      btn.className = "apple-btn power-toggle-btn is-syncing";
      btn.disabled = true;
    }
    if (lbl) {
      lbl.textContent = typeof t === "function" ? t("remote.syncing_tv_state", "Определение состояния...") : "Определение состояния...";
    }
    if (ico) {
      ico.className = "ico ico-lg ico-spin";
      ico.innerHTML = '<circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="2.5" fill="none" stroke-dasharray="28" stroke-dashoffset="10" stroke-linecap="round"></circle>';
    }
    if (desc) {
      desc.textContent = typeof t === "function" ? t("remote.syncing", "Синхронизация...") : "Синхронизация...";
    }
    updateSystemDiagnostics();
    return;
  }

  const isOn = (state === true || state === "on");
  tvPowerState = isOn ? "on" : "off";
  lastTvPowerVerifiedTime = Date.now();

  if (btn) {
    btn.disabled = false;
    btn.className = isOn ? "apple-btn power-toggle-btn is-on" : "apple-btn power-toggle-btn is-off";
  }
  if (ico) {
    ico.className = "ico ico-lg";
    ico.innerHTML = '<path d="M18.36 6.64a9 9 0 1 1-12.73 0"></path><line x1="12" y1="2" x2="12" y2="12"></line>';
  }

  if (isOn) {
    if (lbl) lbl.textContent = typeof t === "function" ? t("remote.turn_off_tv") : "Выключить ТВ";
    if (desc) desc.textContent = typeof t === "function" ? t("remote.connection_active") : "Телевизор включен";
  } else {
    if (lbl) lbl.textContent = typeof t === "function" ? t("remote.turn_on_tv") : "Включить ТВ";
    if (desc) desc.textContent = typeof t === "function" ? t("remote.status_offline") : "В режиме сна";
  }

  updateSystemDiagnostics();
}

/**
 * Интеллектуальное переключение питания
 */
async function toggleTvPower() {
  if (tvPowerState === "syncing") {
    return; // Кнопка неактивна при неопределенном состоянии
  }

  if (typeof window.setAmbientMood === "function") {
    window.setAmbientMood("amber", 2200);
  }

  if (tvPowerState === "on") {
    addLog("[Питание] Выключение экрана телевизора...", "log-tx");
    await standbyTv();
    updateTvPowerButtonUi("off");
    if (typeof window.setAmbientMood === "function") window.setAmbientMood("green", 1800);
  } else {
    addLog("[Питание] Включение экрана телевизора...", "log-tx");
    await wakeAndSelectTv();
    updateTvPowerButtonUi("on");
    if (typeof window.setAmbientMood === "function") window.setAmbientMood("green", 1800);
  }
}

/**
 * Обновление статусов оборудования: цветные лампочки + всплывающая подсказка с деталями
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
      if (!confirmedLineLow && !lineLowDebounceTimer) {
        lineLowDebounceTimer = setTimeout(() => {
          confirmedLineLow = true;
          lastHwStatus.lineHigh = false;
          lastHwStatus.tvConnected = false;
          updateTvPowerButtonUi("syncing");
          updateHardwareStatusUi(null, { checked: true, lineHigh: false, tvConnected: false });
        }, 1200);
      }
    }
  }

  const curPort = lastHwStatus.port || selectedPort || "COM10";

  // 1. USB-контроллер (лампочка #lamp-usb)
  const lampUsb = document.getElementById("lamp-usb");
  const usbBadge = document.getElementById("hw-usb-badge"); // fallback для совместимости

  if (lastHwStatus.usbConnected) {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(true);
    const tipText = `Подключен (${curPort})`;
    if (lampUsb) {
      lampUsb.className = "status-lamp lamp-green";
      lampUsb.setAttribute("data-tooltip", tipText);
      lampUsb.setAttribute("title", tipText);
    }
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-online";
      usbBadge.textContent = tipText;
    }
  } else if (usbState && usbState.reconnecting) {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(false);
    const tipText = "Поиск контроллера...";
    if (lampUsb) {
      lampUsb.className = "status-lamp lamp-yellow";
      lampUsb.setAttribute("data-tooltip", tipText);
      lampUsb.setAttribute("title", tipText);
    }
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-warning";
      usbBadge.textContent = tipText;
    }
  } else {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(false);
    const tipText = "Отключен";
    if (lampUsb) {
      lampUsb.className = "status-lamp lamp-gray";
      lampUsb.setAttribute("data-tooltip", tipText);
      lampUsb.setAttribute("title", tipText);
    }
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-error";
      usbBadge.textContent = tipText;
    }
    updateTvPowerButtonUi("syncing");
  }

  // 2. Связь с ТВ (CEC) (лампочка #lamp-cec)
  const lampCec = document.getElementById("lamp-cec");
  const cecBadge = document.getElementById("hw-cec-badge");

  if (!lastHwStatus.usbConnected) {
    const tipText = "Нет питания USB";
    if (lampCec) {
      lampCec.className = "status-lamp lamp-gray";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.setAttribute("title", tipText);
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-neutral";
      cecBadge.textContent = tipText;
    }
  } else if (lastHwStatus.lineHigh) {
    if (lastHwStatus.tvConnected) {
      const tipText = "3.3V ТВ в сети (Контакт надежен)";
      if (lampCec) {
        lampCec.className = "status-lamp lamp-green";
        lampCec.setAttribute("data-tooltip", tipText);
        lampCec.setAttribute("title", tipText);
      }
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-online";
        cecBadge.textContent = "3.3V ТВ в сети";
      }
    } else {
      const tipText = "Шина свободна (ТВ не отвечает)";
      if (lampCec) {
        lampCec.className = "status-lamp lamp-yellow";
        lampCec.setAttribute("data-tooltip", tipText);
        lampCec.setAttribute("title", tipText);
      }
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-warning";
        cecBadge.textContent = "Шина свободна";
      }
    }
  } else if (confirmedLineLow) {
    const tipText = "0V / Ошибка линии CEC (КЗ или кабель)";
    if (lampCec) {
      lampCec.className = "status-lamp lamp-red";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.setAttribute("title", tipText);
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-error";
      cecBadge.textContent = "0V / Ошибка";
    }
    if (typeof window.setAmbientMood === "function") {
      window.setAmbientMood("red");
    }
  } else {
    const tipText = "Опрос шины...";
    if (lampCec) {
      lampCec.className = "status-lamp lamp-yellow";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.setAttribute("title", tipText);
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-warning";
      cecBadge.textContent = "Опрос...";
    }
  }

  // 3. Перехват пульта (лампочка #lamp-remote)
  const lampRemote = document.getElementById("lamp-remote");
  const remBadge = document.getElementById("hw-remote-badge");
  const chkRemote = document.getElementById("chk-remote-os");
  const isPass = remoteState !== undefined ? remoteState : (chkRemote ? chkRemote.checked : true);

  if (isPass) {
    const tipText = "Включен (управление ПК)";
    if (lampRemote) {
      lampRemote.className = "status-lamp lamp-green";
      lampRemote.setAttribute("data-tooltip", tipText);
      lampRemote.setAttribute("title", tipText);
    }
    if (remBadge) {
      remBadge.className = "hw-badge badge-online";
      remBadge.textContent = "Включен";
    }
  } else {
    const tipText = "Выключен (пульт освобожден для ТВ)";
    if (lampRemote) {
      lampRemote.className = "status-lamp lamp-gray";
      lampRemote.setAttribute("data-tooltip", tipText);
      lampRemote.setAttribute("title", tipText);
    }
    if (remBadge) {
      remBadge.className = "hw-badge badge-neutral";
      remBadge.textContent = "Выключен";
    }
  }

  // Обновление состояния Ambient Fluid
  if (typeof window.setAmbientMood === "function") {
    if (!lastHwStatus.usbConnected || confirmedLineLow) {
      window.setAmbientMood("red");
    } else if (lastHwStatus.lineHigh && lastHwStatus.tvConnected) {
      window.setAmbientMood("blue");
    }
  }

  // Проверка актуальности состояния питания телевизора
  ensureTvPowerFreshness();

  // Обновление детальной системной диагностики
  updateSystemDiagnostics();

  // Управление размытием раздела «Пульт»
  const isTvOnline = lastHwStatus.usbConnected && lastHwStatus.lineHigh && lastHwStatus.tvConnected;
  updateRemoteOfflineOverlay(isTvOnline, !!(usbState && usbState.reconnecting), curPort);

  if (typeof updateDynamicWindowTitle === "function") {
    updateDynamicWindowTitle();
  }
}

/**
 * Проверка свежести пинга питания ТВ: если за 35 секунд не было подтверждения,
 * кнопка переводится в режим безопасного определения ("syncing").
 */
function ensureTvPowerFreshness() {
  if (!lastHwStatus.usbConnected || confirmedLineLow) {
    if (tvPowerState !== "syncing") {
      updateTvPowerButtonUi("syncing");
    }
    return;
  }

  if (tvPowerState !== "syncing" && lastTvPowerVerifiedTime > 0) {
    const elapsed = Date.now() - lastTvPowerVerifiedTime;
    if (elapsed > 35000) {
      updateTvPowerButtonUi("syncing");
      sendCmd("RAW 00 8F"); // фоновый повторный запрос статуса
    }
  }
}

/**
 * Обновление подробной аппаратной диагностики в разделе «Система»
 */
function updateSystemDiagnostics() {
  const elPort = document.getElementById("diag-usb-port");
  const elConn = document.getElementById("diag-usb-conn");
  const elChip = document.getElementById("diag-usb-chip");

  const elLevel = document.getElementById("diag-cec-level");
  const elPull = document.getElementById("diag-cec-pull");
  const elContact = document.getElementById("diag-cec-contact");

  const elPwr = document.getElementById("diag-tv-pwr");
  const elPhys = document.getElementById("diag-tv-phys");
  const elLa = document.getElementById("diag-tv-la");

  const elRemActive = document.getElementById("diag-remote-active");
  const elRemKeys = document.getElementById("diag-remote-keys-cnt");
  const elRemSubsys = document.getElementById("diag-remote-subsys");

  const curPort = lastHwStatus.port || selectedPort || "COM10";

  if (elPort) elPort.textContent = lastHwStatus.usbConnected ? curPort : "Не подключен";
  if (elConn) {
    elConn.textContent = lastHwStatus.usbConnected ? "Связь установлена (115200)" : "Отключен";
    elConn.style.color = lastHwStatus.usbConnected ? "var(--apple-green)" : "var(--apple-red)";
  }
  if (elChip) elChip.textContent = "RP2040-Zero (USB CDC • GP0/GP1)";

  if (elLevel) {
    if (!lastHwStatus.usbConnected) {
      elLevel.textContent = "Нет питания";
      elLevel.style.color = "var(--text-secondary)";
    } else if (lastHwStatus.lineHigh) {
      elLevel.textContent = "3.3V (Высокий уровень)";
      elLevel.style.color = "var(--apple-green)";
    } else if (confirmedLineLow) {
      elLevel.textContent = "0V (КЗ / Обрыв)";
      elLevel.style.color = "var(--apple-red)";
    } else {
      elLevel.textContent = "Опрос шины...";
      elLevel.style.color = "var(--apple-amber)";
    }
  }

  if (elPull) {
    elPull.textContent = lastHwStatus.tvConnected ? "Активна (ТВ в сети 27k)" : "Внутренняя RP2040";
  }

  if (elContact) {
    elContact.textContent = (lastHwStatus.lineHigh && lastHwStatus.tvConnected) ? "Надежный контакт" : "Нет ответа ТВ";
    elContact.style.color = (lastHwStatus.lineHigh && lastHwStatus.tvConnected) ? "var(--apple-green)" : "var(--apple-amber)";
  }

  if (elPwr) {
    if (tvPowerState === "on") {
      elPwr.textContent = "Включен (ON)";
      elPwr.style.color = "var(--apple-green)";
    } else if (tvPowerState === "off") {
      elPwr.textContent = "Режим ожидания (Standby)";
      elPwr.style.color = "var(--apple-amber)";
    } else {
      elPwr.textContent = "Определение состояния...";
      elPwr.style.color = "var(--text-secondary)";
    }
  }

  if (elPhys) {
    elPhys.textContent = typeof selectedPa !== "undefined" && selectedPa ? `${selectedPa[0]}.${selectedPa[1]}.${selectedPa[2]}.${selectedPa[3]} (HDMI ${selectedPa[0]})` : "3.0.0.0 (HDMI 3)";
  }
  if (elLa) elLa.textContent = "0x04 (Playback Device 1)";

  const chkRemote = document.getElementById("chk-remote-os");
  const isPass = chkRemote ? chkRemote.checked : true;
  if (elRemActive) {
    elRemActive.textContent = isPass ? "Активен (Перехват кнопок)" : "Отключен (Штатный пульт)";
    elRemActive.style.color = isPass ? "var(--apple-green)" : "var(--text-secondary)";
  }

  const countVal = document.getElementById("lbl-count-value");
  if (elRemKeys && countVal) {
    elRemKeys.textContent = `${countVal.textContent || "0"} кнопок`;
  }
  if (elRemSubsys) elRemSubsys.textContent = "Windows SendInput / Enigo";
}

/**
 * Показ/скрытие модального перекрытия раздела «Пульт»
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
          reasonEl.textContent = "Поиск и переподключение к контроллеру...";
        } else {
          reasonEl.textContent = "Нет связи с ПК: контроллер отключен или порт не отвечает";
        }
      } else if (confirmedLineLow) {
        reasonEl.textContent = "Линия HDMI-CEC 0V: проверьте кабель или питание телевизора";
      } else {
        reasonEl.textContent = curPort ? `Контроллер подключен (${curPort}), но телевизор не отвечает` : "Контроллер подключен, но телевизор не отвечает";
      }
    }
  }
}

/**
 * Ручной опрос состояния шины CEC
 */
async function checkHardwareLineNow() {
  const btn = document.getElementById("btn-refresh-hw");
  if (btn) {
    btn.classList.add("spinning");
    setTimeout(() => btn.classList.remove("spinning"), 600);
  }

  const lampCec = document.getElementById("lamp-cec");
  if (lampCec) {
    lampCec.className = "status-lamp lamp-yellow";
    lampCec.setAttribute("data-tooltip", "Замер линии...");
  }

  if (typeof window.setAmbientMood === "function") {
    window.setAmbientMood("amber", 1200);
  }

  await sendCmd("STATE");
}

/**
 * Разбор входящей строки протокола CEC и обновление индикаторов UI
 */
function decodeCecLine(line) {
  if (line.includes("Logical address assigned: 4") || line.startsWith("OK:READY_LA_4") || line.startsWith("OK:AUTO_ANNOUNCED")) {
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
      : (isHigh ? "Замер: Шина свободна (3.3V RP2040, ТВ не обнаружен)" : "Замер: КЗ на шине CEC (0V)");
  }

  if (line.startsWith("OK:REMOTE_CAPTURE_ON")) {
    return "Перехват пульта включен";
  }

  if (!(line.startsWith("RX ") || line.startsWith("TX "))) return "";
  if (line.startsWith("RX ")) {
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

  // Статус питания (0x90): Report Power Status
  if (op === 0x90 && bytes.length >= 2) {
    const p = POWER_MAP[bytes[1]] || "";
    if (bytes[1] === 0) {
      updateTvPowerButtonUi(true);
    } else {
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

// Первоначальная инициализация в безопасном состоянии
window.addEventListener("DOMContentLoaded", () => {
  updateTvPowerButtonUi("syncing");
  updateHardwareStatusUi({ connected: false }, { checked: false }, true);
});
