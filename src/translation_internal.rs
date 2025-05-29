// Import necessary collections
use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::database::dictionary_database::TermEntry;
use crate::dictionary::{self, InflectionRuleChainCandidate, InflectionSource};
use crate::translation::FindTermsTextReplacement;

pub struct TextDeinflectionOptions {
    pub text_replacements: Option<Vec<FindTermsTextReplacement>>,
    pub half_width: bool,
    pub numeric: bool,
    pub alphabetic: bool,
    pub katakana: bool,
    pub hiragana: bool,
    /// [collapse_emphatic, collapse_emphatic_full]
    pub emphatic: (bool, bool),
}

pub struct TextDeinflectionOptionsArrays {
    pub text_replacements: Vec<Option<Vec<FindTermsTextReplacement>>>,
    pub half_width: Vec<bool>,
    pub numeric: Vec<bool>,
    pub alphabetic: Vec<bool>,
    pub katakana: Vec<bool>,
    pub hiragana: Vec<bool>,
    pub emphatic: Vec<(bool, bool)>,
}

pub type TextProcessorRuleChainCandidate = Vec<String>;

pub type VariantAndTextProcessorRuleChainCandidatesMap =
    HashMap<String, Vec<TextProcessorRuleChainCandidate>>;

pub struct TermDictionaryEntry {
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
}

pub struct DatabaseDeinflection {
    pub original_text: String,
    pub transformed_text: String,
    pub deinflected_text: String,
    pub conditions: u64,
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    pub database_entries: Vec<TermEntry>,
}

pub struct DictionaryEntryGroup {
    pub ids: HashSet<u64>,
    pub dictionary_entries: Vec<TermDictionaryEntry>,
}
