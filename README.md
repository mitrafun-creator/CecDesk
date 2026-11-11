# HDMI-CEC Studio & Hardware Emulator

> **Устройство:** `CEC-EMU dev/r.1 by Dimitryi Garutt`  
> **Архитектура ПО:** HDMI-CEC Studio (Tauri v2 + Модульный Rust Backend + Чистый Vanilla WebUI)  
> **Микроконтроллеры:** RP2040-Zero (PIO State Machine) / ESP32-S3 (Open-Drain CEC GPIO4, USB CDC)  
> **Целевые ТВ:** Универсальный профиль HDMI-CEC 1.4/2.0, Philips EasyLink, LG SimpLink, Samsung Anynet+, Sony Bravia Sync, Panasonic VIERA Link, TCL, Hisense VIDAA, Xiaomi, Toshiba, Sharp

---

## Возможности проекта

1. **Аппаратный эмулятор HDMI-CEC (`CEC-EMU`)**:
   - Автоматически считывает физический адрес HDMI порта и производителя ТВ из EDID (Windows Registry / DRM EDID в Linux).
   - Прописывает экранное имя устройства в ТВ (**GAMING PC**).
   - Единая интеллектуальная кнопка питания («Включить ТВ» / «Выключить ТВ»), синхронизированная с реальным состоянием экрана по протоколу CEC (`0x90` / `0x36`).
   - Встроенная прошивка контроллеров прямо из интерфейса **Настройки -> Прошивка контроллера** (UF2 для RP2040-Zero и Serial OTA для ESP32-S3).

2. **Динамический конструктор пульта и режим калибровки**:
   - Конструктор интерфейса собирает пульт на лету из реально доступных клавиш.
   - Интерактивный режим обучения (калибровка): пользователь нажимает кнопки на физическом пульте, система перехватывает их коды на уровне шины и автоматически формирует персональную конфигурацию пульта и таблицу маппинга.
   - Защита ядра: во время обучения эмуляция системных действий ОС полностью блокируется.

3. **Мост пульта ДУ телевизора в Windows и Linux / SteamOS**:
   - Полный перехват кнопок пульта ДУ и трансляция в действия Windows / SteamOS (Gamescope & Desktop).
   - При выключении перехвата пульта микроконтроллер освобождает кнопки обратно для меню телевизора.
   - **Режим мыши с прогрессивным разгоном**:
     - Управление курсором стрелками пульта: микро-шаг 4 px для точности и разгон при удержании.
     - Быстрые прыжки по 9 секторам экрана с цифровых клавиш, ЛКМ / ПКМ / 2x-клик.
     - Авто-таймаут возврата в обычный режим через 40 секунд неактивности.
   - **Умная перемотка (Hold-to-Accelerate Seek)**:
     - Одиночный клик: шаг 5 сек. Удержание: ускорение до 15–30 сек.
   - **Экранная клавиатура (OSK) и HUD Overlay**:
     - Всплывающие уведомления при регулировке звука и переключении режимов без потери фокуса полноэкранных игр.
     - 2D-навигация стрелками пульта по виртуальной клавиатуре.
   - **Аварийное закрытие зависших задач**:
     - Опционально: удержание кнопки «Назад» более 3.8 секунд завершает зависший полноэкранный процесс.

4. **Выполнение пользовательских скриптов (`scripts/`)**:
   - Запуск `.bat`, `.cmd`, `.ps1`, `.vbs` (Windows) и `.sh` / исполняемых сценариев (Linux / SteamOS) по нажатию любой кнопки пульта.

---

## Архитектура проекта

```text
HDMI_CEC_Emulator/
├── LICENSE                         <-- Лицензия PolyForm Noncommercial License 1.0.0
├── THIRD_PARTY_NOTICES.md          <-- Сторонние компоненты и GPLv2-уведомление для firmware
├── version.json                    <-- Манифест актуальной версии, ссылок и SHA-256 пакета
├── build_release.bat               <-- Сборка релиза + упаковка .zip + обновление SHA-256
├── install_steamos.sh              <-- Автоустановщик для SteamOS (Steam Deck) / Linux
├── cec_studio_config.default.json  <-- Базовый шаблон конфигурации (universal_hdmi_cec)
│
├── bin/                            <-- Внешние бинарные ресурсы
│   ├── AudioTool.exe               <-- Утилита переключения аудиовыхода Windows (WASAPI)
│   └── firmware/                   <-- Встроенные прошивки для обновления из UI
│       ├── rp2040_zero_firmware.uf2
│       └── esp32_s3_firmware.bin
│
├── (артефакты сборки, не коммитятся в Git и публикуются в GitHub Releases)
│   ├── HDMI_CEC_Studio.exe             <-- Исполняемый файл для Windows x64 (Tauri v2)
│   ├── HDMI_CEC_Studio_Windows_x64.zip <-- Полный дистрибутив Windows для релиза и самообновления
│   └── WebView2Loader.dll              <-- Загрузчик WebView2 рядом с .exe
```
│
├── tauri_app/                      <-- Исходный код приложения
│   ├── src-tauri/                  <-- Бэкенд на Rust
│   │   ├── src/
│   │   │   ├── hardware/           <-- Модульный аппаратный слой
│   │   │   │   ├── mod.rs
│   │   │   │   ├── state.rs             # Потокобезопасное состояние AppState
│   │   │   │   ├── serial_transport.rs  # Поток чтения/записи USB Serial CDC
│   │   │   │   ├── remote_dispatcher.rs # Маршрутизация CEC opcodes, OSK, мышь
│   │   │   │   ├── mouse_engine.rs      # Ускорение курсора и таймаут мыши
│   │   │   │   └── watchdog.rs          # USB авто-восстановление и Keep-Alive
│   │   │   ├── platform/           <-- Абстракция ОС (windows.rs / linux.rs / generic.rs)
│   │   │   ├── audio.rs            # Переключение аудиоустройств и дисплея
│   │   │   ├── cec_protocol.rs     # Справочник кнопок и кодов CEC
│   │   │   ├── commands.rs         # Tauri IPC команды
│   │   │   ├── config.rs           # Пути данных (AppData/XDG/Portable), миграция
│   │   │   ├── custom_scripts.rs   # Менеджер пользовательских сценариев scripts/
│   │   │   ├── edid.rs             # Разбор EDID видеовыходов
│   │   │   ├── firmware_flasher.rs # Прошивка RP2040 (UF2) и ESP32-S3 (Serial OTA)
│   │   │   ├── input_simulator.rs  # Кроссплатформенная эмуляция клавиатуры и мыши
│   │   │   ├── logger.rs           # Логи с ротацией, санитизацией и детектором сбоя
│   │   │   ├── osk.rs              # Управление окном экранной клавиатуры
│   │   │   ├── telemetry.rs        # Сборщик отчёта (.zip) и opt-in webhook (Zero-PII)
│   │   │   ├── types.rs            # DTO и структура AppConfig
│   │   │   ├── updater.rs          # Синхронизация профилей ТВ и 1-click self-updater
│   │   │   ├── window_manager.rs   # Управление главным окном и HUD
│   │   │   └── main.rs             # Точка входа, системный трей, жизненный цикл
│   │   └── tauri.conf.json         # Конфигурация Tauri v2 (CSP 'self')
│   └── ui/                         <-- Модульный графический интерфейс (Vanilla ES6)
│       ├── index.html              # Главное окно управления
│       ├── hud.html / hud.css / hud.js # Прозрачный OSD/HUD оверлей
│       ├── osk.html / osk.css / osk.js # Экранная клавиатура
│       ├── style.css               # Оформление (Dark Glassmorphism)
│       └── js/                     # constants.js, i18n.js, catalog.js,
│                                   # hardware_status.js, remote_builder.js,
│                                   # calibration.js, key_mapper.js, ambient_fluid.js, app.js
│
├── firmware/                       <-- Прошивка микроконтроллера (PlatformIO C++)
│   ├── platformio.ini              # Профили esp32-s3-devkitc-1 и rp2040-zero
│   └── src/main.cpp                # Код прошивки CEC-контроллера
│
├── tv_profiles/                    <-- База профилей телевизоров (12 JSON файлов + catalog.json)
├── scripts/                        <-- Каталог пользовательских скриптов и scripts/README.md
└── docs/                           <-- Документация и спецификации
│   ├── DEVELOPMENT_HISTORY.md      # История разработки и аппаратный анализ
│   ├── BETA_TESTING.md             # Чеклист бета-тестера и правила отправки отчётов
│   ├── HARDWARE_AND_FLASHING.md    # Схемы пайки RP2040-Zero / ESP32-S3 и прошивка
│   ├── TELEMETRY_AND_REPORTS.md    # Контракт диагностического .zip, маскирование и privacy
│   └── UNIVERSAL_API_SPEC.md       # Спецификация протокола CBP v1.0
```

---

## Быстрый старт

### 1. Подключение к телевизору и ПК
- **RP2040-Zero**:
  - `Контакт 13 HDMI (CEC)` -> **GP0**
  - `Контакт 17 HDMI (GND)` -> **GND**
  - *(Опционально)* `GP1` -> оптопара кнопки Power материнской платы, `GP2` -> оптопара Reset, `GP3` -> вход контроля `PWR_LED+`
- **ESP32-S3**:
  - `Контакт 13 HDMI (CEC)` -> **GPIO4**
  - `Контакт 17 HDMI (GND)` -> **GND**
  - *(Опционально)* `GPIO5` -> оптопара кнопки Power, `GPIO6` -> Reset, `GPIO7` -> `PWR_LED+`
- Подробные схемы, распиновка, нюанс с дорожкой пина 13 у видеокарт и прошивка: см. [`docs/HARDWARE_AND_FLASHING.md`](docs/HARDWARE_AND_FLASHING.md).

### 2. Установка и запуск на Windows 10 / 11 (x64)
1. Скачайте архив **`HDMI_CEC_Studio_Windows_x64.zip`** со страницы [Releases](https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases).
2. Распакуйте архив целиком в **папку с правами записи текущего пользователя** — например, `C:\Tools\HDMI_CEC_Studio\` или `Documents\HDMI_CEC_Studio\`.
   > Важно: не распаковывайте только один `.exe` без `WebView2Loader.dll`, `tv_profiles/` и `bin/`. Также не рекомендуется помещать программу в системную папку `C:\Program Files`, чтобы встроенное автообновление в 1 клик могло заменять файлы без запроса прав администратора.
3. Запустите `HDMI_CEC_Studio.exe`.
   - Если появится предупреждение **Windows Defender SmartScreen** («Система Windows защитила ваш компьютер»): так как бета-сборки ещё не подписаны коммерческим EV-сертификатом, нажмите **«Подробнее» -> «Выполнить в любом случае»**. Программа не требует доступа администратора; обновления с GitHub проверяются по SHA-256, целостности ZIP CRC-32 и заголовку PE.
4. *(По желанию)* Для полностью портативного режима (чтобы конфигурация, скрипты и логи хранились рядом с `.exe`, а не в `%LOCALAPPDATA%\HDMI-CEC Studio\`) создайте пустой файл `portable.txt` рядом с `HDMI_CEC_Studio.exe` или запускайте с флагом `--portable`.

### 3. Установка на SteamOS (Steam Deck) / Linux
Выполните одну команду в терминале (Konsole в Desktop Mode или по SSH):
```bash
curl -fsSL https://raw.githubusercontent.com/mitrafun-creator/HDMI_CEC_Emulator/main/install_steamos.sh | bash
```
Или через клонирование репозитория:
```bash
git clone https://github.com/mitrafun-creator/HDMI_CEC_Emulator.git && cd HDMI_CEC_Emulator && chmod +x install_steamos.sh && ./install_steamos.sh
```
Установщик скачает готовый бинарник (или соберёт проект из исходников через Rust/Cargo), настроит правила `udev` (USB Serial и виртуальный ввод `uinput` для Gamescope), создаст ярлык и запустит фоновый пользовательский сервис `systemd` (`hdmi-cec-studio.service`).

### 4. Сборка из исходников (Windows)
При наличии настроенного окружения Rust/MinGW запустите из корня репозитория:
```cmd
build_release.bat
```
Скрипт скомпилирует релизный бинарник Rust, обновит `HDMI_CEC_Studio.exe` и `WebView2Loader.dll`, пересоберёт `HDMI_CEC_Studio_Windows_x64.zip` и синхронизирует хэш `windows_sha256` в `version.json`.

---

## Матрица совместимости (бета-статус)

### Платформы и контроллеры

| Компонент | Статус в бета-версии | Примечания |
| --- | --- | --- |
| Windows 10 / 11 (x64) | Основная целевая сборка | Доступна в [`HDMI_CEC_Studio_Windows_x64.zip`](https://github.com/mitrafun-creator/HDMI_CEC_Emulator/releases) (требует WebView2 Runtime, по умолчанию есть в Windows 11/актуальной Windows 10) |
| SteamOS / Linux (x64) | Сборка из исходников | Готовый бинарник в Releases ещё не опубликован; [`install_steamos.sh`](install_steamos.sh) собирает проект локально через `cargo` |
| RP2040-Zero / ESP32-S3 | Прошивки в комплекте | Готовые образы в `bin/firmware/`, схемы и распиновка в [`docs/HARDWARE_AND_FLASHING.md`](docs/HARDWARE_AND_FLASHING.md) |

### Профили телевизоров (`tv_profiles/`)

| Профиль / Бренд | Файл | Статус проверки |
| --- | --- | --- |
| Philips 32PFL9705H (EasyLink) | `philips_32pfl9705h.json` | Проверен на реальном ТВ в ходе разработки |
| Универсальный HDMI-CEC 1.4/2.0 | `universal_hdmi_cec.json` | Базовый шаблон по умолчанию (ожидает широкой проверки в бете) |
| Philips Ambilight (Android / Google TV) | `philips_ambilight_android.json` | Не подтверждён на реальном ТВ (по спецификации CEC, ждёт тестов) |
| LG SimpLink (webOS) | `lg_simplink_webos.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Samsung Anynet+ (Tizen OS) | `samsung_anynet_tizen.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Sony BRAVIA Sync (Google TV) | `sony_bravia_sync.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Panasonic VIERA Link | `panasonic_viera_link.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Hisense VIDAA Smart TV | `hisense_vidaa_cec.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| TCL Mini-LED / QLED (T-Link) | `tcl_google_tv.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Toshiba REGZA (CE-Link) | `toshiba_regza_link.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Sharp AQUOS Link | `sharp_aquos_link.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |
| Xiaomi TV / PatchWall | `xiaomi_mi_tv.json` | Не подтверждён на реальном ТВ (ждёт тестов сообщества) |

---

## Тестирование бета-версии и отчёты об ошибках

Подробный чеклист проверки для участников тестирования находится в [`docs/BETA_TESTING.md`](docs/BETA_TESTING.md).

Если во время тестирования возникла ошибка, интерфейс закрылся некорректно или телевизор/пульт ведёт себя неожиданно:
1. Откройте **Настройки -> Диагностика, отчёты и телеметрия** (или нажмите «Создать отчёт (.zip)» на баннере восстановления после сбоя).
2. Нажмите **«Создать отчёт (.zip)»** — приложение сформирует локальный архив `hdmi_cec_support_<timestamp>.zip` в папке `reports/`.
   - Персональные пути (`C:\Users\<USER>\...`, `/home/<USER>/...`) и токены автоматически заменяются на маски перед сохранением архива (подробнее: [`docs/TELEMETRY_AND_REPORTS.md`](docs/TELEMETRY_AND_REPORTS.md)).
3. Опишите шаги воспроизведения и приложите созданный `.zip` в [GitHub Issues](https://github.com/mitrafun-creator/HDMI_CEC_Emulator/issues) (удобная кнопка «Сообщить об ошибке» открывает форму с уже подставленной сводкой версии, ОС, контроллера и EDID).

*(Опциональная анонимная телеметрия по умолчанию выключена и срабатывает только при явном согласии пользователя и указании собственного HTTPS Webhook-приёмника.)*

---

## Лицензия

- Основной код **HDMI-CEC Studio** (`tauri_app/`, профили `tv_profiles/`, скрипты и документация) распространяется под лицензией **PolyForm Noncommercial License 1.0.0** ([`LICENSE`](LICENSE)): вы можете свободно использовать, изучать, модифицировать и форкать проект для личных, образовательных, исследовательских и иных некоммерческих задач; коммерческое использование, продажа и выпуск коммерческих продуктов на основе проекта или его форков запрещены.
- Прошивка микроконтроллера (`firmware/`) использует библиотеку [`firmware/lib/CEC-master`](firmware/lib/CEC-master) под лицензией **GNU GPL v2.0**. Полный перечень лицензий сторонних компонентов и зависимостей приведён в [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
