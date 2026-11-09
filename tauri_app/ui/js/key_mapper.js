/**
 * key_mapper.js - Назначение действий клавишам пульта, пресеты и кастомные скрипты
 */

let customScripts = [];
let savedMappingsSnapshot = null;

function getCurrentMapperState() {
  const current = {};
  document.querySelectorAll(".mapper-csel").forEach(el => {
    const k = el.getAttribute("data-key");
    if (k) current[k] = el.getAttribute("data-value") || "NONE";
  });
  return current;
}

function captureSavedState() {
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

  document.querySelectorAll(".preset-btn").forEach(b => b.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");

  appConfig.key_mappings = Object.assign({}, p.mappings);
  renderMapperTable(true);
  checkMapperDirtyState();
  addLog(`[Пресет] Применен: ${p.name}`, "log-ok");

  if (window.__TAURI__ && (!appConfig || appConfig.hud_enabled !== false)) {
    window.__TAURI__.event.emit("show-hud", {
      icon: p.icon,
      title: "Пресет кнопок",
      desc: p.name
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
  const tbody = document.getElementById("mapper-tbody");
  if (!tbody) return;
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

  allDisplayKeys.forEach((code, idx) => {
    const isAvail = activeSet.has(code);
    const def = getButtonDefinition(code, letterMap);

    const tr = document.createElement("tr");
    tr.className = isAvail ? "row-remote-available" : "row-remote-unavailable";

    const curVal = mappings[code] || "NONE";
    const selId = `csel-map-${idx}`;

    const actionOptions = [...allActions];
    if (!actionOptions.some(a => a.value === curVal) && curVal.startsWith("SCRIPT:")) {
      const scriptPrefix = typeof t === "function" ? t("action.script_prefix", "Сценарий") : "Сценарий";
      actionOptions.push({ value: curVal, label: `[${scriptPrefix}] ${curVal.substring(7)}` });
    }

    const badgeText = isAvail
      ? (typeof t === "function" ? t("keys.badge_on_remote") : "На пульте")
      : (typeof t === "function" ? t("keys.badge_not_detected") : "Не обнаружена");
    const testText = typeof t === "function" ? t("keys.test") : "Тест";

    const badge = `<span class="${isAvail ? 'badge-remote-avail' : 'badge-remote-unavail'}">${badgeText}</span>`;

    tr.innerHTML = `
      <td>
        <div style="display:flex; align-items:center; gap:8px;">
          <span class="key-symbol-badge ${isAvail ? 'badge-active-key' : ''}">${def.sym}</span>
          <span style="font-weight:${isAvail ? '600' : '400'}; color:${isAvail ? 'var(--text-primary)' : 'var(--text-secondary)'};">${def.name}</span>
          ${badge}
        </div>
      </td>
      <td><div id="${selId}" class="c-select mapper-csel" data-key="${code}"></div></td>
      <td style="text-align:right;">
        <button class="apple-btn" onclick="testMapperRow('${selId}')" style="padding:5px 11px; font-size:11.5px; border-radius:8px;">${testText}</button>
      </td>
    `;
    tbody.appendChild(tr);

    createCustomSelect(selId, actionOptions, curVal, () => checkMapperDirtyState());
  });

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

  appConfig.auto_audio_switch = document.getElementById("chk-auto-audio") ? document.getElementById("chk-auto-audio").checked : true;
  appConfig.soundbar_passthrough = document.getElementById("chk-soundbar") ? document.getElementById("chk-soundbar").checked : false;
  appConfig.hold_seek_accelerate = document.getElementById("chk-hold-seek") ? document.getElementById("chk-hold-seek").checked : true;
  appConfig.emergency_kill_enabled = document.getElementById("chk-emergency-kill") ? document.getElementById("chk-emergency-kill").checked : true;
  appConfig.non_intrusive_cec = document.getElementById("chk-non-intrusive") ? document.getElementById("chk-non-intrusive").checked : true;
  appConfig.auto_healing_usb = document.getElementById("chk-auto-heal") ? document.getElementById("chk-auto-heal").checked : true;
  appConfig.display_disable_on_sleep = document.getElementById("chk-disp-sleep") ? document.getElementById("chk-disp-sleep").checked : false;
  appConfig.hud_enabled = document.getElementById("chk-hud-overlay") ? document.getElementById("chk-hud-overlay").checked : true;

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
  document.querySelectorAll(".mapper-csel").forEach(el => {
    const k = el.getAttribute("data-key");
    appConfig.key_mappings[k] = el.getAttribute("data-value");
  });
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
  captureSavedState();
  setHumanStatus("online", "Готово", "Настройки применены и сохранены");
}
