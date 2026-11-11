use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PortInfoDto {
    pub name: String,
    pub desc: String,
    pub is_preferred: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdidInfoDto {
    pub pnp_id: String,
    pub monitor_id: String,
    pub physical_address: String,
    pub hdmi_port_num: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HdmiPortEntry {
    pub label: String,
    pub pa: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TvProfile {
    pub id: String,
    pub brand: String,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub cec_system_name: String,
    pub edid_pnp_ids: Vec<String>,
    pub vendor_oui: String,
    pub cec_version: String,
    pub wake_sequence: Vec<String>,
    pub standby_command: String,
    pub remote_passthrough_init: String,
    pub supports_tv_ui_keys_tx: bool,
    pub supports_remote_rx_to_pc: bool,
    pub hdmi_ports: Vec<HdmiPortEntry>,
    pub notes: String,
    #[serde(default)]
    pub supported_keys: Vec<String>,
}

fn default_language() -> String {
    "ru".to_string()
}

fn default_pc_off_action() -> String {
    "SLEEP".to_string()
}

fn default_true() -> bool {
    true
}

fn default_update_channel() -> String {
    "beta".to_string()
}

fn default_active_key_profile() -> String {
    "balanced".to_string()
}

fn default_custom_key_profile_name() -> String {
    "Пользовательский".to_string()
}

fn default_key_mappings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("0x00".to_string(), "ENTER".to_string());
    map.insert("0x01".to_string(), "UP".to_string());
    map.insert("0x02".to_string(), "DOWN".to_string());
    map.insert("0x03".to_string(), "LEFT".to_string());
    map.insert("0x04".to_string(), "RIGHT".to_string());
    map.insert("0x09".to_string(), "STEAM_BIG_PICTURE".to_string());
    map.insert("0x0A".to_string(), "SHOW_KEYBOARD".to_string());
    map.insert("0x0D".to_string(), "BACKSPACE".to_string());
    map.insert("0x20".to_string(), "NUM_0".to_string());
    map.insert("0x21".to_string(), "NUM_1".to_string());
    map.insert("0x22".to_string(), "NUM_2".to_string());
    map.insert("0x23".to_string(), "NUM_3".to_string());
    map.insert("0x24".to_string(), "NUM_4".to_string());
    map.insert("0x25".to_string(), "NUM_5".to_string());
    map.insert("0x26".to_string(), "NUM_6".to_string());
    map.insert("0x27".to_string(), "NUM_7".to_string());
    map.insert("0x28".to_string(), "NUM_8".to_string());
    map.insert("0x29".to_string(), "NUM_9".to_string());
    map.insert("0x30".to_string(), "PGUP".to_string());
    map.insert("0x31".to_string(), "PGDN".to_string());
    map.insert("0x41".to_string(), "VOL_UP".to_string());
    map.insert("0x42".to_string(), "VOL_DOWN".to_string());
    map.insert("0x43".to_string(), "VOL_MUTE".to_string());
    map.insert("0x44".to_string(), "MEDIA_PLAY".to_string());
    map.insert("0x45".to_string(), "MEDIA_STOP".to_string());
    map.insert("0x46".to_string(), "MEDIA_PLAY".to_string());
    map.insert("0x48".to_string(), "LEFT".to_string());
    map.insert("0x49".to_string(), "RIGHT".to_string());
    map.insert("0x4B".to_string(), "MEDIA_NEXT".to_string());
    map.insert("0x4C".to_string(), "MEDIA_PREV".to_string());
    map.insert("0x71".to_string(), "MOUSE_TOGGLE".to_string());
    map.insert("0x72".to_string(), "MEDIA_PLAY".to_string());
    map.insert("0x73".to_string(), "VOL_MUTE".to_string());
    map.insert("0x74".to_string(), "SPACE".to_string());
    map
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub active_profile_id: String,
    pub osd_name: String,
    pub pa_override: String,
    pub remote_to_os_enabled: bool,
    pub auto_wake_tv_on_start: bool,
    pub auto_standby_tv_on_exit: bool,
    pub key_mappings: HashMap<String, String>,
    #[serde(default = "default_active_key_profile")]
    pub active_key_profile: String,
    #[serde(default = "default_custom_key_profile_name")]
    pub custom_key_profile_name: String,
    #[serde(default = "default_key_mappings")]
    pub custom_key_mappings: HashMap<String, String>,

    // Сценарии синхронизации питания ПК <-> ТВ и управления оптопарой (PWR_BTN)
    #[serde(default = "default_true")]
    pub wake_pc_on_tv_on: bool,
    #[serde(default)]
    pub power_off_pc_on_tv_off: bool,
    #[serde(default = "default_pc_off_action")]
    pub pc_off_action: String,
    #[serde(default)]
    pub opto_pwr_pin: u8,

    #[serde(default = "default_language")]
    pub language: String,

    // Кастомные пользовательские названия профилей ТВ (сохраняются между обновлениями базы)
    #[serde(default)]
    pub custom_profile_names: HashMap<String, String>,

    // Динамический конструктор пульта
    #[serde(default)]
    pub calibrated_keys: Vec<String>,

    // ==================== ТОП-10 QoL НАСТРОЙКИ ====================
    pub auto_audio_switch: bool,         // #1 Авто-переключение звука
    pub tv_audio_device: String,         // Имя аудиоустройства ТВ
    pub fallback_audio_device: String,   // Имя аудиоустройства наушников/ПК
    pub auto_revert_mouse: bool,         // #2 Авто-выход из режима мыши через 40 сек
    pub hold_seek_accelerate: bool,      // #4 Умная перемотка с ускорением
    pub emergency_kill_enabled: bool,    // #5 Аварийное снятие зависших окон
    pub non_intrusive_cec: bool,         // #6 Вежливый CEC без захвата входа
    pub soundbar_passthrough: bool,      // #7 Саундбар eARC
    pub auto_healing_usb: bool,          // #8 Сторожевой реконнект USB
    pub display_disable_on_sleep: bool,  // #9 Отключение дисплея ТВ в Windows при сне
    #[serde(default = "default_true")]
    pub hud_enabled: bool,               // Экранные уведомления (HUD)

    // ==================== ПРЕДБЕТА: ТЕЛЕМЕТРИЯ И ОБНОВЛЕНИЯ ====================
    #[serde(default)]
    pub install_id: String,              // Анонимный случайный UUID установки (без PII)
    #[serde(default)]
    pub telemetry_opt_in: bool,          // Добровольная анонимная телеметрия (по умолчанию выключена)
    #[serde(default)]
    pub telemetry_endpoint: String,      // Пользовательский HTTPS endpoint для телеметрии и баг-репортов
    #[serde(default = "default_update_channel")]
    pub update_channel: String,          // Канал обновлений ("beta" / "stable")
    #[serde(default)]
    pub last_update_check_ts: u64,       // Unix timestamp последней автоматической проверки обновлений
}

impl Default for AppConfig {
    fn default() -> Self {
        let map = default_key_mappings();

        Self {
            active_profile_id: "universal_hdmi_cec".to_string(),
            osd_name: "GAMING PC".to_string(),
            pa_override: "1000".to_string(),
            remote_to_os_enabled: true,
            auto_wake_tv_on_start: false,
            auto_standby_tv_on_exit: false,
            key_mappings: map.clone(),
            active_key_profile: default_active_key_profile(),
            custom_key_profile_name: default_custom_key_profile_name(),
            custom_key_mappings: map,
            wake_pc_on_tv_on: true,
            power_off_pc_on_tv_off: false,
            pc_off_action: "SLEEP".to_string(),
            opto_pwr_pin: 0,
            language: "ru".to_string(),
            custom_profile_names: HashMap::new(),
            calibrated_keys: vec![
                "0x00".to_string(), "0x01".to_string(), "0x02".to_string(), "0x03".to_string(), "0x04".to_string(),
                "0x0D".to_string(), "0x09".to_string(), "0x0A".to_string(),
                "0x72".to_string(), "0x73".to_string(), "0x74".to_string(), "0x71".to_string(),
                "0x41".to_string(), "0x42".to_string(), "0x43".to_string(),
            ],

            auto_audio_switch: false,
            tv_audio_device: String::new(),
            fallback_audio_device: String::new(),
            auto_revert_mouse: true,
            hold_seek_accelerate: true,
            emergency_kill_enabled: false,
            non_intrusive_cec: true,
            soundbar_passthrough: false,
            auto_healing_usb: true,
            display_disable_on_sleep: false,
            hud_enabled: true,

            install_id: String::new(),
            telemetry_opt_in: false,
            telemetry_endpoint: String::new(),
            update_channel: "beta".to_string(),
            last_update_check_ts: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomScriptInfo {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub ext: String,
}
