use crate::structured_content::MainStructuredContent;

use super::structured_content::{TermGlossary, TermImage};
use serde::{Deserialize, Serialize};

// This helper enum handles the three object types from TermGlossaryContent.
// Since they all have a `type` field, `#[serde(tag = "type")]` is the
// perfect, idiomatic tool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GlossaryObject {
    // Models: {type: 'text', text: string};
    #[serde(rename = "text")]
    Text { text: String },

    // Models: {type: 'image'} & TermImage;
    #[serde(rename = "image")]
    Image(TermImage),

    // Models: {type: 'structured-content', content: StructuredContent.Content};
    #[serde(rename = "structured-content")]
    StructuredContent { content: MainStructuredContent },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermV3 {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub glossary: Vec<TermGlossary>, // <-- This now uses our new, correct enum
    pub sequence: i64,
    pub term_tags: String,
}
