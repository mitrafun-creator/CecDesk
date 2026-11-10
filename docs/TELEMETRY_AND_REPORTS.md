# Телеметрия, логирование и сбор баг-репортов (HDMI-CEC Studio)

## Принципы приватности (Opt-In & Zero-PII)

1. **По умолчанию телеметрия полностью выключена (`telemetry_opt_in: false`)**. Приложение не отправляет никаких фоновых отчётов до явного согласия пользователя и указания HTTPS-адреса приёмника.
2. **Анонимный идентификатор (`install_id`)**: случайный UUID v4, сгенерированный криптографическим генератором ОС (`BCryptGenRandom` на Windows, `/dev/urandom` на Linux/SteamOS). Не использует MAC-адреса, серийные номера дисков/материнских плат, имя компьютера или имя учётной записи ОС.
3. **Автоматическое маскирование чувствительных данных (`redact_sensitive`)**:
   - Домашние каталоги и имена пользователей ОС (`C:\Users\<USER>\...`, `/home/<USER>/...`, `/Users/<USER>/...`) заменяются на `<USER>` / `<USER_HOME>`.
   - Query-токены и ключи авторизации (`?token=...`, `Bearer ...`, `telemetry_endpoint?...`) заменяются на `[REDACTED]`.
   - Файл конфигурации внутри отчёта проходит рекурсивную санитизацию перед сохранением в `.zip`.

## Хранилище данных (AppData / XDG / Portable)

- **Windows (стандартный режим)**: `%LOCALAPPDATA%\HDMI-CEC Studio\` (`cec_studio_config.json`, `logs/`, `scripts/`, `tv_profiles/`, `reports/`, `session.active`).
- **Linux / SteamOS**: `$XDG_DATA_HOME/hdmi-cec-studio/` или `~/.local/share/hdmi-cec-studio/`.
- **Portable-режим**: активируется при наличии пустого файла `portable.txt` (или `.portable`) рядом с исполняемым файлом, переменной среды `CEC_PORTABLE=1` или флага `--portable`. Все данные сохраняются рядом с программой.
- При первом запуске после обновления выполняется однократная неразрушающая миграция legacy-конфига и пользовательских скриптов.

## Структура диагностического архива (`hdmi_cec_support_<ts>.zip`)

Создаётся локально в папке `reports/` по нажатию кнопки в интерфейсе:

- `diagnostics.json` — снимок среды (версия приложения, ОС/архитектура, тип дисплейной сессии `Windows Desktop` / `Gamescope` / `Wayland` / `X11`, тип и порт контроллера, EDID PNP/Physical Address, счётчики ошибок/предупреждений, признак прошлого аварийного завершения, последние санитизированные ошибки и заметка пользователя).
- `cec_studio_config.json` — копия конфигурации без секретных query-параметров.
- `logs/cec_latest.log` (хвост до 512 КБ, санитизированный).
- `logs/cec_session_latest.log` (хвост до 512 КБ).
- `logs/trace.log` и `logs/trace.prev.log` (если есть, до 512 КБ).
- `logs/panic.txt` (если фиксировался Rust panic).

## Контракт HTTPS Webhook Collector

При настройке `telemetry_endpoint` (обязательно `https://...`) приложение отправляет `POST` с заголовками:
- `Content-Type: application/json`
- `X-HDMI-CEC-Studio-Schema: 1.0`

### 1. Анонимный heartbeat (`kind: "telemetry_heartbeat"`)

```json
{
  "kind": "telemetry_heartbeat",
  "schema_version": "1.0",
  "trigger": "startup",
  "timestamp": "2026-11-10T12:00:00.000Z",
  "install_id": "550e8400-e29b-41d4-a716-446655440000",
  "app_version": "1.1.0-beta.1",
  "update_channel": "beta",
  "os": "windows",
  "arch": "x86_64",
  "display_session": "Windows Desktop",
  "storage_mode": "AppData (Local)",
  "controller_type": "RP2040-Zero",
  "edid_pnp_id": "PHL",
  "session_error_count": 0,
  "session_warn_count": 0,
  "dirty_shutdown_detected": false,
  "has_pending_panic": false
}
```

### 2. Пользовательский баг-репорт (`kind: "bug_report"`)

Отправляется **только** по явному действию пользователя из окна отчёта об ошибке:

```json
{
  "kind": "bug_report",
  "schema_version": "1.0",
  "snapshot": { "...": "объект DiagnosticsSnapshot" },
  "recent_logs": ["[2026-11-10T12:00:01.000Z] [INFO] ..."]
}
```
