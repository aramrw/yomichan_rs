// Import necessary collections
use std::any::Any;

use deinflector::language_d::TextProcessorSetting;
use deinflector::transformer::{
    InflectionRuleChainCandidate, InternalInflectionRuleChainCandidate,
};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::database::dictionary_database::TermEntry;
use crate::dictionary::{
    self, TermDefinition, TermDictionaryEntry, TermFrequency, TermHeadword, TermPronunciation,
    TermSourceMatchSource,
};

pub type TextProcessorRuleChainCandidate = Vec<String>;

pub type VariantAndTextProcessorRuleChainCandidatesMap =
    IndexMap<String, Vec<TextProcessorRuleChainCandidate>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// A dictionary entry for a term or group of terms.
/// The only difference here is that it's inflection_rule_chain_candidates are strings
pub struct InternalTermDictionaryEntry {
    /// This should always be [TermSourceMatchSource::Term]
    pub entry_type: TermSourceMatchSource,
    /// Whether or not any of the sources is a primary source. Primary sources are derived from the
    /// original search text, while non-primary sources originate from related terms.
    pub is_primary: bool,
    /// Ways that a looked-up word might be an transformed into this term.
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    /// Ways that a looked-up word might be an inflected form of this term.
    pub inflection_rule_chain_candidates: Vec<InternalInflectionRuleChainCandidate>,
    /// A score for the dictionary entry.
    pub score: i128,
    /// The sorting value based on the determined term frequency.
    pub frequency_order: i128,
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

#[derive(Clone, Debug, PartialEq)]
pub struct DatabaseDeinflection {
    pub original_text: String,
    pub transformed_text: String,
    pub deinflected_text: String,
    pub conditions: usize,
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    pub inflection_rule_chain_candidates: Vec<InternalInflectionRuleChainCandidate>,
    pub database_entries: Vec<TermEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryEntryGroup {
    pub ids: IndexSet<String>,
    pub dictionary_entries: Vec<InternalTermDictionaryEntry>,
}

#[derive(Clone, Debug, Default)]
pub struct FindInternalTermsResult {
    pub dictionary_entries: Vec<InternalTermDictionaryEntry>,
    pub original_text_length: i128,
}

pub type TextCache = IndexMap<String, IndexMap<String, IndexMap<TextProcessorSetting, String>>>;
