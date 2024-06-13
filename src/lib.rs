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

