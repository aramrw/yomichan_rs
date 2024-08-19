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

pub type TermEnabledDictionaryMap<'a> = HashMap<&'a str, FindTermDictionary>;
