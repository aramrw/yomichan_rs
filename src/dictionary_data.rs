use crate::dictionary::{PhoneticTranscription, VecNumOrNum};
use crate::dictionary_importer::{FrequencyMode, StructuredContent};
use crate::structured_content::ImageElement;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::string::String;

#[derive(Serialize, Deserialize, Debug)]
pub struct TermEntry {
    pub dictionary: String,
    pub expression: String,
    pub reading: String,
    pub sequence: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TermGlossaryType {
    Text,
    Image,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryImage {
    pub term_glossary_type: TermGlossaryType,
    pub term_image: TermImage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermImage {
    pub image_element_base: ImageElement,
    pub vertical_align: Option<()>,
    pub border: Option<()>,
    pub border_radius: Option<()>,
    pub size_units: Option<()>,
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
    ///
    /// Versions can include: `1 - 3`.
    pub version: Option<u8>,
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
    pub tag_meta: Option<HashMap<String, IndexTag>>,
}

#[deprecated(since = "0.0.1", note = "individual tag files should be used instead")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Tag information for terms and kanji.
///
/// This object is deprecated, and individual tag files should be used instead.
pub struct IndexTagMeta {
    pub tags: HashMap<String, IndexTag>,
}

#[deprecated(since = "0.0.1", note = "individual tag files should be used instead")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Tag information for terms and kanji.
///
/// This object is deprecated, and individual tag files should be used instead.
pub struct IndexTag {
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Information about a single tag.
pub struct Tag {
    /// Tag name.
    pub name: String,
    /// Category for the tag.
    pub category: String,
    /// Sorting order for the tag.
    pub order: i8,
    /// Notes for the tag.
    pub notes: String,
    /// Score used to determine popularity.
    ///
    /// Negative values are more rare and positive values are more frequent.
    /// This score is also used to sort search results.
    pub score: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TermGlossary {
    Content(Box<TermGlossaryContent>),
    Deinflection(TermGlossaryDeinflection),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryContent {
    pub term_glossary_string: String,
    pub term_glossary_text: TermGlossaryText,
    pub term_glossary_image: TermGlossaryImage,
    pub term_glossary_structured_content: TermGlossaryStructuredContent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryText {
    pub term_glossary_type: TermGlossaryType,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Represents the structured content of a term.
///
/// An entry's entire HTML [`StructuredContent`]is [`Deserialize`]d into a String and pushed into `content`.
/// As such, it is up to the program to render `content` properly.
///
/// If the program is unable/unwilling to render html:
/// See: [`TermV4`]
pub struct TermGlossaryStructuredContent {
    content: String,
}

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
/// Allows access to `entry` data _(ie: definitions)_-
/// as concatenated String instead of raw HTML.
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// TermGlossaryDeinflection represents the deinflection information of a term.
pub struct TermGlossaryDeinflection {
    uninflected: String,
    inflection_rule_chain: Vec<String>,
}

/************* Term Meta *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMeta {
    pub expression: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaDataMatchType,
}

/// The metadata of a term.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermMetaDataMatchType {
    Frequency(TermMetaFrequencyDataType),
    Pitch(TermMetaPitchData),
    Phonetic(TermMetaPhoneticData),
}

/// The metadata of a term.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermMetaMatchType {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
impl<'de> Deserialize<'de> for TermMetaDataMatchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .string(|str| {
                Ok(TermMetaDataMatchType::Frequency(
                    TermMetaFrequencyDataType::Generic(GenericFrequencyData::String(
                        str.to_string(),
                    )),
                ))
            })
            .u32(|int| {
                Ok(TermMetaDataMatchType::Frequency(
                    TermMetaFrequencyDataType::Generic(GenericFrequencyData::Integer(int)),
                ))
            })
            .map(|map| {
                let value = map.deserialize::<serde_json::Value>()?;
                #[allow(clippy::if_same_then_else)]
                if value.get("frequency").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaDataMatchType::Frequency)
                        .map_err(serde::de::Error::custom)
                } else if value.get("value").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaDataMatchType::Frequency)
                        .map_err(serde::de::Error::custom)
                } else if value.get("pitches").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaDataMatchType::Pitch)
                        .map_err(serde::de::Error::custom)
                } else if value.get("transcriptions").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaDataMatchType::Phonetic)
                        .map_err(serde::de::Error::custom)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Unknown term meta data type: {:?}",
                        value
                    )))
                }
            })
            .deserialize(deserializer)
}

/// A helper Enum to select the mode for TermMeta data structures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermMetaModeType {
    Freq,
    Pitch,
    Ipa,
}

/************* Frequency *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The frequency metadata of a term.
pub struct TermMetaFrequency {
    expression: String,
    /// This will be `"freq"` in the json.
    mode: TermMetaModeType,
    data: TermMetaFrequencyDataType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermMetaFrequencyDataType {
    Generic(GenericFrequencyData),
    WithReading(TermMetaFrequencyDataWithReading),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
impl<'de> Deserialize<'de> for TermMetaFrequencyDataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .string(|str| {
                Ok(TermMetaFrequencyDataType::Generic(
                    GenericFrequencyData::String(str.to_string()),
                ))
            })
            .u32(|int| {
                Ok(TermMetaFrequencyDataType::Generic(
                    GenericFrequencyData::Integer(int),
                ))
            })
            .map(|map| {
                let value = map.deserialize::<serde_json::Value>()?;
                if value.get("reading").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaFrequencyDataType::WithReading)
                        .map_err(serde::de::Error::custom)
                } else if value.get("value").is_some() {
                    serde_json::from_value(value)
                        .map(TermMetaFrequencyDataType::Generic)
                        .map_err(serde::de::Error::custom)
                } else {
                    Err(serde::de::Error::custom("Unknown term meta data type"))
                }
            })
            .deserialize(deserializer)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GenericFrequencyData {
    Integer(u32),
    String(String),
    Object {
        value: u32,
        displayValue: Option<String>,
    },
}

impl<'de> Deserialize<'de> for GenericFrequencyData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .string(|str| Ok(GenericFrequencyData::String(str.to_string())))
            .u32(|int| Ok(GenericFrequencyData::Integer(int)))
            .map(|map| {
                let obj = map.deserialize::<serde_json::Value>()?;
                let value: u32 =
                    obj.get("value").and_then(|v| v.as_u64()).ok_or_else(|| {
                        serde::de::Error::custom("Missing or invalid 'value' field")
                    })? as u32;

                let display_value = if let Some(display_value) = obj.get("displayValue") {
                    display_value.as_str().map(String::from)
                } else {
                    None
                };

                Ok(GenericFrequencyData::Object {
                    value,
                    display_value,
                })
            })
            .deserialize(deserializer)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaFrequencyDataWithReading {
    reading: String,
    frequency: GenericFrequencyData,
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
    position: u8,
    /// Positions of a morae with nasal sound.
    nasal: Option<VecNumOrNum>,
    /// Positions of morae with devoiced sound.
    devoice: Option<VecNumOrNum>,
    /// List of tags for this pitch accent.
    /// This typically corresponds to a certain type of part of speech.
    tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The pitch data of a term.
pub struct TermMetaPitchData {
    pub reading: String,
    pub pitches: Vec<Pitch>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaPhoneticData {
    reading: String,
    /// List of different IPA transcription information for the term and reading combination.
    transcriptions: Vec<PhoneticTranscription>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaPhonetic {
    expression: String,
    mode: TermMetaModeType,
    data: String,
}

/************* Kanji Data *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiMetaFrequency {
    character: String,
    mode: TermMetaModeType,
    data: GenericFrequencyData,
}
