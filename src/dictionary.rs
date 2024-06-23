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

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictionaryEntryType {
    Kanji,
    Term,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InflectionSource {
    Algorithm,
    Dictionary,
    Both,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pronunciation {
    PitchAccent(PitchAccent),
    PhoneticTranscription(PhoneticTranscription),
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictionaryEntry {
    KanjiDictEntry(KanjiDictionaryEntry),
    TermDictEntry(TermDictionaryEntry),
}

// structs

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PitchAccent {
    term: TermPronunciationMatchType,
    position: u8,
    nasal_positions: u8,
    devoic_positions: u8,
    tags: Vec<Tag>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneticTranscription {
    match_type: TermPronunciationMatchType,
    ipa: String,
    tags: Vec<Tag>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InflectionRuleChainCandidate {
    source: InflectionSource,
    inflection_rules: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A tag represents some brief information about part of a dictionary entry.
pub struct Tag {
    /// The name of the tag.
    name: String,
    /// The category of the tag.
    category: String,
    /// A number indicating the sorting order of the tag.
    order: u16,
    /// A score value for the tag.
    score: u16,
    /// An array of descriptions for the tag. If there are multiple entries,
    /// the values will typically have originated from different dictionaries.
    /// However, there is no correlation between the length of this array and
    /// the length of the `dictionaries` field, as duplicates are removed.
    content: Vec<String>,
    /// An array of dictionary names that contained a tag with this name and category.
    dictionaries: Vec<String>,
    /// Whether or not this tag is redundant with previous tags.
    redundant: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiStat {
    name: String,
    category: String,
    content: String,
    order: u16,
    score: u64,
    dictionary: String,
    value: NumOrStr,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryOrder {
    index: u16,
    priority: u16,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiFrequency {
    index: u64,
    dictionary: String,
    dictionary_index: u16,
    dictionary_priority: u16,
    character: String,
    frequency: NumOrStr,
    display_value: Option<String>,
    display_value_parsed: bool,
}

pub type KanjiStatGroups = HashMap<String, Vec<KanjiStat>>;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiDictionaryEntry {
    entry_type: DictionaryEntryType,
    character: String,
    dictionary: String,
    onyomi: Vec<String>,
    kunyomi: Vec<String>,
    tags: Vec<Tag>,
    stats: KanjiStatGroups,
    definitions: Vec<String>,
    frequencies: Vec<KanjiFrequency>,
}

/// Frequency information corresponds to how frequently a term appears in a corpus,
/// which can be a number of occurrences or an overall rank.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermFrequency {
    index: u32,
    headword_index: u32,
    dictionary: String,
    dictionary_index: u16,
    dictionary_priority: u16,
    has_reading: bool,
    frequency: u32,
    display_value: Option<String>,
    display_value_parsed: bool,
}

