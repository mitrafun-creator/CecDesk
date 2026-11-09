pub fn cec_key_name(code: u8) -> &'static str {
    match code {
        0x00 => "OK",
        0x01 => "Вверх",
        0x02 => "Вниз",
        0x03 => "Влево",
        0x04 => "Вправо",
        0x09 => "Меню",
        0x0A | 0x0B => "Опции",
        0x0D => "Назад",
        0x20 => "0",
        0x21 => "1",
        0x22 => "2",
        0x23 => "3",
        0x24 => "4",
        0x25 => "5",
        0x26 => "6",
        0x27 => "7",
        0x28 => "8",
        0x29 => "9",
        0x2A => "Точка (.)",
        0x2B => "Enter",
        0x30 => "Канал +",
        0x31 => "Канал −",
        0x35 => "Инфо",
        0x41 => "Громкость +",
        0x42 => "Громкость −",
        0x43 => "Mute",
        0x44 => "Play",
        0x45 => "Stop",
        0x46 => "Pause",
        0x48 => "Перемотка назад",
        0x49 => "Перемотка вперед",
        0x4B => "След.",
        0x4C => "Пред.",
        0x53 => "Гид (EPG)",
        0x71 => "Синяя",
        0x72 => "Красная",
        0x73 => "Зеленая",
        0x74 => "Желтая",
        _ => "Кнопка",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cec_key_name() {
        assert_eq!(cec_key_name(0x00), "OK");
        assert_eq!(cec_key_name(0x0D), "Назад");
        assert_eq!(cec_key_name(0x44), "Play");
        assert_eq!(cec_key_name(0x71), "Синяя");
        assert_eq!(cec_key_name(0xFF), "Кнопка");
    }
}
