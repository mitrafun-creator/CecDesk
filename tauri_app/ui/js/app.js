/**
 * app.js - Главный контроллер интерфейса, транспорт команд и подписки Tauri
 */

const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const listen = window.__TAURI__ ? window.__TAURI__.event.listen : null;

const _recentClientErrors = new Map();
function reportWebviewError(message, stack = "") {
  if (!invoke) return;
  const msgStr = String(message || "Unknown UI error").slice(0, 800);
  const key = msgStr.slice(0, 120);
  const now = Date.now();
  const prev = _recentClientErrors.get(key) || 0;
  if (now - prev < 5000) return;
  _recentClientErrors.set(key, now);
  if (_recentClientErrors.size > 50) {
    const firstKey = _recentClientErrors.keys().next().value;
    if (firstKey !== undefined) _recentClientErrors.delete(firstKey);
  }
  const stackStr = stack ? String(stack).slice(0, 1500) : null;
  invoke("report_client_error", {
    source: "main_ui",
    message: msgStr,
    stack: stackStr
  }).catch(() => {});
}

window.addEventListener("error", (ev) => {
  const loc = ev.filename ? ` (${ev.filename}:${ev.lineno || 0}:${ev.colno || 0})` : "";
  const msg = `${ev.message || "Script error"}${loc}`;
  const st = ev.error && ev.error.stack ? ev.error.stack : "";
  reportWebviewError(msg, st);
});

window.addEventListener("unhandledrejection", (ev) => {
  const reason = ev.reason;
  const msg = reason && reason.message ? reason.message : String(reason || "Unhandled rejection");
  const st = reason && reason.stack ? reason.stack : "";
  reportWebviewError(`Unhandled rejection: ${msg}`, st);
});

window.addEventListener("securitypolicyviolation", (ev) => {
  const directive = ev.violatedDirective || ev.effectiveDirective || "unknown";
  const blocked = ev.blockedURI || "inline";
  const msg = `CSP violation: ${directive} blocked ${blocked}`;
  const loc = ev.sourceFile ? `${ev.sourceFile}:${ev.lineNumber || 0}:${ev.columnNumber || 0}` : "";
  reportWebviewError(msg, loc);
});

// Отключение контекстного меню в UI
window.addEventListener("contextmenu", (e) => e.preventDefault());

// Закрытие выпадающих списков при клике вне, скролле или изменении размера окна
window.addEventListener("click", (e) => {
  if (!e.target.closest(".c-select") && !e.target.closest(".c-select-menu")) {
    closeAllCustomSelects();
  }
});

window.addEventListener("scroll", (e) => {
  if (!e.target || !(e.target instanceof Element) || !e.target.closest(".c-select-menu")) {
    closeAllCustomSelects();
  }
}, { capture: true, passive: true });

window.addEventListener("resize", () => {
  closeAllCustomSelects();
}, { passive: true });

let isConnected = false;
let tvProfiles = [];
let currentProfile = null;
let appConfig = null;
let selectedPort = "";
let selectedPa = "1000";

/**
 * Управление кастомным окном (Titlebar controls)
 */
async function appWinMinimize() {
  if (invoke) {
    try {
      await invoke("app_win_minimize");
    } catch (e) {
      console.warn("Minimize error:", e);
    }
  }
}

async function appWinToggleMaximize() {
  if (invoke) {
    try {
      await invoke("app_win_toggle_maximize");
    } catch (e) {
      console.warn("Toggle maximize error:", e);
    }
  }
}

function openCloseConfirmModal() {
  const modal = document.getElementById("close-confirm-modal");
  const chk = document.getElementById("chk-close-dont-ask");
  if (chk) chk.checked = false;
  if (modal) modal.classList.add("is-open");
}

function closeCloseConfirmModal() {
  const modal = document.getElementById("close-confirm-modal");
  if (modal) modal.classList.remove("is-open");
}

async function appWinHideToTray() {
  const mode = (appConfig && appConfig.close_behavior) ? appConfig.close_behavior : "ask";
  if (mode === "exit") {
    if (invoke) {
      try {
        await invoke("app_win_quit");
      } catch (e) {
        console.warn("Quit error:", e);
      }
    }
    return;
  }
  if (mode === "tray") {
    if (invoke) {
      try {
        await invoke("app_win_hide_to_tray");
      } catch (e) {
        console.warn("Hide to tray error:", e);
      }
    }
    return;
  }
  openCloseConfirmModal();
}

function renderCloseBehaviorSelect() {
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const cur = (appConfig && appConfig.close_behavior) ? appConfig.close_behavior : "ask";
  const opts = [
    { value: "ask", label: _t("settings.close_opt_ask", "Спрашивать каждый раз") },
    { value: "tray", label: _t("settings.close_opt_tray", "Сворачивать в трей") },
    { value: "exit", label: _t("settings.close_opt_exit", "Закрывать полностью") }
  ];
  createCustomSelect("csel-close-behavior", opts, cur, async (val) => {
    if (!appConfig) return;
    appConfig.close_behavior = val;
    if (invoke) {
      await invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }
  });
}
window.renderCloseBehaviorSelect = renderCloseBehaviorSelect;

/**
 * Инициализация перетаскивания окна за шапку (fallback вне нативного Tauri drag.js)
 */
function initTitlebarDragging() {
  const tb = document.querySelector(".app-titlebar");
  if (!tb) return;
  tb.addEventListener("mousedown", (e) => {
    if (window.__TAURI_INTERNALS__) {
      // Нативный Tauri drag.js сам обрабатывает drag и double-click по data-tauri-drag-region="deep"
      return;
    }
    if (e.buttons === 1 && !e.target.closest("button, nav, .segmented-nav, .win-btn, .c-select, input, select, textarea")) {
      if (e.detail === 2) {
        appWinToggleMaximize();
      } else if (invoke) {
        invoke("app_win_start_dragging").catch(() => {});
      }
    }
  });
}

/**
 * Позиционирование (i) всплывающих подсказок без обрезания сверху и справа
 */
function initHelpPopovers() {
  document.querySelectorAll(".info-tip-anchor").forEach((anchor) => {
    if (anchor.dataset.popoverBound) return;
    anchor.dataset.popoverBound = "1";
    const adjust = () => {
      const rect = anchor.getBoundingClientRect();
      const vp = anchor.closest(".viewport");
      const topEdge = vp ? vp.getBoundingClientRect().top : 48;
      const rightEdge = vp ? vp.getBoundingClientRect().right : window.innerWidth;
      anchor.classList.toggle("open-below", (rect.top - topEdge) < 185);
      anchor.classList.toggle("align-right", (rect.left + 265) > (rightEdge - 24));
    };
    anchor.addEventListener("pointerenter", adjust);
    anchor.addEventListener("focusin", adjust);
  });
}

/**
 * Централизованная привязка всех статических обработчиков UI (CSP 'self' без inline onclick)
 */
let uiBindingsInitialized = false;
function initUiBindings() {
  if (uiBindingsInitialized) return;
  uiBindingsInitialized = true;

  const onClick = (id, handler) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener("click", handler);
  };
  const onChange = (id, handler) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener("change", () => handler(el));
  };

  // Шапка и вкладки
  document.querySelectorAll(".seg-btn[data-view-index]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const idx = Number(btn.dataset.viewIndex);
      if (Number.isInteger(idx)) switchViewIndex(idx, btn);
    });
  });
  onClick("btn-win-min", appWinMinimize);
  onClick("btn-win-max", appWinToggleMaximize);
  onClick("btn-win-close", appWinHideToTray);

  // Баннер восстановления после сбоя
  onClick("btn-crash-create-report", () => openBugReportModal(true));
  onClick("btn-crash-dismiss", dismissCrashBanner);

  // Экран 0: Пульт
  onClick("btn-tv-power", toggleTvPower);
  onClick("btn-start-calib", startCalibration);
  onChange("chk-remote-os", toggleRemotePass);
  onClick("btn-refresh-hw", checkHardwareLineNow);
  onClick("btn-remote-offline-refresh", handleRemoteOfflineAction);

  // Экран 1: Кнопки
  onClick("btn-scripts-folder", openScriptsFolder);
  onClick("btn-refresh-scripts", refreshCustomScripts);
  onClick("btn-save-keys", saveMapperConfig);
  document.querySelectorAll(".preset-btn[data-preset]").forEach((btn) => {
    btn.addEventListener("click", () => applyKeyPreset(btn.dataset.preset, btn));
  });
  const customPresetBtn = document.getElementById("pbtn-custom");
  if (customPresetBtn) {
    customPresetBtn.addEventListener("dblclick", (e) => {
      e.preventDefault();
      if (typeof startCustomProfileInlineRename === "function") {
        startCustomProfileInlineRename();
      }
    });
  }
  const inpProfName = document.getElementById("inp-profile-name");
  if (inpProfName) {
    inpProfName.addEventListener("keydown", (e) => {
      if (e.key === "Enter") renameCurrentProfile();
    });
  }
  onClick("btn-rename-profile", renameCurrentProfile);

  // Экран 2: Система
  onClick("btn-refresh-ports", refreshPorts);
  onClick("btn-conn", () => connectDeviceNow());
  onClick("btn-disconn", disconnectDeviceNow);
  onClick("btn-check-hw-sys", checkHardwareLineNow);
  const inpSearchProf = document.getElementById("inp-search-prof");
  if (inpSearchProf) inpSearchProf.addEventListener("input", renderProfilesCatalog);
  onClick("btn-sync-profiles", syncTvProfilesOnlineUi);
  onClick("btn-save-profile-json", saveCurrentProfileJson);

  document.querySelectorAll("button[data-cec-cmd]").forEach((btn) => {
    btn.addEventListener("click", () => sendCmd(btn.dataset.cecCmd));
  });
  document.querySelectorAll("button[data-os-key]").forEach((btn) => {
    btn.addEventListener("click", () => pressOsKey(btn.dataset.osKey));
  });

  const inpRaw = document.getElementById("inp-raw");
  if (inpRaw) {
    inpRaw.addEventListener("keydown", (e) => {
      if (e.key === "Enter") sendCustomRaw();
    });
  }
  onClick("btn-send-raw", sendCustomRaw);
  onClick("btn-copy-logs", copySelectedOrAllLogs);
  onClick("btn-open-logs", openLogsFolder);
  onClick("btn-clear-logs", clearLog);

  // Экран 3: Настройки
  const inpOsd = document.getElementById("inp-osd");
  if (inpOsd) {
    inpOsd.addEventListener("input", checkOsdDirtyState);
    inpOsd.addEventListener("keydown", (e) => {
      if (e.key === "Enter") saveOsdName();
    });
  }
  onClick("btn-save-osd", saveOsdName);
  onClick("btn-apply-pa", applyPaOverride);

  onChange("chk-wake-pc-on-tv", autoSaveQolSettings);
  onChange("chk-wake-tv-on-pc", autoSaveQolSettings);
  onChange("chk-tv-off-on-pc", autoSaveQolSettings);
  onChange("chk-pc-off-on-tv", onPcOffOnTvToggleChanged);
  onChange("chk-standalone-hid", () => {
    const chk = document.getElementById("chk-standalone-hid");
    updateStandaloneHidBodyVisibility();
    if (chk && !chk.checked) {
      saveStandaloneHidConfig({
        toastKey: "settings.standalone_disabled_toast",
        toastFallback: "Автономный HID-режим выключен"
      });
    } else {
      setStandaloneHidDirty(true);
    }
  });
  const inpStandalonePin = document.getElementById("inp-standalone-pin");
  if (inpStandalonePin) {
    inpStandalonePin.addEventListener("input", () => setStandaloneHidDirty(true));
  }
  onClick("btn-save-standalone-hid", saveStandaloneHidConfig);
  onClick("btn-toggle-standalone-pin", () => {
    const inp = document.getElementById("inp-standalone-pin");
    const btn = document.getElementById("btn-toggle-standalone-pin");
    if (!inp || !btn) return;
    const isPwd = inp.type === "password";
    inp.type = isPwd ? "text" : "password";
    const _t = typeof t === "function" ? t : (_k, fb) => fb;
    btn.textContent = isPwd ? _t("settings.standalone_pin_hide", "Скрыть") : _t("settings.standalone_pin_show", "Показать");
  });

  document.querySelectorAll(".lang-choice-btn[data-lang]").forEach((btn) => {
    btn.addEventListener("click", () => switchLanguage(btn.dataset.lang));
  });

  onClick("btn-check-update-settings", () => {
    const btn = document.getElementById("btn-check-update-settings");
    if (btn && btn.dataset.selfUpdate === "1") {
      installAppUpdateUi();
    } else if (btn && btn.dataset.updateUrl) {
      openReleasesPage();
    } else {
      checkAppUpdatesUi(false);
    }
  });
  onClick("btn-open-releases-settings", openReleasesPage);
  onClick("btn-flash-bundled-fw", () => flashModuleFirmwareUi(false));
  onClick("btn-flash-custom-fw", () => flashModuleFirmwareUi(true));
  onClick("btn-export-xml", exportXmlProfileStub);
  onClick("btn-import-xml", importXmlProfileStub);

  onChange("chk-autostart", (el) => toggleAutostart(el.checked));
  onChange("chk-hud-overlay", autoSaveQolSettings);
  onChange("chk-auto-audio", onAutoAudioToggleChanged);
  onChange("chk-soundbar", autoSaveQolSettings);
  onChange("chk-hold-seek", autoSaveQolSettings);
  onChange("chk-emergency-kill", autoSaveQolSettings);
  onChange("chk-non-intrusive", autoSaveQolSettings);
  onChange("chk-auto-heal", autoSaveQolSettings);
  onChange("chk-disp-sleep", autoSaveQolSettings);

  onChange("chk-telemetry-opt-in", onTelemetryOptInChanged);
  onClick("btn-save-telemetry-endpoint", saveTelemetrySettings);
  onClick("btn-open-bug-modal", () => openBugReportModal());
  onClick("btn-open-reports-dir", openReportsFolderUi);
  onClick("btn-open-github-issue", openGithubIssueDirect);

  // Модальные окна
  onClick("btn-calib-close", cancelCalibration);
  onClick("btn-calib-finish", finishCalibration);
  onClick("btn-calib-cancel", cancelCalibration);
  onClick("btn-calib-reset", resetToDefaultKeys);

  onClick("btn-bug-close-top", closeBugReportModal);
  onClick("btn-modal-create-zip", createSupportBundleFromModal);
  onClick("btn-modal-open-reports", openReportsFolderUi);
  onClick("btn-modal-send-webhook", sendBugReportWebhookFromModal);
  onClick("btn-modal-github-issue", openGithubBugIssueFromModal);
  onClick("btn-bug-close-bottom", closeBugReportModal);

  initBiosPowerHintModal();
  initCloseConfirmModal();
  initHelpPopovers();
}

function initCloseConfirmModal() {
  const modal = document.getElementById("close-confirm-modal");
  const btnX = document.getElementById("btn-close-modal-x");
  const btnTray = document.getElementById("btn-close-to-tray");
  const btnExit = document.getElementById("btn-close-exit-app");

  if (btnX) btnX.addEventListener("click", closeCloseConfirmModal);
  if (modal) {
    modal.addEventListener("click", (ev) => {
      if (ev.target === modal) closeCloseConfirmModal();
    });
  }
  if (btnTray) {
    btnTray.addEventListener("click", async () => {
      const chk = document.getElementById("chk-close-dont-ask");
      if (chk && chk.checked && appConfig) {
        appConfig.close_behavior = "tray";
        renderCloseBehaviorSelect();
        if (invoke) await invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
      }
      closeCloseConfirmModal();
      if (invoke) await invoke("app_win_hide_to_tray").catch(() => {});
    });
  }
  if (btnExit) {
    btnExit.addEventListener("click", async () => {
      const chk = document.getElementById("chk-close-dont-ask");
      if (chk && chk.checked && appConfig) {
        appConfig.close_behavior = "exit";
        renderCloseBehaviorSelect();
        if (invoke) await invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
      }
      closeCloseConfirmModal();
      if (invoke) await invoke("app_win_quit").catch(() => {});
    });
  }
}

let detectedMotherboardModel = "";

function initBiosPowerHintModal() {
  const btnHint = document.getElementById("btn-bios-power-hint");
  const modal = document.getElementById("bios-power-modal");
  const btnOk = document.getElementById("btn-bios-modal-ok");
  const btnMore = document.getElementById("btn-bios-modal-more");
  const lblMb = document.getElementById("lbl-bios-mb-model");

  if (btnHint) {
    try {
      if (localStorage.getItem("cecdesk_bios_hint_seen") === "1") {
        btnHint.classList.remove("is-pulsing");
      }
    } catch (_) {}

    btnHint.addEventListener("click", async () => {
      btnHint.classList.remove("is-pulsing");
      try {
        localStorage.setItem("cecdesk_bios_hint_seen", "1");
      } catch (_) {}

      const _t = typeof t === "function" ? t : (_k, fb) => fb;
      if (!detectedMotherboardModel && invoke) {
        try {
          detectedMotherboardModel = (await invoke("get_motherboard_model")) || "";
        } catch (_) {}
      }
      if (lblMb) {
        const prefix = _t("bios.mb_prefix", "Материнская плата:");
        const unknown = _t("bios.mb_unknown", "не определена");
        lblMb.textContent = `${prefix} ${detectedMotherboardModel || unknown}`;
      }
      if (modal) modal.classList.add("is-open");
    });
  }

  const closeBiosModal = () => {
    if (modal) modal.classList.remove("is-open");
  };
  if (btnOk) btnOk.addEventListener("click", closeBiosModal);
  if (modal) {
    modal.addEventListener("click", (ev) => {
      if (ev.target === modal) closeBiosModal();
    });
  }
  if (btnMore) {
    btnMore.addEventListener("click", async () => {
      const isEn = typeof currentLang !== "undefined" && currentLang === "en";
      const baseQuery = isEn
        ? "how to enable USB power when PC is off disable ErP BIOS"
        : "как включить питание USB при выключенном ПК отключить ErP BIOS";
      const fullQuery = detectedMotherboardModel
        ? `${baseQuery} ${detectedMotherboardModel}`
        : baseQuery;
      const url = `https://www.google.com/search?q=${encodeURIComponent(fullQuery)}`;
      if (invoke) {
        try {
          await invoke("open_browser_url", { url });
        } catch (_) {
          window.open(url, "_blank");
        }
      } else {
        window.open(url, "_blank");
      }
    });
  }
}

/**
 * Плавное переключение экранов без микрофризов и с анимацией скроллбара
 */
let slideTimeout = null;
let currentViewIndex = 0;
function switchViewIndex(idx, btn) {
  const clamped = Math.max(0, Math.min(3, Number(idx) || 0));
  currentViewIndex = clamped;
  idx = clamped;
  if (!btn) {
    btn = document.querySelectorAll(".seg-btn")[idx] || null;
  }
  closeAllCustomSelects();
  const globalTip = document.getElementById("global-floating-tooltip");
  if (globalTip) globalTip.classList.remove("visible");

  document.querySelectorAll(".seg-btn").forEach(b => b.classList.remove("active"));
  if (btn) btn.classList.add("active");
  if (idx === 1 && typeof window.ensureMapperRendered === "function") {
    window.ensureMapperRendered();
  }
  if (idx === 3 && typeof window.refreshDiagnosticsSummary === "function") {
    window.refreshDiagnosticsSummary();
  }
  const viewport = document.querySelector(".viewport");
  const slider = document.getElementById("screens-slider");
  if (viewport) {
    viewport.classList.add("is-sliding");
    viewport.classList.remove("slide-settled");
  }
  if (slider) {
    slider.style.transform = `translateX(-${idx * 25}%)`;
  }
  if (typeof window.triggerAmbientRipple === "function" && btn) {
    const rect = btn.getBoundingClientRect();
    window.triggerAmbientRipple(rect.left + rect.width / 2, rect.bottom + 10, 1.25);
  }
  if (slideTimeout) clearTimeout(slideTimeout);
  slideTimeout = setTimeout(() => {
    if (viewport) {
      viewport.classList.remove("is-sliding");
      viewport.classList.add("slide-settled");
    }
  }, 380);
}

function ensureCustomSelectPortal() {
  let layer = document.getElementById("cselect-portal-layer");
  if (!layer) {
    layer = document.createElement("div");
    layer.id = "cselect-portal-layer";
    layer.className = "cselect-portal-layer";
    document.body.appendChild(layer);
  }
  return layer;
}

function closeAllCustomSelects() {
  document.querySelectorAll(".c-select.open").forEach(el => el.classList.remove("open"));
  document.querySelectorAll(".c-select-menu.is-open").forEach(m => {
    m.classList.remove("is-open", "open-up");
    if (m.parentNode && m.parentNode.id === "cselect-portal-layer") {
      m.parentNode.removeChild(m);
    }
  });
  document.querySelectorAll(".glass-panel").forEach(p => { p.style.zIndex = ""; });
}

function positionCustomSelectMenu(trigger, menu) {
  const rect = trigger.getBoundingClientRect();
  const gap = 5;
  const margin = 8;
  const menuWidth = Math.max(rect.width, 200);
  let left = rect.left;
  if (left + menuWidth > window.innerWidth - margin) {
    left = Math.max(margin, window.innerWidth - menuWidth - margin);
  }
  menu.style.width = `${Math.round(rect.width)}px`;
  menu.style.minWidth = `${Math.round(menuWidth)}px`;
  menu.style.left = `${Math.round(left)}px`;
  menu.style.right = "auto";

  const spaceBelow = window.innerHeight - rect.bottom - gap - margin;
  const spaceAbove = rect.top - gap - margin;
  const openUp = spaceBelow < 180 && spaceAbove > spaceBelow;
  const maxAvail = Math.max(110, Math.min(240, openUp ? spaceAbove : spaceBelow));
  menu.style.maxHeight = `${Math.round(maxAvail)}px`;

  if (openUp) {
    menu.classList.add("open-up");
    menu.style.top = "auto";
    menu.style.bottom = `${Math.round(window.innerHeight - rect.top + gap)}px`;
  } else {
    menu.classList.remove("open-up");
    menu.style.bottom = "auto";
    menu.style.top = `${Math.round(rect.bottom + gap)}px`;
  }
}

/**
 * Кастомный компонент выпадающего списка в стиле Apple macOS (с безопасным порталом)
 */
function createCustomSelect(containerOrId, options, currentValue, onChange) {
  const root = typeof containerOrId === "string" ? document.getElementById(containerOrId) : containerOrId;
  if (!root) return;
  if (root._portalMenu && root._portalMenu.parentNode) {
    root._portalMenu.parentNode.removeChild(root._portalMenu);
  }
  root.classList.remove("open");
  root.innerHTML = "";
  root.setAttribute("data-value", currentValue || "");

  const curOpt = options.find(o => o.value === currentValue) || options[0] || { value: "", label: "—" };
  root.setAttribute("data-value", curOpt.value);

  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "c-select-trigger";
  const labelSpan = document.createElement("span");
  labelSpan.className = "c-select-label";
  labelSpan.textContent = curOpt.label;
  trigger.appendChild(labelSpan);
  trigger.insertAdjacentHTML(
    "beforeend",
    '<svg class="ico c-select-chevron" viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"></polyline></svg>'
  );

  const menu = document.createElement("div");
  menu.className = "c-select-menu";
  root._portalMenu = menu;

  const frag = document.createDocumentFragment();
  for (let i = 0; i < options.length; i++) {
    const opt = options[i];
    const item = document.createElement("div");
    item.className = "c-select-option" + (opt.value === curOpt.value ? " selected" : "");
    item.dataset.value = opt.value;
    item.dataset.label = opt.label;
    item.textContent = opt.label;
    frag.appendChild(item);
  }
  menu.appendChild(frag);

  menu.addEventListener("click", (e) => {
    const item = e.target.closest(".c-select-option");
    if (!item) return;
    e.stopPropagation();
    const val = item.dataset.value;
    const lbl = item.dataset.label;
    root.setAttribute("data-value", val);
    labelSpan.textContent = lbl;
    const prevSel = menu.querySelector(".c-select-option.selected");
    if (prevSel) prevSel.classList.remove("selected");
    item.classList.add("selected");
    closeAllCustomSelects();
    if (onChange) onChange(val);
  });

  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    const wasOpen = root.classList.contains("open");
    closeAllCustomSelects();
    if (!wasOpen) {
      const portal = ensureCustomSelectPortal();
      portal.appendChild(menu);
      root.classList.add("open");
      menu.classList.add("is-open");
      positionCustomSelectMenu(trigger, menu);
      const selOpt = menu.querySelector(".c-select-option.selected");
      if (selOpt) {
        const topOffset = selOpt.offsetTop - menu.clientHeight / 2 + selOpt.offsetHeight / 2;
        menu.scrollTop = Math.max(0, topOffset);
      }
      const parentPanel = root.closest(".glass-panel");
      if (parentPanel) parentPanel.style.zIndex = "100";
    }
  });

  root.appendChild(trigger);
}

/**
 * Журналирование событий
 */
function addLog(text, cls = "", writeDisk = false) {
  const box = document.getElementById("log-container");
  if (!box) return;
  const now = new Date();
  const ts = now.toTimeString().split(" ")[0] + "." + String(now.getMilliseconds()).padStart(3, "0");
  const fullLine = `[${ts}] ${text}`;
  const div = document.createElement("div");
  div.className = "log-line " + cls;
  div.textContent = fullLine;
  box.appendChild(div);
  while (box.childElementCount > 400) {
    box.removeChild(box.firstElementChild);
  }
  box.scrollTop = box.scrollHeight;

  if (writeDisk && invoke) {
    invoke("append_client_log", { line: fullLine }).catch(() => {});
  }
}

async function copySelectedOrAllLogs() {
  const sel = window.getSelection().toString().trim();
  const box = document.getElementById("log-container");
  const textToCopy = sel.length > 0 ? sel : box.innerText;
  try {
    await navigator.clipboard.writeText(textToCopy);
    addLog("[Скопировано в буфер]", "log-ok");
  } catch (err) {
    addLog("[Ошибка] " + err, "log-err");
  }
}

async function openLogsFolder() {
  if (invoke) await invoke("open_logs_folder");
}

function clearLog() {
  const box = document.getElementById("log-container");
  if (box) box.innerHTML = "";
}

async function toggleAutostart(enabled) {
  if (!invoke) return;
  try {
    await invoke("set_autostart", { enabled });
    addLog(`[Система] Автозапуск с Windows: ${enabled ? 'ВКЛЮЧЕН' : 'ОТКЛЮЧЕН'}`, "log-ok");
  } catch (err) {
    addLog(`[Ошибка автозапуска] ${err}`, "log-err");
  }
}

let availablePortsList = [];
let detectedHardwareModel = "";
let detectedFirmwareVersion = "";
let connectedPort = "";
let hasAutoWokenTvOnce = false;

function getAdapterModelDisplay() {
  const curPort = selectedPort || connectedPort || (lastHwStatus && lastHwStatus.port);
  const curPortObj = availablePortsList.find(p => p.name === curPort);
  if (curPortObj && curPortObj.desc) {
    if (curPortObj.desc.includes("RP2040")) return "RP2040-Zero";
    if (curPortObj.desc.includes("dev/r.1")) return "CecDesk Dongle";
    if (curPortObj.desc.includes("ESP32")) return "ESP32-S3";
  }
  return detectedHardwareModel || "USB CEC";
}

function populatePortsSelect(ports) {
  availablePortsList = ports || [];
  let preferred = null;
  const opts = availablePortsList.map(p => {
    if (p.is_preferred && !preferred) preferred = p.name;
    return { value: p.name, label: p.desc || `${p.name} • CecDesk Dongle` };
  });
  const fallback = availablePortsList.length > 0 ? availablePortsList[0].name : "";
  const stillExists = selectedPort && availablePortsList.some(p => p.name === selectedPort);
  if (!stillExists) {
    selectedPort = preferred || fallback;
  }
  createCustomSelect("csel-port", opts, selectedPort, async (val) => {
    const prevPort = connectedPort || selectedPort;
    selectedPort = val;
    updateDynamicWindowTitle();
    if (isConnected && val && val !== connectedPort) {
      addLog(`[USB] Переключение контроллера: ${prevPort} -> ${val}...`, "log-ok");
      await connectDeviceNow(true);
    } else {
      updateConnectionButtonsState(isConnected);
    }
  });
  updateConnectionButtonsState(isConnected);
  return selectedPort;
}

async function refreshPorts() {
  if (!invoke) return null;
  const ports = await invoke("list_serial_ports");
  return populatePortsSelect(ports);
}

function populateProfiles(profiles, options = {}) {
  tvProfiles = profiles || [];
  const profCount = document.getElementById("prof-count");
  if (profCount) profCount.textContent = tvProfiles.length;

  const targetId = (currentProfile && currentProfile.id)
    ? currentProfile.id
    : ((appConfig && appConfig.active_profile_id) ? appConfig.active_profile_id : "universal_hdmi_cec");
  if (tvProfiles.length > 0 && !currentProfile) {
    const def = tvProfiles.find(p => p.id === targetId) || tvProfiles[0];
    selectProfileById(def.id, { ...options, skipSave: true });
  } else {
    const opts = tvProfiles.map(p => ({ value: p.id, label: `${p.brand} — ${profileShortName(p)}` }));
    createCustomSelect("csel-profile", opts, targetId, (val) => selectProfileById(val));
    createCustomSelect("csel-profile-sys", opts, targetId, (val) => selectProfileById(val));
    renderProfilesCatalog();
  }
}

async function loadProfiles(options = {}) {
  if (!invoke) return;
  const profiles = await invoke("load_tv_profiles");
  populateProfiles(profiles, options);
}

function renderProfilesCatalog() {
  const q = (document.getElementById("inp-search-prof").value || "").toLowerCase();
  const box = document.getElementById("profiles-catalog");
  if (!box) return;
  box.innerHTML = "";
  tvProfiles
    .filter(p => {
      const name = String(p.name || "").toLowerCase();
      const brand = String(p.brand || "").toLowerCase();
      const sys = String(p.cec_system_name || "").toLowerCase();
      return name.includes(q) || brand.includes(q) || sys.includes(q);
    })
    .forEach(p => {
      const div = document.createElement("div");
      div.className = "prof-card" + (currentProfile && currentProfile.id === p.id ? " selected" : "");
      div.onclick = () => selectProfileById(p.id);
      const metaDiv = document.createElement("div");
      metaDiv.style.cssText = "font-size:11px; color:var(--apple-blue); font-weight:600;";
      metaDiv.textContent = `${p.brand} • ${p.cec_system_name}`;
      const nameDiv = document.createElement("div");
      nameDiv.style.cssText = "font-size:13px; font-weight:600;";
      nameDiv.textContent = profileShortName(p);
      div.appendChild(metaDiv);
      div.appendChild(nameDiv);
      box.appendChild(div);
    });
}

function renderPaSelect() {
  const portPrefix = typeof t === "function" ? t("settings.port_prefix", "Разъем") : "Разъем";
  const ports = (currentProfile && Array.isArray(currentProfile.hdmi_ports) && currentProfile.hdmi_ports.length > 0)
    ? currentProfile.hdmi_ports
    : [
        { pa: "1000", label: "HDMI 1" },
        { pa: "2000", label: "HDMI 2" },
        { pa: "3000", label: "HDMI 3" },
        { pa: "4000", label: "HDMI 4" }
      ];
  const paOpts = ports.map((hp, i) => ({ value: hp.pa, label: `${portPrefix} ${i + 1} (${hp.label})` }));
  createCustomSelect("csel-pa", paOpts, selectedPa, (val) => { selectedPa = val; });
}
window.renderPaSelect = renderPaSelect;

function selectProfileById(id, options = {}) {
  const prof = tvProfiles.find(p => p.id === id);
  if (!prof) return;
  currentProfile = prof;
  if (appConfig && appConfig.active_profile_id !== prof.id) {
    appConfig.active_profile_id = prof.id;
    if (!options.skipSave && invoke) {
      invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }
  }

  const opts = tvProfiles.map(p => ({ value: p.id, label: `${p.brand} — ${profileShortName(p)}` }));
  createCustomSelect("csel-profile", opts, prof.id, (val) => selectProfileById(val));
  createCustomSelect("csel-profile-sys", opts, prof.id, (val) => selectProfileById(val));

  const inpName = document.getElementById("inp-profile-name");
  if (inpName) inpName.value = prof.name;

  const badge = document.getElementById("min-system-badge");
  if (badge) badge.textContent = `${prof.brand} ${prof.cec_system_name}`;
  const title = document.getElementById("min-tv-title");
  if (title) title.textContent = profileShortName(prof);
  const txtJson = document.getElementById("txt-profile-json");
  if (txtJson) txtJson.value = JSON.stringify(prof, null, 2);

  const strip = document.getElementById("min-hdmi-strip");
  if (strip) {
    strip.innerHTML = "";
    const ports = Array.isArray(prof.hdmi_ports) ? prof.hdmi_ports : [];
    ports.forEach(hp => {
      const chip = document.createElement("div");
      chip.className = "hdmi-chip" + (hp.pa === selectedPa ? " active-input" : "");
      chip.textContent = hp.label;
      chip.onclick = () => switchHdmi(hp.pa, chip);
      strip.appendChild(chip);
    });
  }

  renderPaSelect();

  // Заполнение информационной карточки параметров профиля
  const elBrand = document.getElementById("pinfo-brand");
  if (elBrand) elBrand.textContent = prof.brand || "—";
  const elProto = document.getElementById("pinfo-protocol");
  if (elProto) elProto.textContent = prof.cec_system_name || "HDMI-CEC";
  const elKeysCount = document.getElementById("pinfo-keys-count");
  const supportedKeysList = (prof.supported_keys && prof.supported_keys.length > 0)
    ? prof.supported_keys
    : ((prof.keys && prof.keys.length > 0) ? prof.keys : (typeof DEFAULT_BASE_KEYS !== "undefined" ? DEFAULT_BASE_KEYS : []));
  if (elKeysCount) elKeysCount.textContent = supportedKeysList.length;
  const elDefPa = document.getElementById("pinfo-default-pa");
  if (elDefPa) {
    const defPortObj = (prof.hdmi_ports && prof.hdmi_ports[0]) ? prof.hdmi_ports[0] : null;
    const defPortLabel = defPortObj ? defPortObj.label : "HDMI 1";
    const defPortPa = defPortObj ? defPortObj.pa : (prof.default_pa || "1000");
    elDefPa.textContent = `${defPortLabel} (${defPortPa})`;
  }

  renderProfilesCatalog();

  renderDynamicRemote();
  if (options && options.skipMapper) {
    if (typeof window.invalidateMapperTable === "function") window.invalidateMapperTable();
  } else {
    renderMapperTable();
  }
  if (typeof updateSystemDiagnostics === "function") {
    updateSystemDiagnostics();
  }
  updateDynamicWindowTitle();
}

async function renameCurrentProfile() {
  if (!currentProfile) return;
  const inp = document.getElementById("inp-profile-name");
  if (!inp) return;
  const newName = inp.value.trim();
  if (!newName) {
    addLog("[Профиль] Название не может быть пустым!", "log-err");
    return;
  }
  currentProfile.name = newName;
  currentProfile.display_name = newName;
  if (!appConfig) appConfig = {};
  if (!appConfig.custom_profile_names) appConfig.custom_profile_names = {};
  appConfig.custom_profile_names[currentProfile.id] = newName;

  try {
    if (invoke) {
      await invoke("save_tv_profile", { profile: currentProfile });
      await invoke("save_app_config", { newCfg: appConfig });
      await loadProfiles();
      selectProfileById(currentProfile.id);
      updateDynamicWindowTitle();
      addLog(`[Профиль] Название сохранено: "${newName}"`, "log-ok");
    }
  } catch (e) {
    addLog(`[Ошибка сохранения] ${e}`, "log-err");
  }
}

async function saveCurrentProfileJson() {
  try {
    const parsed = JSON.parse(document.getElementById("txt-profile-json").value);
    await invoke("save_tv_profile", { profile: parsed });
    await loadProfiles();
    selectProfileById(parsed.id);
  } catch (e) {
    addLog(`[Ошибка JSON] ${e}`, "log-err");
  }
}

async function autoSetupTv(options = {}) {
  if (!invoke) return;
  const edid = await invoke("detect_tv_edid").catch(() => null);
  const skipMapper = typeof mapperNeedsRender !== "undefined" ? mapperNeedsRender : false;
  if (edid && edid.pnp_id && edid.pnp_id !== "UNK") {
    if (edid.physical_address && (!appConfig || !appConfig.pa_override)) {
      selectedPa = edid.physical_address;
    }
    const shouldAutoProfile = !currentProfile || !appConfig || !appConfig.active_profile_id || appConfig.active_profile_id === "universal_hdmi_cec";
    if (shouldAutoProfile) {
      const matched = tvProfiles.find(p => Array.isArray(p.edid_pnp_ids) && p.edid_pnp_ids.includes(edid.pnp_id));
      if (matched) selectProfileById(matched.id, { skipMapper });
    }
  } else if (!currentProfile && tvProfiles.length > 0) {
    const targetId = (appConfig && appConfig.active_profile_id) ? appConfig.active_profile_id : "universal_hdmi_cec";
    const fallbackProf = tvProfiles.find(p => p.id === targetId) || tvProfiles[0];
    if (fallbackProf) selectProfileById(fallbackProf.id, { skipMapper });
  }

  if (!isConnected) {
    await toggleConnect();
    return;
  }

  if (!options.skipState) {
    await sendCmd("STATE");
    if (appConfig && appConfig.remote_to_os_enabled === false) {
      setTimeout(() => sendCmd("REMOTE_OFF"), 300);
    }
  }
  setTimeout(() => sendCmd("RAW 00 8F"), 420);
}

function updateConnectionButtonsState(connected) {
  isConnected = !!connected;
  if (!isConnected) {
    connectedPort = "";
  }
  const isSamePortConnected = isConnected && selectedPort === connectedPort;
  const btnConn = document.getElementById("btn-conn");
  const btnDisconn = document.getElementById("btn-disconn");
  if (btnConn) {
    if (isSamePortConnected) {
      btnConn.setAttribute("disabled", "true");
      btnConn.className = "apple-btn btn-saved-idle";
    } else {
      btnConn.removeAttribute("disabled");
      btnConn.className = "apple-btn accent-blue";
    }
  }
  if (btnDisconn) {
    if (isConnected) {
      btnDisconn.removeAttribute("disabled");
      btnDisconn.className = "apple-btn";
    } else {
      btnDisconn.setAttribute("disabled", "true");
      btnDisconn.className = "apple-btn btn-saved-idle";
    }
  }
}

async function connectDeviceNow(forceSwitch = false) {
  if (!invoke) return;
  if (isConnected && !forceSwitch && selectedPort === connectedPort) return;
  if (!selectedPort) {
    if (typeof window.finishStartupGrace === "function") window.finishStartupGrace();
    updateHardwareStatusUi({ connected: false, error: "Подключите адаптер к USB" }, { checked: false });
    updateDynamicWindowTitle();
    return;
  }
  try {
    lastHwStatus.tvConnected = false;
    lastHwStatus.lineHigh = false;
    confirmedLineLow = false;
    await invoke("connect_esp32", { portName: selectedPort });
    connectedPort = selectedPort;
    updateConnectionButtonsState(true);
    updateHardwareStatusUi({ connected: true, port: selectedPort }, { checked: false });
    updateDynamicWindowTitle();
    addLog(`[USB] Контроллер подключен: ${selectedPort}`, "log-ok");
    autoSetupTv({ skipState: true });
    setTimeout(() => {
      if (isConnected && invoke) {
        invoke("send_cec_command", { cmd: "INFO" }).catch(() => {});
      }
    }, 250);
    if (appConfig && appConfig.auto_wake_tv_on_start && !hasAutoWokenTvOnce) {
      hasAutoWokenTvOnce = true;
      setTimeout(() => wakeAndSelectTv(), 500);
    }
  } catch (e) {
    if (typeof window.finishStartupGrace === "function") window.finishStartupGrace();
    updateConnectionButtonsState(false);
    updateHardwareStatusUi({ connected: false, error: `Порт ${selectedPort} недоступен` }, { checked: false });
    updateDynamicWindowTitle();
    addLog(`[Ошибка подключения] ${e}`, "log-err");
  }
}

async function disconnectDeviceNow() {
  if (!invoke) return;
  if (typeof window.finishStartupGrace === "function") window.finishStartupGrace();
  await invoke("disconnect_esp32");
  connectedPort = "";
  detectedHardwareModel = "";
  detectedFirmwareVersion = "";
  lastHwStatus.tvConnected = false;
  lastHwStatus.lineHigh = false;
  updateConnectionButtonsState(false);
  updateHardwareStatusUi({ connected: false, error: "Адаптер отключен" }, { checked: false });
  updateDynamicWindowTitle();
  if (typeof checkFirmwareUpdatesUi === "function") checkFirmwareUpdatesUi(true);
  addLog("[USB] Контроллер отключен вручную", "log-info");
}

async function toggleConnect() {
  if (isConnected && selectedPort === connectedPort) {
    await disconnectDeviceNow();
  } else {
    await connectDeviceNow(true);
  }
}

const STANDALONE_HID_BUTTONS = [
  { hex: "0x00", ru: "OK (Ввод)", en: "OK (Select)" },
  { hex: "0x0D", ru: "Назад (Exit / Return)", en: "Back (Exit / Return)" },
  { hex: "0x71", ru: "Синяя (Blue)", en: "Blue" },
  { hex: "0x72", ru: "Красная (Red)", en: "Red" },
  { hex: "0x73", ru: "Зеленая (Green)", en: "Green" },
  { hex: "0x74", ru: "Желтая (Yellow)", en: "Yellow" },
  { hex: "0x09", ru: "Меню (Menu)", en: "Menu" },
  { hex: "0x0B", ru: "Info", en: "Info" }
];

const STANDALONE_HID_ACTIONS = [
  { value: "NONE", ru: "Отключено", en: "Disabled" },
  { value: "MOUSE_TOGGLE", ru: "Переключить Мышь / Смарт-пульт", en: "Toggle Mouse / Smart Remote" },
  { value: "WIN_OSK", ru: "Экранная клавиатура (Win+Ctrl+O)", en: "Windows OSK (Win+Ctrl+O)" },
  { value: "AUTO_LOGIN", ru: "Макрос: Логин одной кнопкой", en: "Macro: One-Button Login" },
  { value: "ENTER", ru: "Клавиша Enter", en: "Enter Key" },
  { value: "ESC", ru: "Клавиша Esc", en: "Esc Key" },
  { value: "BACKSPACE", ru: "Клавиша Backspace", en: "Backspace Key" },
  { value: "TAB", ru: "Клавиша Tab", en: "Tab Key" },
  { value: "SPACE", ru: "Клавиша Пробел", en: "Space Key" },
  { value: "RIGHT_CLICK", ru: "Правый клик мыши (ПКМ)", en: "Mouse Right Click" },
  { value: "DOUBLE_CLICK", ru: "Двойной клик (2x ЛКМ)", en: "Mouse Double Click" },
  { value: "CTRL_C", ru: "Ctrl + C", en: "Ctrl + C" },
  { value: "CTRL_V", ru: "Ctrl + V", en: "Ctrl + V" },
  { value: "CTRL_A", ru: "Ctrl + A", en: "Ctrl + A" },
  { value: "ALT_TAB", ru: "Alt + Tab", en: "Alt + Tab" },
  { value: "WIN_L", ru: "Блокировка ПК (Win + L)", en: "Lock PC (Win + L)" }
];

let appToastTimer = null;
function showAppToast(message, isError = false) {
  const el = document.getElementById("app-toast");
  if (!el) return;
  el.textContent = String(message || "");
  el.classList.toggle("is-error", !!isError);
  el.classList.add("is-visible");
  if (appToastTimer) clearTimeout(appToastTimer);
  appToastTimer = setTimeout(() => {
    el.classList.remove("is-visible");
  }, 2800);
}

function setStandaloneHidDirty(dirty) {
  const btn = document.getElementById("btn-save-standalone-hid");
  if (!btn) return;
  btn.disabled = !dirty;
  btn.classList.toggle("accent-blue", !!dirty);
}

function updateStandaloneHidBodyVisibility() {
  const chk = document.getElementById("chk-standalone-hid");
  const body = document.getElementById("wrap-standalone-hid-body");
  if (!body) return;
  body.classList.toggle("is-collapsed", !(chk && chk.checked));
}

function renderStandaloneHidConfig() {
  if (!appConfig) return;
  const chk = document.getElementById("chk-standalone-hid");
  if (chk) chk.checked = !!appConfig.standalone_hid_enabled;
  updateStandaloneHidBodyVisibility();

  const inp = document.getElementById("inp-standalone-pin");
  if (inp) inp.value = appConfig.standalone_login_pin || "";

  const list = document.getElementById("standalone-hid-list");
  if (!list) return;
  list.innerHTML = "";
  const isEn = typeof currentLang !== "undefined" && currentLang === "en";
  const maps = appConfig.standalone_hid_mappings || {};
  const opts = STANDALONE_HID_ACTIONS.map(a => ({ value: a.value, label: isEn ? a.en : a.ru }));

  STANDALONE_HID_BUTTONS.forEach((btn) => {
    const row = document.createElement("div");
    row.className = "row";
    row.style.gap = "8px";
    row.style.alignItems = "center";

    const lbl = document.createElement("span");
    lbl.style.fontSize = "12px";
    lbl.style.minWidth = "142px";
    lbl.style.color = "var(--text-primary)";
    lbl.textContent = isEn ? btn.en : btn.ru;

    const sel = document.createElement("div");
    sel.className = "c-select";
    sel.style.flex = "1";
    const fallbackDef = btn.hex === "0x00" ? "ENTER" : (btn.hex === "0x0D" ? "ESC" : "NONE");
    const curVal = maps[btn.hex] || fallbackDef;
    createCustomSelect(sel, opts, curVal, (val) => {
      if (!appConfig.standalone_hid_mappings) appConfig.standalone_hid_mappings = {};
      appConfig.standalone_hid_mappings[btn.hex] = val;
      setStandaloneHidDirty(true);
    });

    row.appendChild(lbl);
    row.appendChild(sel);
    list.appendChild(row);
  });

  setStandaloneHidDirty(false);
}

async function saveStandaloneHidConfig(opts = {}) {
  if (!invoke || !appConfig) return;
  const chk = document.getElementById("chk-standalone-hid");
  const inp = document.getElementById("inp-standalone-pin");
  if (chk) appConfig.standalone_hid_enabled = chk.checked;
  if (inp) appConfig.standalone_login_pin = inp.value;

  const allowedHex = new Set(STANDALONE_HID_BUTTONS.map(b => b.hex));
  const cleanedMaps = {};
  const existingMaps = appConfig.standalone_hid_mappings || {};
  STANDALONE_HID_BUTTONS.forEach((btn) => {
    const fallbackDef = btn.hex === "0x00" ? "ENTER" : (btn.hex === "0x0D" ? "ESC" : "NONE");
    cleanedMaps[btn.hex] = existingMaps[btn.hex] || fallbackDef;
  });
  Object.keys(cleanedMaps).forEach((k) => {
    if (!allowedHex.has(k)) delete cleanedMaps[k];
  });
  appConfig.standalone_hid_mappings = cleanedMaps;

  try {
    await invoke("save_app_config", { newCfg: appConfig });
    setStandaloneHidDirty(false);
    const _t = typeof t === "function" ? t : (_k, fb) => fb;
    const toastMsg = _t(
      opts.toastKey || "settings.standalone_saved_toast",
      opts.toastFallback || "HID-профиль записан в контроллер"
    );
    showAppToast(toastMsg, false);
    if (typeof addLog === "function") {
      addLog(_t("settings.standalone_saved_log", "[HID] Автономный USB HID-профиль сохранён и отправлен в контроллер."), "log-ok");
    }
  } catch (e) {
    showAppToast(String(e), true);
    if (typeof addLog === "function") addLog(`[HID Ошибка] ${e}`, "log-err");
  }
}

function renderPcOffModeSelect() {
  const curMode = (appConfig && appConfig.pc_off_action) ? appConfig.pc_off_action : "SLEEP";
  const modeOpts = [
    { value: "SLEEP", label: typeof t === "function" ? t("settings.pc_mode_sleep", "Режим сна (Sleep S3 — без выключения)") : "Режим сна (Sleep S3)" },
    { value: "SHUTDOWN", label: typeof t === "function" ? t("settings.pc_mode_shutdown", "Полное выключение ОС (Shutdown)") : "Полное выключение ОС" },
    { value: "HIBERNATE", label: typeof t === "function" ? t("settings.pc_mode_hibernate", "Гибернация (Hibernate)") : "Гибернация" },
    { value: "OPTO_PULSE", label: typeof t === "function" ? t("settings.pc_mode_opto", "Импульс оптопары (Кнопка Power 350 мс)") : "Импульс оптопары (Power 350 мс)" }
  ];
  createCustomSelect("csel-pc-off-mode", modeOpts, curMode, (val) => {
    if (appConfig) {
      appConfig.pc_off_action = val;
      autoSaveQolSettings();
    }
  });
  updatePcOffModeEnabledState();
}

/**
 * Блокировка/разблокировка выбора режима сна при выключении ТВ
 */
function updatePcOffModeEnabledState() {
  const chk = document.getElementById("chk-pc-off-on-tv");
  const wrap = document.getElementById("wrap-pc-off-mode");
  if (!wrap) return;
  const isEnabled = chk ? chk.checked : false;
  if (isEnabled) {
    wrap.classList.remove("is-disabled");
  } else {
    wrap.classList.add("is-disabled");
    const sel = wrap.querySelector(".c-select.open");
    if (sel) closeAllCustomSelects();
  }
}

function onPcOffOnTvToggleChanged() {
  updatePcOffModeEnabledState();
  autoSaveQolSettings();
}
window.onPcOffOnTvToggleChanged = onPcOffOnTvToggleChanged;
window.updatePcOffModeEnabledState = updatePcOffModeEnabledState;

let savedOsdName = "GAMING PC";

async function sendCmd(cmd) {
  if (!invoke || !cmd) return;
  if (typeof window.setAmbientMood === "function") {
    window.setAmbientMood("amber", 1200);
  }
  try {
    await invoke("send_cec_command", { cmd });
  } catch (err) {
    if (typeof window.setAmbientMood === "function") {
      window.setAmbientMood("red", 2500);
    }
    addLog(`[Ошибка отправки (${cmd})] ${err}`, "log-err");
  }
}

function checkOsdDirtyState() {
  const inp = document.getElementById("inp-osd");
  const btn = document.getElementById("btn-save-osd");
  if (!inp || !btn) return;
  const curVal = inp.value.trim();
  if (curVal !== savedOsdName && curVal.length > 0) {
    btn.className = "apple-btn accent-blue";
    btn.removeAttribute("disabled");
  } else {
    btn.className = "apple-btn btn-saved-idle";
    btn.setAttribute("disabled", "true");
  }
}

async function saveOsdName() {
  const inp = document.getElementById("inp-osd");
  if (!inp) return;
  const name = inp.value.trim().substring(0, 14);
  if (!name) return;
  savedOsdName = name;
  checkOsdDirtyState();
  if (appConfig) {
    appConfig.osd_name = name;
    if (invoke) {
      await invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }
  }
  await sendCmd(`NAME ${name}`);
  addLog(`[OSD] Имя компьютера сохранено: ${name}`, "log-ok");
  setHumanStatus("online", "Готово", `Имя ТВ: ${name}`);
}

async function applyPaOverride() {
  if (appConfig) {
    appConfig.pa_override = selectedPa;
    if (invoke) {
      await invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }
  }
  if (typeof updateSystemDiagnostics === "function") {
    updateSystemDiagnostics();
  }
  await sendCmd(`PA ${selectedPa}`);
  addLog(`[HDMI] Номер разъема применен: ${selectedPa}`, "log-ok");
}

async function applyPaAndName() {
  await saveOsdName();
  await applyPaOverride();
}

async function wakeAndSelectTv() {
  setHumanStatus("online", "Готово", "Включение телевизора");
  const seq = (currentProfile && Array.isArray(currentProfile.wake_sequence) && currentProfile.wake_sequence.length > 0)
    ? currentProfile.wake_sequence
    : ["ACTIVE"];
  for (let i = 0; i < seq.length; i++) {
    await sendCmd(seq[i]);
    if (i + 1 < seq.length) {
      await new Promise(r => setTimeout(r, 120));
    }
  }
  const tvTarget = (appConfig && appConfig.tv_audio_device) ? appConfig.tv_audio_device.trim() : "";
  if (appConfig && appConfig.auto_audio_switch && tvTarget && invoke) {
    await invoke("set_audio_device", { target: tvTarget }).catch(() => {});
  }
}

async function standbyTv() {
  setHumanStatus("standby", "Настройка", "Выключение экрана");
  const cmd = (currentProfile && currentProfile.standby_command) ? currentProfile.standby_command : "OFF";
  await sendCmd(cmd);
  const fbTarget = (appConfig && appConfig.fallback_audio_device) ? appConfig.fallback_audio_device.trim() : "";
  if (appConfig && appConfig.auto_audio_switch && fbTarget && invoke) {
    await invoke("set_audio_device", { target: fbTarget }).catch(() => {});
  }
}

let cachedAudioEndpoints = null;

function updateAudioDevicesVisibility() {
  const chk = document.getElementById("chk-auto-audio");
  const wrap = document.getElementById("wrap-audio-devices");
  if (!wrap) return;
  const enabled = chk ? chk.checked : false;
  wrap.style.display = "";
  wrap.classList.toggle("is-visible", enabled);
  if (enabled && cachedAudioEndpoints === null) {
    loadAndRenderAudioDeviceSelects();
  } else if (enabled) {
    renderAudioDeviceSelects();
  }
}

function renderAudioDeviceSelects() {
  const list = Array.isArray(cachedAudioEndpoints) ? cachedAudioEndpoints : [];
  const noneLabel = typeof t === "function" ? t("settings.audio_device_none", "— Не выбрано —") : "— Не выбрано —";
  const opts = [{ value: "", label: noneLabel }];
  const tvCur = (appConfig && appConfig.tv_audio_device) ? appConfig.tv_audio_device : "";
  const fbCur = (appConfig && appConfig.fallback_audio_device) ? appConfig.fallback_audio_device : "";
  const seen = new Set([""]);
  list.forEach(name => {
    if (name && !seen.has(name)) {
      seen.add(name);
      opts.push({ value: name, label: name });
    }
  });
  if (tvCur && !seen.has(tvCur)) {
    opts.push({ value: tvCur, label: `${tvCur} (offline)` });
  }
  if (fbCur && fbCur !== tvCur && !seen.has(fbCur)) {
    opts.push({ value: fbCur, label: `${fbCur} (offline)` });
  }

  createCustomSelect("csel-tv-audio", opts, tvCur, (val) => {
    if (appConfig) {
      appConfig.tv_audio_device = val;
      if (typeof autoSaveQolSettings === "function") autoSaveQolSettings();
    }
  });
  createCustomSelect("csel-fallback-audio", opts, fbCur, (val) => {
    if (appConfig) {
      appConfig.fallback_audio_device = val;
      if (typeof autoSaveQolSettings === "function") autoSaveQolSettings();
    }
  });
}

async function loadAndRenderAudioDeviceSelects() {
  if (!invoke) {
    cachedAudioEndpoints = [];
    renderAudioDeviceSelects();
    return;
  }
  try {
    const endpoints = await invoke("list_audio_endpoints");
    cachedAudioEndpoints = Array.isArray(endpoints) ? endpoints : [];
  } catch (_) {
    cachedAudioEndpoints = [];
  }
  renderAudioDeviceSelects();
}

function onAutoAudioToggleChanged() {
  updateAudioDevicesVisibility();
  if (typeof autoSaveQolSettings === "function") autoSaveQolSettings();
}
window.onAutoAudioToggleChanged = onAutoAudioToggleChanged;
window.renderAudioDeviceSelects = renderAudioDeviceSelects;

async function switchHdmi(pa, chipEl) {
  selectedPa = pa;
  if (appConfig) {
    appConfig.pa_override = pa;
    if (invoke) {
      invoke("save_app_config", { newCfg: appConfig }).catch(() => {});
    }
  }
  if (chipEl) {
    document.querySelectorAll(".hdmi-chip").forEach(c => c.classList.remove("active-input"));
    chipEl.classList.add("active-input");
  }
  renderPaSelect();
  if (typeof updateSystemDiagnostics === "function") {
    updateSystemDiagnostics();
  }
  await sendCmd(`PA ${pa}`);
  const b1 = pa.substring(0, 2);
  const b2 = pa.substring(2, 4);
  await sendCmd(`RAW 0F 86 ${b1} ${b2}`);
  setTimeout(() => sendCmd(`RAW 0F 82 ${b1} ${b2}`), 240);
}

async function sendKey(hexCode) {
  await sendCmd(`KEY ${hexCode}`);
}

async function pressOsKey(action) {
  if (!invoke) return;
  const desc = await invoke("trigger_os_key", { action });
  addLog(`[Клавиша] ${desc}`, "log-rem", true);
}

async function toggleRemotePass() {
  if (!invoke) return;
  const chk = document.getElementById("chk-remote-os");
  const enabled = chk ? chk.checked : true;
  if (appConfig) appConfig.remote_to_os_enabled = enabled;
  await invoke("set_remote_passthrough", { enabled });
  updateHardwareStatusUi(
    { connected: isConnected, port: selectedPort },
    { checked: true, tvConnected: lastHwStatus.tvConnected, lineHigh: lastHwStatus.lineHigh },
    enabled
  );
  if (enabled) {
    addLog("[Пульт] Перехват включен (ПК)", "log-ok");
  } else {
    addLog("[Пульт] Перехват выключен (ТВ)", "log-info");
  }
}

function sendCustomRaw() {
  const v = document.getElementById("inp-raw").value.trim();
  if (v) sendCmd(v);
}

// Логика OTA-обновлений, онлайн-каталога ТВ и прошивки вынесена в js/updater_ui.js

/**
 * Динамический заголовок окна ОС (Модель ТВ • Модель USB адаптера)
 */
let lastSentWindowTitle = "";

function updateDynamicWindowTitle() {
  let tvModel = typeof t === "function" ? t("window.title_tv", "ТВ") : "ТВ";
  if (currentProfile && currentProfile.name) {
    tvModel = profileShortName(currentProfile);
  }

  const model = getAdapterModelDisplay();
  const port = selectedPort || (lastHwStatus && lastHwStatus.port) || "";
  const statusConnecting = typeof t === "function" ? t("remote.connecting", "Подключение...") : "Подключение...";
  const statusDisc = typeof t === "function" ? t("remote.status_disconnected", "Отключен") : "Отключен";

  const fwBadge = document.getElementById("lbl-fw-chip-badge");
  if (fwBadge) {
    fwBadge.textContent = detectedFirmwareVersion
      ? `${model} • FW v${detectedFirmwareVersion}`
      : `${model} • FW —`;
  }

  const adapterStatus = isConnected
    ? `${model} (${port || "USB"})`
    : `${model} (${selectedPort ? statusConnecting : statusDisc})`;

  const dynamicTitle = `CecDesk — ${tvModel} • ${adapterStatus}`;
  if (document.title !== dynamicTitle) {
    document.title = dynamicTitle;
  }

  if (invoke && dynamicTitle !== lastSentWindowTitle) {
    lastSentWindowTitle = dynamicTitle;
    invoke("set_window_title", { title: dynamicTitle }).catch(() => {});
  }
}

// ==================== ИНИЦИАЛИЗАЦИЯ ПРИЛОЖЕНИЯ ====================
window.addEventListener("DOMContentLoaded", async () => {
  initUiBindings();
  initTitlebarDragging();

  const osLabel = document.getElementById("lbl-os-platform");
  if (osLabel) {
    const ua = (navigator.userAgent || "").toLowerCase();
    const plat = (navigator.platform || "").toLowerCase();
    if (ua.includes("linux") || plat.includes("linux")) {
      osLabel.textContent = "Linux / SteamOS x64";
    } else if (ua.includes("mac") || plat.includes("mac")) {
      osLabel.textContent = "macOS";
    } else {
      osLabel.textContent = "Windows 11 / 10 x64";
    }
  }

  updateTvPowerButtonUi("syncing");
  initFloatingTooltips();

  const listenPromise = listen ? Promise.all([
    listen("cec-rx-line", (ev) => {
      const line = ev.payload;
      const summary = decodeCecLine(line);
      if (line.startsWith("OK:INFO_HW_")) {
        const parts = line.split("_");
        if (parts.length >= 3) {
          detectedHardwareModel = parts[2];
          if (parts.length >= 5) {
            detectedFirmwareVersion = parts[4].trim();
          }
          updateDynamicWindowTitle();
          if (typeof checkFirmwareUpdatesUi === "function") {
            checkFirmwareUpdatesUi(true);
          }
        }
      }

      let cls = "log-ok";
      if (line.startsWith("RX ")) cls = "log-rx";
      else if (line.startsWith("TX ")) cls = "log-tx";
      else if (line.includes("ERR") || line.includes("Transmit: 0")) cls = "log-err";

      if (summary) {
        addLog(`${line.padEnd(26, ' ')} │ ${summary}`, cls);
      } else {
        addLog(line, cls);
      }
    }),

    listen("cec-hardware-status", (ev) => {
      const p = ev.payload;
      lastHwStatus.tvConnected = p.tv_connected;
      lastHwStatus.lineHigh = p.line_high;
      updateHardwareStatusUi(
        { connected: isConnected, port: selectedPort },
        { checked: true, tvConnected: p.tv_connected, lineHigh: p.line_high },
        document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
      );
    }),

    listen("cec-remote-key", (ev) => {
      const p = ev.payload;
      pulseVirtualRemoteButton(p.code);

      if (isCalibratingRemote) {
        handleCalibrationKeyCapture(p.code);
      }

      const msg = `${p.button} : ${p.os_action}`;
      addLog(`[Пульт] ${msg}`, "log-rem");
    }),

    listen("cec-remote-release", () => {
      document.querySelectorAll(".rem-active").forEach(el => el.classList.remove("rem-active"));
    }),

    listen("mouse-mode-changed", (ev) => {
      const isMouse = ev.payload;
      if (isMouse) {
        setHumanStatus(
          "online",
          typeof t === "function" ? t("status.mouse_on", "Мышь ВКЛ") : "Мышь ВКЛ",
          typeof t === "function" ? t("status.mouse_on_desc", "Управление курсором (Стрелки / ЛКМ / ПКМ)") : "Управление курсором (Стрелки / ЛКМ / ПКМ)"
        );
      } else {
        setHumanStatus(
          "online",
          typeof t === "function" ? t("status.mouse_off", "Готово") : "Готово",
          typeof t === "function" ? t("status.mouse_off_desc", "Обычный режим пульта") : "Обычный режим пульта"
        );
      }
    }),

    listen("cec-disconnected", () => {
      connectedPort = "";
      updateConnectionButtonsState(false);
      updateHardwareStatusUi(
        { connected: false, error: "Потеряна связь с USB-портом" },
        { checked: false },
        document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
      );
      updateDynamicWindowTitle();
      addLog("[USB] Устройство отключено или порт закрыт!", "log-err");
    }),

    listen("cec-auto-reconnecting", async (ev) => {
      const port = ev.payload;
      if (!isConnected) {
        selectedPort = port;
        await refreshPorts();
        updateHardwareStatusUi(
          { connected: false, reconnecting: true, port },
          { checked: false },
          document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
        );
        updateDynamicWindowTitle();
        addLog(`[Watchdog] Авто-восстановление USB (${port})...`, "log-ok");
        await connectDeviceNow(true);
      }
    }),

    listen("cec-toast", (ev) => {
      const msg = ev.payload;
      addLog(`[Система] ${msg}`, "log-ok");
    }),

    listen("request-close-confirm", () => {
      openCloseConfirmModal();
    }),

    listen("key-profile-changed", (ev) => {
      const newCfg = ev.payload;
      if (!newCfg) return;
      appConfig = newCfg;
      if (typeof syncKeyProfileStripUi === "function") syncKeyProfileStripUi();
      if (typeof renderMapperTable === "function") renderMapperTable(true);
      if (typeof renderDynamicRemote === "function") renderDynamicRemote();
      if (typeof captureSavedState === "function") captureSavedState();
      const activeId = appConfig.active_key_profile || "balanced";
      addLog(`[Профиль кнопок] Переключен с пульта: ${activeId}`, "log-ok");
    }),

    listen("fw-flash-progress", (ev) => {
      const p = ev.payload;
      if (!p) return;
      const wrap = document.getElementById("fw-progress-wrap");
      const bar = document.getElementById("fw-progress-bar");
      const txt = document.getElementById("lbl-fw-progress-text");
      if (wrap) {
        wrap.style.display = "";
        wrap.classList.add("is-visible");
      }
      if (bar) {
        bar.style.width = `${p.percent || 0}%`;
        bar.style.background = p.percent >= 100 ? "var(--apple-green)" : "var(--apple-blue)";
      }
      if (txt) {
        txt.textContent = p.message || "";
        txt.style.color = p.percent >= 100 ? "var(--apple-green)" : "var(--text-secondary)";
      }
    }),

    listen("app-update-progress", (ev) => {
      const p = ev.payload;
      if (!p) return;
      const wrap = document.getElementById("app-update-progress-wrap");
      const bar = document.getElementById("app-update-progress-bar");
      const txt = document.getElementById("lbl-app-update-progress-text");
      if (wrap) {
        wrap.classList.add("is-visible");
      }
      if (bar) {
        bar.style.width = `${p.percent || 0}%`;
        bar.style.background = p.percent >= 100 ? "var(--apple-green)" : "var(--apple-blue)";
      }
      if (txt) {
        txt.textContent = p.message || "";
        txt.style.color = p.percent >= 100 ? "var(--apple-green)" : "var(--text-secondary)";
      }
    })
  ]) : Promise.resolve();

  if (invoke) {
    const [
      ver,
      cfg,
      profiles,
      ports,
      scripts,
      logPaths,
      autostartEnabled,
      osLabel
    ] = await Promise.all([
      invoke("get_app_version").catch(() => null),
      invoke("get_app_config").catch(() => null),
      invoke("load_tv_profiles").catch(() => []),
      invoke("list_serial_ports").catch(() => []),
      invoke("list_custom_scripts").catch(() => []),
      invoke("get_log_paths").catch(() => null),
      invoke("get_autostart").catch(() => false),
      invoke("get_os_platform_label").catch(() => null),
      listenPromise
    ]);

    if (osLabel) {
      const lblOs = document.getElementById("lbl-os-platform");
      if (lblOs) lblOs.textContent = osLabel;
    }

    if (Array.isArray(scripts)) {
      customScripts = scripts;
    }

    if (ver) {
      const lblVer = document.getElementById("lbl-app-version-val");
      if (lblVer) lblVer.textContent = ver;
      const btnVer = document.getElementById("btn-check-update");
      if (btnVer) btnVer.textContent = `v${ver}`;
    }

    const chkAuto = document.getElementById("chk-autostart");
    if (chkAuto) chkAuto.checked = !!autostartEnabled;

    if (logPaths && logPaths.latest_log) {
      const lbl = document.getElementById("lbl-log-file");
      if (lbl) lbl.textContent = logPaths.latest_log;
    }

    if (cfg) {
      appConfig = cfg;
      if (typeof applyLocalization === "function") {
        applyLocalization(appConfig.language || "ru");
      }
      if (typeof syncKeyProfileStripUi === "function") {
        syncKeyProfileStripUi();
      }
      const osdInp = document.getElementById("inp-osd");
      if (osdInp) {
        osdInp.value = appConfig.osd_name || "GAMING PC";
        savedOsdName = osdInp.value.trim();
        checkOsdDirtyState();
      }
      selectedPa = appConfig.pa_override || "1000";
      const chkRem = document.getElementById("chk-remote-os");
      if (chkRem) chkRem.checked = appConfig.remote_to_os_enabled;

      document.getElementById("chk-auto-audio").checked = !!appConfig.auto_audio_switch;
      document.getElementById("chk-soundbar").checked = !!appConfig.soundbar_passthrough;
      document.getElementById("chk-hold-seek").checked = appConfig.hold_seek_accelerate !== false;
      document.getElementById("chk-emergency-kill").checked = !!appConfig.emergency_kill_enabled;
      document.getElementById("chk-non-intrusive").checked = appConfig.non_intrusive_cec !== false;
      document.getElementById("chk-auto-heal").checked = appConfig.auto_healing_usb !== false;
      document.getElementById("chk-disp-sleep").checked = !!appConfig.display_disable_on_sleep;
      const chkHud = document.getElementById("chk-hud-overlay");
      if (chkHud) chkHud.checked = appConfig.hud_enabled !== false;
      renderCloseBehaviorSelect();

      const chkTel = document.getElementById("chk-telemetry-opt-in");
      if (chkTel) chkTel.checked = !!appConfig.telemetry_opt_in;
      if (typeof updateTelemetryEndpointVisibility === "function") updateTelemetryEndpointVisibility();
      const inpTel = document.getElementById("inp-telemetry-endpoint");
      if (inpTel) inpTel.value = appConfig.telemetry_endpoint || "";
      updateAudioDevicesVisibility();

      const chkWakePc = document.getElementById("chk-wake-pc-on-tv");
      if (chkWakePc) chkWakePc.checked = appConfig.wake_pc_on_tv_on !== false;
      const chkWakeTv = document.getElementById("chk-wake-tv-on-pc");
      if (chkWakeTv) chkWakeTv.checked = !!appConfig.auto_wake_tv_on_start;
      const chkTvOff = document.getElementById("chk-tv-off-on-pc");
      if (chkTvOff) chkTvOff.checked = !!appConfig.auto_standby_tv_on_exit;
      const chkPcOff = document.getElementById("chk-pc-off-on-tv");
      if (chkPcOff) chkPcOff.checked = !!appConfig.power_off_pc_on_tv_off;
      renderPcOffModeSelect();
      renderStandaloneHidConfig();
    }

    populateProfiles(profiles, { skipMapper: true });
    const prefPort = populatePortsSelect(ports);
    const hasPreferredAdapter = availablePortsList.some(p => p.name === prefPort && p.is_preferred);

    if (prefPort && hasPreferredAdapter) {
      if (typeof window.startStartupGrace === "function") window.startStartupGrace(900);
      updateHardwareStatusUi(
        { connected: false, reconnecting: true, port: prefPort },
        { checked: false },
        document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
      );
      updateDynamicWindowTitle();
      // Начинаем подключение сразу без блокировки первого кадра окна
      connectDeviceNow(true);
    } else {
      if (typeof window.finishStartupGrace === "function") window.finishStartupGrace();
      updateHardwareStatusUi(
        { connected: false, error: "Подключите адаптер к USB" },
        { checked: false },
        document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
      );
      updateDynamicWindowTitle();
    }
  } else {
    await listenPromise;
    if (typeof window.finishStartupGrace === "function") window.finishStartupGrace();
  }

  // Первый кадр полностью подготовлен: снимаем блокировку анимаций и открываем окно без морганий
  document.body.classList.remove("app-initializing");
  void document.body.offsetHeight;
  setTimeout(() => {
    if (invoke) {
      invoke("show_main_window")
        .then(() => {
          if (typeof window.resumeAmbientMist === "function") {
            window.resumeAmbientMist();
          }
        })
        .catch(() => {});
    }
  }, 0);

  // Отложенная отрисовка таблицы кнопок и проверка обновлений после показа главного окна
  setTimeout(() => {
    if (typeof window.ensureMapperRendered === "function") {
      window.ensureMapperRendered();
    }
  }, 300);
  setTimeout(checkPendingCrashOnStartup, 650);
  setTimeout(() => checkAppUpdatesUi(true), 1800);
  setTimeout(() => {
    if (invoke && appConfig && appConfig.telemetry_opt_in && appConfig.telemetry_endpoint && appConfig.telemetry_endpoint.trim()) {
      invoke("send_telemetry_heartbeat", { trigger: "startup" }).catch(() => {});
    }
  }, 3200);

  // Глобальные горячие клавиши: Ctrl+1..4 переключение вкладок, Ctrl+S сохранение маппера
  window.addEventListener("keydown", (e) => {
    if (e.ctrlKey && !e.shiftKey && !e.altKey) {
      if (e.key === "1") {
        e.preventDefault();
        const b = document.querySelectorAll(".seg-btn")[0];
        if (b) switchViewIndex(0, b);
      } else if (e.key === "2") {
        e.preventDefault();
        const b = document.querySelectorAll(".seg-btn")[1];
        if (b) switchViewIndex(1, b);
      } else if (e.key === "3") {
        e.preventDefault();
        const b = document.querySelectorAll(".seg-btn")[2];
        if (b) switchViewIndex(2, b);
      } else if (e.key === "4") {
        e.preventDefault();
        const b = document.querySelectorAll(".seg-btn")[3];
        if (b) switchViewIndex(3, b);
      } else if (e.key === "s" || e.key === "S" || e.key === "ы" || e.key === "Ы") {
        e.preventDefault();
        saveMapperConfig();
      }
    } else if (e.key === "Escape") {
      if (document.querySelector(".c-select.open")) {
        e.preventDefault();
        closeAllCustomSelects();
        return;
      }
      const bugModal = document.getElementById("bug-report-modal");
      if (bugModal && (bugModal.classList.contains("is-open") || bugModal.style.display === "flex")) {
        e.preventDefault();
        closeBugReportModal();
      }
    }
  });

  const bugModal = document.getElementById("bug-report-modal");
  if (bugModal) {
    bugModal.addEventListener("click", (e) => {
      if (e.target === bugModal) closeBugReportModal();
    });
  }

  initTouchSwipeNavigation();
});

function initTouchSwipeNavigation() {
  const viewport = document.querySelector(".viewport");
  const slider = document.getElementById("screens-slider");
  if (!viewport || !slider) return;

  let startX = 0;
  let startY = 0;
  let startTime = 0;
  let tracking = false;
  let horizontalLocked = false;
  let lastDx = 0;

  const isIgnoredTouchTarget = (el) => {
    if (!el || !el.closest) return false;
    return !!el.closest(
      "input, textarea, select, option, .c-select-menu, .touchpad-surface, .modal-backdrop, [data-no-swipe]"
    );
  };

  viewport.addEventListener("touchstart", (e) => {
    if (e.touches.length !== 1) {
      tracking = false;
      return;
    }
    if (isIgnoredTouchTarget(e.target)) {
      tracking = false;
      return;
    }
    const t = e.touches[0];
    startX = t.clientX;
    startY = t.clientY;
    startTime = Date.now();
    lastDx = 0;
    tracking = true;
    horizontalLocked = false;
  }, { passive: true });

  viewport.addEventListener("touchmove", (e) => {
    if (!tracking || e.touches.length !== 1) return;
    const t = e.touches[0];
    const dx = t.clientX - startX;
    const dy = t.clientY - startY;

    if (!horizontalLocked) {
      if (Math.abs(dy) > 16 && Math.abs(dy) > Math.abs(dx)) {
        tracking = false;
        return;
      }
      if (Math.abs(dx) > 18 && Math.abs(dx) > Math.abs(dy) * 1.25) {
        horizontalLocked = true;
        slider.style.transition = "none";
      } else {
        return;
      }
    }

    lastDx = dx;
    const vw = viewport.clientWidth || window.innerWidth || 1;
    let deltaPct = (dx / vw) * 25;
    if ((currentViewIndex === 0 && dx > 0) || (currentViewIndex === 3 && dx < 0)) {
      deltaPct *= 0.25;
    }
    const basePct = -(currentViewIndex * 25);
    slider.style.transform = `translateX(${basePct + deltaPct}%)`;
  }, { passive: true });

  const finishSwipe = () => {
    if (!tracking) return;
    tracking = false;
    if (!horizontalLocked) return;
    horizontalLocked = false;
    slider.style.transition = "";

    const dt = Date.now() - startTime;
    const vw = viewport.clientWidth || window.innerWidth || 1;
    const threshold = dt < 280 ? 28 : Math.min(85, vw * 0.14);
    let nextIdx = currentViewIndex;
    if (lastDx < -threshold && currentViewIndex < 3) {
      nextIdx = currentViewIndex + 1;
    } else if (lastDx > threshold && currentViewIndex > 0) {
      nextIdx = currentViewIndex - 1;
    }
    const btns = document.querySelectorAll(".seg-btn");
    switchViewIndex(nextIdx, btns[nextIdx] || null);
  };

  viewport.addEventListener("touchend", finishSwipe, { passive: true });
  viewport.addEventListener("touchcancel", finishSwipe, { passive: true });
}

// ==================== ВСПЛЫВАЮЩИЕ ПОДСКАЗКИ (APPLE FLOATING TOOLTIPS) ====================
let activeTooltipEl = null;

function initFloatingTooltips() {
  let tip = document.getElementById("global-floating-tooltip");
  if (!tip) {
    tip = document.createElement("div");
    tip.className = "apple-floating-tooltip";
    tip.id = "global-floating-tooltip";
    tip.innerHTML = '<span class="tooltip-title"></span><span class="tooltip-sub"></span>';
    document.body.appendChild(tip);
  }

  const titleSpan = tip.querySelector(".tooltip-title");
  const subSpan = tip.querySelector(".tooltip-sub");

  document.addEventListener("pointerover", (e) => {
    const target = e.target.closest("[data-tooltip], [title], .remote-btn, .rem-round-btn, .rem-color-btn, .rem-num-btn, .dpad-dir, .clickpad-center, .status-lamp, .win-btn");
    if (!target) return;

    let titleText = target.dataset.tooltip;
    let subText = target.dataset.tooltipSub || "";

    if (target.hasAttribute("title")) {
      if (!titleText) titleText = target.getAttribute("title");
      target.removeAttribute("title"); // Удаляем нативный title во избежание дублирования
    }
    target.querySelectorAll("[title]").forEach(el => el.removeAttribute("title"));

    if (!titleText && !subText) return;

    activeTooltipEl = target;
    titleSpan.textContent = titleText || "";
    titleSpan.style.display = titleText ? "block" : "none";

    subSpan.textContent = subText || "";
    subSpan.style.display = subText ? "block" : "none";

    tip.classList.add("visible");
    positionTooltip(target, tip);
  });

  document.addEventListener("pointerout", (e) => {
    if (activeTooltipEl && (e.target === activeTooltipEl || activeTooltipEl.contains(e.target))) {
      if (e.relatedTarget && activeTooltipEl.contains(e.relatedTarget)) {
        return;
      }
      tip.classList.remove("visible");
      activeTooltipEl = null;
    }
  });

  document.addEventListener("pointerdown", () => {
    if (activeTooltipEl) {
      tip.classList.remove("visible");
      activeTooltipEl = null;
    }
  });

  window.addEventListener("scroll", () => {
    if (activeTooltipEl) {
      tip.classList.remove("visible");
      activeTooltipEl = null;
    }
  }, { capture: true, passive: true });
}

function positionTooltip(target, tip) {
  const rect = target.getBoundingClientRect();
  const tipRect = tip.getBoundingClientRect();

  // По умолчанию размещаем над элементом
  let top = rect.top - tipRect.height - 8;
  let left = rect.left + (rect.width - tipRect.width) / 2;

  // Если сверху не влезает — показываем снизу
  if (top < 8) {
    top = rect.bottom + 8;
  }

  // Ограничиваем по горизонтали экрана
  if (left < 8) left = 8;
  if (left + tipRect.width > window.innerWidth - 8) {
    left = window.innerWidth - tipRect.width - 8;
  }

  tip.style.top = `${Math.round(top)}px`;
  tip.style.left = `${Math.round(left)}px`;
}

// Диагностика, баг-репорты, телеметрия и XML-профили вынесены в js/telemetry_ui.js
