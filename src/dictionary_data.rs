use crate::structured_content::ImageElementBase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// dictionary_data.rs
#[derive(Serialize, Deserialize, Debug)]
pub struct TermEntry {
    pub dictionary: String,
    pub expression: String,
    pub reading: String,
    pub sequence: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermGlossaryType {
    Text,
    Image,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermGlossary {
    Content(TermGlossaryContent),
    Deinflection(TermGlossaryDeinflection),
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryContent {
    pub term_glossary_string: String,
    pub term_glossary_text: TermGlossaryText,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryText {
    pub term_glossary_type: TermGlossaryType,
    pub text: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermGlossaryImage {
    pub term_glossary_type: TermGlossaryType,
    pub term_image: TermImage,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermImage {
    pub image_element_base: ImageElementBase,
    pub vertical_align: Option<()>,
    pub border: Option<()>,
    pub border_radius: Option<()>,
    pub size_units: Option<()>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexTagMeta {
    pub tags: HashMap<String, IndexTag>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// IndexTag represents the metadata of a tag in a dictionary.
pub struct IndexTag {
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    name: String,
    category: String,
    order: u16,
    notes: String,
    score: u16,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermV3 {
    expression: String,
    reading: String,
    definition_tags: Option<String>,
    rules: String,
    score: u16,
    glossary: Vec<TermGlossary>,
    sequence: u64,
    term_tags: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermGlossaryDeinflection represents the deinflection information of a term.
pub struct TermGlossaryDeinflection {
    uninflected: String,
    inflection_rule_chain: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermGlossaryStructuredContent represents the structured content of a term.
pub struct TermGlossaryStructuredContent {
    content: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermMeta represents the metadata of a term.
pub enum TermMeta {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// TermMetaFrequency represents the frequency metadata of a term.
pub struct TermMetaFrequency {
    expression: String,
    mode: String,
    data: GenericFrequencyData,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// GenericFrequencyData represents the frequency data of a term.
pub enum GenericFrequencyData {
    Value(u16),
    DisplayValue(String),
    Reading(String),
}

