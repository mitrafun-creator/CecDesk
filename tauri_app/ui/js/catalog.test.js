const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

// Загружаем catalog.js в изолированный контекст
const catalogCode = fs.readFileSync(path.join(__dirname, "catalog.js"), "utf8");
const fn = new Function(catalogCode + `
  return {
    KEY_DEFINITIONS,
    DEFAULT_BASE_KEYS,
    getExtraKeyLetterMap,
    getButtonDefinition,
    profileShortName
  };
`);
const { KEY_DEFINITIONS, DEFAULT_BASE_KEYS, getExtraKeyLetterMap, getButtonDefinition, profileShortName } = fn();

test("profileShortName extracts clean TV model name", () => {
  assert.strictEqual(profileShortName(null), "");
  assert.strictEqual(profileShortName({ name: "Philips 32PFL9705H (EasyLink)" }), "Philips 32PFL9705H");
  assert.strictEqual(profileShortName({ name: "LG OLED C2" }), "LG OLED C2");
  assert.strictEqual(profileShortName({ display_name: "Мой крутой ТВ", name: "Generic TV" }), "Мой крутой ТВ");
});

test("getExtraKeyLetterMap generates clean alphabetical labels", () => {
  // Стандартные кнопки не должны попадать в extra буквы
  const activeKeys = ["0x00", "0x01", "0x51", "0x52"];
  const map = getExtraKeyLetterMap(activeKeys);
  assert.strictEqual(map["0x51"], "A");
  assert.strictEqual(map["0x52"], "B");
  assert.strictEqual(map["0x00"], undefined);
});

test("getButtonDefinition returns correct labels for known and unknown keys", () => {
  const map = { "0x99": "C" };
  const okDef = getButtonDefinition("0x00", map);
  assert.strictEqual(okDef.name, "Подтверждение");
  assert.strictEqual(okDef.sym, "OK");

  const extraDef = getButtonDefinition("0x99", map);
  assert.strictEqual(extraDef.name, "Кнопка C");
  assert.strictEqual(extraDef.sym, "C");
  assert.strictEqual(extraDef.isExtra, true);
});

test("i18n dictionary parity and completeness between ru and en", () => {
  const i18nCode = fs.readFileSync(path.join(__dirname, "i18n.js"), "utf8");
  const i18nFn = new Function(i18nCode + "\n return { I18N_DICTIONARY };");
  const { I18N_DICTIONARY } = i18nFn();

  const ruKeys = Object.keys(I18N_DICTIONARY.ru).sort();
  const enKeys = Object.keys(I18N_DICTIONARY.en).sort();

  const missingInEn = ruKeys.filter(k => !I18N_DICTIONARY.en[k]);
  const missingInRu = enKeys.filter(k => !I18N_DICTIONARY.ru[k]);

  assert.deepStrictEqual(missingInEn, [], `Keys present in RU but missing in EN: ${missingInEn.join(", ")}`);
  assert.deepStrictEqual(missingInRu, [], `Keys present in EN but missing in RU: ${missingInRu.join(", ")}`);

  for (const k of ruKeys) {
    assert.ok(I18N_DICTIONARY.ru[k].trim().length > 0, `RU key "${k}" is empty`);
    assert.ok(I18N_DICTIONARY.en[k].trim().length > 0, `EN key "${k}" is empty`);
  }
});
