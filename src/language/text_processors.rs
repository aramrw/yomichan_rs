use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use super::{language_d::TextProcessor, transformer_d::LanguageTransformDescriptor};

pub const BASIC_TEXT_PROCESSOR_OPTIONS: [bool; 2] = [false, true];

fn remove_alphabetic_diacritics(text: &str, setting: bool) -> String {
    if setting {
        // Normalize the text to NFD (Normalization Form D) and collect into a String
        let normalized: String = text.nfd().collect();
        // Compile regex for diacritic marks
        let re = Regex::new(r"[\u{0300}-\u{036f}]").unwrap();
        // Remove diacritic marks
        let result = re.replace_all(&normalized, "");
        result.to_string()
    } else {
        text.to_string()
    }
}

pub const DECAPITALIZE: TextProcessor<bool, fn(&str, bool) -> String> = TextProcessor {
    name: "Decapitalize Text",
    description: "CAPITALIZED TEXT → capitalized text",
    options: &BASIC_TEXT_PROCESSOR_OPTIONS,
    process: |text: &str, setting: bool| -> String {
        if setting {
            text.to_lowercase()
        } else {
            text.to_string()
        }
    },
};

pub const CAPITALIZE_FIRST_LETTER: TextProcessor<bool, fn(&str, bool) -> String> = TextProcessor {
    name: "Capitalize First Letter",
    description: "lowercase text → Lowercase text",
    options: &BASIC_TEXT_PROCESSOR_OPTIONS,
    process: |text: &str, setting: bool| -> String {
        if setting {
            let mut str = text.to_owned();
            if let Some(r) = str.get_mut(0..1) {
                r.make_ascii_uppercase();
                return str;
            }
        }
        text.to_owned()
    },
};

pub const REMOVE_ALPHABETIC_DIACRITICS: TextProcessor<bool, fn(&str, bool) -> String> =
    TextProcessor {
        name: "Remove Alphabetic Diacritics",
        description: "ἄήé → αηe",
        options: &BASIC_TEXT_PROCESSOR_OPTIONS,
        process: remove_alphabetic_diacritics,
    };
