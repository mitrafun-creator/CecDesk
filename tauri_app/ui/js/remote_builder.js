/**
 * remote_builder.js - Динамический конструктор пульта и визуальная подсветка нажатий
 */

const pulseTimers = {};

/**
 * Живая анимация нажатия кнопки виртуального пульта
 */
function pulseVirtualRemoteButton(codeHex) {
  const norm = codeHex.toUpperCase().startsWith("0X")
    ? "0x" + codeHex.substring(2).toUpperCase()
    : "0x" + codeHex.toUpperCase();
  const targetId = norm === "0x46" ? "rem-btn-0x44" : `rem-btn-${norm}`;
  const btn = document.getElementById(targetId);
  if (!btn) return;

  btn.classList.add("rem-active");
  if (pulseTimers[targetId]) clearTimeout(pulseTimers[targetId]);
  pulseTimers[targetId] = setTimeout(() => {
    btn.classList.remove("rem-active");
  }, 320);
}

function uiRemoteClick(codeHex, cmdKey) {
  pulseVirtualRemoteButton(codeHex);
  sendKey(cmdKey);
}

function uiOsClick(codeHex, osAction) {
  pulseVirtualRemoteButton(codeHex);
  pressOsKey(osAction);
}

function uiNumClick(codeHex, defaultAction) {
  pulseVirtualRemoteButton(codeHex);
  const action = (appConfig && appConfig.key_mappings && appConfig.key_mappings[codeHex]) || defaultAction;
  pressOsKey(action);
}

function uiRemoteOrOsClick(codeHex) {
  pulseVirtualRemoteButton(codeHex);
  const mapping = (appConfig && appConfig.key_mappings && appConfig.key_mappings[codeHex]);
  if (mapping && mapping !== "NONE") {
    pressOsKey(mapping);
  } else {
    sendKey(codeHex.replace("0x", ""));
  }
}

function getKeysPlural(count) {
  if (typeof currentLang !== "undefined" && currentLang === "en") {
    return count === 1 ? "key" : "keys";
  }
  const n = Math.abs(count) % 100;
  const n1 = n % 10;
  if (n > 10 && n < 20) return "кнопок";
  if (n1 > 1 && n1 < 5) return "кнопки";
  if (n1 === 1) return "кнопка";
  return "кнопок";
}

/**
 * Обогащение кнопки пульта мгновенной всплывающей подсказкой (Apple Floating Tooltip)
 */
function applyButtonTooltip(btn, codeHex) {
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const norm = codeHex.toUpperCase().startsWith("0X")
    ? "0x" + codeHex.substring(2).toUpperCase()
    : "0x" + codeHex.toUpperCase();
  const knownDef = KEY_DEFINITIONS[norm];
  const displayName = knownDef
    ? (typeof getKeyDisplayName === "function" ? getKeyDisplayName(norm, knownDef) : knownDef.name)
    : `${_t("remote.button_prefix", "Кнопка")} ${norm}`;
  const mapping = (appConfig && appConfig.key_mappings && appConfig.key_mappings[norm]) || "NONE";
  let actionLabel = mapping;
  if (mapping.startsWith("SCRIPT:")) {
    actionLabel = `[${_t("action.script_prefix", "Сценарий")}] ${mapping.substring(7)}`;
  } else {
    const actionObj = (typeof OS_ACTIONS !== "undefined" && Array.isArray(OS_ACTIONS))
      ? OS_ACTIONS.find(a => a.value === mapping)
      : null;
    actionLabel = _t("action." + mapping.toLowerCase(), actionObj ? actionObj.label : mapping);
  }

  const actPrefix = _t("remote.pc_action_prefix", "Действие ПК:");
  const actNone = _t("remote.pc_action_none", "не назначено");
  btn.dataset.tooltip = `${displayName} (${norm})`;
  btn.dataset.tooltipSub = mapping !== "NONE" ? `${actPrefix} ${actionLabel}` : `${actPrefix} ${actNone}`;
  btn.removeAttribute("title");
}

/**
 * Динамическая сборка виртуального пульта из реально доступных клавиш
 */
function renderDynamicRemote() {
  const _t = typeof t === "function" ? t : (_k, fb) => fb;
  const container = document.getElementById("remote-chassis");
  if (!container) return;
  container.innerHTML = "";

  const activeKeys = getActiveRemoteKeys();
  const valEl = document.getElementById("lbl-count-value");
  const unitEl = document.getElementById("lbl-count-unit");
  if (valEl) valEl.textContent = activeKeys.length;
  if (unitEl) unitEl.textContent = getKeysPlural(activeKeys.length);
  const lblCount = document.getElementById("lbl-remote-count");
  if (lblCount) {
    lblCount.removeAttribute("title");
    lblCount.dataset.tooltipSub = `${_t("remote.recognized_prefix", "Распознано:")} ${activeKeys.length} ${getKeysPlural(activeKeys.length)}`;
  }

  const keySet = new Set(activeKeys);

  // 1. Верхний ряд (Назад, Меню, Опции, Инфо, Гид)
  const topKeys = ["0x0D", "0x09", "0x0A", "0x35", "0x53"].filter(k => keySet.has(k));
  if (topKeys.length > 0) {
    const topRow = document.createElement("div");
    topRow.className = "remote-top-row";
    topKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k] || { code: k, sym: k, name: k };
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      const svgMarkup = typeof getKeyIconSvg === "function" ? getKeyIconSvg(k) : null;
      if (k === "0x0D") {
        btn.innerHTML = '<svg class="ico ico-18" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>';
        btn.onclick = () => uiRemoteOrOsClick(k);
      } else if (svgMarkup) {
        btn.innerHTML = svgMarkup;
        btn.onclick = () => uiRemoteOrOsClick(k);
      } else {
        if (def.sym && def.sym.length >= 3) {
          btn.classList.add("btn-text-sm");
        }
        btn.textContent = def.sym;
        btn.onclick = () => uiRemoteOrOsClick(k);
      }
      topRow.appendChild(btn);
    });
    container.appendChild(topRow);
  }

  // 2. Круговой Clickpad (Стрелки и ОК)
  const hasNav = ["0x00", "0x01", "0x02", "0x03", "0x04"].some(k => keySet.has(k));
  if (hasNav) {
    const pad = document.createElement("div");
    pad.className = "clickpad-ring";

    if (keySet.has("0x01")) {
      const up = document.createElement("button");
      up.type = "button";
      up.id = "rem-btn-0x01";
      up.className = "dpad-dir dpad-up";
      applyButtonTooltip(up, "0x01");
      up.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="18 15 12 9 6 15"></polyline></svg>';
      up.onclick = () => uiRemoteOrOsClick("0x01");
      pad.appendChild(up);
    }
    if (keySet.has("0x02")) {
      const down = document.createElement("button");
      down.type = "button";
      down.id = "rem-btn-0x02";
      down.className = "dpad-dir dpad-down";
      applyButtonTooltip(down, "0x02");
      down.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"></polyline></svg>';
      down.onclick = () => uiRemoteOrOsClick("0x02");
      pad.appendChild(down);
    }
    if (keySet.has("0x03")) {
      const left = document.createElement("button");
      left.type = "button";
      left.id = "rem-btn-0x03";
      left.className = "dpad-dir dpad-left";
      applyButtonTooltip(left, "0x03");
      left.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>';
      left.onclick = () => uiRemoteOrOsClick("0x03");
      pad.appendChild(left);
    }
    if (keySet.has("0x04")) {
      const right = document.createElement("button");
      right.type = "button";
      right.id = "rem-btn-0x04";
      right.className = "dpad-dir dpad-right";
      applyButtonTooltip(right, "0x04");
      right.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="9 18 15 12 9 6"></polyline></svg>';
      right.onclick = () => uiRemoteOrOsClick("0x04");
      pad.appendChild(right);
    }
    if (keySet.has("0x00")) {
      const center = document.createElement("button");
      center.type = "button";
      center.id = "rem-btn-0x00";
      center.className = "clickpad-center";
      center.textContent = "OK";
      applyButtonTooltip(center, "0x00");
      center.onclick = () => uiRemoteOrOsClick("0x00");
      pad.appendChild(center);
    }
    container.appendChild(pad);
  }

  // 3. Цветовой ряд (Красная, Зеленая, Желтая, Синяя)
  const colorKeys = ["0x72", "0x73", "0x74", "0x71"].filter(k => keySet.has(k));
  if (colorKeys.length > 0) {
    const cRow = document.createElement("div");
    cRow.className = "rem-color-row";
    colorKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k];
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = `rem-color-btn ${def.class}`;
      applyButtonTooltip(btn, k);
      btn.textContent = def.sym;
      btn.onclick = () => uiRemoteOrOsClick(k);
      cRow.appendChild(btn);
    });
    container.appendChild(cRow);
  }

  // 4a. Медиа-управление (Предыдущий трек, Воспроизведение, Следующий трек)
  const transportKeys = ["0x4C", "0x44", "0x4B"].filter(k => keySet.has(k));
  if (transportKeys.length > 0) {
    const tGrid = document.createElement("div");
    tGrid.className = "remote-grid-controls";

    transportKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k] || { code: k, sym: k, name: k };
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      if (k === "0x4C") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="19 20 9 12 19 4 19 20"></polygon><line x1="5" y1="19" x2="5" y2="5"></line></svg>';
        btn.style.gridColumn = "1";
      } else if (k === "0x4B") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="5 4 15 12 5 20 5 4"></polygon><line x1="19" y1="5" x2="19" y2="19"></line></svg>';
        btn.style.gridColumn = "3";
      } else if (k === "0x44") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
        btn.style.gridColumn = "2";
      }

      btn.onclick = () => uiRemoteOrOsClick(k);
      tGrid.appendChild(btn);
    });

    container.appendChild(tGrid);
  }

  // 4b. Регулировка громкости (Громкость -, Без звука, Громкость +)
  const volKeys = ["0x42", "0x43", "0x41"].filter(k => keySet.has(k));
  if (volKeys.length > 0) {
    const vGrid = document.createElement("div");
    vGrid.className = "remote-grid-controls";

    volKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k] || { code: k, sym: k, name: k };
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      if (k === "0x42") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>';
        btn.style.gridColumn = "1";
      } else if (k === "0x43") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><line x1="23" y1="9" x2="17" y2="15"></line><line x1="17" y1="9" x2="23" y2="15"></line></svg>';
        btn.style.gridColumn = "2";
      } else if (k === "0x41") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>';
        btn.style.gridColumn = "3";
      }

      btn.onclick = () => uiRemoteOrOsClick(k);
      vGrid.appendChild(btn);
    });

    container.appendChild(vGrid);
  }

  // 4c. Прочие клавиши управления (Стоп, Пауза, Каналы)
  const otherCtrlKeys = ["0x45", "0x46", "0x48", "0x49", "0x30", "0x31"].filter(k => keySet.has(k));
  if (otherCtrlKeys.length > 0) {
    const oRow = document.createElement("div");
    oRow.className = "remote-extra-row";

    otherCtrlKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k] || { code: k, sym: k, name: k };
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      const svgMarkup = typeof getKeyIconSvg === "function" ? getKeyIconSvg(k) : null;
      if (svgMarkup) {
        btn.innerHTML = svgMarkup;
      } else {
        if (def.sym && def.sym.length >= 3) {
          btn.classList.add("btn-text-sm");
        }
        btn.textContent = def.sym;
      }

      btn.onclick = () => uiRemoteOrOsClick(k);
      oRow.appendChild(btn);
    });

    container.appendChild(oRow);
  }

  // 5. Цифровая клавиатура 0-9
  const numKeys = ["0x21", "0x22", "0x23", "0x24", "0x25", "0x26", "0x27", "0x28", "0x29", "0x20"]
    .filter(k => keySet.has(k));
  if (numKeys.length > 0) {
    const numGrid = document.createElement("div");
    numGrid.className = "remote-num-grid";

    numKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k];
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-num-btn";
      btn.textContent = def.sym;
      applyButtonTooltip(btn, k);
      if (k === "0x20") btn.style.gridColumn = "2";
      btn.onclick = () => uiNumClick(k, `NUM_${def.sym}`);
      numGrid.appendChild(btn);
    });

    container.appendChild(numGrid);
  }

  // 6. Дополнительные пользовательские клавиши без явного назначения (назначаются латинскими буквами A, B, C...)
  const letterMap = getExtraKeyLetterMap(activeKeys);
  const extraKeys = Object.keys(letterMap).sort();
  if (extraKeys.length > 0) {
    const extraWrap = document.createElement("div");
    extraWrap.className = "remote-extra-row";

    const btnPrefix = _t("remote.button_prefix", "Кнопка");
    const actPrefix = _t("remote.pc_action_prefix", "Действие ПК:");
    const actNone = _t("remote.pc_action_none", "не назначено");

    extraKeys.forEach((k) => {
      const letter = letterMap[k];
      const btn = document.createElement("button");
      btn.type = "button";
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn rem-letter-btn";
      btn.textContent = letter;
      const norm = k.toUpperCase().startsWith("0X") ? "0x" + k.substring(2).toUpperCase() : "0x" + k.toUpperCase();
      const mapping = (appConfig && appConfig.key_mappings && appConfig.key_mappings[norm]) || "NONE";
      let actionLabel = mapping;
      if (mapping !== "NONE") {
        if (mapping.startsWith("SCRIPT:")) {
          actionLabel = `[${_t("action.script_prefix", "Сценарий")}] ${mapping.substring(7)}`;
        } else {
          const actionObj = (typeof OS_ACTIONS !== "undefined" && Array.isArray(OS_ACTIONS))
            ? OS_ACTIONS.find(a => a.value === mapping)
            : null;
          actionLabel = _t("action." + mapping.toLowerCase(), actionObj ? actionObj.label : mapping);
        }
      }
      btn.dataset.tooltip = `${btnPrefix} ${letter} (${norm})`;
      btn.dataset.tooltipSub = mapping !== "NONE" ? `${actPrefix} ${actionLabel}` : `${actPrefix} ${actNone}`;
      btn.removeAttribute("title");
      btn.onclick = () => uiRemoteOrOsClick(k);
      extraWrap.appendChild(btn);
    });
    container.appendChild(extraWrap);
  }
}
