use crate::database::dictionary_database::{DBMetaType, TermMetaPhoneticData};
use crate::database::dictionary_importer::FrequencyMode;
use crate::dictionary::VecNumOrNum;
// use crate::dictionary::{PhoneticTranscription, VecNumOrNum};
use crate::structured_content::{ContentMatchType, ImageElement, StructuredContent, TermGlossary};
use native_db::{Key, ToKey};

use bimap::BiHashMap;
use indexmap::IndexMap;
use serde_untagged::UntaggedEnumVisitor;

use serde::{Deserialize, Deserializer, Serialize};
use std::string::String;

use std::sync::LazyLock;

trait StrMacro {
    fn from_static_str(s: &'static ::core::primitive::str) -> Self;
}
impl StrMacro for &::core::primitive::str {
    fn from_static_str(s: &'static ::core::primitive::str) -> Self {
        s
    }
}
impl StrMacro for ::std::string::String {
    fn from_static_str(s: &'static ::core::primitive::str) -> Self {
        ::std::borrow::ToOwned::to_owned(s)
    }
}
macro_rules! str {
    ($s:literal) => {
        StrMacro::from_static_str($s)
    };
}

fn main() {
    let _: &str = str!("foo");
    let _: String = str!("foo");
    let _: &'static str = str!("foo");
}

#[rustfmt::skip]
pub static KANA_MAP: LazyLock<BiHashMap<&'static str, &'static str>> = LazyLock::new(|| {
    BiHashMap::from_iter([
        ("ア", "あ"), ("イ", "い"), ("ウ", "う"), ("エ", "え"), ("オ", "お"),
        ("カ", "か"), ("キ", "き"), ("ク", "く"), ("ケ", "け"), ("コ", "こ"),
        ("サ", "さ"), ("シ", "し"), ("ス", "す"), ("セ", "せ"), ("ソ", "そ"),
        ("タ", "た"), ("チ", "ち"), ("ツ", "つ"), ("テ", "て"), ("ト", "と"),
        ("ナ", "な"), ("ニ", "に"), ("ヌ", "ぬ"), ("ネ", "ね"), ("ノ", "の"),
        ("ハ", "は"), ("ヒ", "ひ"), ("フ", "ふ"), ("ヘ", "へ"), ("ホ", "ほ"),
        ("マ", "ま"), ("ミ", "み"), ("ム", "む"), ("メ", "め"), ("モ", "も"),
        ("ヤ", "や"), ("ユ", "ゆ"), ("ヨ", "よ"), ("ラ", "ら"), ("リ", "り"),
        ("ル", "る"), ("レ", "れ"), ("ロ", "ろ"), ("ワ", "わ"), ("ヲ", "を"),
        ("ン", "ん"), ("ガ", "が"), ("ギ", "ぎ"), ("グ", "ぐ"), ("ゲ", "げ"),
        ("ゴ", "ご"), ("ザ", "ざ"), ("ジ", "じ"), ("ズ", "ず"), ("ゼ", "ぜ"),
        ("ゾ", "ぞ"), ("ダ", "だ"), ("ヂ", "ぢ"), ("ヅ", "づ"), ("デ", "で"),
        ("ド", "ど"), ("バ", "ば"), ("ビ", "び"), ("ブ", "ぶ"), ("ベ", "べ"),
        ("ボ", "ぼ"), ("パ", "ぱ"), ("ピ", "ぴ"), ("プ", "ぷ"), ("ペ", "ぺ"),
        ("ポ", "ぽ"),   ("キャ", "きゃ"), ("キュ", "きゅ"), ("キョ", "きょ"),
        ("シャ", "しゃ"), ("シュ", "しゅ"), ("ショ", "しょ"), ("チャ", "ちゃ"),
        ("チュ", "ちゅ"), ("チョ", "ちょ"), ("ニャ", "にゃ"), ("ニュ", "にゅ"),
        ("ニョ", "にょ"), ("ヒャ", "ひゃ"), ("ヒュ", "ひゅ"), ("ヒョ", "ひょ"),
        ("ミャ", "みゃ"), ("ミュ", "みゅ"), ("ミョ", "みょ"), ("リャ", "りゃ"),
        ("リュ", "りゅ"), ("リョ", "りょ"),  ("ギャ", "ぎゃ"), ("ギュ", "ぎゅ"),
        ("ギョ", "ぎょ"), ("ジャ", "じゃ"), ("ジュ", "じゅ"), ("ジョ", "じょ"),
        ("ビャ", "びゃ"), ("ビュ", "びゅ"), ("ビョ", "びょ"), ("ピャ", "ぴゃ"),
        ("ピュ", "ぴゅ"), ("ピョ", "ぴょ"),
    ])
});

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TermEntry {
//     pub dictionary: String,
//     pub expression: String,
//     pub reading: String,
//     pub sequence: Option<String>,
// }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TermGlossaryType {
    Text,
    Image,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryImage {
    pub term_glossary_type: TermGlossaryType,
    pub term_image: Option<ImageElement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Represents the metadata of a dictionary.
pub struct Index {
    /// Title of the dictionary.
    pub title: String,
    /// Revision of the dictionary.
    ///
    /// This value is only used for displaying information.
    pub revision: String,
    /// Whether or not this dictionary contains sequencing information for related terms.
    pub sequenced: Option<bool>,
    /// Format of data found in the JSON data files.
    pub format: Option<u8>,
    /// Alias for format.
    /// Versions can include: `1 - 3`.
    pub version: Option<u8>,
    pub minimum_yomitan_version: Option<String>,
    pub is_updatable: Option<bool>,
    pub index_url: Option<String>,
    pub download_url: Option<String>,
    /// Creator of the dictionary.
    pub author: Option<String>,
    /// URL for the source of the dictionary.
    pub url: Option<String>,
    /// Description of the dictionary data.
    pub description: Option<String>,
    /// Attribution information for the dictionary data.
    pub attribution: Option<String>,
    /// Language of the terms in the dictionary.
    ///
    /// See: [iso639 code list](https://www.loc.gov/standards/iso639-2/php/code_list.php).
    pub source_language: Option<String>,
    /// Main language of the definitions in the dictionary.
    ///
    /// See: [iso639 code list](https://www.loc.gov/standards/iso639-2/php/code_list.php).
    pub target_language: Option<String>,
    pub frequency_mode: Option<FrequencyMode>,
    pub tag_meta: Option<IndexMap<String, IndexTag>>,
}

// #[deprecated(since = "0.0.1", note = "individual tag files should be used instead")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Tag information for terms and kanji.
/// This object is deprecated, and individual tag files should be used instead.
pub struct IndexTagMeta {
    pub tags: IndexMap<String, IndexTag>,
}

#[deprecated(since = "0.0.1", note = "individual tag files should be used instead")]
/// Tag information for terms and kanji.
///
/// This object is deprecated, and individual tag files should be used instead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexTag {
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Information about a single tag.
pub struct DictionaryDataTag {
    /// Tag name.
    pub name: String,
    /// Category for the tag.
    pub category: String,
    /// Sorting order for the tag.
    pub order: u64,
    /// Notes for the tag.
    pub notes: String,
    /// Score used to determine popularity.
    ///
    /// Negative values are more rare and positive values are more frequent.
    /// This score is also used to sort search results.
    pub score: i128,
}

// #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
// pub struct TermGlossaryText {
//     pub term_glossary_type: TermGlossaryType,
//     pub text: String,
// }

/// Yomichan-like term model.
///
/// Because of how Yomichan is designed, the definition's raw HTML is contained in
/// [`TermGlossaryContent::term_glossary_structured_content`]/`content` as a String.
///
/// If the program is unable/unwilling to render HTML:
/// See: [`TermV4`]
///
/// Related: [`TermGlossaryContent`]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TermV3 {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub glossary: Vec<TermGlossary>,
    pub sequence: i64,
    pub term_tags: String,
}

/// Custom `Yomichan.rs`-unique term model.
/// Allows access to `entry` data _(ie: definitions)_ as a concatenated String instead of raw HTML.
///
/// The String data is simply extracted and concatenated-
/// meaning that there is _no_ formatting; A single string of continuous text.
///
/// If the program _is_ able to render html, this may be preferable:
/// See: [`TermV3`]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TermV4 {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>,
    pub rules: String,
    pub score: i8,
    pub definition: String,
    pub sequence: i128,
    pub term_tags: String,
}

/************* Term Meta *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMeta {
    pub expression: String,
    pub mode: TermMetaModeType,
    pub data: MetaDataMatchType,
}

/// The metadata of a term.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MetaDataMatchType {
    Frequency(TermMetaFreqDataMatchType),
    Pitch(TermMetaPitchData),
    Phonetic(TermMetaPhoneticData),
}

impl<'de> Deserialize<'de> for MetaDataMatchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .string(|str| {
                Ok(MetaDataMatchType::Frequency(
                    TermMetaFreqDataMatchType::Generic(GenericFreqData::String(str.to_string())),
                ))
            })
            .i128(|int| {
                Ok(MetaDataMatchType::Frequency(
                    TermMetaFreqDataMatchType::Generic(GenericFreqData::Integer(int)),
                ))
            })
            .map(|map| {
                let value = map.deserialize::<serde_json::Value>()?;
                #[allow(clippy::if_same_then_else)]
                if value.get("frequency").is_some() {
                    serde_json::from_value(value)
                        .map(MetaDataMatchType::Frequency)
                        .map_err(serde::de::Error::custom)
                } else if value.get("value").is_some() {
                    serde_json::from_value(value)
                        .map(MetaDataMatchType::Frequency)
                        .map_err(serde::de::Error::custom)
                } else if value.get("pitches").is_some() {
                    serde_json::from_value(value)
                        .map(MetaDataMatchType::Pitch)
                        .map_err(serde::de::Error::custom)
                } else if value.get("transcriptions").is_some() {
                    serde_json::from_value(value)
                        .map(MetaDataMatchType::Phonetic)
                        .map_err(serde::de::Error::custom)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Unknown term meta data type: {value:?}"
                    )))
                }
            })
            .deserialize(deserializer)
    }
}

/// A helper Enum to select the mode for TermMeta data structures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermMetaModeType {
    Freq,
    Pitch,
    Ipa,
}
impl From<TermMetaModeType> for u8 {
    fn from(value: TermMetaModeType) -> Self {
        match value {
            TermMetaModeType::Freq => 0,
            TermMetaModeType::Pitch => 1,
            TermMetaModeType::Ipa => 2,
        }
    }
}

/************* Frequency *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The frequency metadata of a term.
///
/// This is currently use to [`Deserialize`] terms from
/// term_meta_bank_$ files.
pub struct TermMetaFrequency {
    pub expression: String,
    // Should be changed to serde_tag instead of an enum.
    /// This will be `"freq"` in the json.
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrequencyInfo {
    pub frequency: i128,
    pub display_value: Option<String>,
    pub display_value_parsed: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
//#[serde(untagged)]
pub enum TermMetaFreqDataMatchType {
    WithReading(TermMetaFreqDataWithReading),
    Generic(GenericFreqData),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
//#[serde(untagged)]
pub enum GenericFreqData {
    Object(FreqObjectData),
    Integer(i128),
    String(String),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FreqObjectData {
    pub value: i128,
    #[serde(rename = "displayValue")]
    pub display_value: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaFreqDataWithReading {
    pub reading: String,
    pub frequency: GenericFreqData,
}

impl GenericFreqData {
    pub fn try_get_reading(&self) -> Option<&String> {
        match self {
            Self::Integer(_) => None,
            Self::String(str) => Some(str),
            Self::Object(obj) => obj.display_value.as_ref(),
            //Self::WithReading(wr) => Some(&wr.reading),
        }
    }
}

/************* Pitch / Speech Data *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The pitch metadata of a term.
pub struct TermMetaPitch {
    expression: String,
    /// This will be `"pitch"` in the json.
    mode: TermMetaModeType,
    data: TermMetaPitchData,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// List of different pitch accent information for the term and reading combination.
pub struct Pitch {
    /// Mora position of the pitch accent downstep.
    /// A value of 0 indicates that the word does not have a downstep (heiban).
    pub position: u8,
    /// Positions of a morae with nasal sound.
    pub nasal: Option<VecNumOrNum>,
    /// Positions of morae with devoiced sound.
    pub devoice: Option<VecNumOrNum>,
    /// List of tags for this pitch accent.
    /// This typically corresponds to a certain type of part of speech.
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The pitch data of a term.
pub struct TermMetaPitchData {
    pub reading: String,
    pub pitches: Vec<Pitch>,
}

/************* Kanji Data *************/

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub struct KanjiMetaFrequency {
//     character: String,
//     mode: TermMetaModeType,
//     data: GenericFreqData,
// }

pub mod dictionary_data_util {
    use fancy_regex::Regex;
    use std::sync::LazyLock;
    use url::{ParseError as UrlParseError, Url};

    pub static SIMPLE_VERSION_TEST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\d+\.)*\d+$").unwrap());

    pub fn compare_revisions(current: &str, latest: &str) -> bool {
        // If either string doesn't match the simple version format,
        // fall back to a lexicographical string comparison.
        if !SIMPLE_VERSION_TEST.is_match(current).unwrap()
            || !SIMPLE_VERSION_TEST.is_match(latest).unwrap()
        {
            return current < latest;
        }

        // The regex ensures all parts are digits, so `unwrap()` is safe here.
        let current_parts: Vec<u32> = current
            .split('.')
            .map(|part| part.parse::<u32>().unwrap())
            .collect();

        let latest_parts: Vec<u32> = latest
            .split('.')
            .map(|part| part.parse::<u32>().unwrap())
            .collect();

        // This logic is from the original JS: if the number of parts is
        // different, fall back to a string comparison. This can cause
        // unexpected results (e.g., "1.5" vs "1.20" would be false).
        if current_parts.len() != latest_parts.len() {
            return current < latest;
        }

        // Compare each version part numerically.
        for i in 0..current_parts.len() {
            if current_parts[i] != latest_parts[i] {
                return current_parts[i] < latest_parts[i];
            }
        }
        false
    }

    pub fn validate_url(s: &str) -> Result<(), UrlParseError> {
        let Err(e) = Url::parse(s) else {
            return Ok(());
        };
        Err(e)
    }
}
