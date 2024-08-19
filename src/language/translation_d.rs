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

pub type TermEnabledDictionaryMap<'a> = HashMap<&'a str, FindTermDictionary>;
