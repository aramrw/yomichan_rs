use std::{fmt::Display, hash::Hash, iter, path::Path, str::FromStr, sync::LazyLock};

use fancy_regex::Regex;
use icu::{
    collator::{options::CollatorOptions, Collator, CollatorBorrowed},
    datetime::provider::neo::marker_attrs::PATTERN_MEDIUM,
    locale::locale,
};
use indexmap::{IndexMap, IndexSet};
use language_transformer::{
    descriptors::{PreAndPostProcessors, PreAndPostProcessorsWithId},
    ja::japanese::is_code_point_japanese,
    language_d::{
        AnyTextProcessor, FindTermsTextReplacement, FindTermsTextReplacements,
        LanguageAndProcessors, LanguageAndReadingNormalizer, ReadingNormalizer, TextProcessor,
        TextProcessorFn, TextProcessorSetting, TextProcessorWithId,
    },
    languages::{get_all_language_reading_normalizers, get_all_language_text_processors},
    multi_language_transformer::MultiLanguageTransformer,
    transformer::{InflectionRule, LanguageTransformer, TransformedText},
    zh::chinese::is_code_point_chinese,
};
use native_db::*;
use native_model::{native_model, Model};
use serde::Serialize;

use crate::{
    database::dictionary_database::{DictionaryDatabase, DictionaryDatabaseTag, TermEntry},
    dictionary::{
        DictionaryTag, EntryInflectionRuleChainCandidatesKey, InflectionRuleChainCandidate,
        InflectionSource, TermDefinition, TermDictionaryEntry, TermHeadword, TermSource,
        TermSourceMatchSource, TermSourceMatchType,
    },
    dictionary_data::{TermGlossary, TermGlossaryContent, TermGlossaryDeinflection},
    regex_util::apply_text_replacement,
    settings::SearchResolution,
    translation::{
        FindKanjiDictionary, FindTermDictionary, FindTermDictionaryMap, FindTermsOptions,
    },
    translation_internal::{
        DatabaseDeinflection, TextCache, TextProcessorRuleChainCandidate,
        VariantAndTextProcessorRuleChainCandidatesMap,
    },
};

type TagCache = IndexMap<&'static str, Option<DictionaryDatabaseTag>>;

static GET_NEXT_SUBSTRING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^\p{L}][\p{L}\p{N}]*$").expect("Invalid get_next_substring_regex pattern")
});

#[derive(Clone, Debug)]
struct SequenceQuery {
    query: u128,
    dictionary: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FindTermsMode {
    Simple,
    Group,
    Merge,
    Split,
}

pub type TermEnabledDictionaryMap = IndexMap<String, FindTermDictionary>;
pub type KanjiEnabledDictionaryMap = IndexMap<String, FindKanjiDictionary>;
#[derive(Clone, Debug, PartialEq)]
pub enum EnabledDictionaryMapType<'a> {
    Term(&'a TermEnabledDictionaryMap),
    Kanji(&'a KanjiEnabledDictionaryMap),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingEntry {
    pub index: usize,
    pub entry: TermDictionaryEntry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TermDictionaryEntryWithIndexes {
    pub index: usize,
    pub dictionary_entry: TermDictionaryEntry,
    pub headword_indexes: Vec<usize>,
}

#[derive(Clone, Debug)]
struct TermReadingItem {
    pub term: String,
    pub reading: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FindTermResult {
    pub dictionary_entries: Vec<TermDictionaryEntry>,
    pub original_text_length: i128,
}

type TextProcessorMap = IndexMap<&'static str, PreAndPostProcessorsWithId>;
type ReadingNormalizerMap = IndexMap<&'static str, ReadingNormalizer>;

#[derive(thiserror::Error, Debug)]
pub enum TranslatorError {
    #[error("Unsupported Language: {0}")]
    UnsupportedLanguage(String),
}

/// class which finds term and kanji dictionary entries for text.
struct Translator {
    db: DictionaryDatabase,
    mlt: MultiLanguageTransformer,
    tag_cache: IndexMap<&'static str, TagCache>,
    /// Invariant Locale
    /// Default: "en-US"
    string_comparer: CollatorBorrowed<'static>,
    number_regex: Regex,
    text_processors: TextProcessorMap,
    reading_normalizers: ReadingNormalizerMap,
}

impl Translator {
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            db: DictionaryDatabase::new(path),
            mlt: MultiLanguageTransformer::default(),
            tag_cache: IndexMap::new(),
            string_comparer: Collator::try_new(locale!("en-US").into(), CollatorOptions::default())
                .unwrap(),
            number_regex: Regex::new(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?").unwrap(),
            text_processors: IndexMap::new(),
            reading_normalizers: IndexMap::new(),
        }
    }

    /// Initializes the instance for use.
    /// The public API should not be used until this function has been called.
    fn prepare(&mut self) {
        let language_processors = get_all_language_text_processors();
        for processor in language_processors {
            let LanguageAndProcessors { iso, pre, post } = processor;
            let pre_and_post_processors = PreAndPostProcessorsWithId { pre, post };
            self.text_processors.insert(iso, pre_and_post_processors);
        }
        let reading_normalizers = get_all_language_reading_normalizers();
        for normalizer in reading_normalizers {
            let LanguageAndReadingNormalizer {
                iso,
                reading_normalizer,
            } = normalizer;
            self.reading_normalizers.insert(iso, reading_normalizer);
        }
    }

    /// Clears the database tag cache.
    /// This should be called if the database is changed.
    fn clear_dbtag_caches(&mut self) {
        self.tag_cache.clear();
    }

    /// Finds term definitions for the given text.
    ///
    /// This function operates asynchronously.
    ///
    /// # Parameters
    ///
    /// `mode`: The [FindTermsMode] to use for finding terms.
    /// Determines the format of the resulting array.
    /// Must be one of: `Group`, `Merge`, `Split`, or `Simple`.
    ///
    ///  `text`: The `&str` text to find terms for.
    ///
    ///  `options`: A reference to `FindTermsOptions`, a struct describing settings
    ///   for the lookup.
    ///
    /// # Returns
    ///
    /// A `Result` which, upon success, yields a struct containing:
    ///  `dictionary_entries`: A `Vec<TermDictionaryEntry>` with the found terms.
    ///  `original_text_length`: The `usize` length of the original source text.
    ///
    /// # Errors
    ///
    /// Returns an error if the term lookup process fails for any reason.
    fn find_terms(
        &self,
        mode: FindTermsMode,
        text: &str,
        opts: &FindTermsOptions,
    ) -> FindTermResult {
        let mut text = text.to_string();
        let FindTermsOptions {
            enabled_dictionary_map,
            exclude_dictionary_definitions,
            sort_frequency_dictionary,
            sort_frequency_dictionary_order,
            language,
            primary_reading,
            ..
        } = opts;
        let mut tag_aggregator = TranslatorTagAggregator::default();
        let FindTermResult {
            dictionary_entries,
            original_text_length,
        } = self.find_terms_internal(&mut text, opts, &mut tag_aggregator, primary_reading);

        // match mode {
        //     FindTermsMode::Group => dictionary_entries = Translator::group,
        // }

        FindTermResult {
            dictionary_entries,
            original_text_length,
        }
    }

    fn find_terms_internal(
        &self,
        text: &mut String,
        opts: &FindTermsOptions,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> FindTermResult {
        let FindTermsOptions {
            remove_non_japanese_characters,
            enabled_dictionary_map,
            ..
        } = opts;
        if *remove_non_japanese_characters && ["ja", "zh", "yue"].contains(&opts.language.as_str())
        {
            *text = Translator::get_japanese_chinese_only_text(text);
        }
        if text.is_empty() {
            return FindTermResult {
                dictionary_entries: vec![],
                original_text_length: 0,
            };
        }
        let deinflections = self._get_deinflections(text, opts);
        Translator::_get_dictionary_entries(
            &deinflections,
            enabled_dictionary_map,
            tag_aggregator,
            primary_reading,
        )
    }

    fn _group_dictionary_entries_by_headword(
        &self,
        language: &str,
        dictionary_entries: &[TermDictionaryEntry],
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> Vec<TermDictionaryEntry> {
        let mut groups: IndexMap<String, Vec<TermDictionaryEntry>> = IndexMap::new();
        let reading_normalizer = self.reading_normalizers.get(language);
        for dictionary_entry in dictionary_entries {
            let TermDictionaryEntry {
                inflection_rule_chain_candidates,
                headwords,
                ..
            } = dictionary_entry;
            let TermHeadword { term, reading, .. } = headwords.first().unwrap_or_else(|| {
                panic!(
                    "in fn `_group_dictionary_entries_by_word`: 
                            headwords[0] is None (this is infallible in JS)"
                )
            });
            let normalized_reading =
                Translator::_get_or_default_normalized_reading(reading_normalizer, reading);
            let key = Translator::_create_map_key(&(
                term,
                normalized_reading,
                inflection_rule_chain_candidates,
            ));
            groups
                .entry(key)
                .or_default()
                .push(dictionary_entry.clone());
        }

        let new_dictionary_entries = groups
            .values()
            .map(|group_dictionary_entries| {
                self._create_grouped_dictionary_entry(
                    language,
                    group_dictionary_entries,
                    false,
                    tag_aggregator,
                    primary_reading,
                )
            })
            .collect();

        new_dictionary_entries
    }

    fn _get_or_default_normalized_reading(
        reading_normalizer: Option<&fn(&str) -> String>,
        reading: &str,
    ) -> String {
        if let Some(reading_normalizer) = reading_normalizer {
            reading_normalizer(reading)
        } else {
            reading.to_string()
        }
    }

    fn _create_grouped_dictionary_entry(
        &self,
        language: &str,
        dictionary_entries: &[TermDictionaryEntry],
        mut check_duplicate_definitions: bool,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> TermDictionaryEntry {
        // Headwords are generated before sorting,
        // so that the order of dictionaryEntries can be maintained
        let mut definition_entries = vec![];
        let mut headwords: IndexMap<String, TermHeadword> = IndexMap::new();
        dictionary_entries.iter().for_each(|dictionary_entry| {
            let headword_index_map = self._add_term_headwords(
                language,
                &mut headwords,
                &dictionary_entry.headwords,
                tag_aggregator,
            );
            let term_dictionary_entry_with_map = TermDictionaryEntryWithIndexes {
                index: definition_entries.len(),
                dictionary_entry: dictionary_entry.clone(),
                headword_indexes: headword_index_map,
            };
            definition_entries.push(term_dictionary_entry_with_map);
        });

        if definition_entries.len() <= 1 {
            check_duplicate_definitions = false;
        }

        // merge dictionary entry data
        let mut score = i128::MIN;
        let mut dictionary_index = usize::MAX;
        let mut dictionary_alias = "".to_string();
        let mut max_original_text_length = 0;
        let mut is_primary = false;
        let mut definitions: Vec<TermDefinition> = vec![];
        let mut definitions_map: Option<IndexMap<String, TermDefinition>> =
            match check_duplicate_definitions {
                true => Some(IndexMap::new()),
                false => None,
            };

        let mut inflections: Option<Vec<InflectionRuleChainCandidate>> = None;
        let mut text_processes: Option<Vec<TextProcessorRuleChainCandidate>> = None;

        for definition_entry in &definition_entries {
            let TermDictionaryEntryWithIndexes {
                dictionary_entry,
                headword_indexes,
                ..
            } = definition_entry;
            score = score.max(dictionary_entry.score);
            dictionary_index = dictionary_index.min(dictionary_entry.dictionary_index);

            if dictionary_entry.is_primary {
                is_primary = true;
                max_original_text_length =
                    max_original_text_length.max(dictionary_entry.max_original_text_length);

                let dictionary_entry_inflections =
                    &dictionary_entry.inflection_rule_chain_candidates;
                let dictionary_entry_text_processes =
                    &dictionary_entry.text_processor_rule_chain_candidates;

                if inflections
                    .as_deref()
                    .is_none_or(|i| dictionary_entry_inflections.len() < i.len())
                {
                    inflections = Some(dictionary_entry_inflections.clone());
                }
                if text_processes
                    .as_deref()
                    .is_none_or(|tp| dictionary_entry_text_processes.len() < tp.len())
                {
                    text_processes = Some(dictionary_entry_text_processes.clone());
                }
            }
            if let Some(definitions_map) = definitions_map.as_mut() {
                Translator::_add_term_definitions(
                    &mut definitions,
                    definitions_map,
                    &dictionary_entry.definitions,
                    headword_indexes,
                    tag_aggregator,
                );
            } else {
                Translator::_add_term_definitions_fast(
                    &mut definitions,
                    &dictionary_entry.definitions,
                    headword_indexes,
                );
            }
        }

        let headwords_array: Vec<TermHeadword> = headwords.values().cloned().collect();
        let mut source_term_exact_match_count = 0;
        let mut match_primary_reading = false;

        for headword in &headwords_array {
            let TermHeadword {
                sources, reading, ..
            } = headword;
            if !primary_reading.is_empty() && reading == primary_reading {
                match_primary_reading = true;
            }
            for source in sources {
                if source.is_primary && source.match_source == TermSourceMatchSource::Term {
                    source_term_exact_match_count += 1;
                    break;
                }
            }
        }

        Translator::_create_term_dictionary_entry(
            is_primary,
            text_processes.unwrap_or_default(),
            inflections.unwrap_or_default(),
            score,
            dictionary_index,
            dictionary_alias,
            source_term_exact_match_count,
            match_primary_reading,
            max_original_text_length,
            headwords_array,
            definitions,
        )
    }

    fn _add_term_definitions_fast(
        definitions: &mut Vec<TermDefinition>,
        new_definitions: &[TermDefinition],
        headword_index_map: &[usize],
    ) {
        for new_def in new_definitions {
            // Map the headword indices using the provided headword_index_map.
            let headword_indices_new: Vec<usize> = new_def
                .headword_indices
                .iter()
                .map(|&idx| headword_index_map[idx])
                .collect();

            // The index of the new definition is the current length of definitions.
            let index = definitions.len();

            definitions.push(Translator::_create_term_definition(
                index,
                headword_indices_new,
                new_def.dictionary.clone(),
                new_def.dictionary_index,
                new_def.dictionary_alias.clone(),
                new_def.id.clone(),
                new_def.score,
                new_def.sequences.clone(),
                new_def.is_primary,
                new_def.tags.clone(),
                new_def.entries.clone(),
            ));
        }
    }

    fn _add_term_definitions(
        definitions: &mut Vec<TermDefinition>,
        definitions_map: &mut IndexMap<String, TermDefinition>,
        new_definitions: &[TermDefinition],
        headword_index_map: &[usize],
        tag_aggregator: &mut TranslatorTagAggregator,
    ) {
        new_definitions.iter().for_each(|new_definition| {
            let TermDefinition {
                index,
                headword_indices,
                dictionary,
                dictionary_index,
                dictionary_alias,
                sequences,
                id,
                score,
                is_primary,
                tags,
                entries,
                frequency_order,
            } = new_definition;
            let key = Translator::_create_map_key(&(dictionary, entries));
            let mut definition = definitions_map.get(&key).cloned();
            if let Some(ref mut definition) = definition {
                if *is_primary {
                    definition.is_primary = true;
                }
                Translator::_add_unique_simple(&mut definition.sequences, sequences);
            } else {
                let TermDefinition {
                    id,
                    index,
                    headword_indices,
                    dictionary,
                    dictionary_index,
                    dictionary_alias,
                    score,
                    frequency_order,
                    sequences,
                    is_primary,
                    tags,
                    entries,
                } = new_definition.clone();
                let new_def = Translator::_create_term_definition(
                    definitions.len(),
                    vec![],
                    dictionary,
                    dictionary_index,
                    dictionary_alias,
                    id,
                    score,
                    sequences,
                    is_primary,
                    vec![],
                    entries,
                );
                definitions.push(new_def.clone());
                definitions_map.insert(key, new_def.clone());
                definition = Some(new_def);
            }
            /// definition is Some after this point, so unwrap() is safe

            /// merge tags doesn't mutate the passed in values so we can save it here cloned
            let definition_ref = definition.as_mut().unwrap();
            let definition_headword_indices: &mut Vec<usize> = &mut definition_ref.headword_indices;

            for headword_index in headword_indices {
                Translator::_add_unique_term_headword_index(
                    definition_headword_indices,
                    headword_index_map[*headword_index],
                );
            }
            tag_aggregator.merge_tags(&definition_ref.tags, tags);
        });
    }

    /// this is a binary search
    fn _add_unique_term_headword_index(headword_indices: &mut Vec<usize>, headword_index: usize) {
        match headword_indices.binary_search(&headword_index) {
            Ok(_) => {} // The element already exists, do nothing.
            Err(pos) => headword_indices.insert(pos, headword_index),
        }
    }

    fn _add_term_headwords(
        &self,
        language: &str,
        headwords_map: &mut IndexMap<String, TermHeadword>,
        headwords: &[TermHeadword],
        tag_aggregator: &mut TranslatorTagAggregator,
    ) -> Vec<usize> {
        headwords
            .iter()
            .map(|current_input_hw| {
                let TermHeadword {
                    term,
                    reading,
                    sources,
                    tags,
                    word_classes,
                    ..
                } = current_input_hw;
                let reading_normalizer_opt = self.reading_normalizers.get(language);
                let normalized_reading =
                    Translator::_get_or_default_normalized_reading(reading_normalizer_opt, reading);
                let key = Translator::_create_map_key(&(term, normalized_reading));
                let new_hw_potential_index = headwords_map.len();
                let map_hw_ref_mut = headwords_map.entry(key).or_insert_with(|| {
                    Translator::_create_term_headword(
                        new_hw_potential_index,
                        term.to_string(),
                        reading.to_string(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(), // Initial empty vectors
                    )
                });

                Translator::_add_unique_sources(&mut map_hw_ref_mut.sources, sources);
                Translator::_add_unique_simple(&mut map_hw_ref_mut.word_classes, word_classes);
                tag_aggregator.merge_tags(&map_hw_ref_mut.tags, tags);

                map_hw_ref_mut.index
            })
            .collect()
    }

    /// this deviates from JS.
    /// js loops through the `list` array everytime within `new_items` for_each(),
    /// here we create a set so it becomes O(1).
    ///
    /// depending on if duplicates existing (on purpose) this might be incorrect.
    fn _add_unique_simple<T>(list: &mut Vec<T>, new_items: &[T])
    where
        T: Eq + Hash + Clone,
    {
        let mut existing_items_set: IndexSet<T> = list.iter().cloned().collect();
        new_items.iter().for_each(|item| {
            if existing_items_set.insert(item.clone()) {
                list.push(item.clone());
            }
        });
    }

    fn _add_unique_sources(sources: &mut Vec<TermSource>, new_sources: &[TermSource]) {
        if new_sources.is_empty() {
            return;
        };
        if sources.is_empty() {
            sources.extend(new_sources.to_vec());
            return;
        }
        new_sources.iter().for_each(|new_source| {
            let TermSource {
                original_text,
                transformed_text,
                deinflected_text,
                match_type,
                match_source,
                is_primary,
            } = new_source;
            let has = sources.iter_mut().any(|src| {
                if src.original_text == *original_text
                    && src.transformed_text == *transformed_text
                    && src.deinflected_text == *deinflected_text
                    && src.match_type == *match_type
                    && src.match_source == *match_source
                {
                    if (*is_primary) {
                        src.is_primary = true;
                    }
                    return true;
                }
                false
            });
            if !has {
                sources.push(new_source.clone());
            }
        });
    }

    fn _create_map_key(v: &(impl Serialize + std::fmt::Debug)) -> String {
        serde_json::to_string(v)
            .unwrap_or_else(|e| panic!("could not serialize {v:?} to map key: {e}"))
    }

    fn _get_dictionary_entries(
        deinflections: &[DatabaseDeinflection],
        enabled_dictionary_map: &FindTermDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> FindTermResult {
        let mut original_text_length = 0;
        let mut dictionary_entries: Vec<TermDictionaryEntry> = vec![];
        let mut ids: IndexSet<String> = IndexSet::new();
        for deinflection in deinflections {
            let DatabaseDeinflection {
                database_entries,
                original_text,
                transformed_text,
                deinflected_text,
                text_processor_rule_chain_candidates,
                inflection_rule_chain_candidates,
                ..
            } = deinflection;

            if database_entries.is_empty() {
                continue;
            }
            original_text_length = original_text.len().max(original_text_length);

            for database_entry in database_entries {
                let id = database_entry.id.clone();
                if !ids.contains(id.as_str()) {
                    let dictionary_entry =
                        Translator::_create_term_dictionary_entry_from_database_entry(
                            database_entry.clone(),
                            original_text,
                            transformed_text,
                            deinflected_text,
                            text_processor_rule_chain_candidates.clone(),
                            inflection_rule_chain_candidates.clone(),
                            true,
                            enabled_dictionary_map,
                            tag_aggregator,
                            primary_reading,
                        );
                    dictionary_entries.push(dictionary_entry);
                    ids.insert(id);
                    continue;
                }

                let existing_entry_info =
                    Translator::_find_existing_entry(&dictionary_entries, &id);
                let Some(existing_entry_info) = existing_entry_info else {
                    continue;
                };

                let ExistingEntry {
                    entry: mut existing_entry,
                    index: existing_index,
                } = existing_entry_info;

                let existing_transformed_len = existing_entry
                    .headwords
                    .first()
                    .expect("existing entries first headword is None (this is infallible in JS)")
                    .sources
                    .first()
                    .expect("existing entries first source is None (this is insaffilble in JS)")
                    .transformed_text
                    .len();
                if transformed_text.len() < existing_transformed_len {
                    continue;
                }
                let term_dictionary_entry =
                    Translator::_create_term_dictionary_entry_from_database_entry(
                        database_entry.clone(),
                        original_text,
                        transformed_text,
                        deinflected_text,
                        text_processor_rule_chain_candidates.clone(),
                        inflection_rule_chain_candidates.clone(),
                        true,
                        enabled_dictionary_map,
                        tag_aggregator,
                        primary_reading,
                    );
                if transformed_text.len() > existing_transformed_len {
                    dictionary_entries.splice(existing_index..1, iter::once(term_dictionary_entry));
                } else {
                    Translator::_merge_inflection_rule_chains(
                        &mut existing_entry,
                        inflection_rule_chain_candidates,
                    );
                    Translator::_merge_text_processor_rule_chains(
                        &mut existing_entry,
                        text_processor_rule_chain_candidates,
                    );
                }
            }
        }

        FindTermResult {
            dictionary_entries,
            original_text_length: original_text_length as i128,
        }
    }

    fn _merge_text_processor_rule_chains(
        existing_entry: &mut TermDictionaryEntry,
        text_processor_rule_chain_candidates: &[TextProcessorRuleChainCandidate],
    ) {
        for text_processor_rules in text_processor_rule_chain_candidates {
            if existing_entry
                .text_processor_rule_chain_candidates
                .iter()
                .any(|chain| {
                    Translator::_are_arrays_equal_ignore_order(chain, text_processor_rules)
                })
            {
            } else {
                existing_entry
                    .text_processor_rule_chain_candidates
                    .push(text_processor_rules.clone());
            }
        }
    }

    /// mutates the existing_entry
    fn _merge_inflection_rule_chains(
        existing_entry: &mut TermDictionaryEntry,
        inflection_rule_chain_candidates: &[InflectionRuleChainCandidate],
    ) {
        for candidate in inflection_rule_chain_candidates {
            let InflectionRuleChainCandidate {
                source,
                inflection_rules,
            } = candidate;
            // Use iter_mut() to get mutable references so we can modify the found chain
            if let Some(duplicate) = existing_entry
                .inflection_rule_chain_candidates
                .iter_mut()
                .find(|chain| {
                    Translator::_are_arrays_equal_ignore_order(
                        &chain.inflection_rules,
                        inflection_rules,
                    )
                })
            {
                if duplicate.source != *source {
                    duplicate.source = InflectionSource::Both;
                }
            } else {
                let new_rule_chain_candidate = InflectionRuleChainCandidate {
                    source: source.clone(),
                    inflection_rules: inflection_rules.clone(),
                };
                existing_entry
                    .inflection_rule_chain_candidates
                    .push(new_rule_chain_candidate);
            }
        }
    }

    fn _are_arrays_equal_ignore_order(x: &[impl AsRef<str>], y: &[impl AsRef<str>]) -> bool {
        x.len() == y.len()
    }

    fn _find_existing_entry(
        dictionary_entries: &[TermDictionaryEntry],
        id: &str,
    ) -> Option<ExistingEntry> {
        dictionary_entries
            .iter()
            .enumerate()
            .find_map(|(index, entry)| {
                entry
                    .definitions
                    .iter()
                    .find(|def| def.id == id)
                    .map(|_| ExistingEntry {
                        index,
                        entry: entry.clone(),
                    })
            })
    }

    /// [TermGlossary]
    fn _create_term_dictionary_entry_from_database_entry(
        database_entry: TermEntry,
        original_text: &str,
        transformed_text: &str,
        deinflected_text: &str,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
        is_primary: bool,
        enabled_dictionary_map: &FindTermDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> TermDictionaryEntry {
        let TermEntry {
            id,
            index: dictionary_index,
            term,
            reading: raw_reading,
            sequence: raw_sequence,
            match_type,
            match_source,
            definition_tags,
            term_tags,
            rules,
            definitions,
            score,
            dictionary,
        } = database_entry;
        let content_definitions: Vec<TermGlossaryContent> = definitions
            .into_iter()
            .filter_map(|def| match def {
                TermGlossary::Content(c) => Some(*c),
                TermGlossary::Deinflection(_) => None,
            })
            .collect();
        let reading = match raw_reading.is_empty() {
            true => term.clone(),
            false => raw_reading,
        };
        let match_primary_reading = !primary_reading.is_empty() && reading == primary_reading;
        let dictionary_order = Translator::_get_dictionary_order(
            &dictionary,
            &EnabledDictionaryMapType::Term(enabled_dictionary_map),
        );
        let dictionary_alias = Translator::_get_dictionary_alias(
            dictionary.clone(),
            &EnabledDictionaryMapType::Term(enabled_dictionary_map),
        );
        let source_term_exact_match_count = match is_primary && deinflected_text == term {
            true => 1,
            false => 0,
        };
        let source = Translator::_create_source(
            original_text.to_owned(),
            transformed_text.to_owned(),
            deinflected_text.to_owned(),
            match_type,
            match_source,
            is_primary,
        );
        let max_original_text_length = original_text.len();
        let has_sequence = raw_sequence >= 0;
        let sequence = match has_sequence {
            true => raw_sequence,
            false => -1,
        };

        let headword_tag_groups: Vec<DictionaryTag> = vec![];
        let definition_tag_groups: Vec<DictionaryTag> = vec![];

        tag_aggregator.add_tags(&headword_tag_groups, &dictionary, &term_tags);
        tag_aggregator.add_tags(&definition_tag_groups, &dictionary, &definition_tags);

        let headwords = vec![Translator::_create_term_headword(
            0,
            term,
            reading,
            vec![source],
            headword_tag_groups,
            rules,
        )];
        let definitions = vec![Translator::_create_term_definition(
            0,
            vec![0],
            dictionary,
            dictionary_index,
            dictionary_alias.clone(),
            id,
            score,
            vec![sequence],
            is_primary,
            definition_tag_groups,
            content_definitions,
        )];

        Translator::_create_term_dictionary_entry(
            is_primary,
            text_processor_rule_chain_candidates,
            inflection_rule_chain_candidates,
            score,
            dictionary_index,
            dictionary_alias,
            source_term_exact_match_count,
            match_primary_reading,
            max_original_text_length,
            headwords,
            definitions,
        )
    }

    fn _create_term_definition(
        index: usize,
        headword_indices: Vec<usize>,
        dictionary: String,
        dictionary_index: usize,
        dictionary_alias: String,
        id: String,
        score: i128,
        sequences: Vec<i128>,
        is_primary: bool,
        tags: Vec<DictionaryTag>,
        entries: Vec<TermGlossaryContent>,
    ) -> TermDefinition {
        TermDefinition {
            id,
            index,
            headword_indices,
            dictionary,
            dictionary_alias,
            dictionary_index,
            score,
            frequency_order: 0,
            sequences,
            is_primary,
            tags,
            entries,
        }
    }

    fn _create_term_headword(
        index: usize,
        term: String,
        reading: String,
        sources: Vec<TermSource>,
        tags: Vec<DictionaryTag>,
        word_classes: Vec<String>,
    ) -> TermHeadword {
        TermHeadword {
            index,
            term,
            reading,
            sources,
            tags,
            word_classes,
        }
    }

    fn _create_term_dictionary_entry(
        is_primary: bool,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
        score: i128,
        dictionary_index: usize,
        dictionary_alias: String,
        source_term_exact_match_count: usize,
        match_primary_reading: bool,
        max_original_text_length: usize,
        headwords: Vec<TermHeadword>,
        definitions: Vec<TermDefinition>,
    ) -> TermDictionaryEntry {
        TermDictionaryEntry {
            entry_type: TermSourceMatchSource::Term,
            is_primary,
            text_processor_rule_chain_candidates,
            inflection_rule_chain_candidates,
            score,
            frequency_order: 0,
            dictionary_index,
            source_term_exact_match_count,
            max_original_text_length,
            headwords,
            definitions,
            dictionary_alias,
            match_primary_reading,
            pronunciations: vec![],
            frequencies: vec![],
        }
    }

    fn _create_source(
        original_text: String,
        transformed_text: String,
        deinflected_text: String,
        match_type: TermSourceMatchType,
        match_source: TermSourceMatchSource,
        is_primary: bool,
    ) -> TermSource {
        TermSource {
            original_text,
            transformed_text,
            deinflected_text,
            match_type,
            match_source,
            is_primary,
        }
    }

    fn _get_dictionary_alias(
        dictionary: String,
        enabled_dictionary_map: &EnabledDictionaryMapType,
    ) -> String {
        match enabled_dictionary_map {
            EnabledDictionaryMapType::Term(m) => match m.get(&dictionary) {
                Some(info) => info.alias.clone(),
                None => dictionary,
            },
            EnabledDictionaryMapType::Kanji(m) => match m.get(&dictionary) {
                Some(info) => info.alias.clone(),
                None => dictionary,
            },
        }
    }

    fn _get_dictionary_order(
        dictionary: &str,
        enabled_dictionary_map: &EnabledDictionaryMapType,
    ) -> usize {
        match enabled_dictionary_map {
            EnabledDictionaryMapType::Term(map) => {
                // If found, extract the index; if not, use the length of the map.
                map.get(dictionary)
                    .map(|info| info.index)
                    .unwrap_or(map.len())
            }
            EnabledDictionaryMapType::Kanji(map) => map
                .get(dictionary)
                .map(|info| info.index)
                .unwrap_or(map.len()),
        }
    }

    fn _get_deinflections(&self, text: &str, opts: &FindTermsOptions) -> Vec<DatabaseDeinflection> {
        let mut deinflections = if opts.deinflect {
            self._get_algorithm_deinflections(text, opts).unwrap()
        } else {
            vec![Translator::_create_deinflection(
                text,
                text,
                text,
                0,
                vec![],
                vec![],
            )]
        };
        if deinflections.is_empty() {
            return vec![];
        }

        let FindTermsOptions {
            match_type,
            language,
            enabled_dictionary_map,
            ..
        } = opts;

        self._add_entries_to_deinflections(
            language,
            &mut deinflections,
            enabled_dictionary_map,
            *match_type,
        );

        let dictionary_deinflections = self._get_dictionary_deinflections(
            language,
            &deinflections,
            enabled_dictionary_map,
            *match_type,
        );
        deinflections.extend(dictionary_deinflections);

        for deinflection in &mut deinflections {
            for mut entry in deinflection.database_entries.iter_mut() {
                entry
                    .definitions
                    .retain(|def| !matches!(def, TermGlossary::Deinflection(_)))
            }
            deinflection
                .database_entries
                .retain(|entry| !entry.definitions.is_empty());
        }
        deinflections.retain(|deinflection| !deinflection.database_entries.is_empty());

        deinflections
    }

    fn _get_dictionary_deinflections(
        &self,
        language: &str,
        deinflections_input: &[DatabaseDeinflection],
        enabled_dictionary_map: &FindTermDictionaryMap,
        match_type: TermSourceMatchType,
    ) -> Vec<DatabaseDeinflection> {
        let mut dictionary_deinflections: Vec<DatabaseDeinflection> = Vec::new();

        for deinflection_item in deinflections_input {
            let DatabaseDeinflection {
                original_text,
                transformed_text,
                text_processor_rule_chain_candidates,
                database_entries,
                ..
            } = deinflection_item;
            let algorithm_chains = &deinflection_item.inflection_rule_chain_candidates;

            for entry in database_entries {
                let TermEntry {
                    dictionary,
                    definitions,
                    ..
                } = entry;
                let entry_dictionary = enabled_dictionary_map.get(dictionary);
                let use_deinflections = match entry_dictionary {
                    Some(ed) => ed.use_deinflections,
                    None => true,
                };
                if !use_deinflections {
                    continue;
                }

                for definition_variant in definitions {
                    if let TermGlossary::Deinflection(term_glossary_deinflection) =
                        definition_variant
                    {
                        let TermGlossaryDeinflection {
                            uninflected: form_of,
                            inflection_rule_chain: inflection_rules,
                        } = term_glossary_deinflection;

                        if form_of.is_empty() {
                            continue;
                        }

                        let inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate> =
                            algorithm_chains
                                .iter()
                                .map(|alg_chain_candidate| {
                                    let alg_inflections = &alg_chain_candidate.inflection_rules;
                                    let source = if alg_inflections.is_empty() {
                                        InflectionSource::Dictionary
                                    } else {
                                        InflectionSource::Both
                                    };
                                    let combined_rules: Vec<String> = alg_inflections
                                        .iter()
                                        .cloned()
                                        .chain(inflection_rules.iter().cloned())
                                        .collect();
                                    InflectionRuleChainCandidate {
                                        source,
                                        inflection_rules: combined_rules,
                                    }
                                })
                                .collect();

                        let dictionary_deinflection = Translator::_create_deinflection(
                            original_text,
                            transformed_text,
                            form_of,
                            0,
                            text_processor_rule_chain_candidates.clone(),
                            inflection_rule_chain_candidates,
                        );
                        dictionary_deinflections.push(dictionary_deinflection);
                    }
                }
            }
        }

        self._add_entries_to_deinflections(
            language,
            &mut dictionary_deinflections,
            enabled_dictionary_map,
            match_type,
        );
        dictionary_deinflections
    }

    fn _get_algorithm_deinflections(
        &self,
        text: &str,
        opts: &FindTermsOptions,
    ) -> Result<Vec<DatabaseDeinflection>, TranslatorError> {
        let language = opts.language.clone();
        let Some(processors_for_lang) = self.text_processors.get(language.as_str()) else {
            return Err(TranslatorError::UnsupportedLanguage(language));
        };
        let PreAndPostProcessorsWithId { pre, post } = processors_for_lang;
        let mut db_deinflections: Vec<DatabaseDeinflection> = Vec::new();
        // for reusing text processor's outputs
        let mut source_cache = IndexMap::new();
        let mut raw_source = text.to_string();
        while !raw_source.is_empty() {
            let text_replacements = Translator::_get_text_replacement_variants(opts.clone());
            let pre_processed_text_variants = Translator::_get_text_variants(
                &raw_source,
                pre,
                text_replacements,
                &mut source_cache,
            );
            for pre_processed_variant in pre_processed_text_variants {
                let (source, preprocessor_rule_chain_candidates) = pre_processed_variant;
                let deinflections = self.mlt.transform(&language, &source);
                for deinflection in deinflections {
                    let TransformedText {
                        trace, conditions, ..
                    } = deinflection;
                    let postprocessed_text_variants = Translator::_get_text_variants(
                        &deinflection.text,
                        post,
                        vec![None],
                        &mut source_cache,
                    );
                    for post_processed_variant in postprocessed_text_variants {
                        let (transformed_text, postprocessor_rule_chain_candidates) =
                            post_processed_variant;
                        let inflection_rule_chain_candidate = InflectionRuleChainCandidate {
                            source: InflectionSource::Algorithm,
                            inflection_rules: trace
                                .iter()
                                .map(|frame| frame.transform.clone())
                                .collect(),
                        };

                        // every combination of preprocessor rule candidates
                        // and postprocessor rule candidates
                        let text_processor_rule_chain_candidates: Vec<Vec<String>> =
                            preprocessor_rule_chain_candidates
                                .iter()
                                .flat_map(|pre_candidate_slice| {
                                    postprocessor_rule_chain_candidates.iter().map(
                                        move |post_candidate_slice| {
                                            pre_candidate_slice
                                                .iter()
                                                .cloned()
                                                .chain(post_candidate_slice.iter().cloned())
                                                .collect::<Vec<String>>()
                                        },
                                    )
                                })
                                .collect();
                        let new_deinflection = Translator::_create_deinflection(
                            &raw_source,
                            &source,
                            &transformed_text,
                            conditions,
                            text_processor_rule_chain_candidates,
                            vec![inflection_rule_chain_candidate],
                        );
                        db_deinflections.push(new_deinflection);
                    }
                }
            }
            raw_source = Translator::_get_next_substring(opts.search_resolution, &raw_source);
        }
        Ok(db_deinflections)
    }

    fn _get_text_variants(
        text: &str,
        text_processors: &[TextProcessorWithId],
        text_replacements: FindTermsTextReplacements,
        text_cache: &mut TextCache,
    ) -> VariantAndTextProcessorRuleChainCandidatesMap {
        let mut variants_map: VariantAndTextProcessorRuleChainCandidatesMap = IndexMap::new();
        variants_map.insert(text.to_string(), vec![vec![]]);
        for (id, replacement) in text_replacements.iter().enumerate() {
            let Some(replacement) = replacement else {
                continue;
            };
            let k = Translator::_apply_text_replacements(text, replacement);
            let v = vec![vec![format!("Text Replacement {id}")]];
            variants_map.insert(k, v);
        }
        for processor in text_processors {
            let TextProcessorWithId { id, processor } = processor;
            let TextProcessor {
                options, process, ..
            } = processor;
            let mut new_variants_map: VariantAndTextProcessorRuleChainCandidatesMap =
                IndexMap::new();
            for variant in variants_map.iter() {
                let (variant, current_preprocessor_rule_chain_candidates) = variant;
                for opt in options.iter() {
                    let processed = Translator::_get_processed_text(
                        text_cache,
                        variant.clone(),
                        id.to_string(),
                        opt.clone(),
                        *process,
                    );
                    let existing_candidates = new_variants_map.get(&processed);
                    let mapped_current_preprocessor_rule_chain_candidates: Vec<Vec<String>> =
                        current_preprocessor_rule_chain_candidates
                            .clone()
                            .into_iter()
                            .map(|mut candidate: Vec<String>| {
                                candidate.push(id.to_string());
                                candidate
                            })
                            .collect();
                    // ignore if applying text_processor !change source
                    if processed == *variant {
                        if let Some(existing_candidates) = existing_candidates {
                            new_variants_map.insert(processed, existing_candidates.clone());
                        } else {
                            new_variants_map.insert(
                                processed,
                                current_preprocessor_rule_chain_candidates.clone(),
                            );
                        }
                    } else if let Some(existing_candidates) = existing_candidates {
                        let concat = existing_candidates
                            .clone()
                            .into_iter()
                            .chain(mapped_current_preprocessor_rule_chain_candidates.into_iter())
                            .collect();
                        new_variants_map.insert(processed, concat);
                    } else {
                        new_variants_map
                            .insert(processed, mapped_current_preprocessor_rule_chain_candidates);
                    }
                }
            }
            variants_map = new_variants_map;
        }
        variants_map
    }

    fn _add_entries_to_deinflections(
        &self,
        language: &str,
        deinflections: &mut [DatabaseDeinflection],
        enabled_dictionary_map: &FindTermDictionaryMap,
        match_type: TermSourceMatchType,
    ) {
        let mut unique_deinflections_map = Translator::_group_deinflections_by_term(deinflections);

        let unique_deinflections_owned_terms: Vec<String> =
            unique_deinflections_map.keys().cloned().collect();

        let unique_deinflections_terms_refs: Vec<&String> =
            unique_deinflections_owned_terms.iter().collect();

        let mut unique_deinflection_arrays: Vec<&mut Vec<DatabaseDeinflection>> =
            unique_deinflections_map.values_mut().collect();

        let database_entries = self
            .db
            .find_terms_bulk(
                &unique_deinflections_terms_refs,
                &enabled_dictionary_map,
                match_type,
            )
            .unwrap(); // Consider error handling instead of unwrap

        self._match_entries_to_deinflections(
            language,
            &database_entries,
            &mut unique_deinflection_arrays,
            enabled_dictionary_map,
        );
    }

    fn _match_entries_to_deinflections(
        &self,
        language: &str,
        database_entries: &[TermEntry],
        unique_deinflection_arrays: &mut Vec<&mut Vec<DatabaseDeinflection>>,
        enabled_dictionary_map: &FindTermDictionaryMap,
    ) {
        for entry in database_entries {
            let entry_dictionary = enabled_dictionary_map
                .get(&entry.dictionary)
                .unwrap_or_else(|| {
                    panic!(
                        "{} was not found in enabled_dictionary_map",
                        &entry.dictionary
                    )
                });
            let parts_of_speech_filter = entry_dictionary.parts_of_speech_filter;
            let definition_conditions = self
                .mlt
                .get_condition_flags_from_parts_of_speech(language, &entry.rules);

            for deinflection in &mut **unique_deinflection_arrays
                .get_mut(entry.index)
                .unwrap_or_else(|| {
                    panic!(
                        "unique_deinflection_array.get({}) doesn't exist inside the array",
                        entry.index
                    )
                })
            {
                if !parts_of_speech_filter
                    || LanguageTransformer::conditions_match(
                        deinflection.conditions,
                        definition_conditions,
                    )
                {
                    deinflection.database_entries.push(entry.clone());
                }
            }
        }
    }

    /// this might be incorrect based on the javascript function
    fn _group_deinflections_by_term(
        deinflections: &[DatabaseDeinflection],
    ) -> IndexMap<String, Vec<DatabaseDeinflection>> {
        let mut result: IndexMap<String, Vec<DatabaseDeinflection>> = IndexMap::new();
        for deinflection in deinflections {
            let key = deinflection.deinflected_text.clone();
            result.entry(key).or_default().push(deinflection.clone());
        }
        result
    }

    /// helper function to return `(opts: FindTermOptions).text_replacements`
    fn _get_text_replacement_variants(opts: FindTermsOptions) -> FindTermsTextReplacements {
        opts.text_replacements
    }

    fn _create_deinflection(
        original_text: &str,
        transformed_text: &str,
        deinflected_text: &str,
        conditions: usize,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
    ) -> DatabaseDeinflection {
        DatabaseDeinflection {
            original_text: original_text.to_string(),
            transformed_text: transformed_text.to_string(),
            deinflected_text: deinflected_text.to_string(),
            conditions,
            text_processor_rule_chain_candidates,
            inflection_rule_chain_candidates,
            database_entries: vec![],
        }
    }

    fn _apply_text_replacements(text: &str, replacements: &[FindTermsTextReplacement]) -> String {
        let mut text = text.to_string();
        for replacement in replacements {
            let FindTermsTextReplacement {
                pattern,
                replacement,
                is_global,
            } = replacement;
            text = apply_text_replacement(&text, pattern, replacement, is_global);
        }
        text
    }

    // `or_default()` might not have the same behavior as the javascript version
    fn _get_processed_text(
        text_cache: &mut TextCache,
        text_key: String,
        id_key: String,
        setting: TextProcessorSetting,
        process: fn(&str, TextProcessorSetting) -> String,
    ) -> String {
        // Level 1: Access or create the cache for the given `text_key`.
        // `entry` API gets a mutable reference to the value if key exists,
        // or inserts a new IndexMap and returns a mutable reference to it.
        let level1_map = text_cache.entry(text_key.clone()).or_default();

        // Level 2: Access or create the cache for the given `id_key` within level1_map.
        let level2_map = level1_map.entry(id_key).or_default();

        // Level 3: Check if the (setting -> processed_text) mapping exists in level2_map.
        if let Some(cached_processed_text_ref) = level2_map.get(&setting) {
            // Cache hit: `cached_processed_text_ref` is `&&'static str`.
            cached_processed_text_ref.to_string()
        } else {
            // Cache miss: process the text, store it in the cache, and then return it.
            let processed_text_string: String = process(&text_key, setting.clone());
            level2_map.insert(setting, processed_text_string.clone());
            processed_text_string
        }
    }

    fn _get_next_substring(search_resolution: SearchResolution, current_str: &str) -> String {
        let end_byte_index: usize;

        if search_resolution == SearchResolution::Word {
            if let Some(mat) = GET_NEXT_SUBSTRING_REGEX.find(current_str).unwrap() {
                end_byte_index = mat.start();
            } else {
                end_byte_index = 0;
            }
        } else {
            let char_count = current_str.chars().count();

            if char_count <= 1 {
                end_byte_index = 0;
            } else {
                end_byte_index = current_str.char_indices().nth(char_count - 1).unwrap().0;
            }
        }

        String::from(&current_str[0..end_byte_index])
    }

    /// Returns the initial portion of a string containing only Japanese or Chinese characters.
    ///
    /// It scans the input string and returns a slice ending just before the first
    /// character that is not considered Japanese or Chinese. If all characters
    /// are Japanese or Chinese, the entire input string slice is returned.
    ///
    /// # Arguments
    /// * `text` - The input string slice to process.
    ///
    /// # Returns
    /// A string slice containing only the leading Japanese/Chinese characters.
    fn get_japanese_chinese_only_text(text: &str) -> String {
        // .char_indices() iterates, giving the starting byte index and the char.
        for (byte_index, c) in text.char_indices() {
            // `c` *is* the character (Unicode Scalar Value).
            let code_point = c as u32;

            // Check if the character is *not* Japanese and *not* Chinese.
            if !is_code_point_japanese(code_point) && !is_code_point_chinese(code_point) {
                // If it's not, we've found the boundary.
                // We return a slice from the beginning (0) up to the
                // *start* of the current non-matching character (byte_index).
                return text[..byte_index].to_string();
            }
        }

        // If the loop finished without returning, it means all characters
        // were either Japanese or Chinese. Return the whole string.
        text.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct TagGroup {
    dictionary: String,
    tag_names: Vec<String>,
}

#[derive(Clone, Debug)]
struct TagTargetItem {
    query: &'static str,
    dictionary: String,
    tag_name: String,
    cache: Option<TagCache>,
    database_tag: DictionaryDatabaseTag,
    targets: Vec<Vec<DictionaryTag>>,
}

#[derive(Clone, Debug)]
struct TagExpansionTarget {
    tags: Vec<DictionaryTag>,
    tag_groups: Vec<TagGroup>,
}

#[derive(Clone, Debug, Default)]
struct TranslatorTagAggregator {
    tag_expansion_target_map: IndexMap<Vec<DictionaryTag>, Vec<TagGroup>>,
}

impl TranslatorTagAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds tags to a specific dictionary group associated with a primary list of tags.
    ///
    /// # Arguments
    /// * `tags_key` - The primary list of DictionaryTags (the key).
    /// * `dictionary_name` - The name of the dictionary to associate the new tag names with.
    /// * `tag_names_to_add` - A slice of tag names to add to the specified dictionary group.
    pub fn add_tags(
        &mut self,
        tags_key: &[DictionaryTag],
        dictionary_name: &str,
        tag_names_to_add: &[String],
    ) {
        if tag_names_to_add.is_empty() {
            return;
        }

        let target_collection_of_tag_groups = self._get_or_create_tag_groups(tags_key);
        let specific_tag_group = Self::_get_or_create_tag_group_in_collection(
            target_collection_of_tag_groups,
            dictionary_name,
        );
        Self::_add_unique_tags_to_group(specific_tag_group, tag_names_to_add);
    }

    /// Retrieves all tag expansion targets.
    /// Each target consists of a list of primary tags and their associated grouped tags.
    pub fn get_tag_expansion_targets(&self) -> Vec<TagExpansionTarget> {
        self.tag_expansion_target_map
            .iter()
            .map(|(tags_vec, tag_groups_vec)| TagExpansionTarget {
                tags: tags_vec.clone(),
                tag_groups: tag_groups_vec.clone(),
            })
            .collect()
    }

    /// Merges tag groups from a source entry (identified by `tags_key_source`)
    /// into a target entry (identified by `tags_key_target`).
    pub fn merge_tags(
        &mut self,
        tags_key_target: &[DictionaryTag], // The key for the destination entry
        tags_key_source: &[DictionaryTag], // The key for the source entry
    ) {
        // Get the source tag groups.
        // We clone them here to avoid borrowing issues,
        // as we'll need mutable access to
        // `self.tag_expansion_target_map` shortly after for the target.
        let source_groups_list_option = self.tag_expansion_target_map.get(tags_key_source).cloned(); // Clones Option<Vec<TagGroup>>

        if let Some(source_groups_list) = source_groups_list_option {
            // Now `source_groups_list` is an owned Vec<TagGroup>.
            // Get or create the target collection of tag groups.
            // This mutably borrows `self`.
            let target_collection_of_tag_groups = self._get_or_create_tag_groups(tags_key_target);

            for source_tag_group_item in source_groups_list {
                // For each TagGroup from the source,
                // find/create a corresponding one in the target's collection
                // and add its tags.
                let specific_target_tag_group = Self::_get_or_create_tag_group_in_collection(
                    target_collection_of_tag_groups,
                    &source_tag_group_item.dictionary,
                );
                Self::_add_unique_tags_to_group(
                    specific_target_tag_group,
                    &source_tag_group_item.tag_names,
                );
            }
        }
        // If `tags_key_source` is not in the map,
        // `source_groups_list_option` is None, and nothing happens,
        // which matches the original JavaScript logic.
    }

    // --- Helper Methods (private by default) ---

    /// Gets or creates a mutable reference to the list of TagGroups for a given key.
    /// The key `tags_key_slice` is cloned to
    /// create an owned `Vec<DictionaryTag>` for map insertion if needed.
    fn _get_or_create_tag_groups(
        &mut self,
        tags_key_slice: &[DictionaryTag],
    ) -> &mut Vec<TagGroup> {
        self.tag_expansion_target_map
            // Clones the slice into an owned Vec for the key
            .entry(tags_key_slice.to_vec())
            .or_default()
    }

    /// Finds or creates a specific TagGroup within a given collection of
    /// TagGroups, based on dictionary name.
    /// Returns a mutable reference to the TagGroup.
    fn _get_or_create_tag_group_in_collection<'cl>(
        collection_of_tag_groups: &'cl mut Vec<TagGroup>,
        dictionary_name: &str,
    ) -> &'cl mut TagGroup {
        // 1. Search for the index using an *immutable* borrow first.
        //    This borrow ends as soon as `position` gets its value.
        let position = collection_of_tag_groups
            .iter()
            .position(|group| group.dictionary == dictionary_name);

        // 2. Check if we found an index.
        if let Some(pos) = position {
            // If yes, *now* we create a mutable borrow using the index
            // and return it immediately. The compiler knows this is safe
            // because we don't proceed to the `push` part.
            return &mut collection_of_tag_groups[pos];
        }

        // 3. If we didn't find it, we reach here. No borrows are active.
        //    Create the new group.
        let new_tag_group = TagGroup {
            dictionary: dictionary_name.to_string(),
            tag_names: Vec::new(),
        };

        // 4. Perform a mutable borrow for `push`. This is safe.
        collection_of_tag_groups.push(new_tag_group);

        // 5. Perform a mutable borrow for `last_mut`. This is safe.
        //    We `unwrap` because we *know* we just pushed an element.
        //    This borrow is then returned.
        collection_of_tag_groups.last_mut().unwrap()
    }

    /// Adds new tag names to a TagGroup, ensuring uniqueness.
    fn _add_unique_tags_to_group(tag_group: &mut TagGroup, new_tag_names_slice: &[String]) {
        for tag_name_to_add in new_tag_names_slice {
            if !tag_group.tag_names.contains(tag_name_to_add) {
                tag_group.tag_names.push(tag_name_to_add.clone());
            }
        }
    }
}
