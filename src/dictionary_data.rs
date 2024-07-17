use crate::dictionary::PhoneticTranscription;
use crate::structured_content::ImageElement;
use crate::dictionary_importer::StructuredContent;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexTagMeta {
    pub tags: HashMap<String, IndexTag>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// IndexTag represents the metadata of a tag in a dictionary.
pub struct IndexTag {
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    name: String,
    category: String,
    order: u16,
    notes: String,
    score: u16,
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
/// An entry's entire [`StructuredContent`]is [`Deserialize`]d into a String and pushed into `content`.
/// As such, it is up to the application to render `content` properly.
///
/// If the application is unable/unwilling to render html:
/// See: [`TermV4`]
pub struct TermGlossaryStructuredContent {
    content: String,
}


/// Yomichan-like term model.
///
/// Because of how Yomichan is designed, the definition's HTML is contained in
/// [`TermGlossaryContent::term_glossary_structured_content`]/`content` as a String.
///
/// If the application is unable/unwilling to render HTML:
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

/// A helper Enum to select the mode for TermMeta data structures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermMetaModeType {
    Pitch,
    Freq,
    Pitch,
    Ipa,
}

/// A helper Enum to select the type of data for TermMetaFrequency data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TermMetaFrequencyDataType {
    Generic(GenericFrequencyData),
    WithReading(TermMetaFrequencyDataWithReading),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// GenericFrequencyData represents the frequency data of a term.
/// Represents the frequency data of a term.
pub enum GenericFrequencyData {
    Value(u16),
    DisplayValue(String),
    Reading(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The metadata of a term.
pub enum TermMeta {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaFrequencyDataWithReading {
    reading: String,
    frequency: GenericFrequencyData,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The frequency metadata of a term.
pub struct TermMetaFrequency {
    expression: String,
    mode: TermMetaModeType,
    data: TermMetaFrequencyDataType,
}

/************* Pitch / Speech Data *************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Represents the pitch of a term.
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
    reading: String,
    pitches: Vec<Pitch>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// The pitch metadata of a term.
pub struct TermMetaPitch {
    expression: String,
    mode: TermMetaModeType,
    data: TermMetaPitchData,
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
