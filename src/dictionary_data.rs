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

