// Import necessary collections
use std::any::Any;

use indexmap::{IndexMap, IndexSet};
use language_transformer::language_d::TextProcessorSetting;

use crate::database::dictionary_database::TermEntry;
use crate::dictionary::{self, InflectionRuleChainCandidate, InflectionSource};

pub type TextProcessorRuleChainCandidate = Vec<String>;

pub type VariantAndTextProcessorRuleChainCandidatesMap =
    IndexMap<String, Vec<TextProcessorRuleChainCandidate>>;

#[derive(Clone, Debug, PartialEq)]
pub struct TermDictionaryEntry {
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatabaseDeinflection {
    pub original_text: String,
    pub transformed_text: String,
    pub deinflected_text: String,
    pub conditions: usize,
    pub text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
    pub inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    pub database_entries: Vec<TermEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryEntryGroup {
    pub ids: IndexSet<u64>,
    pub dictionary_entries: Vec<TermDictionaryEntry>,
}

pub type TextCache = IndexMap<String, IndexMap<String, IndexMap<TextProcessorSetting, String>>>;
