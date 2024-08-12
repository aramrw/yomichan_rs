use std::{
    cmp,
    collections::{HashMap, HashSet},
    net::ToSocketAddrs,
    sync::LazyLock,
};

use crate::language::{
    cjk_utils::{
        is_code_point_in_range, is_code_point_in_ranges, CodepointRange, CJK_IDEOGRAPH_RANGES,
    },
    descriptors::collect_graphemes,
};

pub const HIRAGANA_SMALL_TSU_CODE_POINT: u32 = 0x3063;
pub const KATAKANA_SMALL_TSU_CODE_POINT: u32 = 0x30c3;
pub const KATAKANA_SMALL_KA_CODE_POINT: u32 = 0x30f5;
pub const KATAKANA_SMALL_KE_CODE_POINT: u32 = 0x30f6;
pub const KANA_PROLONGED_SOUND_MARK_CODE_POINT: u32 = 0x30fc;

pub const HIRAGANA_CONVERSION_RANGE: CodepointRange = (0x3041, 0x3096);
pub const KATAKANA_CONVERSION_RANGE: CodepointRange = (0x30a1, 0x30f6);

pub const HIRAGANA_RANGE: CodepointRange = (0x3040, 0x309f);
pub const KATAKANA_RANGE: CodepointRange = (0x30a0, 0x30ff);

pub const KANA_RANGES: &[CodepointRange] = &[HIRAGANA_RANGE, KATAKANA_RANGE];

pub const JP_RANGES_BASE: [CodepointRange; 14] = [
    HIRAGANA_RANGE,
    KATAKANA_RANGE,
    (0xff66, 0xff9f), // Halfwidth katakana
    (0x30fb, 0x30fc), // Katakana punctuation
    (0xff61, 0xff65), // Kana punctuation
    (0x3000, 0x303f), // CJK punctuation
    (0xff10, 0xff19), // Fullwidth numbers
    (0xff21, 0xff3a), // Fullwidth upper case Latin letters
    (0xff41, 0xff5a), // Fullwidth lower case Latin letters
    (0xff01, 0xff0f), // Fullwidth punctuation 1
    (0xff1a, 0xff1f), // Fullwidth punctuation 2
    (0xff3b, 0xff3f), // Fullwidth punctuation 3
    (0xff5b, 0xff60), // Fullwidth punctuation 4
    (0xffe0, 0xffee), // Currency markers
];

pub static JAPANESE_RANGES: LazyLock<[CodepointRange; 26]> = LazyLock::new(|| {
    let mut combined: [CodepointRange; 26] = [(0, 0); 26];
    combined[..14].copy_from_slice(&JP_RANGES_BASE);
    combined[14..].copy_from_slice(&CJK_IDEOGRAPH_RANGES);
    combined
});

pub static SMALL_KANA_SET: LazyLock<HashSet<char>> = LazyLock::new(|| {
    HashSet::from([
        'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'ゃ', 'ゅ', 'ょ', 'ゎ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ャ',
        'ュ', 'ョ', 'ヮ',
    ])
});

#[rustfmt::skip]
pub static HALFWIDTH_KATAKANA_MAP: LazyLock<HashMap<char, &str>> = LazyLock::new(|| {
    HashMap::from([
        ('･', "・"),('ｦ', "ヲヺ"),('ｧ', "ァ"),('ｨ', "ィ"),('ｩ', "ゥ"),('ｪ', "ェ"),
        ('ｫ', "ォ"),('ｬ', "ャ"),('ｭ', "ュ"),('ｮ', "ョ"),('ｯ', "ッ"),('ｰ', "ー"),
        ('ｱ', "ア"),('ｲ', "イ"),('ｳ', "ウヴ"),('ｴ', "エ"),('ｵ', "オ"),('ｶ', "カガ"),
        ('ｷ', "キギ"),('ｸ', "クグ"),('ｹ', "ケゲ"),('ｺ', "コゴ"),('ｻ', "サザ"),
        ('ｼ', "シジ"),('ｽ', "スズ"),('ｾ', "セゼ"),('ｿ', "ソゾ"),('ﾀ', "タダ"),('ﾁ', "チヂ"),
        ('ﾂ', "ツヅ"),('ﾃ', "テデ"),('ﾄ', "トド"),('ﾅ', "ナ"),('ﾆ', "ニ"),('ﾇ', "ヌ"),
        ('ﾈ', "ネ"),('ﾉ', "ノ"),('ﾊ', "ハバパ"),('ﾋ', "ヒビピ"),('ﾌ', "フブプ"),
        ('ﾍ', "ヘベペ"),('ﾎ', "ホボポ"),('ﾏ', "マ"),('ﾐ', "ミ"),('ﾑ', "ム"),
        ('ﾒ', "メ"),('ﾓ', "モ"),('ﾔ', "ヤ"),('ﾕ', "ユ"),('ﾖ', "ヨ"),('ﾗ', "ラ"),
        ('ﾘ', "リ"),('ﾙ', "ル"),('ﾚ', "レ"),('ﾛ', "ロ"),('ﾜ', "ワ"),('ﾝ', "ン"),
    ])
});

#[rustfmt::skip]
static VOWEL_TO_KANA_MAPPING: LazyLock<HashMap<char, &str>> = LazyLock::new(|| {
    HashMap::from([
        ('a', "ぁあかがさざただなはばぱまゃやらゎわヵァアカガサザタダナハバパマャヤラヮワヵヷ"),
        ('i', "ぃいきぎしじちぢにひびぴみりゐィイキギシジチヂニヒビピミリヰヸ"),
        ('u', "ぅうくぐすずっつづぬふぶぷむゅゆるゥウクグスズッツヅヌフブプムュユルヴ"),
        ('e', "ぇえけげせぜてでねへべぺめれゑヶェエケゲセゼテデネヘベペメレヱヶヹ"),
        ('o', "ぉおこごそぞとどのほぼぽもょよろをォオコゴソゾトドノホボポモョヨロヲヺ"),
        ('_', "のノ"),
    ])
});

pub static KANA_TO_VOWEL_MAPPING: LazyLock<HashMap<char, char>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for (&vowel, characters) in VOWEL_TO_KANA_MAPPING.iter() {
        for char in characters.chars() {
            map.insert(char, vowel);
        }
    }
    map
});

#[derive(Clone)]
pub struct FuriganaGroup {
    pub is_kana: bool,
    pub text: String,
    pub text_normalized: Option<String>,
}

#[derive(Clone)]
pub struct FuriganaSegment {
    pub text: String,
    pub reading: Option<String>,
}

impl FuriganaSegment {
    pub fn create_furigana_segment(text: String, reading: Option<String>) -> Self {
        Self { text, reading }
    }
}

pub enum PitchCategory {
    Heiban,
    Kifuku,
    Atamadaka,
    Odaka,
    Nakadaka,
}

#[derive(Clone)]
pub enum DiacriticType {
    Dakuten,
    Handakuten,
}

pub struct DiacriticInfo {
    pub character: char,
    pub diacritic_type: DiacriticType,
}

pub static DIACRITIC_MAPPING: LazyLock<HashMap<char, DiacriticInfo>> = LazyLock::new(|| {
    const KANA: &str = "うゔ-かが-きぎ-くぐ-けげ-こご-さざ-しじ-すず-せぜ-そぞ-ただ-ちぢ-つづ-てで-とど-はばぱひびぴふぶぷへべぺほぼぽワヷ-ヰヸ-ウヴ-ヱヹ-ヲヺ-カガ-キギ-クグ-ケゲ-コゴ-サザ-シジ-スズ-セゼ-ソゾ-タダ-チヂ-ツヅ-テデ-トド-ハバパヒビピフブプヘベペホボポ";
    let mut map = HashMap::new();
    let chars: Vec<char> = KANA.chars().collect();
    for chunk in chars.chunks(3) {
        if let [character, dakuten, handakuten] = *chunk {
            map.insert(
                dakuten,
                DiacriticInfo {
                    character,
                    diacritic_type: DiacriticType::Dakuten,
                },
            );
            if handakuten != '-' {
                map.insert(
                    handakuten,
                    DiacriticInfo {
                        character,
                        diacritic_type: DiacriticType::Handakuten,
                    },
                );
            }
        }
    }
    map
});

fn get_prolonged_hiragana(prev_char: char) -> Option<char> {
    if let Some(char) = KANA_TO_VOWEL_MAPPING.get(&prev_char) {
        match char {
            'a' => Some(String::from('あ')),
            'e' => Some(String::from('い')),
            'i' => Some(String::from('う')),
            'o' => Some(String::from('え')),
            'u' => Some(String::from('う')),
            _ => None,
        };
    }
    None
}

fn create_furigana_segment(text: String, reading: String) -> FuriganaSegment {
    FuriganaSegment {
        text,
        reading: Some(reading),
    }
}

pub trait Unicode16 {
    fn code_point_at(&self, index: usize) -> Option<u32>;
}

impl Unicode16 for str {
    fn code_point_at(&self, index: usize) -> Option<u32> {
        if index >= self.len() {
            return None;
        }

        let mut char_indices = self.char_indices();
        // Find the character at the given byte index
        if let Some((_, ch)) = char_indices.nth(index) {
            // Encode the character to UTF-16 and handle surrogate pairs
            let mut utf16_units = [0u16; 2];
            let encoded_units = ch.encode_utf16(&mut utf16_units);
            if encoded_units.len() == 1 {
                Some(utf16_units[0] as u32)
            } else {
                // Handle surrogate pair
                let high_surrogate = utf16_units[0] as u32;
                let low_surrogate = utf16_units[1] as u32;
                Some(0x10000 + ((high_surrogate - 0xD800) << 10) + (low_surrogate - 0xDC00))
            }
        } else {
            None
        }
    }
}

impl Unicode16 for char {
    fn code_point_at(&self, index: usize) -> Option<u32> {
        if index > 0 {
            // For a single `char`, the index can only be 0
            return None;
        }

        // Encode the character to UTF-16 and handle surrogate pairs
        let mut utf16_units = [0u16; 2];
        let encoded_units = self.encode_utf16(&mut utf16_units);
        if encoded_units.len() == 1 {
            Some(utf16_units[0] as u32)
        } else {
            // Handle surrogate pair
            let high_surrogate = utf16_units[0] as u32;
            let low_surrogate = utf16_units[1] as u32;
            Some(0x10000 + ((high_surrogate - 0xD800) << 10) + (low_surrogate - 0xDC00))
        }
    }
}

// instead of storing functions
// impl them to a trait
// struct JaPreTextProcessors {
//     convert_half_width_characters: ConvertHalfWidthCharacters,
//     alphabetic_to_hiragana: AlphabeticToHiragana,
//     normalize_combining_characters: NormalizeCombiningCharacters,
//     alphanumeric_width_variants: AlphanumericWidthVariants,
//     // convert_hiragana_to_katakana: convert_hiragana_to_katakana, // completed
//     collapse_emphatic_sequences: CollapseEmphaticSequences,
// }
//
// pub struct JaTextProcessors {
//     pre: JaPreTextProcessors,
// }

///  An optional function which returns whether or not a given string may be translatable.
///
///  This is used as a filter for several situations, such as whether the clipboard monitor
///  window should activate when text is copied to the clipboard.
///  If no value is provided, `true` is assumed for all inputs.
fn is_lookup_worthy(str: &str) -> bool {
    let g = collect_graphemes(str);
    if (g.len() == 0) {
        return false;
    }
    for c in g {
        if let Some(c_point) = c.code_point_at(0) {
            if (is_code_point_in_ranges(c_point, &*JAPANESE_RANGES)) {
                return true;
            }
        }
    }
    false
}

// fn get_prolonged_hiragana(ch: char) -> Option<char> {
//     let hiragana_prolonged_map: std::collections::HashMap<char, char> = [
//         ('あ', 'あ'),
//         ('い', 'い'),
//         ('う', 'う'),
//         ('え', 'え'),
//         ('お', 'お'),
//         ('か', 'あ'),
//         ('き', 'い'),
//         ('く', 'う'),
//         ('け', 'え'),
//         ('こ', 'お'),
//         // Add more mappings as needed
//     ]
//     .iter()
//     .cloned()
//     .collect();
//
//     hiragana_prolonged_map.get(&ch).cloned()
// }

fn segmentize_furigana(
    reading: String,
    reading_normalized: String,
    groups: &[FuriganaGroup],
    groups_start: usize,
) -> Option<Vec<FuriganaSegment>> {
    let group_count = groups.len().saturating_sub(groups_start);
    if group_count == 0 {
        return if reading.is_empty() {
            Some(vec![])
        } else {
            None
        };
    }

    let group = &groups[groups_start];
    let FuriganaGroup {
        is_kana,
        text,
        text_normalized,
    } = group;
    let text_length = text.len();

    if *is_kana {
        if let Some(text_normalized) = text_normalized {
            if reading_normalized.starts_with(text_normalized) {
                if let Some(mut segments) = segmentize_furigana(
                    reading[text_length..].to_string(),
                    reading_normalized[text_length..].to_string(),
                    groups,
                    groups_start + 1,
                ) {
                    if reading.starts_with(text) {
                        segments.insert(
                            0,
                            FuriganaSegment::create_furigana_segment(text.clone(), None),
                        );
                    } else {
                        segments.splice(0..0, get_furigana_kana_segments(text, &reading));
                    }
                    return Some(segments);
                }
            }
        }
        None
    } else {
        let mut result = None;
        for i in (text_length..=reading.len()).rev() {
            if let Some(mut segments) = segmentize_furigana(
                reading[i..].to_string(),
                reading_normalized[i..].to_string(),
                groups,
                groups_start + 1,
            ) {
                if result.is_some() {
                    // More than one way to segmentize the tail; mark as ambiguous
                    return None;
                }
                let segment_reading = &reading[..i];
                segments.insert(
                    0,
                    FuriganaSegment::create_furigana_segment(
                        text.clone(),
                        Some(segment_reading.to_string()),
                    ),
                );
                result = Some(segments);

                // There is only one way to segmentize the last non-kana group
                if group_count == 1 {
                    break;
                }
            }
        }
        result
    }
}

fn get_furigana_kana_segments(text: &str, reading: &str) -> Vec<FuriganaSegment> {
    let text_len = text.len();
    let mut new_segments: Vec<FuriganaSegment> = Vec::new();
    let mut start = 0;
    let mut state =
        text.chars().next().unwrap_or_default() == reading.chars().next().unwrap_or_default();

    for i in 1..text_len {
        let new_state =
            text.chars().nth(i).unwrap_or_default() == reading.chars().nth(i).unwrap_or_default();
        if state == new_state {
            continue;
        }
        new_segments.push(FuriganaSegment::create_furigana_segment(
            text[start..i].to_string(),
            if state {
                None
            } else {
                Some(reading[start..i].to_string())
            },
        ));
        state = new_state;
        start = i;
    }
    new_segments.push(FuriganaSegment::create_furigana_segment(
        text[start..text_len].to_string(),
        if state {
            None
        } else {
            Some(reading[start..text_len].to_string())
        },
    ));
    new_segments
}

pub fn get_stem_length<T: AsRef<str>>(text1: T, text2: T) -> u32 {
    let text1 = text1.as_ref();
    let text2 = text2.as_ref();
    let min_len = cmp::min(text1.len(), text2.len());
    if min_len == 0 {
        return 0;
    }

    let mut i = 0;
    loop {
        let char1 = text1.code_point_at(i);
        let char2 = text2.code_point_at(i);
        if char1 != char2 {
            break;
        }
        let char_len = char1
            .map(|cp| {
                let mut buffer = [0u16; 2];
                char::from_u32(cp).unwrap().encode_utf16(&mut buffer).len()
            })
            .unwrap_or(0);
        i += char_len;
        if i >= min_len {
            if i > min_len {
                i -= char_len; // Don't consume partial UTF16 surrogate characters
            }
            break;
        }
    }
    i as u32
}

pub fn is_code_point_kana(code_point: u32) -> bool {
    is_code_point_in_ranges(code_point, &KANA_RANGES)
}

pub fn is_code_point_japanese(code_point: u32) -> bool {
    is_code_point_in_ranges(code_point, &*JAPANESE_RANGES)
}

pub fn is_string_entirely_kana<T: AsRef<str>>(str: T) -> bool {
    let str = str.as_ref();
    if (str.len() == 0) {
        return false;
    }
    for c in str.chars() {
        if !is_code_point_in_ranges(c.code_point_at(0).unwrap_or_default(), KANA_RANGES) {
            return false;
        }
    }
    true
}

pub fn is_string_partially_japanese<T: AsRef<str>>(str: T) -> bool {
    let str = str.as_ref();
    if str.len() == 0 {
        return false;
    }
    for c in str.chars() {
        if (is_code_point_in_ranges(c.code_point_at(0).unwrap_or_default(), &*JAPANESE_RANGES)) {
            return true;
        }
    }
    false
}

pub fn is_mora_pitch_high(mora_index: usize, pitch_accent_downstep_position: usize) -> bool {
    match pitch_accent_downstep_position {
        0 => mora_index > 0,
        1 => mora_index < 1,
        _ => mora_index > 0 && mora_index < pitch_accent_downstep_position,
    }
}

pub fn get_pitch_category(
    text: String,
    pitch_accent_downstep_position: usize,
    is_verb_or_adjective: bool,
) -> Option<PitchCategory> {
    if (pitch_accent_downstep_position == 0) {
        return Some(PitchCategory::Heiban);
    }
    if (is_verb_or_adjective) {
        if pitch_accent_downstep_position > 0 {
            return Some(PitchCategory::Kifuku);
        }
        return None;
    }
    if (pitch_accent_downstep_position == 1) {
        return Some(PitchCategory::Atamadaka);
    }
    if (pitch_accent_downstep_position > 1) {
        if pitch_accent_downstep_position >= get_kana_mora_count(text).into() {
            return Some(PitchCategory::Odaka);
        }
        return Some(PitchCategory::Nakadaka);
    }
    None
}

fn get_kana_morae<T: AsRef<str>>(text: T) -> Vec<String> {
    let text = text.as_ref();
    let mut morae: Vec<String> = Vec::new();
    for char in text.chars() {
        if SMALL_KANA_SET.contains(&char) && !morae.is_empty() {
            if let Some(last) = morae.last_mut() {
                last.push(char);
            }
        } else {
            morae.push(char.to_string());
        }
    }
    morae
}

pub fn get_kana_mora_count<T: AsRef<str>>(text: T) -> u16 {
    let text = text.as_ref();
    let mut mora_count: u16 = 0;
    for c in text.chars() {
        if SMALL_KANA_SET.get(&c).is_none() || mora_count == 0 {
            mora_count += 1;
        }
    }
    mora_count
}

pub fn convert_katakana_to_hiragana<T: AsRef<str>>(
    text: T,
    keep_prolonged_sound_marks: bool,
) -> String {
    let mut result = String::new();
    let text = text.as_ref();
    let offset = HIRAGANA_CONVERSION_RANGE.0 - KATAKANA_CONVERSION_RANGE.0;

    for char in text.chars() {
        let mut converted_char = char;
        if let Some(code_point) = char.code_point_at(0) {
            match code_point {
                KATAKANA_SMALL_KA_CODE_POINT | KATAKANA_SMALL_KE_CODE_POINT => {
                    // No change
                }
                KANA_PROLONGED_SOUND_MARK_CODE_POINT => {
                    if !keep_prolonged_sound_marks && !result.is_empty() {
                        if let Some(char2) = get_prolonged_hiragana(result.chars().last().unwrap())
                        {
                            converted_char = char2;
                        }
                    }
                }
                _ => {
                    if is_code_point_in_range(code_point, KATAKANA_CONVERSION_RANGE) {
                        if let Some(new_char) = std::char::from_u32(code_point + offset) {
                            converted_char = new_char;
                        }
                    }
                }
            }
        }
        result.push(converted_char);
    }

    result
}

pub fn convert_hiragana_to_katakana<T: AsRef<str>>(text: T) -> String {
    let mut result = String::new();
    let text = text.as_ref();
    let offset = KATAKANA_CONVERSION_RANGE.0 - HIRAGANA_CONVERSION_RANGE.0;

    for char in text.chars() {
        let mut converted_char = char;
        if let Some(code_point) = char.code_point_at(0) {
            if is_code_point_in_range(code_point, HIRAGANA_CONVERSION_RANGE) {
                if let Some(new_char) = std::char::from_u32(code_point + offset) {
                    converted_char = new_char;
                }
            }
        }
        result.push(converted_char);
    }

    result
}

pub fn convert_alphanumeric_to_fullwidth<T: AsRef<str>>(text: T) -> String {
    let text = text.as_ref();
    let mut result = String::new();

    for char in text.chars() {
        if let Some(mut code_point) = char.code_point_at(0) {
            if code_point >= 0x30 && code_point <= 0x39 {
                // ['0', '9']
                code_point += 0xff10 - 0x30; // 0xff10 = '0' full width
            } else if code_point >= 0x41 && code_point <= 0x5a {
                // ['A', 'Z']
                code_point += 0xff21 - 0x41; // 0xff21 = 'A' full width
            } else if code_point >= 0x61 && code_point <= 0x7a {
                // ['a', 'z']
                code_point += 0xff41 - 0x61; // 0xff41 = 'a' full width
            }
            if let Some(new_char) = std::char::from_u32(code_point) {
                result.push(new_char);
            } else {
                result.push(char);
            }
        } else {
            result.push(char);
        }
    }

    result
}

pub fn convert_fullwidth_alphanumeric_to_normal<T: AsRef<str>>(text: T) -> String {
    let text = text.as_ref();
    let mut result = String::new();

    for char in text.chars() {
        if let Some(mut code_point) = char.code_point_at(0) {
            if code_point >= 0xff10 && code_point <= 0xff19 {
                // ['０', '９']
                code_point -= 0xff10 - 0x30; // 0x30 = '0'
            } else if code_point >= 0xff21 && code_point <= 0xff3a {
                // ['Ａ', 'Ｚ']
                code_point -= 0xff21 - 0x41; // 0x41 = 'A'
            } else if code_point >= 0xff41 && code_point <= 0xff5a {
                // ['ａ', 'ｚ']
                code_point -= 0xff41 - 0x61; // 0x61 = 'a'
            }
            if let Some(new_char) = std::char::from_u32(code_point) {
                result.push(new_char);
            } else {
                result.push(char);
            }
        } else {
            result.push(char);
        }
    }

    result
}

pub fn convert_halfwidth_kana_to_fullwidth(text: &str) -> String {
    let mut result = String::new();

    let mut i = 0;
    let text_chars: Vec<char> = text.chars().collect();
    while i < text_chars.len() {
        let c = text_chars[i];
        let mapping = HALFWIDTH_KATAKANA_MAP.get(&c);
        if mapping.is_none() {
            result.push(c);
            i += 1;
            continue;
        }

        let mapping = mapping.unwrap();
        let mut index = 0;
        if i + 1 < text_chars.len() {
            match text_chars[i + 1].code_point_at(0).unwrap_or_default() {
                0xff9e => {
                    // Dakuten
                    index = 1;
                }
                0xff9f => {
                    // Handakuten
                    index = 2;
                }
                _ => {}
            }
        }

        let mut c2 = mapping.chars().nth(index).unwrap_or_default();
        if index > 0 {
            if c2 == '-' {
                // Invalid
                index = 0;
                c2 = mapping.chars().nth(0).unwrap_or_default();
            } else {
                i += 1;
            }
        }

        result.push(c2);
        i += 1;
    }

    result
}

pub fn get_kana_diacritic_info<T: AsRef<char>>(character: T) -> Option<DiacriticInfo> {
    let character = character.as_ref();
    DIACRITIC_MAPPING.get(character);
    if let Some(info) = DIACRITIC_MAPPING.get(character) {
        return Some(DiacriticInfo {
            character: info.character.clone(),
            diacritic_type: info.diacritic_type.clone(),
        });
    }
    None
}

pub fn dakuten_allowed(code_point: u32) -> bool {
    // To reduce processing time, some characters which
    // shouldn't have dakuten but are highly unlikely to have a
    // combining character attached are included.
    // かがきぎくぐけげこごさざしじすずせぜそぞただちぢっつづてでとはばぱひびぴふぶぷへべぺほ
    // カガキギクグケゲコゴサザシジスズセゼソゾタダチヂッツヅテデトハバパヒビピフブプヘベペホ
    (code_point >= 0x304B && code_point <= 0x3068)
        || (code_point >= 0x306F && code_point <= 0x307B)
        || (code_point >= 0x30AB && code_point <= 0x30C8)
        || (code_point >= 0x30CF && code_point <= 0x30DB)
}

pub fn handakuten_allowed(code_point: u32) -> bool {
    // To reduce processing time, some characters which
    // shouldn't have handakuten but are highly unlikely to have a
    // combining character attached are included.
    // はばぱひびぴふぶぷへべぺほ
    // ハバパヒビピフブプヘベペホ
    (code_point >= 0x306F && code_point <= 0x307B) || (code_point >= 0x30CF && code_point <= 0x30DB)
}

pub fn normalize_combining_characters(text: &str) -> String {
    let mut result = String::new();
    let mut i = text.len() as isize - 1;
    // Ignoring the first character is intentional, it cannot combine with anything
    while i > 0 {
        if let Some(char) = text.chars().nth(i as usize) {
            if char == '\u{3099}' {
                if let Some(dakuten_combinee) = text.chars().nth(i as usize - 1) {
                    if dakuten_allowed(dakuten_combinee as u32) {
                        if let Some(combined_char) =
                            std::char::from_u32(dakuten_combinee as u32 + 1)
                        {
                            result.insert(0, combined_char);
                            i -= 2;
                            continue;
                        }
                    }
                }
            } else if char == '\u{309A}' {
                if let Some(handakuten_combinee) = text.chars().nth(i as usize - 1) {
                    if handakuten_allowed(handakuten_combinee as u32) {
                        if let Some(combined_char) =
                            std::char::from_u32(handakuten_combinee as u32 + 2)
                        {
                            result.insert(0, combined_char);
                            i -= 2;
                            continue;
                        }
                    }
                }
            } else {
                result.insert(0, char);
            }
        }
        i -= 1;
    }
    // i === -1 when first two characters are combined
    if i == 0 {
        if let Some(char) = text.chars().nth(0) {
            result.insert(0, char);
        }
    }
    result
}

pub fn distribute_furigana(term: String, reading: String) -> Vec<FuriganaSegment> {
    if reading == term {
        // Same
        return vec![FuriganaSegment::create_furigana_segment(term, None)];
    }

    let mut groups: Vec<FuriganaGroup> = vec![];
    let mut group_pre: Option<FuriganaGroup> = None;
    let mut is_kana_pre: Option<bool> = None;

    for c in term.chars() {
        let code_point = c.code_point_at(0).unwrap_or_default();
        let is_kana = is_code_point_kana(code_point);
        if Some(is_kana) == is_kana_pre {
            if let Some(ref mut group) = group_pre {
                group.text.push(c);
            }
        } else {
            let new_group = FuriganaGroup {
                is_kana,
                text: c.to_string(),
                text_normalized: None,
            };
            groups.push(new_group.clone());
            group_pre = Some(new_group);
            is_kana_pre = Some(is_kana);
        }
    }

    for group in &mut groups {
        if group.is_kana {
            group.text_normalized = Some(convert_katakana_to_hiragana(&group.text, false));
        }
    }

    let reading_normalized = convert_katakana_to_hiragana(&reading, false);
    if let Some(segments) = segmentize_furigana(reading.clone(), reading_normalized, &groups, 0) {
        return segments;
    }

    // Fallback
    vec![FuriganaSegment::create_furigana_segment(
        term,
        Some(reading),
    )]
}

pub fn distribute_furigana_inflected(
    term: String,
    mut reading: String,
    source: String,
) -> Vec<FuriganaSegment> {
    let term_normalized = convert_katakana_to_hiragana(&term, false);
    let reading_normalized = convert_katakana_to_hiragana(&reading, false);
    let source_normalized = convert_katakana_to_hiragana(&source, false);

    let mut main_text = term.clone();
    let mut stem_length = get_stem_length(&term_normalized, &source_normalized);

    // Check if source is derived from the reading instead of the term
    let reading_stem_length = get_stem_length(&reading_normalized, &source_normalized);
    if reading_stem_length > 0 && reading_stem_length >= stem_length {
        main_text = reading.clone();
        stem_length = reading_stem_length;
        let new_reading = format!(
            "{}{}",
            &source[..stem_length as usize],
            &reading[stem_length as usize..]
        );
        reading = new_reading;
    }
    let mut segments: Vec<FuriganaSegment> = vec![];
    if stem_length > 0 {
        main_text = format!(
            "{}{}",
            &source[..stem_length as usize],
            &main_text[stem_length as usize..]
        );
        let segments2 = distribute_furigana(main_text.clone(), reading);
        let mut consumed = 0;
        for segment in segments2 {
            let text = &segment.text;
            let start = consumed;
            consumed += text.len();
            if consumed < stem_length as usize {
                segments.push(segment);
            } else if consumed == stem_length as usize {
                segments.push(segment);
                break;
            } else {
                if start < stem_length as usize {
                    segments.push(FuriganaSegment::create_furigana_segment(
                        main_text[start..stem_length as usize].to_string(),
                        None,
                    ));
                }
                break;
            }
        }
    }

    if stem_length < source.len() as u32 {
        let remainder = &source[stem_length as usize..];
        let segment_count = segments.len();
        if segment_count > 0 && segments[segment_count - 1].reading.is_some() {
            // Append to the last segment if it has an empty reading
            segments[segment_count - 1].text.push_str(remainder);
        } else {
            // Otherwise, create a new segment
            segments.push(FuriganaSegment::create_furigana_segment(
                remainder.to_string(),
                None,
            ));
        }
    }

    segments
}

pub fn is_emphatic_code_point(code_point: u32) -> bool {
    code_point == HIRAGANA_SMALL_TSU_CODE_POINT
        || code_point == KATAKANA_SMALL_TSU_CODE_POINT
        || code_point == KANA_PROLONGED_SOUND_MARK_CODE_POINT
}

pub fn collapse_emphatic_sequences(text: String, full_collapse: bool) -> String {
    let mut left = 0;
    while left < text.len() && is_emphatic_code_point(text.code_point_at(left).unwrap_or_default())
    {
        left += 1;
    }

    let mut right = text.len() as isize - 1;
    while right >= 0
        && is_emphatic_code_point(text.code_point_at(right as usize).unwrap_or_default())
    {
        right -= 1;
    }

    // Whole string is emphatic
    if left > right as usize {
        return text;
    }

    let leading_emphatics = text[..left].to_string();
    let trailing_emphatics = text[(right as usize + 1)..].to_string();
    let mut middle = String::new();
    let mut current_collapsed_code_point = -1i32;

    for i in left..=right as usize {
        let char = &text[i..i + 1];
        let code_point = char.code_point_at(0).unwrap_or_default() as i32;
        if is_emphatic_code_point(code_point as u32) {
            if current_collapsed_code_point != code_point {
                current_collapsed_code_point = code_point;
                if !full_collapse {
                    middle.push_str(char);
                    continue;
                }
            }
        } else {
            current_collapsed_code_point = -1;
            middle.push_str(char);
        }
    }

    format!("{}{}{}", leading_emphatics, middle, trailing_emphatics)
}
