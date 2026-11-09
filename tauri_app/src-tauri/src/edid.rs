use crate::platform;
use crate::types::EdidInfoDto;

pub fn parse_edid_bytes(edid: &[u8], fallback_mon_id: &str) -> Option<EdidInfoDto> {
    if edid.len() <= 128 {
        return None;
    }
    let mfg_word = ((edid[8] as u16) << 8) | (edid[9] as u16);
    let c1 = (((mfg_word >> 10) & 0x1F) as u8 + b'A' - 1) as char;
    let c2 = (((mfg_word >> 5) & 0x1F) as u8 + b'A' - 1) as char;
    let c3 = ((mfg_word & 0x1F) as u8 + b'A' - 1) as char;
    let pnp_id = format!("{}{}{}", c1, c2, c3);

    let ext = &edid[128..];
    for i in 0..ext.len().saturating_sub(4) {
        if ext[i] == 0x03 && ext[i + 1] == 0x0C && ext[i + 2] == 0x00 {
            let pa_high = ext[i + 3];
            let pa_low = ext[i + 4];
            let pa_str = format!("{:02X}{:02X}", pa_high, pa_low);
            let hdmi_num = (pa_high >> 4) & 0x0F;
            return Some(EdidInfoDto {
                pnp_id,
                monitor_id: fallback_mon_id.to_string(),
                physical_address: pa_str,
                hdmi_port_num: if hdmi_num > 0 { hdmi_num } else { 1 },
            });
        }
    }
    None
}

pub fn detect_edid_os() -> EdidInfoDto {
    platform::detect_edid_os()
}
