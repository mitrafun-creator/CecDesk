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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_phl_edid(pa_high: u8, pa_low: u8, include_vsdb: bool) -> Vec<u8> {
        let mut edid = vec![0u8; 256];
        // PHL = P(16), H(8), L(12) => (16<<10)|(8<<5)|12 = 0x410C
        edid[8] = 0x41;
        edid[9] = 0x0C;
        if include_vsdb {
            edid[132] = 0x03;
            edid[133] = 0x0C;
            edid[134] = 0x00;
            edid[135] = pa_high;
            edid[136] = pa_low;
        }
        edid
    }

    #[test]
    fn test_parse_edid_valid_hdmi_vsdb() {
        let edid = make_phl_edid(0x20, 0x00, true);
        let dto = parse_edid_bytes(&edid, "TEST_MON").expect("EDID should parse");
        assert_eq!(dto.pnp_id, "PHL");
        assert_eq!(dto.monitor_id, "TEST_MON");
        assert_eq!(dto.physical_address, "2000");
        assert_eq!(dto.hdmi_port_num, 2);
    }

    #[test]
    fn test_parse_edid_zero_port_falls_back_to_one() {
        let edid = make_phl_edid(0x00, 0x00, true);
        let dto = parse_edid_bytes(&edid, "M0").expect("EDID should parse");
        assert_eq!(dto.hdmi_port_num, 1);
    }

    #[test]
    fn test_parse_edid_rejects_short_or_no_vsdb() {
        assert!(parse_edid_bytes(&vec![0u8; 128], "M1").is_none());
        let no_vsdb = make_phl_edid(0x10, 0x00, false);
        assert!(parse_edid_bytes(&no_vsdb, "M2").is_none());
    }
}
