const tauri = window.__TAURI__;
const invoke = tauri ? tauri.core.invoke : null;
const listen = tauri ? tauri.event.listen : null;

let oskErrCount = 0;
function forwardOskError(msg, stack) {
  if (!invoke || oskErrCount >= 10) return;
  oskErrCount++;
  invoke("report_client_error", {
    source: "osk",
    message: String(msg || "Unknown OSK error"),
    stack: stack ? String(stack) : null
  }).catch(() => {});
}
window.addEventListener("error", (ev) => {
  forwardOskError(ev.message, ev.error && ev.error.stack ? ev.error.stack : `${ev.filename || ""}:${ev.lineno || 0}`);
});
window.addEventListener("unhandledrejection", (ev) => {
  const r = ev.reason;
  forwardOskError(r && r.message ? r.message : String(r), r && r.stack ? r.stack : null);
});
window.addEventListener("securitypolicyviolation", (ev) => {
  forwardOskError(
    `CSP violation (${ev.violatedDirective || ev.effectiveDirective || "unknown"}): ${ev.blockedURI || "inline"}`,
    `${ev.sourceFile || ""}:${ev.lineNumber || 0}:${ev.columnNumber || 0}`
  );
});

// Раскладки клавиатуры
let currentMode = "RU"; // "RU", "EN", "SYM"
let uiLang = "ru";
let isCaps = false;
let isShift = false;
let focusRow = 2;
let focusCol = 4;

const OSK_I18N = {
  ru: {
    hint_red: "Красная: Смена языка",
    hint_green: "Зеленая: Пробел",
    hint_yellow: "Желтая: Стереть",
    hint_blue: "Синяя: Ввод",
    mode_ru: "Русский (ЙЦУКЕН)",
    mode_en: "English (QWERTY)",
    mode_sym: "Символы & Цифры",
    enter: "Ввод",
    space: "Пробел",
    hide: "Свернуть",
    lang: "Язык"
  },
  en: {
    hint_red: "Red: Switch language",
    hint_green: "Green: Space",
    hint_yellow: "Yellow: Backspace",
    hint_blue: "Blue: Enter",
    mode_ru: "Russian (JCUKEN)",
    mode_en: "English (QWERTY)",
    mode_sym: "Symbols & Digits",
    enter: "Enter",
    space: "Space",
    hide: "Hide",
    lang: "Lang"
  }
};

function applyOskUiLang(lang) {
  uiLang = lang === "en" ? "en" : "ru";
  document.documentElement.lang = uiLang;
  const dict = OSK_I18N[uiLang];
  const setTxt = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
  setTxt("osk-hint-red", dict.hint_red);
  setTxt("osk-hint-green", dict.hint_green);
  setTxt("osk-hint-yellow", dict.hint_yellow);
  setTxt("osk-hint-blue", dict.hint_blue);
  renderKeyboard();
}

const OSK_ICONS = {
  backspace: '<svg class="osk-ico" viewBox="0 0 24 24"><path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z"></path><line x1="18" y1="9" x2="12" y2="15"></line><line x1="12" y1="9" x2="18" y2="15"></line></svg>',
  enter: '<svg class="osk-ico" viewBox="0 0 24 24"><polyline points="9 10 4 15 9 20"></polyline><path d="M20 4v7a4 4 0 0 1-4 4H4"></path></svg>',
  shift: '<svg class="osk-ico" viewBox="0 0 24 24"><polyline points="17 11 12 6 7 11"></polyline><line x1="12" y1="18" x2="12" y2="6"></line></svg>',
  left: '<svg class="osk-ico" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>',
  right: '<svg class="osk-ico" viewBox="0 0 24 24"><polyline points="9 18 15 12 9 6"></polyline></svg>',
  hide: '<svg class="osk-ico" viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"></polyline><line x1="5" y1="19" x2="19" y2="19"></line></svg>'
};

const LAYOUT_RU = [
  ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", { k: "BACKSPACE", label: "", icon: "backspace", w: "key-w-15 key-special" }],
  [{ k: "TAB", label: "Tab", w: "key-w-15 key-special" }, "й", "ц", "у", "к", "е", "н", "г", "ш", "щ", "з", "х", "ъ"],
  [{ k: "CAPS", label: "Caps", w: "key-w-17 key-special", id: "key-caps" }, "ф", "ы", "в", "а", "п", "р", "о", "л", "д", "ж", "э", { k: "ENTER", label: "Ввод", icon: "enter", w: "key-w-17 key-accent" }],
  [{ k: "SHIFT", label: "Shift", icon: "shift", w: "key-w-17 key-special", id: "key-shift" }, "я", "ч", "с", "м", "и", "т", "ь", "б", "ю", ".", { k: "ESC", label: "Esc", w: "key-w-15 key-special" }],
  [{ k: "MODE_SYM", label: "?123", w: "key-w-15 key-special" }, { k: "MODE_LANG", label: "EN", w: "key-w-15 key-special", id: "btn-lang" }, { k: "SPACE", label: "Пробел", w: "key-w-space" }, { k: "LEFT", label: "", icon: "left", w: "key-special" }, { k: "RIGHT", label: "", icon: "right", w: "key-special" }, { k: "HIDE", label: "Свернуть", icon: "hide", w: "key-w-15 key-red" }]
];

const LAYOUT_EN = [
  ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", { k: "BACKSPACE", label: "", icon: "backspace", w: "key-w-15 key-special" }],
  [{ k: "TAB", label: "Tab", w: "key-w-15 key-special" }, "q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "[", "]"],
  [{ k: "CAPS", label: "Caps", w: "key-w-17 key-special", id: "key-caps" }, "a", "s", "d", "f", "g", "h", "j", "k", "l", ";", "'", { k: "ENTER", label: "Enter", icon: "enter", w: "key-w-17 key-accent" }],
  [{ k: "SHIFT", label: "Shift", icon: "shift", w: "key-w-17 key-special", id: "key-shift" }, "z", "x", "c", "v", "b", "n", "m", ",", ".", "/", { k: "ESC", label: "Esc", w: "key-w-15 key-special" }],
  [{ k: "MODE_SYM", label: "?123", w: "key-w-15 key-special" }, { k: "MODE_LANG", label: "RU", w: "key-w-15 key-special", id: "btn-lang" }, { k: "SPACE", label: "Space", w: "key-w-space" }, { k: "LEFT", label: "", icon: "left", w: "key-special" }, { k: "RIGHT", label: "", icon: "right", w: "key-special" }, { k: "HIDE", label: "Свернуть", icon: "hide", w: "key-w-15 key-red" }]
];

const LAYOUT_SYM = [
  ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", { k: "BACKSPACE", label: "", icon: "backspace", w: "key-w-15 key-special" }],
  [{ k: "TAB", label: "Tab", w: "key-w-15 key-special" }, "!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "_", "+"],
  [{ k: "MODE_ABC", label: "ABC", w: "key-w-17 key-special" }, "~", "`", "{", "}", ":", "\"", ";", "'", "<", ">", "?", { k: "ENTER", label: "Enter", icon: "enter", w: "key-w-17 key-accent" }],
  [{ k: "MODE_ABC", label: "ABC", w: "key-w-17 key-special" }, "[", "]", "\\", "|", "/", ",", ".", "=", "*", "#", { k: "ESC", label: "Esc", w: "key-w-15 key-special" }],
  [{ k: "MODE_ABC", label: "ABC", w: "key-w-15 key-special" }, { k: "MODE_LANG", label: "Язык", w: "key-w-15 key-special" }, { k: "SPACE", label: "Space", w: "key-w-space" }, { k: "LEFT", label: "", icon: "left", w: "key-special" }, { k: "RIGHT", label: "", icon: "right", w: "key-special" }, { k: "HIDE", label: "Свернуть", icon: "hide", w: "key-w-15 key-red" }]
];

// Аудиокликер (Web Audio API)
let audioCtx = null;
function playClickSound() {
  try {
    if (!audioCtx) audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    if (audioCtx.state === "suspended") audioCtx.resume();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = "sine";
    osc.frequency.setValueAtTime(480, audioCtx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(140, audioCtx.currentTime + 0.045);
    gain.gain.setValueAtTime(0.08, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.045);
    osc.connect(gain);
    gain.connect(audioCtx.destination);
    osc.start();
    osc.stop(audioCtx.currentTime + 0.05);
  } catch (e) {}
}

function getCurrentLayout() {
  if (currentMode === "SYM") return LAYOUT_SYM;
  return currentMode === "RU" ? LAYOUT_RU : LAYOUT_EN;
}

function renderKeyboard() {
  const grid = document.getElementById("keyboard-grid");
  grid.innerHTML = "";
  const layout = getCurrentLayout();
  const dict = OSK_I18N[uiLang] || OSK_I18N.ru;

  document.getElementById("osk-lang-badge").textContent =
    currentMode === "RU" ? dict.mode_ru : (currentMode === "EN" ? dict.mode_en : dict.mode_sym);

  layout.forEach((row, rIdx) => {
    const rowEl = document.createElement("div");
    rowEl.className = "keyboard-row";

    row.forEach((item, cIdx) => {
      const btn = document.createElement("div");
      let keyAction = "";
      let label = "";
      let iconName = "";
      let extraClass = "";
      let customId = "";

      if (typeof item === "string") {
        keyAction = item;
        let displayChar = item;
        const upper = (isCaps && !isShift) || (!isCaps && isShift);
        if (upper && /[a-zA-Zа-яА-ЯёЁ]/.test(item)) {
          displayChar = item.toUpperCase();
        }
        label = displayChar;
      } else {
        keyAction = item.k;
        label = item.label || "";
        if (keyAction === "HIDE") label = dict.hide;
        else if (keyAction === "ENTER" && currentMode === "RU") label = dict.enter;
        else if (keyAction === "SPACE" && currentMode === "RU") label = dict.space;
        else if (keyAction === "MODE_LANG" && currentMode === "SYM") label = dict.lang;
        iconName = item.icon || "";
        extraClass = item.w || "";
        customId = item.id || "";
      }

      btn.className = `key ${extraClass}`;
      if (iconName && OSK_ICONS[iconName]) {
        if (label) {
          const textSpan = document.createElement("span");
          textSpan.textContent = label;
          btn.appendChild(textSpan);
          const icoWrap = document.createElement("span");
          icoWrap.className = "osk-ico-wrap";
          icoWrap.innerHTML = OSK_ICONS[iconName];
          btn.appendChild(icoWrap);
        } else {
          btn.innerHTML = OSK_ICONS[iconName];
        }
      } else {
        btn.textContent = label;
      }
      if (customId) btn.id = customId;

      if (rIdx === focusRow && cIdx === focusCol) {
        btn.classList.add("dpad-focus");
      }

      if (keyAction === "CAPS" && isCaps) btn.classList.add("active-toggle");
      if (keyAction === "SHIFT" && isShift) btn.classList.add("active-toggle");

      btn.onmousedown = (e) => {
        e.preventDefault();
        focusRow = rIdx;
        focusCol = cIdx;
        handleKeyPress(keyAction, label);
        renderKeyboard();
      };

      rowEl.appendChild(btn);
    });

    grid.appendChild(rowEl);
  });
}

function getInvoke() {
  const t = window.__TAURI__;
  return t && t.core ? t.core.invoke : (t ? t.invoke : null);
}

async function doInvoke(cmd, args) {
  const inv = getInvoke();
  if (inv) {
    try {
      return await inv(cmd, args);
    } catch (e) {
      console.error("OSK invoke err:", e);
    }
  }
}

async function handleKeyPress(action, displayLabel) {
  playClickSound();

  if (action === "MODE_LANG") {
    currentMode = (currentMode === "RU") ? "EN" : "RU";
    renderKeyboard();
    return;
  }
  if (action === "MODE_SYM") {
    currentMode = "SYM";
    renderKeyboard();
    return;
  }
  if (action === "MODE_ABC") {
    currentMode = "RU";
    renderKeyboard();
    return;
  }
  if (action === "CAPS") {
    isCaps = !isCaps;
    renderKeyboard();
    return;
  }
  if (action === "SHIFT") {
    isShift = !isShift;
    renderKeyboard();
    return;
  }
  if (action === "HIDE") {
    hideOsk();
    return;
  }

  // Отправка в Rust бэкенд
  if (action === "BACKSPACE" || action === "ENTER" || action === "SPACE" || action === "TAB" || action === "ESC" || action === "LEFT" || action === "RIGHT") {
    await doInvoke("type_special_key", { key: action });
  } else {
    const upper = (isCaps && !isShift) || (!isCaps && isShift);
    let ch = displayLabel;
    if (upper && /[a-zA-Zа-яА-ЯёЁ]/.test(ch)) {
      ch = ch.toUpperCase();
    }
    await doInvoke("type_unicode_char", { ch });
  }

  if (isShift) {
    isShift = false;
    renderKeyboard();
  }
}

async function hideOsk() {
  focusRow = 2;
  focusCol = 4;
  isShift = false;
  renderKeyboard();
  await doInvoke("toggle_osk", { force_show: false, forceShow: false });
}

// Пространственная (геометрическая) навигация стрелками с пульта ТВ
function navigateDpad(dir) {
  const layout = getCurrentLayout();
  const numRows = layout.length;

  if (dir === "LEFT") {
    const numCols = layout[focusRow].length;
    focusCol = (focusCol - 1 + numCols) % numCols;
    renderKeyboard();
    return;
  }
  if (dir === "RIGHT") {
    const numCols = layout[focusRow].length;
    focusCol = (focusCol + 1) % numCols;
    renderKeyboard();
    return;
  }

  // Навигация ВВЕРХ / ВНИЗ: находим клавишу в целевом ряду с минимальным
  // горизонтальным смещением относительно центра текущей клавиши
  const rows = document.querySelectorAll(".keyboard-row");
  const currentKey = rows[focusRow]?.children[focusCol];
  let currentCenterX = null;
  if (currentKey) {
    const rect = currentKey.getBoundingClientRect();
    if (rect.width > 0) {
      currentCenterX = rect.left + rect.width / 2;
    }
  }

  const targetRowIdx = dir === "UP" ? (focusRow - 1 + numRows) % numRows : (focusRow + 1) % numRows;
  const targetRowEl = rows[targetRowIdx];

  if (currentCenterX !== null && targetRowEl && targetRowEl.children.length > 0) {
    let bestCol = 0;
    let bestDist = Infinity;
    for (let c = 0; c < targetRowEl.children.length; c++) {
      const kRect = targetRowEl.children[c].getBoundingClientRect();
      const kCenterX = kRect.left + kRect.width / 2;
      const dist = Math.abs(kCenterX - currentCenterX);
      if (dist < bestDist) {
        bestDist = dist;
        bestCol = c;
      }
    }
    focusRow = targetRowIdx;
    focusCol = bestCol;
  } else {
    focusRow = targetRowIdx;
    const numCols = layout[focusRow].length;
    if (focusCol >= numCols) focusCol = numCols - 1;
  }

  renderKeyboard();
}

function selectCurrentDpad() {
  const layout = getCurrentLayout();
  const item = layout[focusRow][focusCol];
  if (typeof item === "string") {
    handleKeyPress(item, item);
  } else {
    handleKeyPress(item.k, item.label);
  }
}

let lastCmdTime = 0;
async function setupListener() {
  const t = window.__TAURI__;
  const listen = t && t.event ? t.event.listen : (t ? t.listen : null);
  if (!listen) {
    setTimeout(setupListener, 120);
    return;
  }
  try {
    await listen("osk-state-changed", (ev) => {
      if (!ev.payload) {
        focusRow = 2;
        focusCol = 4;
        isShift = false;
        renderKeyboard();
      }
    });

    await listen("osk-nav", (ev) => {
      const now = Date.now();
      if (now - lastCmdTime < 130) return; // Дебаунс 130 мс против случайных повторов
      lastCmdTime = now;

      const cmd = ev.payload;
      if (cmd === "UP" || cmd === "DOWN" || cmd === "LEFT" || cmd === "RIGHT") {
        navigateDpad(cmd);
      } else if (cmd === "SELECT") {
        selectCurrentDpad();
      } else if (cmd === "LANG") {
        currentMode = (currentMode === "RU") ? "EN" : "RU";
        renderKeyboard();
      } else if (cmd === "BACKSPACE") {
        handleKeyPress("BACKSPACE", "");
      } else if (cmd === "SPACE") {
        handleKeyPress("SPACE", " ");
      } else if (cmd === "ENTER") {
        handleKeyPress("ENTER", "");
      }
    });

    await listen("language-changed", (ev) => {
      if (ev && ev.payload) applyOskUiLang(String(ev.payload));
    });
  } catch (err) {
    console.error("Failed to register osk listeners:", err);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  if (invoke) {
    try {
      const cfg = await invoke("get_app_config");
      if (cfg && cfg.language) {
        uiLang = cfg.language === "en" ? "en" : "ru";
        currentMode = uiLang === "en" ? "EN" : "RU";
        applyOskUiLang(uiLang);
      } else {
        renderKeyboard();
      }
    } catch (_) {
      renderKeyboard();
    }
  } else {
    renderKeyboard();
  }
  setupListener();
});
