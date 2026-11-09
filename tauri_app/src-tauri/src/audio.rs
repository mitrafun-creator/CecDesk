use crate::platform;

// #1 Переключение устройства вывода звука
pub fn switch_audio_endpoint(target: &str) -> Result<String, String> {
    platform::switch_audio_endpoint(target)
}

// #9 Управление мониторами (DisplaySwitch / DRM)
pub fn switch_display_mode(extend: bool) {
    platform::switch_display_mode(extend);
}

pub fn list_audio_endpoints_internal() -> Vec<String> {
    platform::list_audio_endpoints()
}
