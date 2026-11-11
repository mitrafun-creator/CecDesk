# Журнал изменений CecDesk (Changelog)

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- Конфигурации единого форматирования кода `.editorconfig` и `rustfmt.toml` (`TASK-014`).
- Новые unit-тесты для критических модулей Rust: `edid.rs`, `hardware/mouse_engine.rs`, `hardware/serial_transport.rs`, `hardware/remote_dispatcher.rs` и классификация портов в `commands/hardware.rs`, а также проверка синтаксиса `node --check` всех файлов `tauri_app/ui/js/*.js` в `scripts_dev/release_pipeline.ps1` (`TASK-011`).
- Единый бизнес-процесс разработки с ролевой моделью (`Бизнес`, `Аналитик/Поддержка`, `Тимлид`, `Разработчик`, `QA`, `Релиз-инженер`) в `AGENTS.md`, `docs/PROCESS_AND_ROLES.md` и локальный трекер задач `pm/TASKS.md`.
- Локальный пайплайн контроля версий и патч-ноутов (`scripts_dev/release_pipeline.ps1`, `release_tool.bat`, `docs/RELEASE_AND_VERSIONING.md`, `CHANGELOG.md`).

### Changed
- Декомпозированы монолиты без изменения публичного API и IPC-контрактов (`TASK-012`):
  - `commands.rs` разбит на доменный модуль `tauri_app/src-tauri/src/commands/` (`mod.rs`, `hardware.rs`, `system.rs`, `updates.rs`).
  - `updater.rs` разбит на доменный модуль `tauri_app/src-tauri/src/updater/` (`mod.rs`, `catalog.rs`, `manifest.rs`, `archive.rs`, `self_update.rs`, `firmware_flasher.rs`, `tests.rs`).
  - Из `tauri_app/ui/js/app.js` выделены модули `updater_ui.js` (OTA-обновления, онлайн-каталог профилей ТВ, прошивка) и `telemetry_ui.js` (диагностика, баг-репорты, телеметрия, краш-баннер), подключённые в `tauri_app/ui/index.html`.
- Улучшено оформление репозитория GitHub и `README.md` (логотип, бейджи, исправлено отображение дерева архитектуры, актуальный About/Topics и разметка релизов).

### Fixed
- Исправлены USB-дескрипторы прошивки RP2040-Zero (`initVariant()` в `firmware/src/main.cpp` задаёт `Dimitryi Garutt` / `CecDesk Dongle dev/r.1 by Dimitryi Garutt`) и очищен `firmware/platformio.ini` от `upload_port = COM6`, `monitor_port = COM6` и `-DUSBD_MANUFACTURER='"Antigravity"'`; пересобраны и обновлены бинарники `bin/firmware/rp2040_zero_firmware.uf2` и `bin/firmware/esp32_s3_firmware.bin` (`TASK-010`).

---

## [1.1.0-beta.1] - 2026-11-11

### Added
- Полный ребрендинг комплекса на трёхуровневую систему именования: `CecDesk` (комплекс и репозиторий), `CecDesk Dongle` (USB-адаптер), `CecDesk App` (`CecDesk.exe` / `cecdesk_linux`).
- Новый логотип и полный набор иконок (Windows `.ico`, WebUI, трей, документация).
- Разделение ресурсов программы и пользовательских данных (`%LOCALAPPDATA%\CecDesk`, `~/.local/share/cecdesk`, портативный режим через `portable.txt` / `CEC_PORTABLE=1` / `--portable`).
- Автоматическая неразрушающая миграция legacy-конфигов (`cec_studio_config.json`, `HDMI-CEC Studio`, `hdmi-cec-studio`, `hdmi_cec_studio`).
- Сборщик диагностического архива `cecdesk_support_<timestamp>.zip` с маскированием путей пользователя и токенов, детектор аварийного завершения, опциональная opt-in телеметрия.
- Механизм самообновления Windows в 1 клик через GitHub Releases с проверкой SHA-256, ZIP CRC-32 и PE-заголовка (`build_release.bat` + `version.json`).
- Публичная лицензия PolyForm Noncommercial License 1.0.0 (`LICENSE`) и сводка сторонних лицензий (`THIRD_PARTY_NOTICES.md`).

### Changed
- Оптимизирована релизная сборка Windows (удалены абсолютные локальные пути пользователя из бинарника через `--remap-path-prefix` и `subst`).
- Сохранена обратная совместимость со старыми прошивками (`CEC-EMU`), URL репозитория и бинарниками при обновлении.

### Security
- Исключена утечка системного имени пользователя Windows из релизного `CecDesk.exe`.
- Добавлена защита от Path Traversal / Zip-Slip при распаковке обновлений и запуске скриптов.

---

## [1.1.0] - 2026-11-09

### Note
- Архивный базовый релиз под прежним названием `HDMI-CEC Studio` до ребрендинга в `CecDesk` и запуска публичной беты `1.1.0-beta.1`.
