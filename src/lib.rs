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

