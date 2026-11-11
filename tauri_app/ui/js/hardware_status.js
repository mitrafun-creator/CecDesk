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
  const curPort = lastHwStatus.port || (typeof selectedPort !== "undefined" ? selectedPort : "") || "";
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
    if (spinIco) spinIco.style.display = "";
    if (pwrIco) {
      pwrIco.style.display = "";
      pwrIco.setAttribute("class", "ico ico-lg ico-power");
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
  if (spinIco) spinIco.style.display = "";
  if (pwrIco) {
    pwrIco.style.display = "";
    pwrIco.setAttribute("class", "ico ico-lg ico-power");
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

  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const curPort = lastHwStatus.port || selectedPort || "";

  // 1. USB-контроллер (лампочка #lamp-usb)
  const lampUsb = document.getElementById("lamp-usb");
  const usbBadge = document.getElementById("hw-usb-badge"); // fallback для совместимости

  if (lastHwStatus.usbConnected) {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(true);
    const baseTip = _t("status.usb_connected_port", "Подключен");
    const tipText = curPort ? `${baseTip} (${curPort})` : baseTip;
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
    const tipText = _t("remote.searching_ctrl", "Поиск контроллера...");
    if (lampUsb) {
      lampUsb.className = "status-lamp lamp-yellow";
      lampUsb.setAttribute("data-tooltip", tipText);
      lampUsb.removeAttribute("title");
    }
    if (usbBadge) {
      usbBadge.className = "hw-badge badge-warning";
      usbBadge.textContent = _t("status.reconnecting", "Поиск...");
    }
  } else {
    if (typeof updateConnectionButtonsState === "function") updateConnectionButtonsState(false);
    const tipText = _t("remote.status_disconnected", "Отключен");
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
    const tipText = _t("status.bus_polling", "Опрос шины...");
    if (lampCec) {
      lampCec.className = "status-lamp lamp-yellow";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.removeAttribute("title");
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-warning";
      cecBadge.textContent = _t("status.bus_polling_short", "Опрос...");
    }
  } else if (!lastHwStatus.usbConnected) {
    const tipText = _t("status.bus_no_usb", "Нет питания USB");
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
      const tipText = _t("status.tv_online_contact", "3.3V ТВ в сети (Контакт надежен)");
      if (lampCec) {
        lampCec.className = "status-lamp lamp-green";
        lampCec.setAttribute("data-tooltip", tipText);
        lampCec.removeAttribute("title");
      }
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-online";
        cecBadge.textContent = _t("remote.status_online", "3.3V ТВ в сети");
      }
    } else {
      const tipText = _t("status.bus_free_no_tv", "Шина свободна (ТВ не отвечает)");
      if (lampCec) {
        lampCec.className = "status-lamp lamp-yellow";
        lampCec.setAttribute("data-tooltip", tipText);
        lampCec.removeAttribute("title");
      }
      if (cecBadge) {
        cecBadge.className = "hw-badge badge-warning";
        cecBadge.textContent = _t("status.line_free_badge", "Шина свободна");
      }
    }
  } else if (confirmedLineLow) {
    const tipText = _t("status.bus_error_0v", "0V / Ошибка линии CEC (КЗ или кабель)");
    if (lampCec) {
      lampCec.className = "status-lamp lamp-red";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.removeAttribute("title");
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-error";
      cecBadge.textContent = _t("status.line_err_badge", "0V / Ошибка");
    }
    if (typeof window.setAmbientMood === "function") {
      window.setAmbientMood("red");
    }
  } else {
    const tipText = _t("status.bus_polling", "Опрос шины...");
    if (lampCec) {
      lampCec.className = "status-lamp lamp-yellow";
      lampCec.setAttribute("data-tooltip", tipText);
      lampCec.removeAttribute("title");
    }
    if (cecBadge) {
      cecBadge.className = "hw-badge badge-warning";
      cecBadge.textContent = _t("status.bus_polling_short", "Опрос...");
    }
  }

  // 3. Перехват пульта (лампочка #lamp-remote)
  const lampRemote = document.getElementById("lamp-remote");
  const remBadge = document.getElementById("hw-remote-badge");
  const chkRemote = document.getElementById("chk-remote-os");
  const isPass = remoteState !== undefined ? remoteState : (chkRemote ? chkRemote.checked : true);

  if (isPass) {
    const tipText = _t("status.remote_on_tip", "Включен (управление ПК)");
    if (lampRemote) {
      lampRemote.className = "status-lamp lamp-green";
      lampRemote.setAttribute("data-tooltip", tipText);
      lampRemote.removeAttribute("title");
    }
    if (remBadge) {
      remBadge.className = "hw-badge badge-online";
      remBadge.textContent = _t("remote.status_enabled", "Включен");
    }
  } else {
    const tipText = _t("status.remote_off_tip", "Выключен (пульт освобожден для ТВ)");
    if (lampRemote) {
      lampRemote.className = "status-lamp lamp-gray";
      lampRemote.setAttribute("data-tooltip", tipText);
      lampRemote.removeAttribute("title");
    }
    if (remBadge) {
      remBadge.className = "hw-badge badge-neutral";
      remBadge.textContent = _t("remote.status_disabled", "Выключен");
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
  const _t = typeof t === "function" ? t : (_k, fb) => fb;

  const elPort = document.getElementById("diag-usb-port");
  const elConn = document.getElementById("diag-usb-conn");
  const elChip = document.getElementById("diag-usb-chip");

  const elLevel = document.getElementById("diag-cec-level");
  const elPull = document.getElementById("diag-cec-pull");
  const elContact = document.getElementById("diag-cec-contact");

  const elTvTitle = document.getElementById("diag-tv-card-title");
  const elPwr = document.getElementById("diag-tv-pwr");
  const elPhys = document.getElementById("diag-tv-phys");
  const elLa = document.getElementById("diag-tv-la");

  const elRemActive = document.getElementById("diag-remote-active");
  const elRemKeys = document.getElementById("diag-remote-keys-cnt");
  const elRemSubsys = document.getElementById("diag-remote-subsys");

  const curPort = lastHwStatus.port || (typeof selectedPort !== "undefined" ? selectedPort : "") || "";

  if (elTvTitle) {
    const sysName = (typeof currentProfile !== "undefined" && currentProfile && currentProfile.cec_system_name)
      ? currentProfile.cec_system_name
      : "CEC";
    elTvTitle.textContent = `${_t("sys.card_tv_prefix", "Телевизор")} (${sysName})`;
  }

  if (elPort) elPort.textContent = lastHwStatus.usbConnected ? (curPort || "USB") : _t("remote.status_disconnected", "Отключен");
  if (elConn) {
    elConn.textContent = lastHwStatus.usbConnected
      ? _t("diag.val_connected_baud", "Активен (115200 bps)")
      : _t("diag.val_disconnected", "Отключен");
    elConn.style.color = lastHwStatus.usbConnected ? "var(--apple-green)" : "var(--apple-red)";
  }
  if (elChip) {
    const model = typeof getAdapterModelDisplay === "function" ? getAdapterModelDisplay() : "USB CEC";
    elChip.textContent = `${model} (USB CDC)`;
  }

  if (elLevel) {
    if (!lastHwStatus.usbConnected) {
      elLevel.textContent = _t("status.line_no_power", "Нет питания");
      elLevel.style.color = "var(--text-secondary)";
    } else if (lastHwStatus.lineHigh) {
      elLevel.textContent = _t("diag.val_level_high", "3.3V (Норма)");
      elLevel.style.color = "var(--apple-green)";
    } else if (confirmedLineLow) {
      elLevel.textContent = _t("diag.val_level_low", "0V (КЗ / Обрыв)");
      elLevel.style.color = "var(--apple-red)";
    } else {
      elLevel.textContent = _t("status.bus_polling", "Опрос шины...");
      elLevel.style.color = "var(--apple-amber)";
    }
  }

  if (elPull) {
    elPull.textContent = lastHwStatus.tvConnected
      ? _t("diag.val_pullup_tv", "Активна (ТВ 27k)")
      : _t("diag.val_pullup_mcu", "Внутренняя MCU");
  }

  if (elContact) {
    elContact.textContent = (lastHwStatus.lineHigh && lastHwStatus.tvConnected)
      ? _t("diag.val_contact_ok", "Надежный контакт")
      : _t("diag.val_contact_wait", "Нет ответа ТВ");
    elContact.style.color = (lastHwStatus.lineHigh && lastHwStatus.tvConnected) ? "var(--apple-green)" : "var(--apple-amber)";
  }

  if (elPwr) {
    if (tvPowerState === "on") {
      elPwr.textContent = _t("diag.val_tv_on", "Включен (ON)");
      elPwr.style.color = "var(--apple-green)";
    } else if (tvPowerState === "off") {
      elPwr.textContent = _t("diag.val_tv_standby", "Сон (Standby)");
      elPwr.style.color = "var(--apple-amber)";
    } else {
      elPwr.textContent = _t("remote.syncing_tv_state", "Определение состояния...");
      elPwr.style.color = "var(--text-secondary)";
    }
  }

  if (elPhys) {
    elPhys.textContent = (typeof selectedPa !== "undefined" && selectedPa && selectedPa.length >= 4)
      ? `${selectedPa[0]}.${selectedPa[1]}.${selectedPa[2]}.${selectedPa[3]} (HDMI ${selectedPa[0]})`
      : "1.0.0.0 (HDMI 1)";
  }
  if (elLa) elLa.textContent = "0x04 (Playback 1)";

  const chkRemote = document.getElementById("chk-remote-os");
  const isPass = chkRemote ? chkRemote.checked : true;
  if (elRemActive) {
    elRemActive.textContent = isPass
      ? _t("diag.val_intercept_on", "Активен (ПК)")
      : _t("diag.val_intercept_off", "Отключен (ТВ)");
    elRemActive.style.color = isPass ? "var(--apple-green)" : "var(--text-secondary)";
  }

  const countVal = document.getElementById("lbl-count-value");
  if (elRemKeys) {
    const cnt = parseInt(countVal ? countVal.textContent : "0", 10) || 0;
    const unit = typeof getKeysPlural === "function" ? getKeysPlural(cnt) : _t("remote.keys_unit", "кнопок");
    elRemKeys.textContent = `${cnt} ${unit}`;
  }
  if (elRemSubsys) elRemSubsys.textContent = _t("diag.val_input_subsys", "Win SendInput (Enigo)");
}

/**
 * Показ/скрытие модального перекрытия раздела «Пульт» с контекстными действиями
 */
function updateRemoteOfflineOverlay(isTvOnline, isReconnecting, curPort) {
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
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

  const refreshSvg = '<polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>';
  const plugSvg = '<path d="M12 22v-5"></path><path d="M9 8V2"></path><path d="M15 8V2"></path><path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"></path>';

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
        if (titleEl) titleEl.textContent = _t("remote.searching_ctrl", "Поиск контроллера...");
        if (descEl) {
          descEl.textContent = curPort
            ? `${_t("remote.searching_desc", "Попытка переподключения к адаптеру на порту")} ${curPort}...`
            : _t("remote.offline_reason_reconnecting", "Поиск и переподключение к USB-модулю...");
        }
        if (reasonEl) reasonEl.textContent = _t("remote.searching_port", "Поиск COM-порта...");
        if (dotEl) { dotEl.style.background = "var(--apple-amber)"; }
        if (lblEl) lblEl.textContent = _t("remote.searching_btn", "Поиск...");
        if (btnEl) btnEl.disabled = true;
        if (icoEl) icoEl.innerHTML = refreshSvg;
      } else {
        if (titleEl) titleEl.textContent = _t("remote.usb_off_title", "USB-контроллер отключен");
        if (descEl) descEl.textContent = _t("remote.usb_off_desc", "Связь с адаптером разорвана. Нажмите «Подключить», чтобы открыть COM-порт и возобновить работу.");
        if (reasonEl) {
          reasonEl.textContent = curPort
            ? `${curPort} (${_t("remote.port_disconnected", "отключен")})`
            : _t("remote.no_ctrl_reason", "Контроллер не подключен");
        }
        if (dotEl) { dotEl.style.background = "var(--text-tertiary)"; }
        if (lblEl) lblEl.textContent = _t("remote.btn_connect", "Подключить");
        if (btnEl) btnEl.disabled = false;
        if (icoEl) {
          icoEl.innerHTML = plugSvg;
        }
      }
    } else if (confirmedLineLow) {
      if (titleEl) titleEl.textContent = _t("remote.line_err_title", "Ошибка линии CEC (0V)");
      if (descEl) descEl.textContent = _t("remote.line_err_desc", "Контроллер подключен, но на шине CEC зафиксировано 0V (короткое замыкание или кабель). Проверьте кабель HDMI и питание ТВ.");
      if (reasonEl) reasonEl.textContent = _t("remote.offline_reason_line_low", "Линия HDMI-CEC 0V: проверьте кабель или питание ТВ");
      if (dotEl) { dotEl.style.background = "var(--apple-red)"; }
      if (lblEl) lblEl.textContent = _t("remote.btn_poll_bus", "Опросить шину");
      if (btnEl) btnEl.disabled = false;
      if (icoEl) {
        icoEl.innerHTML = refreshSvg;
      }
    } else {
      if (titleEl) titleEl.textContent = _t("remote.offline_modal_title", "Нет связи с телевизором");
      if (descEl) descEl.textContent = _t("remote.offline_modal_desc", "Модуль не обнаружил ответ от телевизора по шине HDMI-CEC или связь с контроллером потеряна. Проверьте кабель HDMI и питание телевизора.");
      if (reasonEl) {
        reasonEl.textContent = curPort
          ? `${curPort}: ${_t("remote.no_tv_port_reason", "Подключен, но ТВ не отвечает")}`
          : _t("remote.offline_reason_tv_no_resp", "Модуль подключен, но ТВ или пульт не отвечает");
      }
      if (dotEl) { dotEl.style.background = "var(--apple-amber)"; }
      if (lblEl) lblEl.textContent = _t("remote.btn_poll_bus", "Опросить шину");
      if (btnEl) btnEl.disabled = false;
      if (icoEl) {
        icoEl.innerHTML = refreshSvg;
      }
    }
  }
}

/**
 * Обработчик действия в модальном окне отключенного состояния
 */
async function handleRemoteOfflineAction() {
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const btn = document.getElementById("btn-remote-offline-refresh");
  const lbl = document.getElementById("lbl-remote-offline-btn");
  const ico = document.getElementById("ico-remote-offline-refresh");

  if (!lastHwStatus.usbConnected) {
    if (lbl) lbl.textContent = _t("remote.btn_connecting", "Подключение...");
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
      if (lbl) lbl.textContent = lastHwStatus.usbConnected ? _t("remote.btn_poll_bus", "Опросить шину") : _t("remote.btn_connect", "Подключить");
    }
  } else {
    if (lbl) lbl.textContent = _t("remote.btn_polling", "Опрос шины...");
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
        if (lbl) lbl.textContent = lastHwStatus.usbConnected ? _t("remote.btn_poll_bus", "Опросить шину") : _t("remote.btn_connect", "Подключить");
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
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const btn = document.getElementById("btn-refresh-hw");
  if (btn) {
    btn.classList.add("spinning");
    setTimeout(() => btn.classList.remove("spinning"), 600);
  }

  const lampCec = document.getElementById("lamp-cec");
  if (lampCec) {
    lampCec.className = "status-lamp lamp-yellow";
    lampCec.setAttribute("data-tooltip", _t("status.measuring_line", "Замер линии..."));
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
      : (isHigh ? "Замер: Шина свободна (3.3V MCU, ТВ не обнаружен)" : "Замер: КЗ на шине CEC (0V)");
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
