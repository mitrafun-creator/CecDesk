use super::*;

#[cfg(test)]
pub fn compute_sha256_hex(data: &[u8]) -> String {
    compute_sha256_from_reader(std::io::Cursor::new(data)).unwrap_or_default()
}

#[cfg(any(target_os = "windows", test))]
pub fn compute_file_sha256_hex(path: &Path) -> Result<String, String> {
    let file =
        fs::File::open(path).map_err(|e| format!("Не удалось открыть файл для SHA-256: {}", e))?;
    compute_sha256_from_reader(BufReader::new(file))
}

#[cfg(any(target_os = "windows", test))]
fn compute_sha256_from_reader<R: Read>(mut reader: R) -> Result<String, String> {
    const H_INIT: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4,
        0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
        0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f,
        0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
        0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc,
        0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
        0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116,
        0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
        0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7,
        0xc671_78f2,
    ];

    fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let j = i * 4;
            *word = u32::from_be_bytes([block[j], block[j + 1], block[j + 2], block[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut state = H_INIT;
    let mut block = [0u8; 64];
    let mut block_len = 0usize;
    let mut total_bytes = 0u64;
    let mut io_buf = [0u8; 64 * 1024];

    loop {
        let n = reader
            .read(&mut io_buf)
            .map_err(|e| format!("Ошибка чтения при расчете SHA-256: {}", e))?;
        if n == 0 {
            break;
        }
        total_bytes = total_bytes
            .checked_add(n as u64)
            .ok_or_else(|| "Файл слишком велик для SHA-256".to_string())?;
        let mut idx = 0usize;
        while idx < n {
            let take = (64 - block_len).min(n - idx);
            block[block_len..block_len + take].copy_from_slice(&io_buf[idx..idx + take]);
            block_len += take;
            idx += take;
            if block_len == 64 {
                compress(&mut state, &block);
                block_len = 0;
            }
        }
    }

    block[block_len] = 0x80;
    block_len += 1;
    if block_len > 56 {
        block[block_len..64].fill(0);
        compress(&mut state, &block);
        block_len = 0;
    }
    block[block_len..56].fill(0);
    let total_bits = total_bytes.wrapping_mul(8);
    block[56..64].copy_from_slice(&total_bits.to_be_bytes());
    compress(&mut state, &block);

    let mut out = String::with_capacity(64);
    for word in state {
        use std::fmt::Write as FmtWrite;
        let _ = write!(&mut out, "{:08x}", word);
    }
    Ok(out)
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn sanitize_zip_rel_segments(raw_name: &str) -> Result<Option<Vec<String>>, String> {
    if raw_name.contains('\0') {
        return Err("Архив содержит недопустимый нулевой байт в имени файла".to_string());
    }
    let normalized = raw_name.replace('\\', "/");
    if normalized.ends_with('/') {
        return Ok(None);
    }
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.contains(':') || normalized.starts_with('/') {
        return Err(format!("Заблокирован небезопасный путь в архиве: {}", raw_name));
    }

    let mut parts = Vec::new();
    for seg in trimmed.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.chars().any(char::is_control) {
            return Err(format!("Заблокирован выход за пределы каталога (Zip-Slip): {}", raw_name));
        }
        parts.push(seg.to_string());
    }
    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts))
    }
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn normalize_allowed_update_rel_path(segments: &[String], exe_name: &str) -> Option<PathBuf> {
    if segments.is_empty() {
        return None;
    }
    if segments.len() == 1 {
        let f = &segments[0];
        if f.eq_ignore_ascii_case("cecdesk.exe")
            || f.eq_ignore_ascii_case("hdmi_cec_studio.exe")
            || f.eq_ignore_ascii_case(exe_name)
        {
            return Some(PathBuf::from(exe_name));
        }
        if f.eq_ignore_ascii_case("WebView2Loader.dll")
            || f.eq_ignore_ascii_case("README.md")
            || f.eq_ignore_ascii_case("LICENSE")
            || f.eq_ignore_ascii_case("THIRD_PARTY_NOTICES.md")
            || f.eq_ignore_ascii_case("version.json")
        {
            return Some(PathBuf::from(f));
        }
        return None;
    }
    let top_lower = segments[0].to_ascii_lowercase();
    if top_lower == "bin" || top_lower == "tv_profiles" {
        let mut p = PathBuf::from(&top_lower);
        for seg in &segments[1..] {
            p.push(seg);
        }
        return Some(p);
    }
    None
}

/// Безопасная распаковка ZIP-пакета обновления без внешних зависимостей (CRC-32 + защита от Zip-Slip).
#[cfg(any(target_os = "windows", test))]
pub fn extract_safe_update_zip(
    zip_path: &Path,
    stage_dir: &Path,
    exe_name: &str,
) -> Result<Vec<PathBuf>, String> {
    const MAX_ENTRY_BYTES: usize = 150 * 1024 * 1024;
    const MAX_TOTAL_BYTES: usize = 250 * 1024 * 1024;

    let bytes =
        fs::read(zip_path).map_err(|e| format!("Не удалось прочитать скачанный архив: {}", e))?;
    if bytes.len() < 22 {
        return Err("Архив обновления поврежден (слишком малый размер).".to_string());
    }

    let min_eocd = bytes.len().saturating_sub(65_557);
    let mut eocd_off = None;
    let mut pos = bytes.len() - 22;
    loop {
        if bytes[pos..pos + 4] == [0x50, 0x4B, 0x05, 0x06] {
            let comment_len = u16::from_le_bytes([bytes[pos + 20], bytes[pos + 21]]) as usize;
            if pos + 22 + comment_len == bytes.len() {
                eocd_off = Some(pos);
                break;
            }
        }
        if pos <= min_eocd {
            break;
        }
        pos -= 1;
    }
    let eocd = eocd_off.ok_or_else(|| "Файл не является корректным ZIP-архивом (нет EOCD).".to_string())?;

    let total_entries = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
    let cd_size = u32::from_le_bytes([bytes[eocd + 12], bytes[eocd + 13], bytes[eocd + 14], bytes[eocd + 15]]) as usize;
    let cd_offset = u32::from_le_bytes([bytes[eocd + 16], bytes[eocd + 17], bytes[eocd + 18], bytes[eocd + 19]]) as usize;

    let cd_end = cd_offset
        .checked_add(cd_size)
        .filter(|&end| end <= eocd)
        .ok_or_else(|| "Повреждена таблица оглавления ZIP-архива.".to_string())?;

    struct ParsedEntry {
        segments: Vec<String>,
        method: u16,
        crc32: u32,
        comp_size: usize,
        uncomp_size: usize,
        local_hdr_offset: usize,
    }

    let mut entries = Vec::with_capacity(total_entries.min(4096));
    let mut cursor = cd_offset;
    for _ in 0..total_entries {
        if cursor + 46 > cd_end || bytes[cursor..cursor + 4] != [0x50, 0x4B, 0x01, 0x02] {
            return Err("Поврежден заголовок файла в ZIP-архиве.".to_string());
        }
        let method = u16::from_le_bytes([bytes[cursor + 10], bytes[cursor + 11]]);
        let crc32 = u32::from_le_bytes([
            bytes[cursor + 16],
            bytes[cursor + 17],
            bytes[cursor + 18],
            bytes[cursor + 19],
        ]);
        let comp_size = u32::from_le_bytes([
            bytes[cursor + 20],
            bytes[cursor + 21],
            bytes[cursor + 22],
            bytes[cursor + 23],
        ]) as usize;
        let uncomp_size = u32::from_le_bytes([
            bytes[cursor + 24],
            bytes[cursor + 25],
            bytes[cursor + 26],
            bytes[cursor + 27],
        ]) as usize;
        let name_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[cursor + 30], bytes[cursor + 31]]) as usize;
        let comment_len = u16::from_le_bytes([bytes[cursor + 32], bytes[cursor + 33]]) as usize;
        let local_hdr_offset = u32::from_le_bytes([
            bytes[cursor + 42],
            bytes[cursor + 43],
            bytes[cursor + 44],
            bytes[cursor + 45],
        ]) as usize;

        let name_start = cursor + 46;
        let name_end = name_start + name_len;
        let next_cursor = name_end + extra_len + comment_len;
        if next_cursor > cd_end {
            return Err("Некорректные границы имени файла в ZIP-архиве.".to_string());
        }

        let raw_name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();
        if let Some(segments) = sanitize_zip_rel_segments(&raw_name)? {
            if uncomp_size > MAX_ENTRY_BYTES {
                return Err(format!("Файл в архиве превышает допустимый лимит: {}", raw_name));
            }
            entries.push(ParsedEntry {
                segments,
                method,
                crc32,
                comp_size,
                uncomp_size,
                local_hdr_offset,
            });
        }
        cursor = next_cursor;
    }

    let first_root = entries.first().and_then(|e| {
        if e.segments.len() >= 2 {
            Some(e.segments[0].clone())
        } else {
            None
        }
    });
    let strip_wrapper = first_root.as_ref().is_some_and(|root| {
        entries
            .iter()
            .all(|e| e.segments.len() >= 2 && e.segments[0] == *root)
            && entries.iter().any(|e| {
                e.segments.len() == 2
                    && (e.segments[1].eq_ignore_ascii_case("cecdesk.exe")
                        || e.segments[1].eq_ignore_ascii_case("hdmi_cec_studio.exe")
                        || e.segments[1].eq_ignore_ascii_case(exe_name))
            })
    });

    fs::create_dir_all(stage_dir)
        .map_err(|e| format!("Не удалось создать папку распаковки: {}", e))?;

    let mut total_unpacked = 0usize;
    let mut written_paths = Vec::new();
    let mut seen_rel = HashSet::new();

    for entry in &entries {
        let seg_slice: &[String] = if strip_wrapper {
            &entry.segments[1..]
        } else {
            &entry.segments[..]
        };
        let Some(rel_path) = normalize_allowed_update_rel_path(seg_slice, exe_name) else {
            continue;
        };
        if !seen_rel.insert(rel_path.clone()) {
            return Err(format!(
                "Дублирующийся файл в архиве обновления: {}",
                rel_path.display()
            ));
        }

        total_unpacked = total_unpacked
            .checked_add(entry.uncomp_size)
            .filter(|&tot| tot <= MAX_TOTAL_BYTES)
            .ok_or_else(|| "Суммарный размер распакованного обновления превышает лимит.".to_string())?;

        let lh = entry.local_hdr_offset;
        if lh + 30 > cd_offset || bytes[lh..lh + 4] != [0x50, 0x4B, 0x03, 0x04] {
            return Err("Поврежден локальный заголовок ZIP-записи.".to_string());
        }
        let l_name_len = u16::from_le_bytes([bytes[lh + 26], bytes[lh + 27]]) as usize;
        let l_extra_len = u16::from_le_bytes([bytes[lh + 28], bytes[lh + 29]]) as usize;
        let data_start = lh + 30 + l_name_len + l_extra_len;
        let data_end = data_start
            .checked_add(entry.comp_size)
            .filter(|&end| end <= cd_offset)
            .ok_or_else(|| "Некорректный размер сжатых данных в ZIP-архиве.".to_string())?;
        let comp_slice = &bytes[data_start..data_end];

        let unpacked = match entry.method {
            0 => {
                if comp_slice.len() != entry.uncomp_size {
                    return Err("Несовпадение размера stored-файла в ZIP-архиве.".to_string());
                }
                comp_slice.to_vec()
            }
            8 => {
                let mut dec = DeflateDecoder::new(comp_slice).take((entry.uncomp_size as u64) + 1);
                let mut buf = Vec::with_capacity(entry.uncomp_size.min(8 * 1024 * 1024));
                dec.read_to_end(&mut buf)
                    .map_err(|e| format!("Ошибка распаковки DEFLATE в архиве: {}", e))?;
                if buf.len() != entry.uncomp_size {
                    return Err("Фактический размер файла в ZIP не совпал с заголовком.".to_string());
                }
                buf
            }
            other => {
                return Err(format!(
                    "Неподдерживаемый метод сжатия ZIP ({}). Требуется Store или Deflate.",
                    other
                ));
            }
        };

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&unpacked);
        if hasher.finalize() != entry.crc32 {
            return Err(format!(
                "Ошибка контрольной суммы CRC-32 при распаковке {}",
                rel_path.display()
            ));
        }

        let dest = stage_dir.join(&rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать подкаталог {}: {}", parent.display(), e))?;
        }
        fs::write(&dest, &unpacked)
            .map_err(|e| format!("Ошибка записи распакованного файла {}: {}", dest.display(), e))?;
        written_paths.push(rel_path);
    }

    if !written_paths.iter().any(|p| p == Path::new(exe_name)) {
        return Err(format!("В архиве обновления не найден исполняемый файл {}.", exe_name));
    }

    Ok(written_paths)
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn verify_windows_pe_binary(path: &Path) -> Result<(), String> {
    let meta = fs::metadata(path)
        .map_err(|e| format!("Не найден подготовленный бинарник обновления: {}", e))?;
    if meta.len() < 1_000_000 {
        return Err(format!(
            "Скачанный исполняемый файл слишком мал ({} байт) и не похож на полный билд.",
            meta.len()
        ));
    }
    let mut f = fs::File::open(path)
        .map_err(|e| format!("Не удалось проверить заголовок исполняемого файла: {}", e))?;
    let mut magic = [0u8; 2];
    f.read_exact(&mut magic)
        .map_err(|e| format!("Ошибка чтения заголовка PE: {}", e))?;
    if &magic != b"MZ" {
        return Err("Скачанный файл не является корректным Windows PE (.exe) модулем.".to_string());
    }
    Ok(())
}
