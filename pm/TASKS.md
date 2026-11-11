# Реестр задач и бэклог CecDesk

Локальный трекер разработки по регламенту [`docs/PROCESS_AND_ROLES.md`](../docs/PROCESS_AND_ROLES.md).

---

## В работе сейчас (Active)

*(Активных незакрытых задач в разработке нет — готова к приёму новая задача от Бизнеса.)*

---

## Ожидает приёмки Бизнесом (READY)

| ID | Приоритет | Компонент | Задача | Критерии и проверка QA |
| --- | --- | --- | --- | --- |
| `TASK-005` | P1 | `docs`, `release` | Оформление GitHub About/Topics/Releases, пайплайн версий на ПК (`release_tool.bat`, `CHANGELOG.md`) и ролевой бизнес-процесс (`AGENTS.md`, `pm/TASKS.md`) | `release_tool.bat check`, тесты и `README` проверены QA |

---

## Бэклог публичной беты (Planned / Backlog)

| ID | Приоритет | Компонент | Задача | Статус | Источник / Контекст |
| --- | --- | --- | --- | --- | --- |
| `TASK-006` | P1 | `steamos`, `release` | Настройка кросс-сборки / публикации готового Linux/SteamOS бинарника `cecdesk_linux` в Releases | `SPEC` | Пока `install_steamos.sh` при отсутствии бинарника компилирует через `cargo` |
| `TASK-007` | P2 | `tv_profiles` | Подтверждение и уточнение `tv_profiles/*.json` на реальных моделях ТВ по отчётам бета-тестеров | `INBOX` | Сейчас подтверждён `philips_32pfl9705h.json`, остальные 11 ждут тестов сообщества |
| `TASK-008` | P2 | `release` | Подпись Windows-бинарника `CecDesk.exe` сертификатом (снижение предупреждений SmartScreen) | `INBOX` | Запланировано после завершения этапа публичной беты |

---

## Выполненные задачи (Done History)

| ID | Версия | Компонент | Что сделано | Итог QA |
| --- | --- | --- | --- | --- |
| `TASK-004` | `1.1.0-beta.1` | `all` | Единый ребрендинг на `CecDesk` / `CecDesk Dongle` / `CecDesk App`, переименование репозитория `mitrafun-creator/CecDesk`, пересборка и публикация релиза `v1.1.0-beta.1` | `PASS` (20 Rust, 6 JS, `pio run`, release zip + `HTTP 200`) |
| `TASK-003` | `1.1.0-beta.1` | `ui`, `backend` | Интеграция нового логотипа во все форматы иконки (`.ico`, WebUI, трей, `docs/app_icon.png`) | `PASS` |
| `TASK-002` | `1.1.0-beta.1` | `legal` | Лицензирование: `PolyForm Noncommercial 1.0.0` для софта + `GNU GPL v2.0` уведомления для `firmware/` (`THIRD_PARTY_NOTICES.md`) | `PASS` |
| `TASK-001` | `1.1.0-beta.1` | `backend`, `ui` | Подготовка к публичной бете: разделение AppData/Portable, миграция, 1-click self-updater, логи, `cecdesk_support_*.zip`, zero-PII сборка через `subst` | `PASS` |
