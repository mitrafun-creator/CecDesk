/**
 * catalog.js - Реестр кнопок пульта CEC, символы, группы и базовые наборы
 */

// Реестр всех поддерживаемых кнопок пульта
const KEY_DEFINITIONS = {
  "0x00": { code: "0x00", sym: "OK", name: "Подтверждение", group: "nav" },
  "0x01": { code: "0x01", sym: "↑", name: "Вверх", group: "nav" },
  "0x02": { code: "0x02", sym: "↓", name: "Вниз", group: "nav" },
  "0x03": { code: "0x03", sym: "←", name: "Влево", group: "nav" },
  "0x04": { code: "0x04", sym: "→", name: "Вправо", group: "nav" },
  "0x0D": { code: "0x0D", sym: "⟵", name: "Возврат", group: "top" },
  "0x09": { code: "0x09", sym: "⌂", name: "Меню", group: "top" },
  "0x0A": { code: "0x0A", sym: "⋯", name: "Опции", group: "top" },
  "0x0B": { code: "0x0B", sym: "OPT", name: "Опции", group: "top" },
  "0x0C": { code: "0x0C", sym: "FAV", name: "Избранное", group: "top" },
  "0x35": { code: "0x35", sym: "INFO", name: "Инфо", group: "top" },
  "0x53": { code: "0x53", sym: "GUIDE", name: "Гид (EPG)", group: "top" },

  // Цветовой ряд (Красная, Зеленая, Желтая, Синяя)
  "0x72": { code: "0x72", sym: "R", name: "Красная", group: "color", class: "c-red" },
  "0x73": { code: "0x73", sym: "G", name: "Зеленая", group: "color", class: "c-green" },
  "0x74": { code: "0x74", sym: "Y", name: "Желтая", group: "color", class: "c-yellow" },
  "0x71": { code: "0x71", sym: "B", name: "Синяя", group: "color", class: "c-blue" },

  // Звук
  "0x41": { code: "0x41", sym: "VOL+", name: "Громкость +", group: "vol" },
  "0x42": { code: "0x42", sym: "VOL−", name: "Громкость −", group: "vol" },
  "0x43": { code: "0x43", sym: "MUTE", name: "Без звука", group: "vol" },

  // Медиа
  "0x44": { code: "0x44", sym: "PLAY", name: "Воспроизведение", group: "media" },
  "0x46": { code: "0x46", sym: "II", name: "Пауза", group: "media" },
  "0x45": { code: "0x45", sym: "STOP", name: "Стоп", group: "media" },
  "0x4B": { code: "0x4B", sym: ">|", name: "Следующий", group: "media" },
  "0x4C": { code: "0x4C", sym: "|<", name: "Предыдущий", group: "media" },
  "0x48": { code: "0x48", sym: "<<", name: "Перемотка назад", group: "media" },
  "0x49": { code: "0x49", sym: ">>", name: "Перемотка вперед", group: "media" },

  // Каналы
  "0x30": { code: "0x30", sym: "CH+", name: "Канал +", group: "channels" },
  "0x31": { code: "0x31", sym: "CH−", name: "Канал −", group: "channels" },

  // Цифровая клавиатура 0-9
  "0x21": { code: "0x21", sym: "1", name: "Цифра 1", group: "numpad" },
  "0x22": { code: "0x22", sym: "2", name: "Цифра 2", group: "numpad" },
  "0x23": { code: "0x23", sym: "3", name: "Цифра 3", group: "numpad" },
  "0x24": { code: "0x24", sym: "4", name: "Цифра 4", group: "numpad" },
  "0x25": { code: "0x25", sym: "5", name: "Цифра 5", group: "numpad" },
  "0x26": { code: "0x26", sym: "6", name: "Цифра 6", group: "numpad" },
  "0x27": { code: "0x27", sym: "7", name: "Цифра 7", group: "numpad" },
  "0x28": { code: "0x28", sym: "8", name: "Цифра 8", group: "numpad" },
  "0x29": { code: "0x29", sym: "9", name: "Цифра 9", group: "numpad" },
  "0x20": { code: "0x20", sym: "0", name: "Цифра 0", group: "numpad" }
};

// Базовый универсальный набор кнопок (присутствует на подавляющем большинстве пультов)
const DEFAULT_BASE_KEYS = [
  "0x0D", "0x09", "0x0A",
  "0x01", "0x02", "0x03", "0x04", "0x00",
  "0x42", "0x43", "0x41",
  "0x72", "0x73", "0x74", "0x71"
];

/**
 * Получение активного набора клавиш для текущего сеанса
 * Приоритет: 1) Пользовательская калибровка -> 2) Профиль ТВ -> 3) Универсальный базовый набор
 */
function getActiveRemoteKeys() {
  if (appConfig && Array.isArray(appConfig.calibrated_keys) && appConfig.calibrated_keys.length > 0) {
    return appConfig.calibrated_keys;
  }
  if (currentProfile && Array.isArray(currentProfile.supported_keys) && currentProfile.supported_keys.length > 0) {
    return currentProfile.supported_keys;
  }
  return DEFAULT_BASE_KEYS;
}

/**
 * Получение единой карты латинских букв (A, B, C... AA, AB...) для неизвестных кнопок
 */
function getExtraKeyLetterMap(activeKeys) {
  const standardSet = new Set(Object.keys(KEY_DEFINITIONS));
  const extraKeys = (activeKeys || []).filter(k => !standardSet.has(k)).sort();
  const map = {};
  extraKeys.forEach((k, idx) => {
    if (idx < 26) {
      map[k] = String.fromCharCode(65 + idx);
    } else {
      const first = String.fromCharCode(65 + Math.floor(idx / 26) - 1);
      const second = String.fromCharCode(65 + (idx % 26));
      map[k] = first + second;
    }
  });
  return map;
}

/**
 * Получение структуры кнопки с учётом латинских букв
 */
function getButtonDefinition(code, letterMap) {
  if (KEY_DEFINITIONS[code]) {
    return KEY_DEFINITIONS[code];
  }
  const letter = (letterMap && letterMap[code]) || "A";
  return {
    code,
    sym: letter,
    name: `Кнопка ${letter}`,
    isExtra: true
  };
}

// Монохромные векторные иконки (без системных эмодзи и шрифтовых подмен)
const KEY_ICON_SVGS = {
  "0x01": '<svg class="ico" viewBox="0 0 24 24"><polyline points="18 15 12 9 6 15"></polyline></svg>',
  "0x02": '<svg class="ico" viewBox="0 0 24 24"><polyline points="6 9 12 15 18 9"></polyline></svg>',
  "0x03": '<svg class="ico" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>',
  "0x04": '<svg class="ico" viewBox="0 0 24 24"><polyline points="9 18 15 12 9 6"></polyline></svg>',
  "0x0D": '<svg class="ico" viewBox="0 0 24 24"><polyline points="15 18 9 12 15 6"></polyline></svg>',
  "0x09": '<svg class="ico" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2"></rect><line x1="8" y1="20" x2="16" y2="20"></line></svg>',
  "0x0A": '<svg class="ico" viewBox="0 0 24 24"><rect x="2" y="4" width="20" height="16" rx="2"></rect><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 16h10"></path></svg>',
  "0x44": '<svg class="ico" viewBox="0 0 24 24"><polygon points="6 3 20 12 6 21 6 3"></polygon></svg>',
  "0x46": '<svg class="ico" viewBox="0 0 24 24"><line x1="10" y1="4" x2="10" y2="20"></line><line x1="14" y1="4" x2="14" y2="20"></line></svg>',
  "0x45": '<svg class="ico" viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="2"></rect></svg>',
  "0x4B": '<svg class="ico" viewBox="0 0 24 24"><polygon points="5 4 15 12 5 20 5 4"></polygon><line x1="19" y1="5" x2="19" y2="19"></line></svg>',
  "0x4C": '<svg class="ico" viewBox="0 0 24 24"><polygon points="19 20 9 12 19 4 19 20"></polygon><line x1="5" y1="19" x2="5" y2="5"></line></svg>',
  "0x48": '<svg class="ico" viewBox="0 0 24 24"><polygon points="11 19 2 12 11 5 11 19"></polygon><polygon points="22 19 13 12 22 5 22 19"></polygon></svg>',
  "0x49": '<svg class="ico" viewBox="0 0 24 24"><polygon points="13 19 22 12 13 5 13 19"></polygon><polygon points="2 19 11 12 2 5 2 19"></polygon></svg>'
};

function getKeyIconSvg(codeHex) {
  if (!codeHex) return null;
  const raw = String(codeHex).trim();
  const norm = raw.toUpperCase().startsWith("0X")
    ? "0x" + raw.substring(2).toUpperCase()
    : "0x" + raw.toUpperCase();
  return KEY_ICON_SVGS[norm] || null;
}

function renderKeySymbolContent(el, codeHex, fallbackSym) {
  if (!el) return;
  const svg = getKeyIconSvg(codeHex);
  if (svg) {
    el.innerHTML = svg;
    el.classList.remove("is-long-sym");
    return;
  }
  const txt = String(fallbackSym || "");
  el.textContent = txt;
  el.classList.toggle("is-long-sym", txt.length >= 4);
}

/**
 * Извлечение краткого названия модели ТВ (с учётом переименования пользователем)
 */
function profileShortName(prof) {
  if (!prof) return "";
  if (typeof appConfig !== "undefined" && appConfig && appConfig.custom_profile_names && appConfig.custom_profile_names[prof.id]) {
    return appConfig.custom_profile_names[prof.id];
  }
  if (prof.display_name && prof.display_name.trim()) {
    return prof.display_name.trim();
  }
  const name = prof.name || "";
  return name.split("(")[0].trim() || name;
}

