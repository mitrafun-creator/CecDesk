/**
 * hardware_status.js - Управление статусом оборудования, шиной CEC и умным питанием ТВ
 */

let lastHwStatus = { usbConnected: false, tvConnected: false, lineHigh: false, port: "" };
let tvPowerState = "syncing"; // "syncing" | "on" | "off"
let lineLowDebounceTimer = null;
let confirmedLineLow = false;
let tvPowerPingWatchdog = null;
let lastTvPowerVerifiedTime = 0;
let isStartupGrace = true;
let startupGraceTimer = null;

function startStartupGrace(durationMs = 900) {
  isStartupGrace = true;
  if (startupGraceTimer) clearTimeout(startupGraceTimer);
  startupGraceTimer = setTimeout(() => {
    finishStartupGrace();
  }, durationMs);
}

function finishStartupGrace() {
  if (startupGraceTimer) {
    clearTimeout(startupGraceTimer);
    startupGraceTimer = null;
  }
  if (!isStartupGrace) return;
  isStartupGrace = false;
  const isTvOnline = lastHwStatus.usbConnected && lastHwStatus.lineHigh && lastHwStatus.tvConnected;
  const curPort = lastHwStatus.port || (typeof selectedPort !== "undefined" ? selectedPort : "") || "COM10";
  updateRemoteOfflineOverlay(isTvOnline, false, curPort);
}

function setHumanStatus(mode, shortText, desc) {
  const descEl = document.getElementById("min-status-desc");
  const text = desc || shortText;
  if (descEl && text) {
    descEl.textContent = text;
  }
  if (typeof window.setAmbientMood === "function") {
    if (mode === "offline" || mode === "error") {
      window.setAmbientMood("red");
    } else if (mode === "standby" || mode === "warning") {
      window.setAmbientMood("amber", 1800);
    } else if (mode === "online") {
      window.setAmbientMood("green", 1400);
    }
  }
}

window.startStartupGrace = startStartupGrace;
window.finishStartupGrace = finishStartupGrace;
window.setHumanStatus = setHumanStatus;

/**
 * Единая умная кнопка питания ТВ: Включить / Выключить / Определение состояния
 * @param {"syncing" | boolean | "on" | "off"} state
 */
function updateTvPowerButtonUi(state) {
  const btn = document.getElementById("btn-tv-power");
  const lbl = document.getElementById("lbl-tv-power");
  const spinIco = document.getElementById("spin-tv-power");
  const pwrIco = document.getElementById("ico-tv-power");
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
    if (spinIco) spinIco.style.display = "inline-block";
    if (pwrIco) pwrIco.style.display = "none";
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
  if (spinIco) spinIco.style.display = "none";
  if (pwrIco) {
    pwrIco.style.display = "inline-block";
    pwrIco.setAttribute("class", "ico ico-lg");
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
      lampUsb.removeAttribute("title");
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
      lampUsb.removeAttribute("title");
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
      lampUsb.removeAttribute("title");
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
  const isReconnecting = !!(usbState && usbState.reconnecting);

  if (!lastHwStatus.usbConnected && isReconnecting) {
    const tipText = "Опрос шины...";
    if (lampCec) {
      lampCec.className = "status-lamp lamp-yellow";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.removeAttribute("title");
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-warning";
      cecBadge.textContent = "Опрос...";
    }
  } else if (!lastHwStatus.usbConnected) {
    const tipText = "Нет питания USB";
    if (lampCec) {
      lampCec.className = "status-lamp lamp-gray";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.removeAttribute("title");
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
        lampCec.removeAttribute("title");
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
        lampCec.removeAttribute("title");
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
      lampCec.removeAttribute("title");
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
      lampCec.removeAttribute("title");
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
      lampRemote.removeAttribute("title");
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
      lampRemote.removeAttribute("title");
    }
    if (remBadge) {
      remBadge.className = "hw-badge badge-neutral";
      remBadge.textContent = "Выключен";
    }
  }

  // Обновление состояния Ambient Fluid
  if (typeof window.setAmbientMood === "function") {
    if ((!lastHwStatus.usbConnected && !isReconnecting) || confirmedLineLow) {
      window.setAmbientMood("red");
    } else if (isReconnecting || isStartupGrace) {
      window.setAmbientMood("blue");
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
 * Показ/скрытие модального перекрытия раздела «Пульт» с контекстными действиями
 */
function updateRemoteOfflineOverlay(isTvOnline, isReconnecting, curPort) {
  const stage = document.getElementById("remote-stage");
  const overlay = document.getElementById("remote-offline-overlay");
  const titleEl = document.getElementById("remote-offline-title");
  const descEl = document.getElementById("remote-offline-desc");
  const reasonEl = document.getElementById("remote-offline-reason");
  const dotEl = document.getElementById("remote-offline-dot");
  const btnEl = document.getElementById("btn-remote-offline-refresh");
  const lblEl = document.getElementById("lbl-remote-offline-btn");
  const icoEl = document.getElementById("ico-remote-offline-refresh");

  if (!stage || !overlay) return;

  if (isTvOnline) {
    if (startupGraceTimer) {
      clearTimeout(startupGraceTimer);
      startupGraceTimer = null;
    }
    isStartupGrace = false;
    stage.classList.remove("is-blurred");
    overlay.classList.remove("visible");
  } else if (isStartupGrace) {
    stage.classList.remove("is-blurred");
    overlay.classList.remove("visible");
  } else {
    stage.classList.add("is-blurred");
    overlay.classList.add("visible");

    if (!lastHwStatus.usbConnected) {
      if (isReconnecting) {
        if (titleEl) titleEl.textContent = "Поиск контроллера...";
        if (descEl) descEl.textContent = "Попытка автоматического переподключения к адаптеру RP2040 на порту " + (curPort || "COM") + "...";
        if (reasonEl) reasonEl.textContent = "Поиск COM-порта...";
        if (dotEl) { dotEl.style.background = "var(--apple-amber)"; }
        if (lblEl) lblEl.textContent = "Поиск...";
        if (btnEl) btnEl.disabled = true;
      } else {
        if (titleEl) titleEl.textContent = "USB-контроллер отключен";
        if (descEl) descEl.textContent = "Связь с адаптером RP2040 разорвана. Нажмите «Подключить», чтобы открыть COM-порт и возобновить управление телевизором.";
        if (reasonEl) reasonEl.textContent = curPort ? `Порт ${curPort} отключен` : "Контроллер не подключен";
        if (dotEl) { dotEl.style.background = "var(--apple-gray)"; }
        if (lblEl) lblEl.textContent = "Подключить";
        if (btnEl) btnEl.disabled = false;
        if (icoEl) {
          icoEl.innerHTML = '<polygon points="5 3 19 12 5 21 5 3"></polygon>';
        }
      }
    } else if (confirmedLineLow) {
      if (titleEl) titleEl.textContent = "Ошибка линии CEC (0V)";
      if (descEl) descEl.textContent = "Контроллер подключен к " + (curPort || "порту") + ", но на шине CEC зафиксировано 0V (короткое замыкание или обрыв). Проверьте кабель HDMI и пины.";
      if (reasonEl) reasonEl.textContent = "Линия HDMI-CEC 0V: проверьте кабель или питание телевизора";
      if (dotEl) { dotEl.style.background = "var(--apple-red)"; }
      if (lblEl) lblEl.textContent = "Опросить шину";
      if (btnEl) btnEl.disabled = false;
      if (icoEl) {
        icoEl.innerHTML = '<polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>';
      }
    } else {
      if (titleEl) titleEl.textContent = "Нет связи с телевизором";
      if (descEl) descEl.textContent = curPort
        ? `Контроллер подключен к ${curPort}, но телевизор не отвечает по шине HDMI-CEC. Убедитесь, что ТВ включен в розетку и провод HDMI подключен.`
        : "Контроллер подключен, но телевизор не отвечает по шине HDMI-CEC. Убедитесь, что ТВ включен в розетку и провод HDMI подключен.";
      if (reasonEl) reasonEl.textContent = curPort ? `Контроллер подключен (${curPort}), но телевизор не отвечает` : "Контроллер подключен, но телевизор не отвечает";
      if (dotEl) { dotEl.style.background = "var(--apple-amber)"; }
      if (lblEl) lblEl.textContent = "Опросить шину";
      if (btnEl) btnEl.disabled = false;
      if (icoEl) {
        icoEl.innerHTML = '<polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>';
      }
    }
  }
}

/**
 * Обработчик действия в модальном окне отключенного состояния
 */
async function handleRemoteOfflineAction() {
  const btn = document.getElementById("btn-remote-offline-refresh");
  const lbl = document.getElementById("lbl-remote-offline-btn");
  const ico = document.getElementById("ico-remote-offline-refresh");

  if (!lastHwStatus.usbConnected) {
    if (lbl) lbl.textContent = "Подключение...";
    if (btn) btn.disabled = true;
    if (ico) ico.classList.add("spinning");
    try {
      if (typeof refreshPorts === "function") {
        await refreshPorts();
      }
      if (typeof connectDeviceNow === "function") {
        await connectDeviceNow(true);
      }
    } catch (e) {
      console.error(e);
      if (typeof addLog === "function") addLog(`[Подключение] Ошибка: ${e}`, "log-err");
    } finally {
      if (ico) ico.classList.remove("spinning");
      if (btn) btn.disabled = false;
      if (lbl) lbl.textContent = lastHwStatus.usbConnected ? "Опросить шину" : "Подключить";
    }
  } else {
    if (lbl) lbl.textContent = "Опрос шины...";
    if (btn) btn.disabled = true;
    if (ico) ico.classList.add("spinning");
    try {
      if (typeof checkHardwareLineNow === "function") {
        await checkHardwareLineNow();
      }
    } catch (e) {
      console.error(e);
    } finally {
      setTimeout(() => {
        if (ico) ico.classList.remove("spinning");
        if (btn) btn.disabled = false;
        if (lbl) lbl.textContent = "Опросить шину";
      }, 700);
    }
  }
}
window.refreshRemoteConnectionModal = handleRemoteOfflineAction;
window.handleRemoteOfflineAction = handleRemoteOfflineAction;

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
