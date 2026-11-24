# Реестр задач и бэклог CecDesk

Локальный трекер разработки по регламенту [`docs/PROCESS_AND_ROLES.md`](../docs/PROCESS_AND_ROLES.md).

---

## В работе сейчас (Active)

| ID | Приоритет | Компонент | Задача | Статус | Краткое ТЗ и критерии |
| --- | --- | --- | --- | --- | --- |
| `BUG-006` | P0 | `firmware` | Устранить ложное КЗ (`LINE_LOW`) на `RP2040-Zero` от неиспользуемого параллельного CEC-пина (`GP11`/`GP0`), добавить автовыбор `activeCecPin` и увеличить стабилизацию в `probeCecLine()` до 500 мкс (`FW 1.1.2`) | `READY` | Неиспользуемый пин не вызывает ложное КЗ и не блокирует шину CEC; `bin/firmware/rp2040_zero_firmware.uf2` и `esp32_s3_firmware.bin` обновлены до `1.1.2` |
| `BUG-005` | P0 | `backend`, `ui` | Исправить рендер HUD-оверлея мыши и OSK в Windows (WebView2 оставался в `SetIsVisible(false)` из-за отсутствия `show()` и ручного `GWL_EXSTYLE` в обход `tao`) | `READY` | Вызываются `set_ignore_cursor_events(true)` и штатные `show()`/`hide()` в `setup_hud_window`, `setup_osk_window`, `hide_osk_window` с сохранением `WS_EX_NOACTIVATE`; удалён избыточный `backdrop-filter` в `hud.css` |
| `BUG-001` | P0 | `ui`, `steamos` | Оверлей мыши на Linux/SteamOS: появляется по центру экрана, слишком непрозрачный (`0.93`), не клик-сквозной и не исчезает | `READY` | Позиционирован сверху по центру, фон `rgba(14, 16, 24, 0.55)`, включён click-through (`set_ignore_cursor_events`), отключён blur на Linux, гарантированное скрытие |
| `BUG-002` | P0 | `backend`, `steamos` | Фантомная активация режимов мыши и клавиатуры: флаг поднимается без проверки реальной активации и блокирует остальные режимы | `READY` | Единая `set_mouse_mode_checked` с проверкой `can_simulate_mouse()` (`/dev/uinput`/`ydotool`/`xdotool`), проверка `osk_win.is_visible()`, нативная Steam-клавиатура не залипает в `osk_visible`, взаимное отключение мыши и OSK |
| `BUG-003` | P0 | `firmware`, `steamos` | Steam Deck уходит в сон, но не просыпается по USB от пульта/ТВ | `READY` | Включён `tud_remote_wakeup()` для `RP2040-Zero` (на кнопки пульта и включение ТВ), `tud_suspended()` в `sendReply`/`isPcHostActive`, пересобраны `.uf2`/`.bin`, добавлено `ATTR{power/wakeup}="enabled"` в `install_steamos.sh` |
| `BUG-004` | P0 | `steamos`, `ui` | Очень низкий FPS UI на Steam Deck (программный CPU-рендеринг WebKitGTK из-за `WEBKIT_DISABLE_DMABUF_RENDERER=1`) | `READY` | Убран принудительный `WEBKIT_DISABLE_DMABUF_RENDERER=1` из `main.rs` и `install_steamos.sh` (оставлен опциональный `CECDESK_SOFTWARE_RENDER=1`), исправлен `Restart=on-failure` в `platform/linux.rs` |
| `TASK-020` | P1 | `backend`, `ui`, `steamos` | Унификация и разделение платформенного функционала и названий (Windows / SteamOS / Linux) | `READY` | Единый `PlatformProfile` (`get_platform_profile`), скрытие оптопары `PWR_BTN` и Windows display toggle на SteamOS, платформенные переопределения `PLATFORM_I18N_OVERRIDES` (`ru`/`en`) |
| `TASK-019` | P1 | `firmware` | Реализовать цветовую индикацию статуса работы модуля на встроенном RGB-светодиоде WS2812 (`RP2040-Zero` GP16 / `ESP32-S3` GPIO48) и пересобрать `bin/firmware/` | `READY` | Выключен = модуль не работает; Зелёный = ПК+ТВ ок; Оранжевое мигание (без гашения) = нет сигнала ТВ; Красное мигание = КЗ/0V; Синий = нет связи с софтом ПК; Белая вспышка = событие RX/оптопары |
| `TASK-018` | P1 | `ui`, `qa` | Убрать эмодзи в UI и исправить переполнение/выход иконок и символов за границы бейджей и кнопок (`catalog.js`, `key_mapper.js`, `remote_builder.js`, `calibration.js`, `hud.html`, `osk.js`, `style.css`) + пересобрать локальный `CecDesk.exe` | `READY` | Ни один бейдж/кнопка не обрезает символ (`VOL+`/`MUTE`/`CH+`/`GUIDE`), 0 эмодзи/пиктограмм по автотесту, пересобранный `CecDesk.exe` |
| `TASK-015` | P1 | `ui` | Убрать дублирующие плашки «На пульте» / «Не обнаружена» в таблице кнопок (`key_mapper.js`) | `READY` | Подсветка строки и иконки остаётся единственным индикатором |
| `TASK-016` | P1 | `backend`, `ui` | Добавить пользовательский профиль кнопок (`Пользовательский`) и переименование по двойному клику на его ярлык (встроенные профили не переименовываются) | `READY` | Двойной клик на ярлык `custom` сохраняет новое имя; встроенные 5 профилей защищены |
| `TASK-017` | P1 | `backend`, `ui` | Добавить в список действий пульта «Следующий/Предыдущий профиль» и прямой выбор каждого профиля (включая пользовательский) | `READY` | Работает как с физического пульта (`remote_dispatcher.rs`), так и по кнопке «Тест», обновляя UI и HUD |
| `TASK-010` | P1 | `firmware` | Исправить USB-дескрипторы (`CecDesk Dongle`), убрать `COM6`/`Antigravity` в `platformio.ini`, пересобрать `bin/firmware/` | `READY` | `pio run` RP2040/ESP32-S3 `SUCCESS`, строка `CecDesk Dongle dev/r.1 by Dimitryi Garutt` в `.uf2` |
| `TASK-011` | P1 | `backend`, `ui` | Покрыть unit-тестами `edid.rs`, `mouse_engine.rs`, `serial_transport.rs`, `remote_dispatcher.rs`, `commands/hardware.rs` + `node --check` всех JS в `gate` | `READY` | `31 passed` (Rust) + `6 passed` (JS) + `node --check` всех 14 JS-файлов |
| `TASK-012` | P2 | `backend`, `ui` | Декомпозиция монолитов `updater.rs`, `commands.rs` и `app.js` без изменения публичного API и IPC | `READY` | `updater/`, `commands/`, `updater_ui.js`, `telemetry_ui.js`, Clippy 0 warn, тесты `PASS` |
| `TASK-014` | P2 | `workspace` | Добавить `.editorconfig`, `rustfmt.toml`; подготовить переименование локальной папки ПК в `CecDesk` | `READY` | Конфиги добавлены; переименование `HDMI_CEC_Emulator` -> `CecDesk` на ПК доступно после закрытия `CecDesk.exe` |

---

## Ожидает внешнего действия (Blocked)

| ID | Приоритет | Компонент | Задача | Блокер |
| --- | --- | --- | --- | --- |
| `TASK-013` | P1 | `steamos`, `ci` | Актуализировать `.github/workflows/build_linux_steamos.yml` под `CecDesk`, убрать `.github/` из `.gitignore` и запустить CI (`TASK-006`) | Требует OAuth-подтверждения Бизнесом (`gh auth refresh -s workflow`) для push workflow-файлов |

---

## Заморожено по решению Бизнеса (ICEBOX)

| ID | Компонент | Идея / Предложение Аналитика | Причина заморозки |
| --- | --- | --- | --- |
| `ICE-001` | `backend`, `ui` | Режим `--mock-dongle` (виртуальный донгл и пульт без ТВ/платы) | Отложено Бизнесом в морозилку |
| `ICE-002` | `qa`, `docs` | Формализованный чеклист ручного регресса с физическим ТВ/оптопарой | Отложено Бизнесом в морозилку |
| `ICE-003` | `support` | Скрипт авторазбора баг-репортов `cecdesk_support_*.zip` + связка с GitHub Issues | Отложено Бизнесом в морозилку |

---

## Бэклог публичной беты (Planned / Backlog)

| ID | Приоритет | Компонент | Задача | Статус | Источник / Контекст |
| --- | --- | --- | --- | --- | --- |
| `TASK-006` | P1 | `steamos`, `release` | Публикация готового Linux/SteamOS бинарника `cecdesk_linux` в Releases | `SPEC` | Связано с `TASK-013` |
| `TASK-007` | P2 | `tv_profiles` | Подтверждение и уточнение `tv_profiles/*.json` на реальных моделях ТВ по отчётам бета-тестеров | `INBOX` | Сейчас подтверждён `philips_32pfl9705h.json`, остальные 11 ждут тестов сообщества |
| `TASK-008` | P2 | `release` | Подпись Windows-бинарника `CecDesk.exe` сертификатом (снижение предупреждений SmartScreen) | `INBOX` | Запланировано после завершения этапа публичной беты |

---

## Выполненные задачи (Done History)

| ID | Версия | Компонент | Что сделано | Итог QA |
| --- | --- | --- | --- | --- |
| `TASK-009` | `1.1.0-beta.1` | `workspace`, `qa` | Очистка рабочей папки ПК от 19 legacy-хвостов в `_CecDesk_Legacy_Archive` и проверка целостности проекта | `PASS` (17 JSON, ZIP `testzip()=None`, `git fsck=0`, 20 Rust + 6 JS) |
| `TASK-005` | `1.1.0-beta.1` | `docs`, `release` | Оформление GitHub About/Topics/Releases, пайплайн версий на ПК (`release_tool.bat`, `CHANGELOG.md`) и ролевой бизнес-процесс (`AGENTS.md`, `pm/TASKS.md`) | `PASS` |
| `TASK-004` | `1.1.0-beta.1` | `all` | Единый ребрендинг на `CecDesk` / `CecDesk Dongle` / `CecDesk App`, переименование репозитория `mitrafun-creator/CecDesk`, пересборка и публикация релиза `v1.1.0-beta.1` | `PASS` (20 Rust, 6 JS, `pio run`, release zip + `HTTP 200`) |
| `TASK-003` | `1.1.0-beta.1` | `ui`, `backend` | Интеграция нового логотипа во все форматы иконки (`.ico`, WebUI, трей, `docs/app_icon.png`) | `PASS` |
| `TASK-002` | `1.1.0-beta.1` | `legal` | Лицензирование: `PolyForm Noncommercial 1.0.0` для софта + `GNU GPL v2.0` уведомления для `firmware/` (`THIRD_PARTY_NOTICES.md`) | `PASS` |
| `TASK-001` | `1.1.0-beta.1` | `backend`, `ui` | Подготовка к публичной бете: разделение AppData/Portable, миграция, 1-click self-updater, логи, `cecdesk_support_*.zip`, zero-PII сборка через `subst` | `PASS` |
