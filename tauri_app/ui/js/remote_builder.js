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
  const norm = codeHex.toUpperCase().startsWith("0X")
    ? "0x" + codeHex.substring(2).toUpperCase()
    : "0x" + codeHex.toUpperCase();
  const def = KEY_DEFINITIONS[norm] || { name: `Кнопка ${norm}`, sym: norm };
  const mapping = (appConfig && appConfig.key_mappings && appConfig.key_mappings[norm]) || "NONE";
  const actionObj = (typeof OS_ACTIONS !== "undefined" && Array.isArray(OS_ACTIONS))
    ? OS_ACTIONS.find(a => a.value === mapping)
    : null;
  const actionLabel = actionObj ? actionObj.label : mapping;

  btn.dataset.tooltip = `${def.name} (${norm})`;
  btn.dataset.tooltipSub = mapping !== "NONE" ? `Действие ПК: ${actionLabel}` : "Действие ПК: не назначено";
  btn.removeAttribute("title");
}

/**
 * Динамическая сборка виртуального пульта из реально доступных клавиш
 */
function renderDynamicRemote() {
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
    lblCount.title = `Распознано: ${activeKeys.length} ${getKeysPlural(activeKeys.length)}`;
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
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      if (k === "0x0D") {
        btn.innerHTML = '<svg class="ico ico-lg" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>';
        btn.onclick = () => uiRemoteOrOsClick(k);
      } else if (k === "0x09") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"></rect><line x1="8" y1="20" x2="16" y2="20"></line></svg>';
        btn.onclick = () => uiRemoteOrOsClick(k);
      } else if (k === "0x0A") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><rect x="2" y="4" width="20" height="16" rx="2"></rect><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 16h10"></path></svg>';
        btn.onclick = () => uiRemoteOrOsClick(k);
      } else {
        if (def.sym && def.sym.length >= 4) {
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
      up.id = "rem-btn-0x01";
      up.className = "dpad-dir dpad-up";
      applyButtonTooltip(up, "0x01");
      up.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="18 15 12 9 6 15"></polyline></svg>';
      up.onclick = () => uiRemoteOrOsClick("0x01");
      pad.appendChild(up);
    }
    if (keySet.has("0x02")) {
      const down = document.createElement("button");
      down.id = "rem-btn-0x02";
      down.className = "dpad-dir dpad-down";
      applyButtonTooltip(down, "0x02");
      down.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"></polyline></svg>';
      down.onclick = () => uiRemoteOrOsClick("0x02");
      pad.appendChild(down);
    }
    if (keySet.has("0x03")) {
      const left = document.createElement("button");
      left.id = "rem-btn-0x03";
      left.className = "dpad-dir dpad-left";
      applyButtonTooltip(left, "0x03");
      left.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>';
      left.onclick = () => uiRemoteOrOsClick("0x03");
      pad.appendChild(left);
    }
    if (keySet.has("0x04")) {
      const right = document.createElement("button");
      right.id = "rem-btn-0x04";
      right.className = "dpad-dir dpad-right";
      applyButtonTooltip(right, "0x04");
      right.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polyline points="9 18 15 12 9 6"></polyline></svg>';
      right.onclick = () => uiRemoteOrOsClick("0x04");
      pad.appendChild(right);
    }
    if (keySet.has("0x00")) {
      const center = document.createElement("button");
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
      btn.id = `rem-btn-${k}`;
      btn.className = `rem-color-btn ${def.class}`;
      applyButtonTooltip(btn, k);
      btn.textContent = def.sym;
      btn.onclick = () => uiRemoteOrOsClick(k);
      cRow.appendChild(btn);
    });
    container.appendChild(cRow);
  }

  // 4. Сетка звука и медиа
  const ctrlKeys = ["0x4C", "0x44", "0x4B", "0x42", "0x43", "0x41", "0x45", "0x46", "0x48", "0x49", "0x30", "0x31"]
    .filter(k => keySet.has(k));
  if (ctrlKeys.length > 0) {
    const grid = document.createElement("div");
    grid.className = "remote-grid-controls";

    ctrlKeys.forEach(k => {
      const def = KEY_DEFINITIONS[k] || { code: k, sym: k, name: k };
      const btn = document.createElement("button");
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn";
      applyButtonTooltip(btn, k);

      if (k === "0x41") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>';
      } else if (k === "0x42") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>';
      } else if (k === "0x43") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><line x1="23" y1="9" x2="17" y2="15"></line><line x1="17" y1="9" x2="23" y2="15"></line></svg>';
      } else if (k === "0x44") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>';
      } else if (k === "0x45") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"></rect></svg>';
      } else if (k === "0x4B") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="5 4 15 12 5 20 5 4"></polygon><line x1="19" y1="5" x2="19" y2="19"></line></svg>';
      } else if (k === "0x4C") {
        btn.innerHTML = '<svg class="ico" viewBox="0 0 24 24"><polygon points="19 20 9 12 19 4 19 20"></polygon><line x1="5" y1="19" x2="5" y2="5"></line></svg>';
      } else {
        btn.textContent = def.sym;
      }

      btn.onclick = () => uiRemoteOrOsClick(k);
      grid.appendChild(btn);
    });

    container.appendChild(grid);
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

    extraKeys.forEach((k) => {
      const letter = letterMap[k];
      const btn = document.createElement("button");
      btn.id = `rem-btn-${k}`;
      btn.className = "rem-round-btn rem-letter-btn";
      btn.textContent = letter;
      const norm = k.toUpperCase().startsWith("0X") ? "0x" + k.substring(2).toUpperCase() : "0x" + k.toUpperCase();
      const mapping = (appConfig && appConfig.key_mappings && appConfig.key_mappings[norm]) || "NONE";
      btn.dataset.tooltip = `Кнопка ${letter} (${norm})`;
      btn.dataset.tooltipSub = mapping !== "NONE" ? `Действие ПК: ${mapping}` : "Действие ПК: не назначено";
      btn.onclick = () => uiRemoteOrOsClick(k);
      extraWrap.appendChild(btn);
    });
    container.appendChild(extraWrap);
  }
}
