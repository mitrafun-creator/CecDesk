/**
 * i18n.js - Модуль мультиязычной локализации интерфейса (Русский / English)
 */

let currentLang = "ru";

const I18N_DICTIONARY = {
  ru: {
    // Окно
    "win.minimize": "Свернуть",
    "win.maximize": "Развернуть",
    "win.hide_to_tray": "Свернуть в трей",
    "window.title_tv": "ТВ",

    // Навигация
    "nav.remote": "Пульт",
    "nav.keys": "Кнопки",
    "nav.system": "Система",
    "nav.settings": "Настройки",

    // Экран Пульт
    "remote.connection_active": "Связь с телевизором активна",
    "remote.syncing": "Синхронизация...",
    "remote.connecting": "Подключение...",
    "remote.turn_on_tv": "Включить ТВ",
    "remote.turn_off_tv": "Выключить ТВ",
    "remote.syncing_tv_state": "Определение состояния...",
    "remote.intercept_title": "Перехват пульта",
    "remote.intercept_sub": "Управление ПК пультом ТВ",
    "remote.intercept_tip": "Управление компьютером при помощи пульта телевизора",
    "remote.keys_count_tip": "Доступно кнопок в конструкторе",
    "remote.calib_tip": "Обучить пульт нажатиями кнопок",
    "remote.status_refresh_tip": "Обновить статус",
    "remote.hw_status_title": "Статус",
    "remote.usb_adapter": "USB-контроллер",
    "remote.cec_bus": "Связь с ТВ (CEC)",
    "remote.bridge": "Перехват пульта",
    "remote.status_connected": "Подключен",
    "remote.status_disconnected": "Отключен",
    "remote.status_online": "3.3V ТВ в сети",
    "remote.status_offline": "Нет сигнала (ТВ выкл)",
    "remote.status_enabled": "Включен",
    "remote.status_disabled": "Выключен",
    "remote.title": "Пульт ТВ",
    "remote.btn_calib": "Калибровка",
    "remote.btn_reset": "Сброс к профилю",
    "remote.keys_unit": "кнопок",
    "remote.offline_modal_title": "Нет связи с телевизором",
    "remote.offline_modal_desc": "Модуль не обнаружил ответ от телевизора по шине HDMI-CEC или связь с контроллером потеряна. Проверьте кабель HDMI и питание телевизора.",
    "remote.offline_reason_init": "Ожидание ответа от модуля...",
    "remote.offline_reason_reconnecting": "Поиск и переподключение к USB-модулю...",
    "remote.offline_reason_no_usb": "Нет связи с ПК: модуль отключен или порт не отвечает",
    "remote.offline_reason_line_low": "Линия HDMI-CEC 0V: проверьте кабель или питание ТВ",
    "remote.offline_reason_tv_no_resp": "Модуль подключен, но ТВ или пульт не отвечает",
    "remote.offline_reason_checking": "Опрос модуля и проверка связи с телевизором...",
    "remote.searching_ctrl": "Поиск контроллера...",
    "remote.searching_desc": "Попытка переподключения к адаптеру на порту",
    "remote.searching_port": "Поиск COM-порта...",
    "remote.searching_btn": "Поиск...",
    "remote.usb_off_title": "USB-контроллер отключен",
    "remote.usb_off_desc": "Связь с адаптером разорвана. Нажмите «Подключить», чтобы открыть COM-порт и возобновить работу.",
    "remote.port_disconnected": "отключен",
    "remote.no_ctrl_reason": "Контроллер не подключен",
    "remote.btn_connect": "Подключить",
    "remote.btn_connecting": "Подключение...",
    "remote.btn_poll_bus": "Опросить шину",
    "remote.btn_polling": "Опрос шины...",
    "remote.line_err_title": "Ошибка линии CEC (0V)",
    "remote.line_err_desc": "Контроллер подключен, но на шине CEC зафиксировано 0V (короткое замыкание или кабель). Проверьте кабель HDMI и питание ТВ.",
    "remote.no_tv_port_reason": "Подключен, но ТВ не отвечает",
    "remote.pc_action_prefix": "Действие ПК:",
    "remote.pc_action_none": "не назначено",
    "remote.recognized_prefix": "Распознано:",
    "remote.button_prefix": "Кнопка",

    // Экран Кнопки
    "keys.title": "Кнопки пульта",
    "keys.scripts_folder": "Папка сценариев",
    "keys.scripts_folder_tip": "Открыть папку scripts со своими .bat/.ps1 скриптами",
    "keys.refresh_scripts_tip": "Пересканировать папку сценариев",
    "keys.save": "Сохранить",
    "keys.save_name_tip": "Сохранить новое название профиля",
    "keys.preset_hud_title": "Пресет кнопок",
    "keys.preset_base": "Универсальный",
    "keys.preset_cinema": "Кино",
    "keys.preset_steam": "Игры",
    "keys.preset_surf": "Браузер",
    "keys.preset_numpad": "Цифры",
    "keys.col_tv_btn": "Кнопка ТВ",
    "keys.col_pc_act": "Действие на ПК",
    "keys.test": "Тест",
    "keys.badge_on_remote": "На пульте",
    "keys.badge_not_detected": "Не обнаружена",
    "keys.profile_title": "Профиль телевизора",
    "keys.current_profile": "Текущий профиль ТВ",
    "keys.profile_name": "Название профиля",
    "keys.profile_details": "Параметры профиля",
    "keys.brand": "Бренд:",
    "keys.protocol": "Протокол CEC:",
    "keys.supported_keys_count": "Клавиш в базе:",
    "keys.default_pa": "Адрес по умолчанию:",
    "keys.mapping_hint": "Назначения кнопок автоматически связываются с выбранным профилем телевизора. Синий индикатор в таблице указывает на клавиши, которые присутствуют на вашем пульте.",

    // Клавиши пульта
    "key.0x00": "Подтверждение",
    "key.0x01": "Вверх",
    "key.0x02": "Вниз",
    "key.0x03": "Влево",
    "key.0x04": "Вправо",
    "key.0x0D": "Возврат",
    "key.0x09": "Меню",
    "key.0x0A": "Опции",
    "key.0x0B": "Опции",
    "key.0x0C": "Избранное",
    "key.0x35": "Инфо",
    "key.0x53": "Гид (EPG)",
    "key.0x72": "Красная",
    "key.0x73": "Зеленая",
    "key.0x74": "Желтая",
    "key.0x71": "Синяя",
    "key.0x41": "Громкость +",
    "key.0x42": "Громкость −",
    "key.0x43": "Без звука",
    "key.0x44": "Воспроизведение",
    "key.0x46": "Пауза",
    "key.0x45": "Стоп",
    "key.0x4B": "Следующий",
    "key.0x4C": "Предыдущий",
    "key.0x48": "Перемотка назад",
    "key.0x49": "Перемотка вперед",
    "key.0x30": "Канал +",
    "key.0x31": "Канал −",
    "key.0x21": "Цифра 1",
    "key.0x22": "Цифра 2",
    "key.0x23": "Цифра 3",
    "key.0x24": "Цифра 4",
    "key.0x25": "Цифра 5",
    "key.0x26": "Цифра 6",
    "key.0x27": "Цифра 7",
    "key.0x28": "Цифра 8",
    "key.0x29": "Цифра 9",
    "key.0x20": "Цифра 0",

    // Экран Система
    "sys.device": "Устройство:",
    "sys.refresh": "Обновить",
    "sys.connect": "Подключить",
    "sys.disconnect": "Отключить",
    "sys.profile_tv": "Профиль ТВ:",
    "sys.diag_title": "Подробная диагностика оборудования и шины CEC",
    "sys.check_bus_btn": "Запросить статус шины",
    "sys.card_usb": "USB-контроллер",
    "sys.lbl_port": "Порт:",
    "sys.lbl_state": "Состояние:",
    "sys.lbl_firmware": "Контроллер:",
    "sys.card_bus": "Физическая шина CEC",
    "sys.lbl_line_level": "Уровень линии:",
    "sys.lbl_pullup": "Подтяжка Pull-Up:",
    "sys.lbl_contact": "Состояние контакта:",
    "sys.card_tv_prefix": "Телевизор",
    "sys.lbl_tv_pwr": "Питание матрицы:",
    "sys.lbl_phys_addr": "Физический адрес:",
    "sys.lbl_log_addr": "Логический адрес:",
    "sys.card_remote": "Служба перехвата пульта",
    "sys.lbl_intercept": "Перехват команд:",
    "sys.lbl_keys_cnt": "Обучено кнопок:",
    "sys.lbl_input_sys": "Подсистема ввода:",
    "sys.line_test": "Тест линии",
    "sys.tv_status": "Статус ТВ",
    "sys.check_updates": "Проверить обновления",
    "sys.tv_models": "Модели ТВ",
    "sys.search_ph": "Поиск...",
    "sys.sync_db": "Обновить базу",
    "sys.sync_db_tip": "Синхронизировать онлайн базу профилей ТВ",
    "sys.json_config": "Конфигурация JSON",
    "sys.bus_queries": "Запросы шины",
    "sys.q_tv_addr": "Адрес ТВ",
    "sys.q_tv_brand": "Бренд ТВ",
    "sys.q_cec_ver": "Версия CEC",
    "sys.q_active_in": "Активный вход",
    "sys.q_tv_name": "Имя ТВ",
    "sys.q_pc_announce": "Анонс ПК",
    "sys.command_label": "Команда",
    "sys.send": "Отпр.",
    "sys.event_log": "Журнал событий",
    "sys.copy": "Копировать",
    "sys.logs_folder": "Папка логов",
    "sys.clear": "Очистить",

    // Диагностические значения
    "diag.val_connected_baud": "Активен (115200 bps)",
    "diag.val_disconnected": "Отключен",
    "diag.val_level_high": "3.3V (Норма)",
    "diag.val_level_low": "0V (КЗ / Обрыв)",
    "diag.val_pullup_tv": "Активна (ТВ 27k)",
    "diag.val_pullup_mcu": "Внутренняя MCU",
    "diag.val_contact_ok": "Надежный контакт",
    "diag.val_contact_wait": "Нет ответа ТВ",
    "diag.val_tv_on": "Включен (ON)",
    "diag.val_tv_standby": "Сон (Standby)",
    "diag.val_intercept_on": "Активен (ПК)",
    "diag.val_intercept_off": "Отключен (ТВ)",
    "diag.val_input_subsys": "Win SendInput (Enigo)",

    // Экран Настройки
    "settings.port_params_title": "Параметры порта HDMI",
    "settings.pc_name": "Имя компьютера на ТВ (OSD)",
    "settings.hdmi_num": "Номер разъема HDMI",
    "settings.apply": "Применить",
    "settings.qol_title": "Умные функции (QoL)",
    "settings.autostart_title": "Автозапуск системы",
    "settings.autostart_sub": "Фоновый старт сразу в трей без всплывающих окон",
    "settings.autostart_desc": "Автоматический запуск службы HDMI-CEC в фоновом режиме при старте Windows без отображения главного окна (сворачивание в трей).",
    "settings.hud_title": "Экранные уведомления (HUD)",
    "settings.hud_sub": "Всплывающие плашки мыши, громкости и действий",
    "settings.hud_desc": "Элегантные полупрозрачные плашки поверх окон при изменении громкости, переключении режима мыши, навигации и выполнении макросов.",
    "settings.audio_title": "Маршрутизация звука ПК / ТВ",
    "settings.audio_sub": "ТВ при включении ⇄ Наушники при сне",
    "settings.audio_desc": "Автоматический вывод звука на динамики телевизора при его включении и плавный возврат на наушники или колонки ПК при переходе ТВ в спящий режим.",
    "settings.soundbar_title": "Сквозной звук eARC",
    "settings.soundbar_sub": "Не менять громкость микшера Windows",
    "settings.soundbar_desc": "Регулировка уровня громкости напрямую на телевизоре или саундбаре по шине CEC без изменения системного микшера Windows.",
    "settings.seek_title": "Адаптивная перемотка медиа",
    "settings.seek_sub": "Клик: 5 сек • Удержание: 1 мин / 5 мин",
    "settings.seek_desc": "Одиночное нажатие стрелок мотает видео на 5 секунд, а непрерывное удержание ускоряет шаг до 1 и 5 минут.",
    "settings.kill_title": "Аварийное закрытие зависших игр",
    "settings.kill_sub": "Удержание кнопки «Назад» 4 секунды",
    "settings.kill_desc": "Удержание кнопки «Назад» на пульте в течение 4 секунд принудительно завершает зависший полноэкранный процесс без вызова диспетчера задач.",
    "settings.polite_title": "Фоновое присутствие (CEC)",
    "settings.polite_sub": "Без самовольного захвата HDMI входа",
    "settings.polite_desc": "Компьютер объявляет себя на шине CEC для работы пульта, но не перехватывает HDMI-вход при включении или фоновых событиях.",
    "settings.autoheal_title": "Сторожевой таймер связи (Watchdog)",
    "settings.autoheal_sub": "Мгновенный реконнект при обрыве провода",
    "settings.autoheal_desc": "Непрерывный мониторинг COM-порта микроконтроллера и мгновенное автоматическое переподключение при отсоединении провода.",
    "settings.disp_sleep_title": "Отключение видеовыхода при сне ТВ",
    "settings.disp_sleep_sub": "Курсор мыши не теряется на темном экране",
    "settings.disp_sleep_desc": "Программное отключение второго дисплея в Windows при выключении ТВ, чтобы курсор и окна не терялись на отключенном экране.",
    "settings.lang_title": "Язык интерфейса / Language",
    "settings.about_title": "О программе",
    "settings.os_badge": "ОС",
    "settings.os_version_title": "Windows 11 / 10 x64",
    "settings.xml_title": "Профили XML",
    "settings.xml_box_tip": "Обмен конфигурациями кнопок пульта в формате XML",
    "settings.xml_export": "Экспорт в XML",
    "settings.xml_export_tip": "Сохранить текущий профиль в файл .xml",
    "settings.xml_import": "Импорт из XML",
    "settings.xml_import_tip": "Загрузить назначения кнопок из файла .xml",
    "settings.app_name": "CecDesk",
    "settings.app_desc": "Аппаратный мост CecDesk Dongle (HDMI-CEC)",
    "settings.version": "Версия",
    "settings.check_update_btn": "Проверить обновления",
    "settings.up_to_date": "Установлена последняя версия",

    // Калибровка
    "calib.title": "Калибровка пульта ТВ",
    "calib.instruction": "Нажимайте по очереди все нужные кнопки на вашем реальном пульте телевизора (кроме кнопки выключения ТВ). Каждая нажатая кнопка мгновенно добавится в конструктор.",
    "calib.detected_count": "Распознано кнопок:",
    "calib.waiting": "Ожидание нажатия...",
    "calib.power_skipped": "Кнопка питания ТВ пропущена для безопасности",
    "calib.pressed_prefix": "Нажата:",
    "calib.build_btn": "Собрать пульт",
    "calib.cancel_btn": "Отмена",
    "calib.reset_btn": "Сбросить к базовому",

    // Статусы оборудования
    "status.usb_connected_port": "Подключен",
    "status.usb_disconnected": "USB: Отключен",
    "status.usb_disconnected_desc": "Подключите адаптер к компьютеру",
    "status.ready_online": "Готово • ТВ в сети",
    "status.ready_online_desc": "Связь с телевизором активна",
    "status.tv_no_response": "ТВ не отвечает",
    "status.tv_no_response_desc": "Проверьте HDMI кабель или питание ТВ",
    "status.line_error": "Линия CEC 0V",
    "status.line_error_desc": "Короткое замыкание или ТВ выключен из розетки",
    "status.line_err_badge": "0V / Ошибка",
    "status.line_free_badge": "Шина свободна",
    "status.line_no_power": "Нет питания",
    "status.bus_polling": "Опрос шины...",
    "status.bus_polling_short": "Опрос...",
    "status.bus_no_usb": "Нет питания USB",
    "status.tv_online_contact": "3.3V ТВ в сети (Контакт надежен)",
    "status.bus_free_no_tv": "Шина свободна (ТВ не отвечает)",
    "status.bus_error_0v": "0V / Ошибка линии CEC (КЗ или кабель)",
    "status.remote_on_tip": "Включен (управление ПК)",
    "status.remote_off_tip": "Выключен (пульт освобожден для ТВ)",
    "status.measuring_line": "Замер линии...",
    "status.mouse_on": "Мышь ВКЛ",
    "status.mouse_on_desc": "Управление курсором (Стрелки / ЛКМ / ПКМ)",
    "status.mouse_off": "Готово",
    "status.mouse_off_desc": "Обычный режим пульта",
    "status.reconnecting": "Поиск...",

    // Настройки и общие элементы
    "settings.saved_toast": "Настройки сохранены",
    "settings.auto_saved": "Сохранено",
    "settings.port_prefix": "Разъем",
    "settings.checking_updates": "Проверка обновлений...",
    "settings.update_available": "Доступна новая версия",
    "settings.update_error": "Ошибка проверки",
    "settings.open_releases": "Скачать обновление",
    "settings.install_update_btn": "Обновить до",
    "settings.installing_update": "Загрузка и проверка обновления...",
    "settings.open_releases_alt": "Страница релиза",
    "settings.update_restarting": "Перезапуск в новую версию...",

    // Действия кнопок (Key Mapper)
    "action.steam_big_picture": "Steam Big Picture (Открыть / Закрыть)",
    "action.steam_exit_to_desktop": "Steam: Выход из Big Picture на рабочий стол",
    "action.steam_keyboard": "SteamOS: Нативная клавиатура Steam",
    "action.show_keyboard": "Экранная клавиатура (Авто Steam / OSK)",
    "action.mouse_toggle": "Режим мыши (Вкл / Выкл)",
    "action.mouse_lclick": "ЛКМ (Клик мыши)",
    "action.mouse_rclick": "ПКМ (Правый клик)",
    "action.enter": "OK / Enter",
    "action.up": "Вверх",
    "action.down": "Вниз",
    "action.left": "Влево",
    "action.right": "Вправо",
    "action.backspace": "Стереть / Backspace",
    "action.browser_back": "Назад в браузере (Alt+Влево)",
    "action.esc": "Отмена / Esc",
    "action.space": "Пауза (Пробел)",
    "action.media_play": "Play / Pause",
    "action.media_stop": "Стоп",
    "action.media_next": "След. трек",
    "action.media_prev": "Пред. трек",
    "action.vol_up": "Громкость +",
    "action.vol_down": "Громкость −",
    "action.vol_mute": "Mute (Без звука)",
    "action.f11": "Полный экран (F11)",
    "action.pgup": "Прокрутка вверх",
    "action.pgdn": "Прокрутка вниз",
    "action.tab": "Переключить окно (Tab)",
    "action.win_d": "Рабочий стол (Win+D)",
    "action.alt_tab": "Переключение задач (Alt+Tab)",
    "action.pc_sleep": "Режим сна ПК (Sleep)",
    "action.pc_hibernate": "Гибернация ПК (Hibernate)",
    "action.pc_shutdown": "Выключение ПК (Shutdown)",
    "action.opto_power": "Оптопара: Кнопка Power (350 мс)",
    "action.num_1": "Ввод: 1",
    "action.num_2": "Ввод: 2",
    "action.num_3": "Ввод: 3",
    "action.num_4": "Ввод: 4",
    "action.num_5": "Ввод: 5",
    "action.num_6": "Ввод: 6",
    "action.num_7": "Ввод: 7",
    "action.num_8": "Ввод: 8",
    "action.num_9": "Ввод: 9",
    "action.num_0": "Ввод: 0",
    "action.none": "Отключено",
    "action.script_prefix": "Сценарий",

    // Управление питанием ПК и ТВ (Оптопара)
    "settings.power_sync_title": "Питание ПК и ТВ (Оптопара)",
    "settings.wake_pc_on_tv_title": "Включать ПК при включении ТВ (Оптопара)",
    "settings.wake_pc_on_tv_desc": "Микроконтроллер автономно замыкает оптопару кнопки Power материнской платы (GP1 на RP2040 / GPIO5 на ESP32) на 350 мс при включении телевизора или выборе HDMI-входа ПК.",
    "settings.wake_tv_on_pc_title": "Включать ТВ при включении ПК",
    "settings.wake_tv_on_pc_desc": "Автоматически отправлять сигнал включения экрана (Image View On) и активации входа при старте программы на ПК.",
    "settings.tv_off_on_pc_title": "Выключать ТВ при выключении / сне ПК",
    "settings.tv_off_on_pc_desc": "Микроконтроллер автоматически отправляет телевизору команду Standby (0x36), когда ПК выключается или уходит в спящий режим.",
    "settings.pc_off_on_tv_title": "Выключать / усыплять ПК при выключении ТВ",
    "settings.pc_off_on_tv_desc": "Автоматически переводить компьютер в режим сна или выключать его, когда телевизор переходит в спящий режим.",
    "settings.pc_off_mode_label": "Режим при выключении ТВ:",
    "settings.pc_mode_sleep": "Режим сна (Sleep S3 — без выключения)",
    "settings.pc_mode_shutdown": "Полное выключение ОС (Shutdown)",
    "settings.pc_mode_hibernate": "Гибернация (Hibernate)",
    "settings.pc_mode_opto": "Импульс оптопары (Кнопка Power 350 мс)",
    "settings.test_opto_btn": "Тест оптопары (350 мс)",
    "settings.test_sleep_btn": "Сон ПК",
    "sys.q_opto_pulse": "Оптопара Power",
    "sys.q_pc_sleep": "Сон ПК",
    "settings.fw_box_title": "Прошивка контроллера",
    "settings.fw_box_tip": "Автономное обновление микроконтроллера напрямую (UF2 / Serial OTA)",
    "settings.fw_flash_bundled_btn": "Прошить (v1.1.0)",
    "settings.fw_flash_custom_btn": "Из файла (.uf2 / .bin)",
    "settings.fw_custom_tip": "Выбрать файл прошивки (.uf2 / .bin)",
    "settings.fw_preparing": "Подготовка...",
    "settings.audio_tv_device_label": "Устройство вывода ТВ (при включении):",
    "settings.audio_fallback_device_label": "Устройство ПК / Наушники (при сне ТВ):",
    "settings.audio_device_none": "— Не выбрано —",

    // Сбой и диагностика
    "crash.banner_default": "Предыдущий запуск завершился некорректно. Вы можете собрать диагностический отчёт для отправки разработчику.",
    "crash.banner_panic": "Обнаружен отчёт о прошлом аварийном завершении. Рекомендуется собрать .zip отчёт об ошибке.",
    "crash.banner_dirty": "Прошлый запуск завершился некорректно (возможен сбой). Вы можете сохранить диагностический отчёт.",
    "crash.create_report": "Создать отчёт (.zip)",
    "crash.dismiss": "Скрыть",
    "diag.panel_title": "Диагностика, отчёты и телеметрия",
    "diag.telemetry_title": "Анонимная телеметрия запуска",
    "diag.telemetry_desc": "Отключена по умолчанию. Передает только случайный install_id, версию ПО, ОС, модель адаптера, EDID и счетчики ошибок на указанный вами HTTPS webhook. Не содержит имен, путей и личных данных.",
    "diag.storage_mode_label": "Хранилище данных:",
    "diag.counters_label": "Ошибки / Предупреждения сессии:",
    "diag.endpoint_label": "HTTPS Webhook для отчётов / телеметрии (опционально):",
    "diag.btn_bundle": "Создать отчёт (.zip)",
    "diag.btn_reports_folder": "Папка отчётов",
    "diag.btn_github_issue": "Сообщить об ошибке",
    "diag.status_enabled": "Анонимная телеметрия включена.",
    "diag.status_disabled": "Телеметрия отключена.",
    "bug.modal_title": "Диагностический отчёт",
    "bug.modal_desc": "В архив включаются конфигурация, информация о системе/EDID, контроллере и очищенные от персональных путей журналы.",
    "bug.note_label": "Описание проблемы (необязательно):",
    "bug.note_ph": "Опишите, что произошло и как воспроизвести проблему...",
    "bug.btn_create_zip": "Собрать .zip",
    "bug.btn_send_webhook": "Отправить на Webhook",
    "bug.btn_github_issue": "GitHub Issue",
    "bug.btn_close": "Закрыть"
  },
  en: {
    // Window
    "win.minimize": "Minimize",
    "win.maximize": "Maximize",
    "win.hide_to_tray": "Minimize to Tray",
    "window.title_tv": "TV",

    // Navigation
    "nav.remote": "Remote",
    "nav.keys": "Buttons",
    "nav.system": "System",
    "nav.settings": "Settings",

    // Remote Screen
    "remote.connection_active": "TV connection active",
    "remote.syncing": "Syncing...",
    "remote.connecting": "Connecting...",
    "remote.turn_on_tv": "Turn TV On",
    "remote.turn_off_tv": "Turn TV Off",
    "remote.syncing_tv_state": "Detecting state...",
    "remote.intercept_title": "Remote Intercept",
    "remote.intercept_sub": "Control PC via TV Remote",
    "remote.intercept_tip": "Control your computer using the TV remote",
    "remote.keys_count_tip": "Available buttons in remote builder",
    "remote.calib_tip": "Learn remote layout by pressing buttons",
    "remote.status_refresh_tip": "Refresh status",
    "remote.hw_status_title": "Status",
    "remote.usb_adapter": "USB Controller",
    "remote.cec_bus": "TV Link (CEC)",
    "remote.bridge": "Remote Interception",
    "remote.status_connected": "Connected",
    "remote.status_disconnected": "Disconnected",
    "remote.status_online": "3.3V TV Online",
    "remote.status_offline": "No signal (TV off)",
    "remote.status_enabled": "Enabled",
    "remote.status_disabled": "Disabled",
    "remote.title": "TV Remote",
    "remote.btn_calib": "Calibration",
    "remote.btn_reset": "Reset to Profile",
    "remote.keys_unit": "keys",
    "remote.offline_modal_title": "No Connection to TV",
    "remote.offline_modal_desc": "The module did not detect a response from the TV on the HDMI-CEC bus or controller connection was lost. Check the HDMI cable and TV power.",
    "remote.offline_reason_init": "Waiting for module response...",
    "remote.offline_reason_reconnecting": "Searching and reconnecting to USB module...",
    "remote.offline_reason_no_usb": "No PC link: module disconnected or port not responding",
    "remote.offline_reason_line_low": "HDMI-CEC Line 0V: check cable or TV power",
    "remote.offline_reason_tv_no_resp": "Module connected, but TV or remote is not responding",
    "remote.offline_reason_checking": "Polling module and checking TV connection...",
    "remote.searching_ctrl": "Searching controller...",
    "remote.searching_desc": "Attempting to reconnect to adapter on port",
    "remote.searching_port": "Scanning COM ports...",
    "remote.searching_btn": "Searching...",
    "remote.usb_off_title": "USB Controller Disconnected",
    "remote.usb_off_desc": "Adapter link is closed. Click Connect to open the COM port and resume control.",
    "remote.port_disconnected": "disconnected",
    "remote.no_ctrl_reason": "Controller not connected",
    "remote.btn_connect": "Connect",
    "remote.btn_connecting": "Connecting...",
    "remote.btn_poll_bus": "Poll CEC Bus",
    "remote.btn_polling": "Polling bus...",
    "remote.line_err_title": "CEC Line Error (0V)",
    "remote.line_err_desc": "Controller connected, but 0V detected on CEC line (short circuit or cable issue). Check the HDMI cable and TV power.",
    "remote.no_tv_port_reason": "Connected, no response from TV",
    "remote.pc_action_prefix": "PC Action:",
    "remote.pc_action_none": "unassigned",
    "remote.recognized_prefix": "Detected:",
    "remote.button_prefix": "Button",

    // Buttons Screen
    "keys.title": "Remote Buttons",
    "keys.scripts_folder": "Scripts Folder",
    "keys.scripts_folder_tip": "Open scripts folder with custom .bat/.ps1 files",
    "keys.refresh_scripts_tip": "Rescan scripts folder",
    "keys.save": "Save",
    "keys.save_name_tip": "Save new profile name",
    "keys.preset_hud_title": "Button Preset",
    "keys.preset_base": "Universal",
    "keys.preset_cinema": "Cinema",
    "keys.preset_steam": "Games",
    "keys.preset_surf": "Browser",
    "keys.preset_numpad": "Numpad",
    "keys.col_tv_btn": "TV Button",
    "keys.col_pc_act": "Action on PC",
    "keys.test": "Test",
    "keys.badge_on_remote": "On Remote",
    "keys.badge_not_detected": "Not Detected",
    "keys.profile_title": "TV Profile",
    "keys.current_profile": "Current TV Profile",
    "keys.profile_name": "Profile Name",
    "keys.profile_details": "Profile Parameters",
    "keys.brand": "Brand:",
    "keys.protocol": "CEC Protocol:",
    "keys.supported_keys_count": "Keys in Database:",
    "keys.default_pa": "Default Address:",
    "keys.mapping_hint": "Button mappings are automatically linked to the selected TV profile. The blue indicator marks keys detected on your remote.",

    // Remote Keys
    "key.0x00": "Confirm (OK)",
    "key.0x01": "Up",
    "key.0x02": "Down",
    "key.0x03": "Left",
    "key.0x04": "Right",
    "key.0x0D": "Return",
    "key.0x09": "Menu",
    "key.0x0A": "Options",
    "key.0x0B": "Options",
    "key.0x0C": "Favorites",
    "key.0x35": "Info",
    "key.0x53": "Guide (EPG)",
    "key.0x72": "Red",
    "key.0x73": "Green",
    "key.0x74": "Yellow",
    "key.0x71": "Blue",
    "key.0x41": "Volume +",
    "key.0x42": "Volume −",
    "key.0x43": "Mute",
    "key.0x44": "Play",
    "key.0x46": "Pause",
    "key.0x45": "Stop",
    "key.0x4B": "Next",
    "key.0x4C": "Previous",
    "key.0x48": "Rewind",
    "key.0x49": "Fast Forward",
    "key.0x30": "Channel +",
    "key.0x31": "Channel −",
    "key.0x21": "Digit 1",
    "key.0x22": "Digit 2",
    "key.0x23": "Digit 3",
    "key.0x24": "Digit 4",
    "key.0x25": "Digit 5",
    "key.0x26": "Digit 6",
    "key.0x27": "Digit 7",
    "key.0x28": "Digit 8",
    "key.0x29": "Digit 9",
    "key.0x20": "Digit 0",

    // System Screen
    "sys.device": "Device:",
    "sys.refresh": "Refresh",
    "sys.connect": "Connect",
    "sys.disconnect": "Disconnect",
    "sys.profile_tv": "TV Profile:",
    "sys.diag_title": "Detailed Hardware & CEC Bus Diagnostics",
    "sys.check_bus_btn": "Poll Bus Status",
    "sys.card_usb": "USB Controller",
    "sys.lbl_port": "Port:",
    "sys.lbl_state": "State:",
    "sys.lbl_firmware": "Controller:",
    "sys.card_bus": "Physical CEC Bus",
    "sys.lbl_line_level": "Line Level:",
    "sys.lbl_pullup": "Pull-Up Resistor:",
    "sys.lbl_contact": "Bus Contact:",
    "sys.card_tv_prefix": "TV",
    "sys.lbl_tv_pwr": "Panel Power:",
    "sys.lbl_phys_addr": "Physical Address:",
    "sys.lbl_log_addr": "Logical Address:",
    "sys.card_remote": "Remote Intercept Service",
    "sys.lbl_intercept": "Key Capture:",
    "sys.lbl_keys_cnt": "Trained Keys:",
    "sys.lbl_input_sys": "Input Backend:",
    "sys.line_test": "Line Test",
    "sys.tv_status": "TV Status",
    "sys.check_updates": "Check Updates",
    "sys.tv_models": "TV Models",
    "sys.search_ph": "Search...",
    "sys.sync_db": "Sync Online DB",
    "sys.sync_db_tip": "Synchronize online TV profile catalog",
    "sys.json_config": "JSON Configuration",
    "sys.bus_queries": "Bus Commands",
    "sys.q_tv_addr": "TV Address",
    "sys.q_tv_brand": "TV Brand",
    "sys.q_cec_ver": "CEC Version",
    "sys.q_active_in": "Active Input",
    "sys.q_tv_name": "TV Name",
    "sys.q_pc_announce": "Announce PC",
    "sys.command_label": "Command",
    "sys.send": "Send",
    "sys.event_log": "Event Log",
    "sys.copy": "Copy",
    "sys.logs_folder": "Logs Folder",
    "sys.clear": "Clear",

    // Diagnostics values
    "diag.val_connected_baud": "Active (115200 bps)",
    "diag.val_disconnected": "Disconnected",
    "diag.val_level_high": "3.3V (Normal)",
    "diag.val_level_low": "0V (Short / Open)",
    "diag.val_pullup_tv": "Active (TV 27k)",
    "diag.val_pullup_mcu": "Internal MCU",
    "diag.val_contact_ok": "Reliable link",
    "diag.val_contact_wait": "No TV response",
    "diag.val_tv_on": "Powered On (ON)",
    "diag.val_tv_standby": "Standby",
    "diag.val_intercept_on": "Active (PC)",
    "diag.val_intercept_off": "Disabled (TV)",
    "diag.val_input_subsys": "Win SendInput (Enigo)",

    // Settings Screen
    "settings.port_params_title": "HDMI Port Parameters",
    "settings.pc_name": "Computer Name on TV (OSD)",
    "settings.hdmi_num": "HDMI Port Number",
    "settings.apply": "Apply",
    "settings.qol_title": "Smart Features (QoL)",
    "settings.autostart_title": "System Autostart",
    "settings.autostart_sub": "Background start directly to system tray",
    "settings.autostart_desc": "Automatically launch HDMI-CEC service in the background on Windows startup directly minimized to tray.",
    "settings.hud_title": "Screen Overlay (HUD)",
    "settings.hud_sub": "Popups for mouse, volume and custom actions",
    "settings.hud_desc": "Elegant translucent heads-up notifications for volume changes, mouse mode, key actions, and sector macro jumps.",
    "settings.audio_title": "PC / TV Audio Routing",
    "settings.audio_sub": "TV when active ⇄ Headphones when asleep",
    "settings.audio_desc": "Automatically route audio to TV speakers when turned on, and seamlessly revert to PC headphones when TV sleeps.",
    "settings.soundbar_title": "eARC Soundbar Passthrough",
    "settings.soundbar_sub": "Do not modify Windows master mixer volume",
    "settings.soundbar_desc": "Directly adjust volume on TV or soundbar via CEC commands without modifying the Windows master volume mixer.",
    "settings.seek_title": "Adaptive Media Seek",
    "settings.seek_sub": "Click: 5s • Hold: 1 min / 5 min acceleration",
    "settings.seek_desc": "Single arrow press seeks 5 seconds, while continuous holding accelerates seek step up to 1 and 5 minutes.",
    "settings.kill_title": "Emergency Force Close Frozen Apps",
    "settings.kill_sub": "Hold 'Back' button for 4 seconds",
    "settings.kill_desc": "Holding the 'Back' remote button for 4 seconds forcefully terminates the foreground hanging process without Task Manager.",
    "settings.polite_title": "Polite Background Presence",
    "settings.polite_sub": "Do not forcefully hijack HDMI input",
    "settings.polite_desc": "Announces PC on CEC bus for remote control without forcefully taking over HDMI input during boot or tasks.",
    "settings.autoheal_title": "Connection Watchdog (Auto-Heal)",
    "settings.autoheal_sub": "Instant reconnect if USB wire is interrupted",
    "settings.autoheal_desc": "Continuous COM-port link monitoring and instantaneous automatic reconnection if the USB cable is interrupted.",
    "settings.disp_sleep_title": "Disable Video Output on TV Sleep",
    "settings.disp_sleep_sub": "Mouse cursor won't get lost on sleeping TV",
    "settings.disp_sleep_desc": "Software deactivation of the TV display in Windows when the TV turns off, preventing cursor and windows from getting lost.",
    "settings.lang_title": "Interface Language / Язык",
    "settings.about_title": "About Application",
    "settings.os_badge": "OS",
    "settings.os_version_title": "Windows 11 / 10 x64",
    "settings.xml_title": "XML Profiles",
    "settings.xml_box_tip": "Exchange remote button mappings in XML format",
    "settings.xml_export": "Export XML",
    "settings.xml_export_tip": "Save current profile as an .xml file",
    "settings.xml_import": "Import XML",
    "settings.xml_import_tip": "Load button mappings from an .xml file",
    "settings.app_name": "CecDesk",
    "settings.app_desc": "Hardware Bridge for TV Remote & PC (HDMI-CEC)",
    "settings.version": "Version",
    "settings.check_update_btn": "Check for Updates",
    "settings.up_to_date": "Up to date",

    // Calibration
    "calib.title": "TV Remote Calibration",
    "calib.instruction": "Press all desired buttons on your physical TV remote one by one (except the TV Power button). Each detected button will appear in the remote builder.",
    "calib.detected_count": "Detected buttons:",
    "calib.waiting": "Waiting for button press...",
    "calib.power_skipped": "TV Power button skipped for safety",
    "calib.pressed_prefix": "Pressed:",
    "calib.build_btn": "Build Remote",
    "calib.cancel_btn": "Cancel",
    "calib.reset_btn": "Reset to Default",

    // Hardware status
    "status.usb_connected_port": "Connected",
    "status.usb_disconnected": "USB: Disconnected",
    "status.usb_disconnected_desc": "Connect adapter to computer",
    "status.ready_online": "Ready • TV Online",
    "status.ready_online_desc": "TV communication active",
    "status.tv_no_response": "TV not responding",
    "status.tv_no_response_desc": "Check HDMI cable or TV power",
    "status.line_error": "CEC Line 0V",
    "status.line_error_desc": "Short circuit or TV unplugged",
    "status.line_err_badge": "0V / Error",
    "status.line_free_badge": "Bus idle",
    "status.line_no_power": "No power",
    "status.bus_polling": "Polling bus...",
    "status.bus_polling_short": "Polling...",
    "status.bus_no_usb": "No USB power",
    "status.tv_online_contact": "3.3V TV Online (Solid link)",
    "status.bus_free_no_tv": "Bus idle (TV not responding)",
    "status.bus_error_0v": "0V / CEC line error (short or cable)",
    "status.remote_on_tip": "Enabled (PC control)",
    "status.remote_off_tip": "Disabled (TV native remote)",
    "status.measuring_line": "Measuring line...",
    "status.mouse_on": "Mouse ON",
    "status.mouse_on_desc": "Cursor control (Arrows / LMB / RMB)",
    "status.mouse_off": "Ready",
    "status.mouse_off_desc": "Standard remote mode",
    "status.reconnecting": "Searching...",

    // General & Settings
    "settings.saved_toast": "Settings saved",
    "settings.auto_saved": "Saved",
    "settings.port_prefix": "Port",
    "settings.checking_updates": "Checking for updates...",
    "settings.update_available": "New version available",
    "settings.update_error": "Check failed",
    "settings.open_releases": "Download Update",
    "settings.install_update_btn": "Install",
    "settings.installing_update": "Downloading and verifying update...",
    "settings.open_releases_alt": "Release Page",
    "settings.update_restarting": "Restarting into new version...",

    // Actions (Key Mapper)
    "action.steam_big_picture": "Steam Big Picture (Open / Close)",
    "action.steam_exit_to_desktop": "Steam: Exit Big Picture to Desktop",
    "action.steam_keyboard": "SteamOS: Native Steam Keyboard",
    "action.show_keyboard": "On-Screen Keyboard (Auto Steam / OSK)",
    "action.mouse_toggle": "Mouse Mode (Toggle)",
    "action.mouse_lclick": "LMB (Mouse Click)",
    "action.mouse_rclick": "RMB (Right Click)",
    "action.enter": "OK / Enter",
    "action.up": "Up",
    "action.down": "Down",
    "action.left": "Left",
    "action.right": "Right",
    "action.backspace": "Backspace",
    "action.browser_back": "Browser Back (Alt+Left)",
    "action.esc": "Cancel / Esc",
    "action.space": "Pause (Space)",
    "action.media_play": "Play / Pause",
    "action.media_stop": "Stop",
    "action.media_next": "Next Track",
    "action.media_prev": "Previous Track",
    "action.vol_up": "Volume +",
    "action.vol_down": "Volume −",
    "action.vol_mute": "Mute Audio",
    "action.f11": "Fullscreen (F11)",
    "action.pgup": "Page Up",
    "action.pgdn": "Page Down",
    "action.tab": "Next Element (Tab)",
    "action.win_d": "Show Desktop (Win+D)",
    "action.alt_tab": "Switch Task (Alt+Tab)",
    "action.pc_sleep": "PC Sleep Mode",
    "action.pc_hibernate": "PC Hibernate",
    "action.pc_shutdown": "PC Shutdown",
    "action.opto_power": "Optocoupler: Power Button (350ms)",
    "action.num_1": "Input: 1",
    "action.num_2": "Input: 2",
    "action.num_3": "Input: 3",
    "action.num_4": "Input: 4",
    "action.num_5": "Input: 5",
    "action.num_6": "Input: 6",
    "action.num_7": "Input: 7",
    "action.num_8": "Input: 8",
    "action.num_9": "Input: 9",
    "action.num_0": "Input: 0",
    "action.none": "Disabled",
    "action.script_prefix": "Script",

    // Power Sync & Optocoupler
    "settings.power_sync_title": "PC & TV Power Control (Optocoupler)",
    "settings.wake_pc_on_tv_title": "Turn on PC when TV turns on (Optocoupler)",
    "settings.wake_pc_on_tv_desc": "MCU autonomously triggers the motherboard Power button optocoupler (GP1 on RP2040 / GPIO5 on ESP32) for 350ms when TV turns on or selects PC input.",
    "settings.wake_tv_on_pc_title": "Turn on TV when PC starts",
    "settings.wake_tv_on_pc_desc": "Automatically send Image View On and Active Source commands to the TV when the application starts on PC.",
    "settings.tv_off_on_pc_title": "Turn off TV when PC sleeps or shuts down",
    "settings.tv_off_on_pc_desc": "MCU automatically sends Standby (0x36) to the TV when PC shuts down or enters sleep mode.",
    "settings.pc_off_on_tv_title": "Sleep / Shutdown PC when TV turns off",
    "settings.pc_off_on_tv_desc": "Automatically put the computer to sleep or shut it down when the TV enters standby mode.",
    "settings.pc_off_mode_label": "PC mode on TV off:",
    "settings.pc_mode_sleep": "Sleep Mode (S3 — without full shutdown)",
    "settings.pc_mode_shutdown": "Full OS Shutdown",
    "settings.pc_mode_hibernate": "Hibernate",
    "settings.pc_mode_opto": "Optocoupler Pulse (Power Button 350ms)",
    "settings.test_opto_btn": "Test Optocoupler (350ms)",
    "settings.test_sleep_btn": "Sleep PC",
    "sys.q_opto_pulse": "Opto Power",
    "sys.q_pc_sleep": "PC Sleep",
    "settings.fw_box_title": "Controller Firmware",
    "settings.fw_box_tip": "Direct controller update without third-party tools (UF2 / Serial OTA)",
    "settings.fw_flash_bundled_btn": "Flash (v1.1.0)",
    "settings.fw_flash_custom_btn": "From File (.uf2 / .bin)",
    "settings.fw_custom_tip": "Select firmware file (.uf2 / .bin)",
    "settings.fw_preparing": "Preparing...",
    "settings.audio_tv_device_label": "TV Audio Device (on TV wake):",
    "settings.audio_fallback_device_label": "PC / Headphones Device (on TV sleep):",
    "settings.audio_device_none": "— Not selected —",

    // Crash Recovery & Diagnostics
    "crash.banner_default": "The previous session ended abnormally. You can collect a diagnostic support bundle to report the issue.",
    "crash.banner_panic": "An unexpected crash report was found from the previous run. Collecting a .zip support bundle is recommended.",
    "crash.banner_dirty": "The previous session closed unexpectedly. You can save a diagnostic support bundle.",
    "crash.create_report": "Create Report (.zip)",
    "crash.dismiss": "Dismiss",
    "diag.panel_title": "Diagnostics, Reports & Telemetry",
    "diag.telemetry_title": "Anonymous Startup Telemetry",
    "diag.telemetry_desc": "Disabled by default. Sends only a random install_id, app version, OS, controller model, EDID and session error counters to your configured HTTPS webhook. No personal names or paths.",
    "diag.storage_mode_label": "Data storage mode:",
    "diag.counters_label": "Session Errors / Warnings:",
    "diag.endpoint_label": "HTTPS Webhook for reports / telemetry (optional):",
    "diag.btn_bundle": "Create Bundle (.zip)",
    "diag.btn_reports_folder": "Reports Folder",
    "diag.btn_github_issue": "Report Issue",
    "diag.status_enabled": "Anonymous telemetry enabled.",
    "diag.status_disabled": "Telemetry disabled.",
    "bug.modal_title": "Diagnostic Support Report",
    "bug.modal_desc": "The archive includes configuration, system/EDID & controller summary, and logs sanitized from user paths and tokens.",
    "bug.note_label": "Problem description (optional):",
    "bug.note_ph": "Describe what happened and steps to reproduce...",
    "bug.btn_create_zip": "Build .zip",
    "bug.btn_send_webhook": "Send to Webhook",
    "bug.btn_github_issue": "GitHub Issue",
    "bug.btn_close": "Close"
  }
};

/**
 * Получение перевода строки по ключу
 */
function t(key, fallback = "") {
  const dict = I18N_DICTIONARY[currentLang] || I18N_DICTIONARY.ru;
  return dict[key] !== undefined ? dict[key] : (I18N_DICTIONARY.ru[key] !== undefined ? I18N_DICTIONARY.ru[key] : fallback || key);
}

/**
 * Получение локализованного названия клавиши пульта
 */
function getKeyDisplayName(code, def) {
  if (!def) return code;
  if (def.isExtra) {
    return `${t("remote.button_prefix", "Кнопка")} ${def.sym}`;
  }
  return t(`key.${code}`, def.name || code);
}

/**
 * Применение языка ко всему DOM
 */
function applyLocalization(lang) {
  if (lang && (lang === "ru" || lang === "en")) {
    currentLang = lang;
  }
  if (document.documentElement) {
    document.documentElement.lang = currentLang;
  }

  document.querySelectorAll("[data-i18n]").forEach(el => {
    const key = el.getAttribute("data-i18n");
    const val = t(key);
    if (val) el.textContent = val;
  });

  document.querySelectorAll("[data-i18n-title]").forEach(el => {
    const key = el.getAttribute("data-i18n-title");
    const val = t(key);
    if (val) el.setAttribute("title", val);
  });

  document.querySelectorAll("[data-i18n-tooltip]").forEach(el => {
    const key = el.getAttribute("data-i18n-tooltip");
    const val = t(key);
    if (val) el.setAttribute("data-tooltip", val);
  });

  document.querySelectorAll("[data-i18n-placeholder]").forEach(el => {
    const key = el.getAttribute("data-i18n-placeholder");
    const val = t(key);
    if (val) el.setAttribute("placeholder", val);
  });

  // Обновление активной кнопки языка в настройках
  document.querySelectorAll(".lang-choice-btn").forEach(btn => {
    const bLang = btn.getAttribute("data-lang");
    if (bLang === currentLang) {
      btn.classList.add("active");
    } else {
      btn.classList.remove("active");
    }
  });

  // Обновление заголовка окна и локализованной диагностики/статуса питания
  if (typeof updateDynamicWindowTitle === "function") {
    updateDynamicWindowTitle();
  }
  if (typeof updateSystemDiagnostics === "function") {
    updateSystemDiagnostics();
  }
  if (typeof renderPaSelect === "function" && typeof currentProfile !== "undefined" && currentProfile) {
    renderPaSelect();
  }
}

/**
 * Смена языка пользователем
 */
async function switchLanguage(newLang) {
  if (newLang !== "ru" && newLang !== "en") return;
  currentLang = newLang;

  if (typeof appConfig !== "undefined" && appConfig) {
    appConfig.language = newLang;
    if (typeof invoke === "function") {
      try {
        await invoke("save_app_config", { newCfg: appConfig });
      } catch (e) {
        console.error("Ошибка сохранения языка:", e);
      }
    }
  }

  applyLocalization(newLang);

  // Перерисовка таблиц и пульта под новый язык
  if (typeof renderMapperTable === "function") {
    renderMapperTable();
  }
  if (typeof renderDynamicRemote === "function") {
    renderDynamicRemote();
  }
  if (typeof renderProfilesCatalog === "function") {
    renderProfilesCatalog();
  }
  if (typeof renderPcOffModeSelect === "function") {
    renderPcOffModeSelect();
  }
  if (typeof renderAudioDeviceSelects === "function" && typeof cachedAudioEndpoints !== "undefined" && cachedAudioEndpoints !== null) {
    renderAudioDeviceSelects();
  }
  if (typeof renderAppUpdateUiState === "function") {
    renderAppUpdateUiState();
  }
  if (typeof tvPowerState !== "undefined" && typeof updateTvPowerButtonUi === "function") {
    updateTvPowerButtonUi(tvPowerState);
  }
  if (typeof updateHardwareStatusUi === "function" && typeof lastHwStatus !== "undefined") {
    updateHardwareStatusUi(
      { connected: typeof isConnected !== "undefined" ? isConnected : false, port: typeof selectedPort !== "undefined" ? selectedPort : "" },
      { checked: true, tvConnected: lastHwStatus.tvConnected, lineHigh: lastHwStatus.lineHigh },
      document.getElementById("chk-remote-os") ? document.getElementById("chk-remote-os").checked : true
    );
  }
}
