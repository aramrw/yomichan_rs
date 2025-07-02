use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize, Debug, Clone)]
struct KanjiMapping {
    oyaji: String,
    itaiji: Vec<String>,
}

// The main processor struct
#[derive(Debug, Clone)]
pub struct KanjiProcessor {
    regex: Regex,
    conversion_map: IndexMap<char, char>,
}

impl KanjiProcessor {
    /// Creates a new KanjiProcessor instance, loading and processing data.
    ///
    /// This function will panic if the embedded JSON data is invalid
    /// or if the regex cannot be compiled.
    fn new() -> Self {
        // Embed JSON at compile time. Ensure these files are in `src`.
        let mapping_list_json = include_str!("../json_lists/full_list.json");
        let itaiji_list_json = include_str!("../json_lists/itaiji_list.json");

        // Parse the JSON data
        let mappings: Vec<KanjiMapping> =
            serde_json::from_str(mapping_list_json).expect("Failed to parse full_list.json");
        let itaiji_list: Vec<String> =
            serde_json::from_str(itaiji_list_json).expect("Failed to parse itaiji_list.json");

        let regex_string: String = itaiji_list.join("");
        let regex =
            Regex::new(&format!("[{regex_string}]")).expect("Failed to compile Kanji regex");

        let mut conversion_map = IndexMap::new();
        for mapping in mappings {
            let oyaji_char = mapping
                .oyaji
                .chars()
                .next()
                .expect("Oyaji should be a single character");

            for itaiji in mapping.itaiji {
                let itaiji_char = itaiji
                    .chars()
                    .next()
                    .expect("Itaiji should be a single character");
                conversion_map.insert(itaiji_char, oyaji_char);
            }
        }

        KanjiProcessor {
            regex,
            conversion_map,
        }
    }

    /// Converts variant Kanji (itaiji) in a string to their parent (oyaji) forms.
    pub fn convert_to_parent(&self, text: &str) -> String {
        // Use replace_all with a closure to look up the replacement
        self.regex
            .replace_all(text, |caps: &regex::Captures| {
                // Get the matched character (should be the first char of the first capture group)
                let matched_char = caps[0].chars().next().unwrap_or_default();

                // Look up in the map. If found, use the parent; otherwise, use the original match.
                // We need to return something that can be part of the new string.
                // Since our map values are chars, we convert them back to strings.
                match self.conversion_map.get(&matched_char) {
                    Some(&parent_char) => parent_char.to_string(),
                    None => unreachable!(), // Should not happen if regex is built correctly
                }
            })
            .to_string() // Convert the resulting Cow<str> to a String
    }
}

// --- Global Instance and Function (like TS exports) ---

// Create a lazily initialized static instance (thread-safe)
static DEFAULT_PROCESSOR: LazyLock<KanjiProcessor> = LazyLock::new(KanjiProcessor::new);

/// Converts variant Kanji using the default processor instance.
pub fn convert_variants(text: &str) -> String {
    DEFAULT_PROCESSOR.convert_to_parent(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        // Checks if new() runs without panicking and basic map/regex checks.
        let processor = KanjiProcessor::new();
        assert!(
            !processor.conversion_map.is_empty(),
            "Conversion map should not be empty"
        );
        assert!(
            !processor.regex.as_str().is_empty(),
            "Regex should not be empty"
        );
        println!(
            "Regex contains {} chars.",
            processor.regex.as_str().chars().count()
        );
        println!("Map contains {} entries.", processor.conversion_map.len());
    }

    #[test]
    fn test_single_variant() {
        // Tests '國' -> '国'
        let input = "大日本帝國";
        let expected = "大日本帝国";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_multiple_variants() {
        // Tests '萬' -> '万', '鐵' -> '鉄', '學' -> '学', '體' -> '体'
        let input = "一萬円の鐵道で體育を學ぶ";
        let expected = "一万円の鉄道で体育を学ぶ";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_mixed_variants_and_parents() {
        // Tests a mix, ensuring parents aren't changed
        let input = "學問と学問、國と国";
        let expected = "学問と学問、国と国";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_no_variants() {
        // Tests a string with no known variants
        let input = "こんにちは世界、ABC 123";
        let expected = "こんにちは世界、ABC 123";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_empty_string() {
        let input = "";
        let expected = "";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_variants_only() {
        // Tests '舊' -> '旧', '龍' -> '竜', '靜' -> '静'
        let input = "舊龍靜";
        let expected = "旧竜静";
        assert_eq!(convert_variants(input), expected);
    }

    #[test]
    fn test_some_specific_cases() {
        // From oyaji_list.json and itaiji_list.json
        assert_eq!(convert_variants("萬"), "万"); // 万 -> 萬
        assert_eq!(convert_variants("與"), "与"); // 与 -> 與
        assert_eq!(convert_variants("龜"), "亀"); // 亀 -> 龜
        assert_eq!(convert_variants("佛"), "仏"); // 仏 -> 佛
        assert_eq!(convert_variants("傳"), "伝"); // 伝 -> 傳
        assert_eq!(convert_variants("僞"), "偽"); // 偽 -> 僞
        assert_eq!(convert_variants("圓"), "円"); // 円 -> 圓
        assert_eq!(convert_variants("寫"), "写"); // 写 -> 寫
        assert_eq!(convert_variants("劍"), "剣"); // 剣 -> 劍
        assert_eq!(convert_variants("勞"), "労"); // 労 -> 勞
        assert_eq!(convert_variants("單"), "単"); // 単 -> 單
        assert_eq!(convert_variants("臺"), "台"); // 台 -> 臺
        assert_eq!(convert_variants("國"), "国"); // 国 -> 國
        assert_eq!(convert_variants("鹽"), "塩"); // 塩 -> 鹽
        assert_eq!(convert_variants("學"), "学"); // 学 -> 學
        assert_eq!(convert_variants("壽"), "寿"); // 寿 -> 壽
        assert_eq!(convert_variants("德"), "徳"); // 徳 -> 德
        assert_eq!(convert_variants("戰"), "戦"); // 戦 -> 戰
        assert_eq!(convert_variants("戲"), "戯"); // 戯 -> 戲
        assert_eq!(convert_variants("擴"), "拡"); // 拡 -> 擴
        assert_eq!(convert_variants("搖"), "揺"); // 揺 -> 搖
        assert_eq!(convert_variants("樣"), "様"); // 様 -> 樣
        assert_eq!(convert_variants("齒"), "歯"); // 歯 -> 齒
        assert_eq!(convert_variants("氣"), "気"); // 気 -> 氣
        assert_eq!(convert_variants("澤"), "沢"); // 沢 -> 澤
        assert_eq!(convert_variants("燒"), "焼"); // 焼 -> 燒
        assert_eq!(convert_variants("獨"), "独"); // 独 -> 獨
        assert_eq!(convert_variants("畫"), "画"); // 画 -> 畫
        assert_eq!(convert_variants("發"), "発"); // 発 -> 發
        assert_eq!(convert_variants("禮"), "礼"); // 礼 -> 禮
        assert_eq!(convert_variants("稻"), "稲"); // 稲 -> 稻
        assert_eq!(convert_variants("龍"), "竜"); // 竜 -> 龍
        assert_eq!(convert_variants("絲"), "糸"); // 糸 -> 絲
        assert_eq!(convert_variants("繪"), "絵"); // 絵 -> 繪
        assert_eq!(convert_variants("續"), "続"); // 続 -> 續
        assert_eq!(convert_variants("緣"), "縁"); // 縁 -> 緣
        assert_eq!(convert_variants("聽"), "聴"); // 聴 -> 聽
        assert_eq!(convert_variants("脫"), "脱"); // 脱 -> 脫
        assert_eq!(convert_variants("藏"), "蔵"); // 蔵 -> 藏
        assert_eq!(convert_variants("證"), "証"); // 証 -> 證
        assert_eq!(convert_variants("鐵"), "鉄"); // 鉄 -> 鐵
        assert_eq!(convert_variants("顏"), "顔"); // 顔 -> 顏
        assert_eq!(convert_variants("驛"), "駅"); // 駅 -> 驛
        assert_eq!(convert_variants("魚"), "魚"); // Should be unchanged
        assert_eq!(convert_variants("𩵋"), "魚"); // Fish variant -> 魚 (If 𩵋 is in itaiji_list)
    }
}
