// Assuming these types will be defined elsewhere in your Rust codebase
// For example:
// pub mod dictionary {
//     pub type TermSourceMatchType = String; // Placeholder
// }
// pub type SearchResolution = String; // Placeholder

use fancy_regex::Regex;
use indexmap::{IndexMap, IndexSet};
use language_transformer::language_d::FindTermsTextReplacements;

use crate::{
    database::dictionary_database::DictionarySet, dictionary::TermSourceMatchType,
    settings::SearchResolution,
};

// Kanji

/// An options object for use with `Translator.find_kanji`.
#[derive(Debug, Clone)]
pub struct FindKanjiOptions {
    /// The mapping of dictionaries to search for kanji in.
    /// The key is the dictionary name.
    pub enabled_dictionary_map: KanjiEnabledDictionaryMap,
    /// Whether or not non-Japanese characters should be searched.
    pub remove_non_japanese_characters: bool,
}

/// Details about a dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct FindKanjiDictionary {
    /// The index of the dictionary
    pub index: usize,
    /// The alias of the dictionary
    pub alias: String,
}

// Terms

/// An options object for use with `Translator.find_terms`.
#[derive(Debug, Clone)]
pub struct FindTermsOptions {
    /// The matching type for looking up terms.
    pub match_type: FindTermsMatchType,
    /// Whether or not deinflection should be performed.
    pub deinflect: bool,
    /// The reading which will be sorted to the top of the results.
    pub primary_reading: String,
    /// The name of the primary dictionary to search.
    pub main_dictionary: String,
    /// The name of the frequency dictionary used for sorting
    pub sort_frequency_dictionary: Option<String>,
    /// The order used when using a sorting dictionary.
    pub sort_frequency_dictionary_order: FindTermsSortOrder,
    /// Whether or not non-Japanese characters should be searched.
    pub remove_non_japanese_characters: bool,
    /// An iterable sequence of text replacements to be applied during the term lookup process.
    pub text_replacements: FindTermsTextReplacements,
    /// The mapping of dictionaries to search for terms in.
    /// The key is the dictionary name.
    pub enabled_dictionary_map: TermEnabledDictionaryMap,
    /// A set of dictionary names which should have definitions removed.
    pub exclude_dictionary_definitions: Option<IndexSet<String>>,
    /// Whether every substring should be searched for, or only whole words.
    pub search_resolution: SearchResolution,
    /// ISO-639 code of the language.
    pub language: String,
}

/// The matching type for looking up terms.
pub type FindTermsMatchType = TermSourceMatchType;

/// A sorting order to use when finding terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindTermsSortOrder {
    Ascending,
    Descending,
}

// Helper to convert from string if needed, though Rust enums are typically used directly.
impl FindTermsSortOrder {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "ascending" => Ok(FindTermsSortOrder::Ascending),
            "descending" => Ok(FindTermsSortOrder::Descending),
            _ => Err(format!("Invalid FindTermsSortOrder: {s}")),
        }
    }
}

pub type FindTermDictionaryMap = IndexMap<String, FindTermDictionary>;

/// Details about a dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct FindTermDictionary {
    /// The index of the dictionary
    pub index: usize, // Or usize
    /// The alias of the dictionary
    pub alias: String,
    /// Whether or not secondary term searches are allowed for this dictionary.
    pub allow_secondary_searches: bool,
    /// Whether this dictionary's part of speech rules should be used to filter results.
    pub parts_of_speech_filter: bool,
    /// Whether to use the deinflections from this dictionary.
    pub use_deinflections: bool,
}

// impl DictionarySet  for FindTermDictionary {
//     fn has(&self, value: &str) -> bool {
//
//     }
// }

pub type TermEnabledDictionaryMap = IndexMap<String, FindTermDictionary>;
pub type KanjiEnabledDictionaryMap = IndexMap<String, FindKanjiDictionary>;
