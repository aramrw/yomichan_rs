use wana_kana::ConvertJapanese;

use super::japanese::Unicode16;

/// Converts text to Hiragana.
pub fn convert_alphabetic_part_to_kana<T: AsRef<str>>(text: T) -> String {
    text.as_ref().to_hiragana()
}

/// Convert Romaji to Kana.
/// lowercase text will result in Hiragana,
/// and UPPERCASE text will result in Katakana.
pub fn convert_to_kana<T: AsRef<str>>(text: T) -> String {
    text.as_ref().to_kana()
}

pub fn convert_to_romaji<T: AsRef<str>>(text: T) -> String {
    text.as_ref().to_romaji()
}

pub fn convert_alphabetic_to_kana<T: AsRef<str>>(text: T) -> String {
    let mut part = String::new();
    let mut result = String::new();

    for char in text.as_ref().chars() {
        let mut c = char as u32;
        let normalized_c = match c {
            0x41..=0x5a => {
                c += 0x61; // First add 0x61
                c -= 0x41; // Then subtract 0x41
                c
            }
            0x61..=0x7a => c, // ['a', 'z']
            0xff21..=0xff3a => {
                c += 0x61; // First add 0x61
                c -= 0xff21; // Then subtract 0xff21
                c
            }
            0xff41..=0xff5a => {
                c += 0x61; // First add 0x61
                c -= 0xff41; // Then subtract 0xff41
                c
            }
            0x2d | 0xff0d => 0x2d, // '-' or fullwidth dash
            _ => {
                if !part.is_empty() {
                    result.push_str(&convert_alphabetic_part_to_kana(&part));
                    part.clear();
                }
                result.push(char);
                continue;
            }
        };
        part.push(char::from_u32(normalized_c).unwrap());
    }

    if !part.is_empty() {
        result.push_str(&convert_alphabetic_part_to_kana(&part));
    }

    result
}
