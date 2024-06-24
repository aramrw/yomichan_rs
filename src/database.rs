use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFrequencyData, TermGlossary, TermMetaFrequencyDataType, TermMetaModeType,
    TermMetaPhoneticData, TermMetaPitchData,
};
use crate::errors;
use crate::Yomichan;

//use redb::TableDefinition;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ! Javascript's `ArrayBuffer` most likely fundamentally differs from any Rust ds'
// ! Should look up how to map these before continuing
// ! So media can be displayed (or at least stored) within the lib
// TODO https://github.com/themoeway/yomitan/blob/8eea3661714c64857aa32a8662f7cca6674dd3a4/types/ext/dictionary-database.d.ts#L41
#[derive(Serialize, Deserialize, Debug)]
pub struct MediaDataBase<TContentType> {
    dictionary: String,
    path: String,
    media_type: String,
    width: u16,
    height: u16,
    content: TContentType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseTermEntry {
    expression: String,
    reading: String,
    expression_reverse: String,
    reading_reverse: String,
    definition_tags: Option<String>,
    tags: String,
    rules: String,
    score: u16,
    glossary: Vec<TermGlossary>,
    sequence: i64,
    term_tags: String,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TermEntry {
    index: u16,
    match_type: TermSourceMatchType,
    match_source: TermSourceMatchSource,
    term: String,
    reading: String,
    definition_tags: String,
    term_tags: Vec<String>,
    rules: Vec<String>,
    definitions: Vec<TermGlossary>,
    score: u16,
    dictionary: String,
    id: u128,
    sequence: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseKanjiEntry {
    character: String,
    onyomi: String,
    kunyomi: String,
    tags: String,
    meanings: Vec<String>,
    dictionary: String,
    stats: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KanjiEntry {
    index: i32,
    character: String,
    onyomi: Vec<String>,
    kunyomi: Vec<String>,
    tags: Vec<String>,
    definitions: Vec<String>,
    stats: std::collections::HashMap<String, String>,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    name: String,
    category: String,
    order: i32,
    notes: String,
    score: i32,
    dictionary: String,
}

