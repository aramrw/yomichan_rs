use crate::{
    dictionary_data::TermGlossaryContent, translation_internal::TextProcessorRuleChainCandidate,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InflectionSource {
    Algorithm,
    Dictionary,
    Both,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InflectionRuleChainCandidate {
    pub source: InflectionSource,
    pub inflection_rules: Vec<String>,
}

/// Helper enum to match expected schema types more accurately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NumOrStr {
    Num(i128),
    Str(String),
}

/// Helper enum to match expected schema types more accurately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VecNumOrNum {
    Vec(u8),
    Str(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// A tag represents some brief information about part of a dictionary entry.
pub struct DictionaryTag {
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

/*************** Kanji ***************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DictionaryEntry {
    KanjiDictEntry(KanjiDictionaryEntry),
    TermDictEntry(TermDictionaryEntry),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DictionaryEntryType {
    Kanji,
    Term,
}

/// A stat represents a generic piece of information about a kanji character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiStat {
    /// The name of the stat.
    name: String,
    /// The category of the stat.
    category: String,
    /// A description of the stat.
    content: String,
    /// A number indicating the sorting order of the stat.
    order: u16,
    /// A score value for the stat.
    score: u64,
    /// The name of the dictionary that the stat originated from.
    dictionary: String,
    /// A value for the stat.
    value: NumOrStr,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// An object with groups of stats about a kanji character.
pub type KanjiStatGroups = IndexMap<String, Vec<KanjiStat>>;

/// A dictionary entry for a kanji character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiDictionaryEntry {
    /// The type of the entry.
    /// Should be `"kanji"` in the json.
    entry_type: DictionaryEntryType,
    /// The kanji character that was looked up.
    character: String,
    /// The name of the dictionary that the information originated from.
    dictionary: String,
    /// Onyomi readings for the kanji character.
    onyomi: Vec<String>,
    /// Kunyomi readings for the kanji character.
    kunyomi: Vec<String>,
    /// Tags for the kanji character.
    tags: Vec<DictionaryTag>,
    /// An object containing stats about the kanji character.
    stats: KanjiStatGroups,
    /// Definitions for the kanji character.
    definitions: Vec<String>,
    /// Frequency information for the kanji character.
    frequencies: Vec<KanjiFrequency>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DictionaryOrder {
    index: u16,
    priority: u16,
}

/*************** Term ***************/

/// Enum representing what database field was used to match the source term.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermSourceMatchSource {
    Term,
    Reading,
    Sequence,
}

/// Enum representing how the search term relates to the final term.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermSourceMatchType {
    Exact,
    Prefix,
    Suffix,
}

/// Frequency information corresponds to how frequently a term appears in a corpus,
/// which can be a number of occurrences or an overall rank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermFrequency {
    /// The original order of the frequency, which is usually used for sorting.
    index: u32,
    /// Which headword this frequency corresponds to.
    headword_index: u32,
    /// The name of the dictionary that the frequency information originated from.
    dictionary: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    dictionary_index: u16,
    /// The priority of the dictionary.
    dictionary_priority: u16,
    /// Whether or not the frequency had an explicit reading specified.
    has_reading: bool,
    /// The frequency for the term, as a number of occurrences or an overall rank.
    frequency: u32,
    /// A display value to show to the user.
    display_value: Option<String>,
    /// Whether or not the displayValue string was parsed to determine the frequency value.
    display_value_parsed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// A term headword is a combination of a term, reading, and auxiliary information.
pub struct TermHeadword {
    /// The original order of the headword, which is usually used for sorting.
    pub index: usize,
    /// The text for the term.
    pub term: String,
    /// The reading of the term.
    pub reading: String,
    /// The sources of the term.
    pub sources: Vec<TermSource>,
    /// Tags for the headword.
    pub tags: Vec<DictionaryTag>,
    /// List of word classes (part of speech) for the headword.
    pub word_classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// A dictionary entry for a term or group of terms.
pub struct TermDefinition {
    /// Database ID for the definition.
    pub id: String,
    /// The original order of the definition, which is usually used for sorting.
    pub index: usize,
    /// A list of headwords that this definition corresponds to.
    pub headword_indices: Vec<usize>,
    /// The name of the dictionary that the definition information originated from.
    pub dictionary: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    pub dictionary_index: usize,
    /// A score for the definition.
    pub score: usize,
    /// The sorting value based on the determined term frequency.
    pub frequency_order: usize,
    /// A list of database sequence numbers for the term.
    /// A value of `-1` corresponds to no sequence.
    /// The list can have multiple values if multiple definitions with
    /// different sequences have been merged.
    /// The list should always have at least one item.
    pub sequences: Vec<i128>,
    /// Whether or not any of the sources is a primary source. Primary sources are derived from the
    /// original search text, while non-primary sources originate from related terms.
    pub is_primary: bool,
    /// Tags for the definition.
    pub tags: Vec<DictionaryTag>,
    /// The definition entries.
    pub entries: Vec<TermGlossaryContent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// A term pronunciation represents different ways to pronounce one of the headwords.
pub struct TermPronunciation {
    /// The original order of the pronunciation, which is usually used for sorting.
    index: u16,
    /// Which headword this pronunciation corresponds to.
    headword_index: u64,
    /// The name of the dictionary that the pronunciation information originated from.
    dictionary: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    dictionary_index: u16,
    /// The priority of the dictionary.
    dictionary_priority: u16,
    /// The pronunciations for the term.
    pronunciations: Vec<Pronunciation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Source information represents how the original text was transformed to get to the final term.
pub struct TermSource {
    /// The original text that was searched.
    pub original_text: String,
    /// The original text after being transformed, but before applying deinflections.
    pub transformed_text: String,
    /// The final text after applying deinflections.
    pub deinflected_text: String,
    /// How the deinflected text matches the value from the database.
    pub match_type: TermSourceMatchType,
    /// Which field was used to match the database entry.
    pub match_source: TermSourceMatchSource,
    /// Whether or not this source is a primary source. Primary sources are derived from the
    /// original search text, while non-primary sources originate from related terms.
    pub is_primary: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// A dictionary entry for a term or group of terms.
pub struct TermDictionaryEntry {
    /// This should always be [TermSourceMatchSource::Term]
    pub entry_type: TermSourceMatchSource,
    /// Whether or not any of the sources is a primary source. Primary sources are derived from the
    /// original search text, while non-primary sources originate from related terms.
    pub is_primary: bool,
    /// Ways that a looked-up word might be an transformed into this term.
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    /// Ways that a looked-up word might be an inflected form of this term.
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    /// A score for the dictionary entry.
    pub score: usize,
    /// The sorting value based on the determined term frequency.
    pub frequency_order: usize,
    /// The alias of the dictionary.
    pub dictionary_alias: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    pub dictionary_index: usize,
    /// The number of primary sources that had an exact text match for the term.
    pub source_term_exact_match_count: usize,
    /// Whether the term reading matched the primary reading.
    pub match_primary_reading: bool,
    /// The maximum length of the original text for all primary sources.
    pub max_original_text_length: usize,
    /// Headwords for the entry.
    pub headwords: Vec<TermHeadword>,
    /// Definitions for the entry.
    pub definitions: Vec<TermDefinition>,
    /// Pronunciations for the entry.
    pub pronunciations: Vec<TermPronunciation>,
    /// Frequencies for the entry.
    pub frequencies: Vec<TermFrequency>,
}

/*************** Pitch Accent & Pronunciation ***************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TermPronunciationMatchType {
    #[serde(rename = "lowercase")]
    PitchAccent,
    #[serde(rename = "phonetic-transcription")]
    PhoneticTranscription,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pronunciation {
    PitchAccent(PitchAccent),
    PhoneticTranscription(PhoneticTranscription),
}

/// Pitch accent information for a term, represented as the position of the downstep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PitchAccent {
    /// Type of the pronunciation, for disambiguation between union type members.
    /// Should be `"pitch-accent"` in the json.
    term: TermPronunciationMatchType,
    /// Position of the downstep, as a number of mora.
    position: u8,
    /// Positions of morae with a nasal sound.
    nasal_positions: Vec<u8>,
    /// Positions of morae with a devoiced sound.
    devoic_positions: Vec<u8>,
    /// Tags for the pitch accent.
    tags: Vec<DictionaryTag>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhoneticTranscription {
    /// Type of the pronunciation, for disambiguation between union type members.
    /// Should be `"phonetic-transcription"` in the json.
    match_type: TermPronunciationMatchType,
    /// IPA transcription for the term.
    ipa: String,
    /// List of tags for this IPA transcription.
    tags: Vec<DictionaryTag>,
}
