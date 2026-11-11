# Регламент контроля версий и пайплайн патч-ноутов CecDesk

## 1. Схема версионирования (SemVer)

Проект использует единую версию формата `MAJOR.MINOR.PATCH[-prerelease]`:
- **Публичные бета-сборки:** `1.1.0-beta.1`, `1.1.0-beta.2`, `1.1.0-rc.1`
- **Стабильные релизы:** `1.1.0`, `1.2.0`
- **Хотфиксы (исправление критических багов):** `1.1.1`

Единый номер версии должен строго совпадать во всех контрольных точках проекта (Single Source of Truth):
1. [`version.json`](../version.json) -> поле `"version"`, URL-ы пакета и SHA-256
2. [`tauri_app/src-tauri/Cargo.toml`](../tauri_app/src-tauri/Cargo.toml) -> `[package].version`
3. [`tauri_app/src-tauri/tauri.conf.json`](../tauri_app/src-tauri/tauri.conf.json) -> `"version"`
4. [`tauri_app/src-tauri/Cargo.lock`](../tauri_app/src-tauri/Cargo.lock) -> запись пакета `cecdesk`
5. [`CHANGELOG.md`](../CHANGELOG.md) -> заголовок `## [X.Y.Z]` с датой и списком изменений

---

## 2. Как ведутся патч-ноуты (Changelog-First)

1. В процессе работы над каждой задачей **до коммита** изменения записываются в секцию `## [Unreleased]` файла [`CHANGELOG.md`](../CHANGELOG.md) по категориям:
   - `Added` — новые возможности и команды
   - `Changed` — изменения существующего поведения, UI, архитектуры
   - `Fixed` — исправления багов и регрессий
   - `Security` — исправления безопасности, приватности и прав доступа
2. Когда Бизнес даёт команду на выпуск новой версии / беты, патч-ноуты **не пишутся вручную с нуля в GitHub** — они переносятся из `[Unreleased]` в релизный блок `[X.Y.Z]` и автоматически подтягиваются в `version.json` и описание GitHub Release.

---

## 3. Автоматический локальный пайплайн (`release_tool.bat`)

Для управления версиями и проверки готовности релиза используется скрипт [`scripts_dev/release_pipeline.ps1`](../scripts_dev/release_pipeline.ps1) и обёртка [`release_tool.bat`](../release_tool.bat) в корне проекта:

### Проверить синхронность версий и манифеста
```cmd
release_tool.bat check
```
Проверяет совпадение версии в `Cargo.toml`, `tauri.conf.json`, `version.json`, `Cargo.lock` и наличие раздела в `CHANGELOG.md`, а также валидность `windows_package_url` и SHA-256 локального архива `CecDesk_Windows_x64.zip`.

### Поднять версию и сформировать патч-ноуты
```cmd
release_tool.bat bump 1.1.0-beta.2
```
Выполняет атомарно:
- Обновляет номер версии в `Cargo.toml`, `tauri.conf.json`, `version.json`.
- Переводит ссылки `download_url` и `windows_package_url` в `version.json` на тег `v1.1.0-beta.2`.
- Переносит содержимое `## [Unreleased]` в `## [1.1.0-beta.2] - YYYY-MM-DD` в `CHANGELOG.md` (и оставляет чистый шаблон `[Unreleased]`).
- Записывает сжатые патч-ноуты в поле `"notes"` файла `version.json` (для экрана обновлений в самом приложении).

### Экспортировать патч-ноуты текущей версии для GitHub Release
```cmd
release_tool.bat notes
```

### Полный релизный шлюз перед публикацией (`Release Gate`)
```cmd
release_tool.bat gate
```
Запускает полный конвейер качества:
1. Проверка консистентности версий (`check`).
2. JS-тесты UI и словарей i18n (`node --test tauri_app/ui/js/catalog.test.js`).
3. Статический анализ Rust (`run_clippy.bat`).
4. Модульные тесты Rust (`run_tests.bat`).
5. Сборка релизного бинарника, упаковка `CecDesk_Windows_x64.zip` и обновление `windows_sha256` в `version.json` (`build_release.bat`).
6. Проверка собранного `CecDesk.exe` на отсутствие утечки локального имени пользователя Windows (UTF-8 / UTF-16LE).
7. Повторная финальная проверка `version.json` vs SHA-256 готового ZIP и вывод точных команд публикации (`git tag`, `git push`, `gh release create`).
