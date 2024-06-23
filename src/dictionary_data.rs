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

