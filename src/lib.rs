use std::collections::HashMap;
mod freq;

/// Enum representing what database field was used to match the source term.
pub enum TermSourceMatchSource {
    Term,
    Reading,
    Sequence,
}
/// Enum representing how the search term relates to the final term.
pub enum TermSourceMatchType {
    Exact,
    Prefix,
    Suffix,
}

pub enum TermPronounciationMatchType {
    PitchAccent,
    PhoneticTranscription
}

pub enum DictionaryEntryType {
    Kanji,
    Term
}

pub struct InflectionRuleChainCandidate {}

pub struct TermHeadword {}

pub struct TermDefinition {}

pub struct TermPronunciation {}

#[allow(dead_code)]
pub struct PitchAccent {
    term: TermPronounciationMatchType,
    position: u8,
    nasal_positions: u8,
    devoic_positions: u8,
    tags: Vec<Tag>
}

/// A tag represents some brief information about part of a dictionary entry.
#[allow(dead_code)]
pub struct Tag {
    name: String,
    category: String,
    order: u16,
    score: u16,
    content: Vec<String>,
    dictionaries: Vec<String>,
    redundant: bool,
}

pub type KanjiStatGroups = HashMap<String, Vec<KanjiStat>>;

pub struct KanjiDictionaryEntry {
    entry_type: DictionaryEntryType,
    character: String,
    dictionary: String,
    onyomi: Vec<String>,
    kunyomi: Vec<String>,
    tags: Vec<Tag>,
    stats: KanjiStatsGroup, 
    definitions: Vec<String>,
    frequencies: Vec<KanjiFrequency>,

}

/// Frequency information corresponds to how frequently a term appears in a corpus,
/// which can be a number of occurrences or an overall rank.
#[allow(dead_code)]
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

