use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::{dictionary::TermSourceMatchType, settings::SearchResolution};

pub type KanjiEnabledDictionaryMap<'a> = HashMap<&'a str, FindKanjiDictionary>;

/// An options object for use with `Translator.findKanji`.
pub struct FindKanjiOptions<'a> {
    /// The mapping of dictionaries to search for kanji in.
    /// The key is the dictionary name.
    enabled_dictionary_map: KanjiEnabledDictionaryMap<'a>,
    /// Whether or not non-Japanese characters should be searched.
    remove_non_japanese_characters: bool,
}

/// Details about a dictionary.
pub struct FindKanjiDictionary {
    /// The index of the dictionary
    index: u8,
    /// The priority of the dictionary
    priority: u16,
}

/// A sorting order to use when finding terms.
pub enum FindTermsSortOrder {
    Ascending,
    Descending,
}

/// Information about how text should be replaced when looking up terms.
pub struct FindTermsTextReplacement {
    /// The pattern to replace.
    pattern: Regex,
    /// The replacement string. This can contain special sequences, such as `$&`.
    replacement: String,
}

pub type TermEnabledDictionaryMap<'a> = HashMap<&'a str, FindTermDictionary>;

/// Details about a dictionary.
pub struct FindTermDictionary {
    /// The index of the dictionary
    index: u8,
    /// The priority of the dictionary
    priority: u16,
    /// Whether or not secondary term searches are allowed for this dictionary.
    allow_secondary_searches: bool,
    /// Whether this dictionary's part of speech rules should be used to filter results.
    parts_of_speech_filter: bool,
    /// Whether to use the deinflections from this dictionary.
    use_deinflections: bool,
}

