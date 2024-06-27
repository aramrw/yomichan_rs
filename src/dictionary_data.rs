use crate::dictionary::PhoneticTranscription;
use crate::structured_content::ImageElementBase;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct TermEntry {
    pub dictionary: String,
    pub expression: String,
    pub reading: String,
    pub sequence: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermGlossaryType {
    Text,
    Image,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermGlossary {
    Content(Box<TermGlossaryContent>),
    Deinflection(TermGlossaryDeinflection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryContent {
    pub term_glossary_string: String,
    pub term_glossary_text: TermGlossaryText,
    pub term_glossary_image: TermGlossaryImage,
    pub term_glossary_structured_content: TermGlossaryStructuredContent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryText {
    pub term_glossary_type: TermGlossaryType,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryImage {
    pub term_glossary_type: TermGlossaryType,
    pub term_image: TermImage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermImage {
    pub image_element_base: ImageElementBase,
    pub vertical_align: Option<()>,
    pub border: Option<()>,
    pub border_radius: Option<()>,
    pub size_units: Option<()>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Index represents the metadata of a dictionary.
pub struct Index {
    format: Option<u8>,
    version: Option<u8>,
    title: String,
    revision: String,
    sequenced: Option<bool>,
    author: Option<String>,
    url: Option<String>,
    description: Option<String>,
    attribution: Option<String>,
    source_language: Option<String>,
    target_language: Option<String>,
    frequency_mode: Option<String>,
    tag_meta: Option<HashMap<String, IndexTag>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexTagMeta {
    pub tags: HashMap<String, IndexTag>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// IndexTag represents the metadata of a tag in a dictionary.
pub struct IndexTag {
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    name: String,
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermV3 {
    expression: String,
    reading: String,
    definition_tags: Option<String>,
    rules: String,
    score: u16,
    glossary: Vec<TermGlossary>,
    sequence: i64,
    term_tags: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermGlossaryDeinflection represents the deinflection information of a term.
pub struct TermGlossaryDeinflection {
    uninflected: String,
    inflection_rule_chain: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermGlossaryStructuredContent represents the structured content of a term.
pub struct TermGlossaryStructuredContent {
    content: String,
}

/************* Term Meta *************/

/// A helper Enum to select the mode for TermMeta data structures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermMetaModeType {
    Pitch,
    Freq,
    Ipa,
}

/// A helper Enum to select the type of data for TermMetaFrequency data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermMetaFrequencyDataType {
    Generic(GenericFrequencyData),
    WithReading(TermMetaFrequencyDataWithReading),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// GenericFrequencyData represents the frequency data of a term.
pub enum GenericFrequencyData {
    Value(u16),
    DisplayValue(String),
    Reading(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// The metadata of a term.
pub enum TermMeta {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMetaFrequencyDataWithReading {
    reading: String,
    frequency: GenericFrequencyData,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// The frequency metadata of a term.
pub struct TermMetaFrequency {
    expression: String,
    mode: TermMetaModeType,
    data: TermMetaFrequencyDataType,
}

/************* Pitch / Speech Data *************/

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Represents the pitch of a term.
pub struct Pitch {
    position: u16,
    nasal: Option<Vec<u16>>,
    devoice: Option<Vec<u16>>,
    tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// The pitch data of a term.
pub struct TermMetaPitchData {
    reading: String,
    pitches: Vec<Pitch>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// The pitch metadata of a term.
pub struct TermMetaPitch {
    expression: String,
    mode: TermMetaModeType,
    data: TermMetaPitchData,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMetaPhoneticData {
    reading: String,
    transcriptions: Vec<PhoneticTranscription>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMetaPhonetic {
    expression: String,
    mode: TermMetaModeType,
    data: String,
}

/************* Kanji Data *************/

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiMetaFrequency {
    character: String,
    mode: TermMetaModeType,
    data: GenericFrequencyData,
}
