/**
 * key_mapper.js - Назначение действий клавишам пульта, пресеты и кастомные скрипты
 */

let customScripts = [];
let savedMappingsSnapshot = null;
let mapperNeedsRender = true;

function invalidateMapperTable() {
  mapperNeedsRender = true;
}

function ensureMapperRendered() {
  if (mapperNeedsRender) {
    renderMapperTable();
  }
}
window.invalidateMapperTable = invalidateMapperTable;
window.ensureMapperRendered = ensureMapperRendered;

function getCurrentMapperState() {
  const current = {};
  document.querySelectorAll(".mapper-csel").forEach(el => {
    const k = el.getAttribute("data-key");
    if (k) current[k] = el.getAttribute("data-value") || "NONE";
  });
  return current;
}

function captureSavedState() {
  if (document.querySelectorAll(".mapper-csel").length === 0) return;
  savedMappingsSnapshot = JSON.stringify(getCurrentMapperState());
  markMapperClean();
}

function checkMapperDirtyState() {
  if (!savedMappingsSnapshot) {
    markMapperClean();
    return;
  }
  const current = JSON.stringify(getCurrentMapperState());
  if (current !== savedMappingsSnapshot) {
    markMapperDirty();
  } else {
    markMapperClean();
  }
}

function markMapperDirty() {
  const b1 = document.getElementById("btn-save-keys");
  if (b1) {
    b1.className = "apple-btn accent-blue";
    b1.removeAttribute("disabled");
  }
}

function markMapperClean() {
  const b1 = document.getElementById("btn-save-keys");
  if (b1) {
    b1.className = "apple-btn btn-saved-idle";
    b1.setAttribute("disabled", "true");
  }
}

function applyKeyPreset(presetKey, btnEl) {
  const p = KEY_PRESETS[presetKey];
  if (!p || !appConfig) return;

  const presetI18nMap = {
    balanced: "keys.preset_base",
    cinema: "keys.preset_cinema",
    steam: "keys.preset_steam",
    desktop: "keys.preset_surf",
    numpad: "keys.preset_numpad"
  };
  const localizedName = (typeof t === "function" && presetI18nMap[presetKey])
    ? t(presetI18nMap[presetKey], p.name)
    : p.name;
  const hudTitle = typeof t === "function" ? t("keys.preset_hud_title", "Пресет кнопок") : "Пресет кнопок";

  document.querySelectorAll(".preset-btn").forEach(b => b.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");

  appConfig.key_mappings = Object.assign({}, p.mappings);
  renderMapperTable(true);
  if (typeof renderDynamicRemote === "function") renderDynamicRemote();
  checkMapperDirtyState();
  addLog(`[Пресет] Применен: ${p.name}`, "log-ok");

  if (window.__TAURI__ && (!appConfig || appConfig.hud_enabled !== false)) {
    window.__TAURI__.event.emit("show-hud", {
      icon: p.icon,
      title: hudTitle,
      desc: localizedName
    });
  }
}

async function loadCustomScripts() {
  if (!invoke) return;
  try {
    customScripts = await invoke("list_custom_scripts");
  } catch (e) {
    console.error("Ошибка загрузки сценариев:", e);
  }
}

function getFullActionList() {
  const scriptPrefix = typeof t === "function" ? t("action.script_prefix", "Сценарий") : "Сценарий";
  const list = OS_ACTIONS.map(a => {
    const key = "action." + a.value.toLowerCase();
    const label = typeof t === "function" ? t(key, a.label) : a.label;
    return { value: a.value, label };
  });

  if (customScripts && customScripts.length > 0) {
    customScripts.forEach(s => {
      list.push({
        value: s.id,
        label: `[${scriptPrefix}] ${s.name}`
      });
    });
  }
  return list;
}

async function openScriptsFolder() {
  if (invoke) {
    await invoke("open_scripts_folder");
    addLog("[Сценарии] Открыта папка scripts", "log-ok");
  }
}

async function refreshCustomScripts() {
  await loadCustomScripts();
  renderMapperTable();
  addLog(`[Сценарии] Обновлено. Найдено сценариев: ${customScripts.length}`, "log-ok");
}

/**
 * Отрисовка таблицы сопоставления кнопок пульта и команд ОС
 */
function renderMapperTable(isPresetChange = false) {
  if (typeof closeAllCustomSelects === "function") closeAllCustomSelects();
  const tbody = document.getElementById("mapper-tbody");
  if (!tbody) return;
  mapperNeedsRender = false;
  tbody.innerHTML = "";
  const mappings = (appConfig && appConfig.key_mappings) ? appConfig.key_mappings : {};
  const allActions = getFullActionList();
  const activeKeys = getActiveRemoteKeys();
  const activeSet = new Set(activeKeys);

  // Единая карта латинских букв (A, B, C...) для неизвестных кнопок
  const letterMap = getExtraKeyLetterMap(activeKeys);

  // 1. Сначала сортируем доступные на пульте кнопки
  const sortedActive = [...activeKeys].sort((a, b) => {
    const idxA = Object.keys(KEY_DEFINITIONS).indexOf(a);
    const idxB = Object.keys(KEY_DEFINITIONS).indexOf(b);
    return (idxA >= 0 ? idxA : 999) - (idxB >= 0 ? idxB : 999);
  });

  // 2. Затем стандартные кнопки, которых нет на пульте
  const unavailableKeys = Object.keys(KEY_DEFINITIONS).filter(k => !activeSet.has(k));

  const allDisplayKeys = [...sortedActive, ...unavailableKeys];
  const frag = document.createDocumentFragment();
  const scriptPrefix = typeof t === "function" ? t("action.script_prefix", "Сценарий") : "Сценарий";
  const availBadgeText = typeof t === "function" ? t("keys.badge_on_remote") : "На пульте";
  const unavailBadgeText = typeof t === "function" ? t("keys.badge_not_detected") : "Не обнаружена";
  const testText = typeof t === "function" ? t("keys.test") : "Тест";

  allDisplayKeys.forEach((code, idx) => {
    const isAvail = activeSet.has(code);
    const def = getButtonDefinition(code, letterMap);

    const tr = document.createElement("tr");
    tr.className = isAvail ? "row-remote-available" : "row-remote-unavailable";

    const curVal = mappings[code] || "NONE";
    const selId = `csel-map-${idx}`;

    let actionOptions = allActions;
    if (!allActions.some(a => a.value === curVal) && curVal.startsWith("SCRIPT:")) {
      actionOptions = [...allActions, { value: curVal, label: `[${scriptPrefix}] ${curVal.substring(7)}` }];
    }

    const tdKey = document.createElement("td");
    const rowDiv = document.createElement("div");
    rowDiv.style.cssText = "display:flex; align-items:center; gap:8px;";

    const symSpan = document.createElement("span");
    symSpan.className = "key-symbol-badge" + (isAvail ? " badge-active-key" : "");
    symSpan.textContent = def.sym;

    const nameSpan = document.createElement("span");
    nameSpan.style.fontWeight = isAvail ? "600" : "400";
    nameSpan.style.color = isAvail ? "var(--text-primary)" : "var(--text-secondary)";
    nameSpan.textContent = typeof getKeyDisplayName === "function" ? getKeyDisplayName(code, def) : def.name;

    const badgeSpan = document.createElement("span");
    badgeSpan.className = isAvail ? "badge-remote-avail" : "badge-remote-unavail";
    badgeSpan.textContent = isAvail ? availBadgeText : unavailBadgeText;

    rowDiv.appendChild(symSpan);
    rowDiv.appendChild(nameSpan);
    rowDiv.appendChild(badgeSpan);
    tdKey.appendChild(rowDiv);

    const tdSel = document.createElement("td");
    const selEl = document.createElement("div");
    selEl.id = selId;
    selEl.className = "c-select mapper-csel";
    selEl.setAttribute("data-key", code);
    tdSel.appendChild(selEl);

    const tdBtn = document.createElement("td");
    tdBtn.style.textAlign = "right";
    const testBtn = document.createElement("button");
    testBtn.className = "apple-btn";
    testBtn.type = "button";
    testBtn.style.cssText = "padding:5px 11px; font-size:11.5px; border-radius:8px;";
    testBtn.textContent = testText;
    testBtn.addEventListener("click", () => testMapperRow(selId));
    tdBtn.appendChild(testBtn);

    tr.appendChild(tdKey);
    tr.appendChild(tdSel);
    tr.appendChild(tdBtn);

    createCustomSelect(selEl, actionOptions, curVal, () => checkMapperDirtyState());
    frag.appendChild(tr);
  });

  tbody.appendChild(frag);

  if (!isPresetChange) {
    captureSavedState();
  }
}

function testMapperRow(selId) {
  const el = document.getElementById(selId);
  if (el) pressOsKey(el.getAttribute("data-value"));
}

let qolSaveDebounce = null;
async function autoSaveQolSettings() {
  if (!invoke || !appConfig) return;

  appConfig.auto_audio_switch = document.getElementById("chk-auto-audio") ? document.getElementById("chk-auto-audio").checked : false;
  appConfig.soundbar_passthrough = document.getElementById("chk-soundbar") ? document.getElementById("chk-soundbar").checked : false;
  appConfig.hold_seek_accelerate = document.getElementById("chk-hold-seek") ? document.getElementById("chk-hold-seek").checked : true;
  appConfig.emergency_kill_enabled = document.getElementById("chk-emergency-kill") ? document.getElementById("chk-emergency-kill").checked : false;
  appConfig.non_intrusive_cec = document.getElementById("chk-non-intrusive") ? document.getElementById("chk-non-intrusive").checked : true;
  appConfig.auto_healing_usb = document.getElementById("chk-auto-heal") ? document.getElementById("chk-auto-heal").checked : true;
  appConfig.display_disable_on_sleep = document.getElementById("chk-disp-sleep") ? document.getElementById("chk-disp-sleep").checked : false;
  appConfig.hud_enabled = document.getElementById("chk-hud-overlay") ? document.getElementById("chk-hud-overlay").checked : true;

  const tvAudioSel = document.getElementById("csel-tv-audio");
  if (tvAudioSel && tvAudioSel.hasAttribute("data-value")) {
    appConfig.tv_audio_device = tvAudioSel.getAttribute("data-value") || "";
  }
  const fbAudioSel = document.getElementById("csel-fallback-audio");
  if (fbAudioSel && fbAudioSel.hasAttribute("data-value")) {
    appConfig.fallback_audio_device = fbAudioSel.getAttribute("data-value") || "";
  }
  const chkTel = document.getElementById("chk-telemetry-opt-in");
  if (chkTel) {
    appConfig.telemetry_opt_in = chkTel.checked;
  }

  appConfig.wake_pc_on_tv_on = document.getElementById("chk-wake-pc-on-tv") ? document.getElementById("chk-wake-pc-on-tv").checked : true;
  appConfig.auto_wake_tv_on_start = document.getElementById("chk-wake-tv-on-pc") ? document.getElementById("chk-wake-tv-on-pc").checked : false;
  appConfig.auto_standby_tv_on_exit = document.getElementById("chk-tv-off-on-pc") ? document.getElementById("chk-tv-off-on-pc").checked : false;
  appConfig.power_off_pc_on_tv_off = document.getElementById("chk-pc-off-on-tv") ? document.getElementById("chk-pc-off-on-tv").checked : false;

  const modeSel = document.getElementById("csel-pc-off-mode");
  if (modeSel && modeSel.getAttribute("data-value")) {
    appConfig.pc_off_action = modeSel.getAttribute("data-value");
  }

  if (qolSaveDebounce) clearTimeout(qolSaveDebounce);
  qolSaveDebounce = setTimeout(async () => {
    try {
      await invoke("save_app_config", { newCfg: appConfig });
    } catch (e) {
      console.error("Ошибка автосохранения QoL:", e);
    }
  }, 200);
}

async function saveMapperConfig() {
  if (!invoke || !appConfig) return;
  const mapperSelects = document.querySelectorAll(".mapper-csel");
  if (!mapperNeedsRender && mapperSelects.length > 0) {
    mapperSelects.forEach(el => {
      const k = el.getAttribute("data-key");
      appConfig.key_mappings[k] = el.getAttribute("data-value");
    });
  }
  appConfig.osd_name = document.getElementById("inp-osd").value.trim();
  appConfig.pa_override = selectedPa;
  if (currentProfile) appConfig.active_profile_id = currentProfile.id;

  const chkRemote = document.getElementById("chk-remote-os");
  if (chkRemote) appConfig.remote_to_os_enabled = chkRemote.checked;

  appConfig.auto_audio_switch = document.getElementById("chk-auto-audio").checked;
  appConfig.soundbar_passthrough = document.getElementById("chk-soundbar").checked;
  appConfig.hold_seek_accelerate = document.getElementById("chk-hold-seek").checked;
  appConfig.emergency_kill_enabled = document.getElementById("chk-emergency-kill").checked;
  appConfig.non_intrusive_cec = document.getElementById("chk-non-intrusive").checked;
  appConfig.auto_healing_usb = document.getElementById("chk-auto-heal").checked;
  appConfig.display_disable_on_sleep = document.getElementById("chk-disp-sleep").checked;
  if (document.getElementById("chk-hud-overlay")) {
    appConfig.hud_enabled = document.getElementById("chk-hud-overlay").checked;
  }
  const tvAudioSel = document.getElementById("csel-tv-audio");
  if (tvAudioSel && tvAudioSel.hasAttribute("data-value")) {
    appConfig.tv_audio_device = tvAudioSel.getAttribute("data-value") || "";
  }
  const fbAudioSel = document.getElementById("csel-fallback-audio");
  if (fbAudioSel && fbAudioSel.hasAttribute("data-value")) {
    appConfig.fallback_audio_device = fbAudioSel.getAttribute("data-value") || "";
  }
  const chkTel = document.getElementById("chk-telemetry-opt-in");
  if (chkTel) {
    appConfig.telemetry_opt_in = chkTel.checked;
  }

  if (document.getElementById("chk-wake-pc-on-tv")) {
    appConfig.wake_pc_on_tv_on = document.getElementById("chk-wake-pc-on-tv").checked;
  }
  if (document.getElementById("chk-wake-tv-on-pc")) {
    appConfig.auto_wake_tv_on_start = document.getElementById("chk-wake-tv-on-pc").checked;
  }
  if (document.getElementById("chk-tv-off-on-pc")) {
    appConfig.auto_standby_tv_on_exit = document.getElementById("chk-tv-off-on-pc").checked;
  }
  if (document.getElementById("chk-pc-off-on-tv")) {
    appConfig.power_off_pc_on_tv_off = document.getElementById("chk-pc-off-on-tv").checked;
  }
  const modeSel = document.getElementById("csel-pc-off-mode");
  if (modeSel && modeSel.getAttribute("data-value")) {
    appConfig.pc_off_action = modeSel.getAttribute("data-value");
  }

  await invoke("save_app_config", { newCfg: appConfig });
  if (typeof renderDynamicRemote === "function") renderDynamicRemote();
  captureSavedState();
  setHumanStatus("online", "Готово", "Настройки применены и сохранены");
}
