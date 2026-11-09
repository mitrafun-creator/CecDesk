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
  "0x44": { code: "0x44", sym: "▶", name: "Воспроизведение", group: "media" },
  "0x46": { code: "0x46", sym: "⏸", name: "Пауза", group: "media" },
  "0x45": { code: "0x45", sym: "⏹", name: "Стоп", group: "media" },
  "0x4B": { code: "0x4B", sym: "⏭", name: "Следующий", group: "media" },
  "0x4C": { code: "0x4C", sym: "⏮", name: "Предыдущий", group: "media" },
  "0x48": { code: "0x48", sym: "⏪", name: "Перемотка назад", group: "media" },
  "0x49": { code: "0x49", sym: "⏩", name: "Перемотка вперед", group: "media" },

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

