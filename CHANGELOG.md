# Журнал изменений CecDesk (Changelog)

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), нумерация версий следует [Semantic Versioning](https://semver.org/lang/ru/).

## [Unreleased]

### Added
- Статусная RGB-индикация состояния на аппаратном модуле `CecDesk Dongle` (`TASK-019`): отображение активной связи с ПК и ТВ, автономного режима ожидания ТВ, отсутствия сигнала линии CEC, короткого замыкания линии и короткой индикации приёма команд.
- Профиль телевизора `tv_profiles/samsung_ue43j5272.json` для тестирования на Samsung UE43J5272 (2015, платформа Orsay, Anynet+ HDMI-CEC, прошивка `T-N14MJDEUC-1150.0`) и запись в `tv_profiles/catalog.json`.
- Пользовательский профиль кнопок пульта (`custom`, по умолчанию «Пользовательский») с инлайн-переименованием по двойному клику на его ярлык в шапке конструктора без возможности переименования встроенных пресетов (`TASK-016`).
- Действия переключения профиля кнопок в списке действий пульта: `Следующий профиль`, `Предыдущий профиль` и прямое переключение на каждый из 5 встроенных профилей и пользовательский профиль с обновлением конфигурации на лету, событием `key-profile-changed` и HUD-уведомлением (`TASK-017`).
- Конфигурации единого форматирования кода `.editorconfig` и `rustfmt.toml` (`TASK-014`).
- Новые unit-тесты для критических модулей Rust: `edid.rs`, `hardware/mouse_engine.rs`, `hardware/serial_transport.rs`, `hardware/remote_dispatcher.rs` и классификация портов в `commands/hardware.rs`, а также проверка синтаксиса `node --check` всех файлов `tauri_app/ui/js/*.js` в `scripts_dev/release_pipeline.ps1` (`TASK-011`).
- Единый бизнес-процесс разработки с ролевой моделью (`Бизнес`, `Аналитик/Поддержка`, `Тимлид`, `Разработчик`, `QA`, `Релиз-инженер`) в `AGENTS.md`, `docs/PROCESS_AND_ROLES.md` и локальный трекер задач `pm/TASKS.md`.
- Локальный пайплайн контроля версий и патч-ноутов (`scripts_dev/release_pipeline.ps1`, `release_tool.bat`, `docs/RELEASE_AND_VERSIONING.md`, `CHANGELOG.md`).

### Changed
- Убраны текстовые приписки-бейджи «На пульте» / «Не обнаружена» из списка кнопок в маппере, дублирующие подсветку активных клавиш (`TASK-015`).
- Декомпозированы монолиты без изменения публичного API и IPC-контрактов (`TASK-012`):
  - `commands.rs` разбит на доменный модуль `tauri_app/src-tauri/src/commands/` (`mod.rs`, `hardware.rs`, `system.rs`, `updates.rs`).
  - `updater.rs` разбит на доменный модуль `tauri_app/src-tauri/src/updater/` (`mod.rs`, `catalog.rs`, `manifest.rs`, `archive.rs`, `self_update.rs`, `firmware_flasher.rs`, `tests.rs`).
  - Из `tauri_app/ui/js/app.js` выделены модули `updater_ui.js` (OTA-обновления, онлайн-каталог профилей ТВ, прошивка) и `telemetry_ui.js` (диагностика, баг-репорты, телеметрия, краш-баннер), подключённые в `tauri_app/ui/index.html`.
- Улучшено оформление репозитория GitHub и `README.md` (логотип, бейджи, исправлено отображение дерева архитектуры, актуальный About/Topics и разметка релизов).

### Fixed
- Исправлено оформление и геометрия иконок и бейджей клавиш в WebUI, HUD и экранной клавиатуре OSK (`TASK-018`):
  - В маппере кнопок удалены остаточные CSS-классы `.badge-remote-avail` / `.badge-remote-unavail` и расширены/выровнены бейджи `.key-symbol-badge` (`min-width: 48px`, корректный шрифт/отступы и `overflow: hidden`), чтобы `VOL+`, `VOL−`, `MUTE`, `CH+`, `CH−`, `INFO`, `GUIDE`, `OPT`, `FAV` не выходили за рамки; сужена минимальная ширина `.c-select` в таблице для исключения горизонтальной прокрутки.
  - Заменены все Unicode-символы медиа, навигации и системные эмодзи (`⏪`, `⏩`, `▶`, `⏸`, `⏹`, `⏭`, `⏮`, `◀`, `⌫`, `↵`, `⇧`, `⤓`) на единые монохромные векторные SVG-иконки в `catalog.js` (`KEY_ICON_SVGS`), `key_mapper.js`, `remote_builder.js`, `calibration.js`, `hud.html` и `osk.js`.
  - Добавлен автотест в `tauri_app/ui/js/catalog.test.js`, гарантирующий отсутствие эмодзи и пиктограмм в исходниках UI.
- Унифицированы USB-дескрипторы адаптера `CecDesk Dongle` и очищены отладочные параметры сборки (`TASK-010`).

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
