use crate::dictionary_data::TermGlossaryContent;
use crate::errors;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;
use std::io::{self, BufRead, BufReader};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumOrStr {
    Num(u64),
    Str(String),
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Enum representing what database field was used to match the source term.
pub enum TermSourceMatchSource {
    Term,
    Reading,
    Sequence,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Enum representing how the search term relates to the final term.
pub enum TermSourceMatchType {
    Exact,
    Prefix,
    Suffix,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TermPronunciationMatchType {
    PitchAccent,
    PhoneticTranscription,
}

