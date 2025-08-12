use std::sync::Arc;

use crate::{
    database::dictionary_database::TermPronunciationMatchType,
    translation_internal::TextProcessorRuleChainCandidate, translator::TermType,
};
use deinflector::transformer::{InflectionRuleChainCandidate, InflectionSource};
use derive_more::derive::From;
use getset::MutGetters;
pub use importer::{
    dictionary_database::{DictionaryTag, Pronunciation, TermSourceMatchSource, TermSourceMatchType},
    structured_content::{TermGlossaryContent, TermGlossaryContentGroup},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Dictionary InflectionRuleChainCandidate
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryInflectionRuleChainCandidate {
    pub source: InflectionSource,
    pub inflection_rules: Vec<String>,
}

/// Dictionary InflectionRuleChainCandidateKey
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryInflectionRuleChainCandidatesKey {
    pub term: String,
    pub reading: String,
    pub inflection_rule_chain_candidates: Vec<DictionaryInflectionRuleChainCandidate>,
}

// #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
// /// A tag represents some brief information about part of a dictionary entry.
// pub struct DictionaryTag {
//     /// The name of the tag.
//     pub name: String,
//     /// The category of the tag.
//     pub category: String,
//     /// A number indicating the sorting order of the tag.
//     pub order: usize,
//     /// A score value for the tag.
//     pub score: usize,
//     /// An array of descriptions for the tag. If there are multiple entries,
//     /// the values will typically have originated from different dictionaries.
//     /// However, there is no correlation between the length of this array and
//     /// the length of the `dictionaries` field, as duplicates are removed.
//     pub content: Vec<String>,
//     /// An array of dictionary names that contained a tag with this name and category.
//     pub dictionaries: Vec<String>,
//     /// Whether or not this tag is redundant with previous tags.
//     pub redundant: bool,
// }
// impl DictionaryTag {
//     /// sets the category to "default"
//     pub fn new_default(name: String, dictionary: String) -> Self {
//         Self {
//             name,
//             category: "default".to_string(),
//             order: 0,
//             score: 0,
//             content: vec![],
//             dictionaries: vec![dictionary],
//             redundant: false,
//         }
//     }
// }
//
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

/// Helper enum to match expected schema types more accurately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NumOrStr {
    Num(i128),
    Str(String),
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



/// Frequency information corresponds to how frequently a term appears in a corpus,
/// which can be a number of occurrences or an overall rank.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TermFrequency {
    /// The original order of the frequency, which is usually used for sorting.
    pub index: usize,
    /// Which headword this frequency corresponds to.
    pub headword_index: usize,
    /// The name of the dictionary that the frequency information originated from.
    pub dictionary: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    pub dictionary_index: usize,
    /// The alias for the dictionary
    pub dictionary_alias: String,
    /// Whether or not the frequency had an explicit reading specified.
    pub has_reading: bool,
    /// The frequency for the term, as a number of occurrences or an overall rank.
    pub frequency: i128,
    /// A display value to show to the user.
    pub display_value: Option<String>,
    /// Whether or not the displayValue string was parsed to determine the frequency value.
    pub display_value_parsed: bool,
}

/// Represents the written form and reading of a term.
///
/// A single dictionary entry can have multiple headwords, for example, to represent
/// different kanji writings of the same word (e.g., "見る" and "観る"). This struct
/// holds the term's text, its reading, and associated metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TermHeadword {
    /// The original order of the headword as it appeared in the source dictionary,
    /// used for stable sorting.
    pub index: usize,
    /// The term's written form (e.g., "日本語").
    pub term: String,
    /// The reading of the term (e.g., "にほんご").
    pub reading: String,
    /// Information about how this headword was derived from the original search text.
    pub sources: Vec<TermSource>,
    /// Tags providing additional information about the headword (e.g., "usually written
    /// using kana alone").
    pub tags: Vec<importer::dictionary_database::DictionaryTag>,
    /// A list of parts of speech associated with this headword (e.g., "Noun", "Verb").
    pub word_classes: Vec<String>,
}

/// Represents a single definition for a term from a specific dictionary.
///
/// A term can have multiple definitions, and this struct holds the content
/// of one such definition, along with metadata about its source and relevance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermDefinition {
    /// A unique identifier for this specific definition.
    pub id: String,
    /// The original order of the definition as it appeared in the source dictionary,
    /// used for stable sorting.
    pub index: usize,
    /// A list of indices pointing to the `TermHeadword`(s) this definition applies to.
    pub headword_indices: Vec<usize>,
    /// The name of the dictionary from which this definition was sourced.
    pub dictionary: String,
    /// The index of the source dictionary in the user's configured list.
    pub dictionary_index: usize,
    /// The user-defined alias for the source dictionary.
    pub dictionary_alias: String,
    /// A relevance score assigned to the definition, used for ranking search results.
    pub score: i128,
    /// A sorting value derived from the term's frequency, used to order definitions
    /// by how common they are.
    pub frequency_order: i128,
    /// A list of database sequence numbers associated with the term. A value of `-1`
    /// indicates no sequence number. Multiple values can exist if definitions with
    /// different sequences were merged.
    pub sequences: Vec<i128>,
    /// Indicates if this definition is from a primary source (i.e., directly from the
    /// initial search text) or a related term.
    pub is_primary: bool,
    /// Tags providing additional information about the definition (e.g., usage notes,
    /// field of study).
    pub tags: Vec<importer::dictionary_database::DictionaryTag>,
    /// The structured content of the definition, typically a list of glossary entries.
    /// See [`TermGlossaryContentGroup`] for more details.
    pub entries: Vec<TermGlossaryContentGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// A term pronunciation represents different ways to pronounce one of the headwords.
pub struct TermPronunciation {
    /// The original order of the pronunciation, which is usually used for sorting.
    pub index: usize,
    /// Which headword this pronunciation corresponds to.
    pub headword_index: usize,
    /// The name of the dictionary that the pronunciation information originated from.
    pub dictionary: String,
    /// The index of the dictionary in the original list of dictionaries used for the lookup.
    pub dictionary_index: usize,
    /// The alias of the dictionary
    pub dictionary_alias: String,
    /// The pronunciations for the term.
    pub pronunciations: Vec<Pronunciation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Source information represents how the original text was transformed to get to the final term.
pub struct TermSource {
    /// The original text that was searched.
    pub original_text: String,
    /// The original text after being transformed, but before applying deinflections.
    pub transformed_text: String,
    /// The final text after applying deinflections.
    pub deinflected_text: String,
    /// How the deinflected text matches the value from the database.
    pub match_type: importer::dictionary_database::TermSourceMatchType,
    /// Which field was used to match the database entry.
    pub match_source: importer::dictionary_database::TermSourceMatchSource,
    /// Whether or not this source is a primary source. Primary sources are derived from the
    /// original search text, while non-primary sources originate from related terms.
    pub is_primary: bool,
}

/// Represents a complete dictionary entry for a term, aggregating all related information
/// such as headwords, definitions, pronunciations, and frequencies.
///
/// This is one of the core data structures returned by a search. It contains all the
/// information associated with a single term as found in one or more dictionaries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermDictionaryEntry {
    /// The type of entry, indicating how it was matched (e.g., as a term).
    /// This should always be [`TermSourceMatchSource::Term`].
    pub entry_type: TermSourceMatchSource,
    /// Indicates if this entry is from a primary source (directly from the initial search)
    /// or a related term.
    pub is_primary: bool,
    /// A list of potential transformation rule chains that could have produced this term
    /// from the original text.
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    /// A list of potential de-inflection rule chains that could have produced this term
    /// from an inflected form in the original text.
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    /// A relevance score for the entire dictionary entry.
    pub score: i128,
    /// A sorting value based on the term's overall frequency.
    pub frequency_order: i128,
    /// The user-defined alias of the dictionary this entry belongs to.
    pub dictionary_alias: String,
    /// The index of the source dictionary in the user's configured list.
    pub dictionary_index: usize,
    /// The number of times the exact term was found in the primary source text.
    pub source_term_exact_match_count: usize,
    /// Indicates whether the term's reading matched the primary reading from the source.
    pub match_primary_reading: bool,
    /// The maximum character length of the original text that this entry matched.
    pub max_original_text_length: usize,
    /// A list of headwords associated with this entry. A single entry can have multiple
    /// headwords (e.g., different kanji writings for the same word).
    pub headwords: Vec<TermHeadword>,
    /// A list of definitions for the term.
    pub definitions: Vec<TermDefinition>,
    /// A list of pronunciations for the term.
    pub pronunciations: Vec<TermPronunciation>,
    /// A list of frequency data points for the term.
    pub frequencies: Vec<TermFrequency>,
}

impl TermDictionaryEntry {
    pub fn get_headword_text_joined(&self) -> String {
        self.headwords
            .iter()
            .map(|th| th.term.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

