/**
 * calibration.js - Интерактивный режим обучения (калибровки) пульта
 */

let isCalibratingRemote = false;
let calibratedKeySet = new Set();

// Защита от случайных срабатываний интерфейса от клавиш Enter/Space/Tab во время калибровки
window.addEventListener("keydown", (e) => {
  if (isCalibratingRemote) {
    if (e.key === "Escape") {
      e.preventDefault();
      cancelCalibration();
      return;
    }
    if (e.key === "Enter" || e.key === " " || e.key === "Tab") {
      e.preventDefault();
      e.stopPropagation();
    }
  }
}, true);

/**
 * Запуск сеанса калибровки: блокирует эмуляцию действий в ОС
 */
async function startCalibration() {
  isCalibratingRemote = true;
  calibratedKeySet.clear();

  // Отключаем действия кнопок на уровне ядра Windows
  if (invoke) {
    await invoke("set_calibration_mode", { enabled: true });
  }

  // Снимаем фокус с любых кнопок интерфейса
  if (document.activeElement && document.activeElement.blur) {
    document.activeElement.blur();
  }

  const modal = document.getElementById("calibration-modal");
  if (modal) modal.style.display = "flex";
  document.getElementById("calib-count").textContent = "0";
  document.getElementById("calib-keys-list").innerHTML = "";
  document.getElementById("calib-last-detected").textContent = "Ожидание нажатия...";
  addLog("[Калибровка] Нажимайте кнопки на пульте ТВ (действия на ПК заблокированы)...", "log-ok");
}

/**
 * Отмена калибровки и возврат к нормальной работе
 */
async function cancelCalibration() {
  isCalibratingRemote = false;
  calibratedKeySet.clear();

  if (invoke) {
    await invoke("set_calibration_mode", { enabled: false });
  }

  const modal = document.getElementById("calibration-modal");
  if (modal) modal.style.display = "none";
}

/**
 * Захват нажатия в режиме калибровки
 */
function handleCalibrationKeyCapture(codeHex) {
  const norm = codeHex.toUpperCase().startsWith("0X")
    ? "0x" + codeHex.substring(2).toUpperCase()
    : "0x" + codeHex.toUpperCase();

  // Игнорируем выключение (0x40, 0x6B, 0x6C), чтобы ТВ не гас при случайном нажатии
  if (norm === "0x40" || norm === "0x6B" || norm === "0x6C") {
    document.getElementById("calib-last-detected").textContent = "Кнопка питания ТВ пропущена для безопасности";
    return;
  }

  const def = KEY_DEFINITIONS[norm] || { code: norm, sym: norm.substring(2), name: `Код ${norm}` };
  if (!calibratedKeySet.has(norm)) {
    calibratedKeySet.add(norm);
    document.getElementById("calib-count").textContent = calibratedKeySet.size;

    const list = document.getElementById("calib-keys-list");
    const pill = document.createElement("div");
    pill.className = "calib-key-pill";
    pill.id = `calib-pill-${norm}`;
    const symSpan = document.createElement("span");
    symSpan.textContent = def.sym;
    const nameSpan = document.createElement("span");
    nameSpan.style.opacity = "0.75";
    nameSpan.style.fontWeight = "400";
    nameSpan.textContent = def.name;
    pill.appendChild(symSpan);
    pill.appendChild(nameSpan);
    list.appendChild(pill);
    list.scrollTop = list.scrollHeight;
  }

  const pEl = document.getElementById(`calib-pill-${norm}`);
  if (pEl) {
    pEl.style.borderColor = "var(--apple-green)";
    pEl.style.boxShadow = "0 0 12px rgba(48, 209, 88, 0.6)";
    setTimeout(() => {
      pEl.style.borderColor = "";
      pEl.style.boxShadow = "";
    }, 350);
  }

  document.getElementById("calib-last-detected").textContent = `Нажата: ${def.name} (${norm})`;
}

/**
 * Завершение калибровки: сохранение списка распознанных кнопок в конфигурацию
 */
async function finishCalibration() {
  if (calibratedKeySet.size === 0) {
    addLog("[Калибровка] Кнопки не были нажаты. Раскладка сохранена без изменений.", "log-ok");
    await cancelCalibration();
    return;
  }

  const newKeys = Array.from(calibratedKeySet);
  if (!appConfig) appConfig = {};
  appConfig.calibrated_keys = newKeys;

  if (!appConfig.key_mappings) appConfig.key_mappings = {};
  newKeys.forEach(k => {
    if (!appConfig.key_mappings[k]) {
      if (k === "0x00") appConfig.key_mappings[k] = "ENTER";
      else if (k === "0x01") appConfig.key_mappings[k] = "UP";
      else if (k === "0x02") appConfig.key_mappings[k] = "DOWN";
      else if (k === "0x03") appConfig.key_mappings[k] = "LEFT";
      else if (k === "0x04") appConfig.key_mappings[k] = "RIGHT";
      else if (k === "0x0D") appConfig.key_mappings[k] = "ESC";
      else if (k === "0x09") appConfig.key_mappings[k] = "STEAM_BIG_PICTURE";
      else if (k === "0x0A" || k === "0x0B") appConfig.key_mappings[k] = "SHOW_KEYBOARD";
      else if (k === "0x53") appConfig.key_mappings[k] = "F11";
      else if (k === "0x35") appConfig.key_mappings[k] = "TAB";
      else if (k === "0x71") appConfig.key_mappings[k] = "MOUSE_TOGGLE";
      else if (k === "0x72") appConfig.key_mappings[k] = "STEAM_BIG_PICTURE";
      else if (k === "0x73") appConfig.key_mappings[k] = "VOL_MUTE";
      else if (k === "0x74") appConfig.key_mappings[k] = "SPACE";
      else if (k === "0x41") appConfig.key_mappings[k] = "VOL_UP";
      else if (k === "0x42") appConfig.key_mappings[k] = "VOL_DOWN";
      else if (k === "0x43") appConfig.key_mappings[k] = "VOL_MUTE";
      else if (k.startsWith("0x2")) appConfig.key_mappings[k] = "NUM_" + k.substring(3);
      else appConfig.key_mappings[k] = "NONE";
    }
  });

  if (invoke) {
    await invoke("save_app_config", { newCfg: appConfig });
    await invoke("set_calibration_mode", { enabled: false });
  }

  isCalibratingRemote = false;
  const modal = document.getElementById("calibration-modal");
  if (modal) modal.style.display = "none";

  renderDynamicRemote();
  renderMapperTable();
  addLog(`[Калибровка] Пульт сконструирован! Собрано кнопок: ${newKeys.length}`, "log-ok");
}

/**
 * Сброс калибровки к базовому универсальному набору или профилю ТВ
 */
async function resetToDefaultKeys() {
  if (!appConfig) appConfig = {};
  appConfig.calibrated_keys = [];
  if (invoke) {
    await invoke("save_app_config", { newCfg: appConfig });
    await invoke("set_calibration_mode", { enabled: false });
  }

  isCalibratingRemote = false;
  const modal = document.getElementById("calibration-modal");
  if (modal) modal.style.display = "none";

  renderDynamicRemote();
  renderMapperTable();
  addLog("[Калибровка] Пульт сброшен к конфигурации профиля ТВ.", "log-ok");
}
