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

test("UI HTML/CSS avoids Tauri CSP nonce/hash invalidation and hides modals by default", () => {
  const uiDir = path.join(__dirname, "..");
  const htmlFiles = ["index.html", "hud.html", "osk.html"];
  for (const file of htmlFiles) {
    const html = fs.readFileSync(path.join(uiDir, file), "utf8");
    const htmlWithoutComments = html.replace(/<!--[\s\S]*?-->/g, "");
    assert.ok(!/<style\b/i.test(htmlWithoutComments), `${file} must not contain inline <style> (triggers Tauri style-src nonce)`);
    assert.ok(
      !/<script(?![^>]*\bsrc=)[^>]*>[\s\S]*?<\/script>/i.test(htmlWithoutComments),
      `${file} must not contain inline <script> (triggers Tauri script-src hash)`
    );
    assert.ok(
      !/\son[a-z]+\s*=/i.test(htmlWithoutComments),
      `${file} must not contain inline on*= event attributes (blocked by Tauri script-src hash/self CSP)`
    );
    assert.ok(
      !/\bstyle\s*=\s*["'][^"']*display\s*:\s*none/i.test(htmlWithoutComments),
      `${file} must not rely on inline style="display:none"`
    );
  }

  const css = fs.readFileSync(path.join(uiDir, "style.css"), "utf8");
  assert.match(css, /\.cselect-portal-layer\s*\{/);
  assert.match(css, /\.c-select-menu\.is-open/);
  assert.match(css, /\.mapper-table-scroll\s*\{/);
  assert.match(css, /\.calib-modal-backdrop\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.calib-modal-backdrop\.is-open\s*\{[^}]*display:\s*flex\s*;/s);
  assert.match(css, /\.crash-recovery-banner\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.crash-recovery-banner\.is-visible\s*\{[^}]*display:\s*flex\s*;/s);
  assert.match(css, /\.audio-routing-subbox\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.audio-routing-subbox\.is-visible\s*\{[^}]*display:\s*flex\s*;/s);
  assert.match(css, /\.fw-progress-wrap\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.fw-progress-wrap\.is-visible\s*\{[^}]*display:\s*block\s*;/s);
  assert.match(css, /\.app-update-progress-wrap\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.app-update-progress-wrap\.is-visible\s*\{[^}]*display:\s*block\s*;/s);
  assert.match(css, /\.update-releases-btn\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.update-releases-btn\.is-visible\s*\{[^}]*display:\s*inline-flex\s*;/s);
  assert.match(css, /\.bug-result-banner\s*\{[^}]*display:\s*none\s*;/s);
  assert.match(css, /\.bug-result-banner\.is-visible\s*\{[^}]*display:\s*block\s*;/s);
});

test("Beta readiness defaults exclude hardcoded dev COM ports and personal TV profile", () => {
  const uiDir = path.join(__dirname, "..");
  const repoDir = path.join(uiDir, "..", "..");
  const appJs = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
  const hwJs = fs.readFileSync(path.join(__dirname, "hardware_status.js"), "utf8");
  const html = fs.readFileSync(path.join(uiDir, "index.html"), "utf8");
  const defaultCfg = JSON.parse(fs.readFileSync(path.join(repoDir, "cecdesk_config.default.json"), "utf8"));

  assert.strictEqual(defaultCfg.active_profile_id, "universal_hdmi_cec");
  assert.ok(!/"COM(?:6|10)"/.test(appJs), "app.js must not hardcode COM6 or COM10 fallback");
  assert.ok(!/"COM(?:6|10)"/.test(hwJs), "hardware_status.js must not hardcode COM6 or COM10 fallback");
  assert.ok(!appJs.includes("philips_32pfl9705h"), "app.js must default to universal_hdmi_cec");
  assert.ok(!html.includes(">Philips<"), "index.html initial brand placeholder must be neutral");
  assert.ok(!html.includes("HDMI 3 (3000)"), "index.html initial HDMI address placeholder must be HDMI 1 (1000)");
});
