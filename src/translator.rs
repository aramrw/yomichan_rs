use crate::{
    backend::FindTermsDetails,
    collect_variant_data_ref,
    database::{
        self,
        dictionary_database::{
            DatabaseTag, DatabaseTermMeta, DictionaryDatabase, DictionarySet, GenericQueryRequest,
            PhoneticTranscription, PitchAccent, Pronunciation, QueryRequestMatchType, QueryType,
            TermEntry, TermExactQueryRequest, TermPronunciationMatchType,
        },
    },
    dictionary::{
        self, DictionaryEntryType, DictionaryTag, EntryInflectionRuleChainCandidatesKey,
        TermDefinition, TermDictionaryEntry, TermFrequency, TermHeadword, TermPronunciation,
        TermSource, TermSourceMatchSource, TermSourceMatchType, VecNumOrNum,
    },
    dictionary_data::{
        FrequencyInfo, GenericFreqData, MetaDataMatchType, Pitch, TermMetaFreqDataMatchType,
        TermMetaModeType,
    },
    freq, iter_type_to_iter_variant, iter_variant_to_iter_type,
    regex_util::apply_text_replacement,
    settings::{
        DictionaryOptions, GeneralOptions, ProfileOptions, ScanningOptions, SearchResolution,
        SortFrequencyDictionaryOrder, TranslationOptions, TranslationTextReplacementGroup,
        TranslationTextReplacementOptions,
    },
    structured_content::{TermGlossary, TermGlossaryContent, TermGlossaryDeinflection},
    test_utils, to_variant,
    translation::{
        FindKanjiDictionary, FindTermDictionary, FindTermDictionaryMap, FindTermsMatchType,
        FindTermsOptions, FindTermsSortOrder,
    },
    translation_internal::{
        DatabaseDeinflection, DictionaryEntryGroup, FindInternalTermsResult,
        InternalTermDictionaryEntry, TextCache, TextProcessorRuleChainCandidate,
        VariantAndTextProcessorRuleChainCandidatesMap,
    },
};
use derive_more::From;
use fancy_regex::Regex;
use icu::{
    collator::{options::CollatorOptions, Collator, CollatorBorrowed},
    datetime::provider::neo::marker_attrs::PATTERN_MEDIUM,
    locale::locale,
};
use indexmap::{IndexMap, IndexSet};
use language_transformer::transformer::{
    InflectionRuleChainCandidate, InflectionSource, InternalInflectionRuleChainCandidate,
};
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
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::VecDeque,
    fmt::Display,
    hash::Hash,
    iter, mem,
    ops::Index,
    path::Path,
    rc::Rc,
    str::FromStr,
    sync::{Arc, LazyLock},
};

/// class which finds term and kanji dictionary entries for text.
pub struct Translator<'a> {
    pub db: Arc<DictionaryDatabase<'a>>,
    pub mlt: MultiLanguageTransformer,
    pub tag_cache: IndexMap<String, TagCache>,
    /// Invariant Locale
    /// Default: "en-US"
    pub string_comparer: CollatorBorrowed<'a>,
    pub number_regex: &'static Regex,
    pub text_processors: TextProcessorMap,
    pub reading_normalizers: ReadingNormalizerMap,
}

static TRANSLATOR_NUMBER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?").unwrap());

impl<'a> Translator<'a> {
    pub fn new(db: Arc<DictionaryDatabase<'a>>) -> Self {
        let mut translator = Self::init(db);
        translator.prepare();
        translator
    }
    fn init(db: Arc<DictionaryDatabase<'a>>) -> Self {
        Self {
            db,
            mlt: MultiLanguageTransformer::default(),
            tag_cache: IndexMap::new(),
            string_comparer: Collator::try_new(locale!("en-US").into(), CollatorOptions::default())
                .unwrap(),
            number_regex: &*TRANSLATOR_NUMBER_REGEX,
            text_processors: IndexMap::new(),
            reading_normalizers: IndexMap::new(),
        }
    }
    /// Initializes the instance for use.
    /// The public API should not be used until this function has been called.
    /// fn new() will call it automatically,
    /// if something needs to be done before call .init() -> work -> .prepare();
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
    /// `mode`: The [FindTermsMode] determines the format of the resulting array.
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
    pub fn find_terms(
        &mut self,
        mode: FindTermsMode,
        text: &str,
        opts: &FindTermsOptions,
    ) -> FindTermsResult {
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
        let FindInternalTermsResult {
            mut dictionary_entries,
            original_text_length,
        } = self.find_terms_internal(&mut text, opts, &mut tag_aggregator, primary_reading);
        match mode {
            FindTermsMode::Group => {
                dictionary_entries = self._group_dictionary_entries_by_headword(
                    language,
                    &dictionary_entries,
                    &mut tag_aggregator,
                    primary_reading,
                )
            }
            FindTermsMode::Merge => {
                dictionary_entries = self._get_related_dictionary_entries(
                    &dictionary_entries,
                    opts,
                    &mut tag_aggregator,
                )
            }
            _ => {}
        }
        if let Some(exclude_dictionary_definitions) = exclude_dictionary_definitions {
            Translator::_remove_excluded_definitions(
                &mut dictionary_entries,
                exclude_dictionary_definitions,
            );
        }
        match mode != FindTermsMode::Simple {
            true => {
                self._add_term_meta(
                    &mut dictionary_entries,
                    enabled_dictionary_map,
                    &mut tag_aggregator,
                );
                self._expand_tag_groups_and_group(&mut tag_aggregator.get_tag_expansion_targets());
            }
            // this is most likely incorrect because of references
            false => {
                if let Some(sort_frequency_dictionary) = sort_frequency_dictionary {
                    let mut sort_dictionary_map: TermEnabledDictionaryMap = IndexMap::new();
                    let value = enabled_dictionary_map.get(sort_frequency_dictionary);
                    // in js the values inside the map (sort_frequency_dictionary & value)
                    // return mutable references with .get(),
                    // if _add_term_meta updates these, it's updating clones in rust
                    // sort_dictionary_map might need
                    // to hold mutable refs since it doesnt get used after this block
                    //
                    // if this is true, don't use the TermEnabledDictionaryMap type,
                    // as it holds owned values
                    if let Some(value) = value {
                        sort_dictionary_map
                            .insert(sort_frequency_dictionary.clone(), value.clone());
                    }
                    self._add_term_meta(
                        &mut dictionary_entries,
                        &sort_dictionary_map,
                        &mut tag_aggregator,
                    );
                }
            }
        }

        if let Some(sort_frequency_dictionary) = sort_frequency_dictionary {
            Translator::_update_sort_frequencies(
                &mut dictionary_entries,
                sort_frequency_dictionary,
                *sort_frequency_dictionary_order == FindTermsSortOrder::Ascending,
            );
        }
        if dictionary_entries.len() > 1 {
            self._sort_term_dictionary_entries(&mut dictionary_entries);
        }

        dictionary_entries.iter_mut().for_each(|mut entry| {
            let definitions = &mut entry.definitions;
            Translator::_flag_redundant_definition_tags(definitions);
            if definitions.len() > 1 {
                Translator::_sort_term_dictionary_entry_definitions_mut(definitions);
            }

            // Frequencies part
            if entry.frequencies.len() > 1 {
                let mut sorted_frequencies_as_term_type: Vec<TermType> =
                    entry.frequencies.iter().map(|f| f.clone().into()).collect();

                Translator::_sort_term_dictionary_entry_simple_data_mut(
                    &mut sorted_frequencies_as_term_type,
                );

                entry.frequencies = sorted_frequencies_as_term_type
                    .into_iter()
                    .map(|tt| tt.try_into().unwrap())
                    .collect();
            }

            if entry.pronunciations.len() > 1 {
                let mut sorted_pronunciations_as_term_type: Vec<TermType> = entry
                    .pronunciations
                    .iter()
                    .map(|p| p.clone().into())
                    .collect();

                Translator::_sort_term_dictionary_entry_simple_data_mut(
                    &mut sorted_pronunciations_as_term_type,
                );
                entry.pronunciations = sorted_pronunciations_as_term_type
                    .into_iter()
                    .map(|tt| tt.try_into().unwrap())
                    .collect();
            }
        });
        let with_user_facing_inflections =
            self._add_user_facing_inflections(language, dictionary_entries);

        FindTermsResult {
            dictionary_entries: with_user_facing_inflections,
            original_text_length,
        }
    }

    /// Creates an options object for use with `Translator.findTerms`.
    pub fn _get_translator_find_terms_options(
        mode: FindTermsMode,
        details: &FindTermsDetails,
        opts: &ProfileOptions,
    ) -> FindTermsOptions {
        let FindTermsDetails {
            mut match_type,
            mut deinflect,
            mut primary_reading,
        } = details.clone();
        if match_type.is_none() {
            match_type = Some(FindTermsMatchType::Exact);
        }
        if deinflect.is_none() {
            deinflect = Some(true);
        }
        if primary_reading.is_none() {
            primary_reading = Some("".to_string());
        }

        let mut enabled_dictionary_map = Self::_get_translator_enabled_dictionary_map(opts);
        let ProfileOptions {
            general:
                GeneralOptions {
                    main_dictionary,
                    sort_frequency_dictionary,
                    sort_frequency_dictionary_order,
                    language,
                    ..
                },
            scanning: ScanningOptions { alphanumeric, .. },
            translation:
                TranslationOptions {
                    text_replacements: text_replacements_opts,
                    search_resolution,
                    ..
                },
            ..
        } = opts;

        let text_replacements = Self::_get_translator_text_replacements(text_replacements_opts);
        let mut exclude_dictionary_definitions: Option<IndexSet<String>> = None;
        if mode == FindTermsMode::Merge && !enabled_dictionary_map.contains_key(main_dictionary) {
            let new = FindTermDictionary {
                index: enabled_dictionary_map.len(),
                alias: main_dictionary.to_string(),
                allow_secondary_searches: false,
                parts_of_speech_filter: true,
                use_deinflections: true,
            };
            enabled_dictionary_map.insert(main_dictionary.clone(), new);
            exclude_dictionary_definitions = Some(IndexSet::new());
            // safe
            exclude_dictionary_definitions
                .as_mut()
                .unwrap()
                .insert(main_dictionary.clone());
        }
        FindTermsOptions {
            match_type: match_type.unwrap(),
            deinflect: deinflect.unwrap(),
            primary_reading: primary_reading.unwrap(),
            main_dictionary: main_dictionary.to_string(),
            sort_frequency_dictionary: sort_frequency_dictionary.clone(),
            sort_frequency_dictionary_order: *sort_frequency_dictionary_order,
            remove_non_japanese_characters: !*alphanumeric,
            text_replacements: text_replacements.clone().into(),
            enabled_dictionary_map,
            exclude_dictionary_definitions: exclude_dictionary_definitions.clone(),
            search_resolution: *search_resolution,
            language: language.to_string(),
        }
    }

    fn _get_translator_enabled_dictionary_map(opts: &ProfileOptions) -> TermEnabledDictionaryMap {
        let mut enabled_dictionary_map: TermEnabledDictionaryMap = IndexMap::new();
        for (key, dictionary) in &opts.dictionaries {
            if !dictionary.enabled {
                continue;
            }
            let DictionaryOptions {
                name,
                alias,
                allow_secondary_searches,
                definitions_collapsible,
                parts_of_speech_filter,
                use_deinflections,
                styles,
                ..
            } = dictionary;
            let new = FindTermDictionary {
                index: enabled_dictionary_map.len(),
                alias: alias.clone(),
                allow_secondary_searches: *allow_secondary_searches,
                parts_of_speech_filter: *parts_of_speech_filter,
                use_deinflections: *use_deinflections,
            };
            enabled_dictionary_map.insert(name.clone(), new);
        }
        enabled_dictionary_map
    }

    fn _get_translator_text_replacements(
        text_replacements_options: &TranslationTextReplacementOptions,
    ) -> VecDeque<Option<Vec<FindTermsTextReplacement>>> {
        let mut text_replacements = VecDeque::new();
        for group in &text_replacements_options.groups {
            let mut text_replacement_entries: Vec<FindTermsTextReplacement> = vec![];
            for TranslationTextReplacementGroup {
                pattern,
                ignore_case,
                replacement,
                ..
            } in group
            {
                let re_pattern = if *ignore_case {
                    format!("(?i){pattern}")
                } else {
                    pattern.to_string()
                };
                let Ok(pattern_regex) = Regex::new(&re_pattern) else {
                    // invalid pattern
                    continue;
                };
                let new = FindTermsTextReplacement {
                    pattern: pattern_regex,
                    replacement: replacement.to_string(),
                    is_global: true,
                };
                text_replacement_entries.push(new);
            }
            if !text_replacement_entries.is_empty() {
                text_replacements.push_back(Some(text_replacement_entries));
            }
        }
        if !text_replacements.is_empty() || text_replacements_options.search_original {
            text_replacements.push_front(None);
        }
        text_replacements
    }

    fn _add_user_facing_inflections(
        &self,
        language: &str,
        dictionary_entries: Vec<InternalTermDictionaryEntry>,
    ) -> Vec<TermDictionaryEntry> {
        let mut result: Vec<TermDictionaryEntry> = Vec::with_capacity(dictionary_entries.len());

        for dictionary_entry in dictionary_entries {
            let expanded_chains: Vec<InflectionRuleChainCandidate> = dictionary_entry
                .inflection_rule_chain_candidates
                .iter()
                .map(|original_candidate| {
                    let inflection_rules = &original_candidate.inflection_rules;
                    let new_rules = self
                        .mlt
                        .get_user_facing_inflection_rules(language, inflection_rules);

                    InflectionRuleChainCandidate {
                        source: original_candidate.source,
                        inflection_rules: new_rules,
                    }
                })
                .collect();

            let InternalTermDictionaryEntry {
                entry_type,
                is_primary,
                text_processor_rule_chain_candidates,
                inflection_rule_chain_candidates,
                score,
                frequency_order,
                dictionary_alias,
                dictionary_index,
                source_term_exact_match_count,
                match_primary_reading,
                max_original_text_length,
                headwords,
                definitions,
                pronunciations,
                frequencies,
            } = dictionary_entry;

            // Make the InternalTermDictionaryEntry into a TermDictionaryEntry
            // only replacing its inflectionRuleChainCandidates.
            let mut new_entry = TermDictionaryEntry {
                entry_type,
                is_primary,
                text_processor_rule_chain_candidates,
                inflection_rule_chain_candidates: expanded_chains,
                score,
                frequency_order,
                dictionary_alias,
                dictionary_index,
                source_term_exact_match_count,
                match_primary_reading,
                max_original_text_length,
                headwords,
                definitions,
                pronunciations,
                frequencies,
            };
            result.push(new_entry);
        }

        result
    }

    /// Sorts a slice of items implementing `IsTermType` in place.
    ///
    /// This function is intended for lists containing only
    /// [TermFrequency] or [TermPronunciation]
    /// variants of `TermType`
    /// (or other types that correctly implement `IsTermType` for these fields).
    ///
    /// Sorting criteria (all ascending):
    /// 1. `ipa_or_freq_headword_index`
    /// 2. `dictionary_index`
    /// 3. `index`
    pub fn _sort_term_dictionary_entry_simple_data_mut(data_list: &mut [impl IsTermType]) {
        data_list.sort_by(|v1, v2| {
            // 1. Sort by headword order (using ipa_or_freq_headword_index)
            v1._get_ipa_or_freq_headword_index()
                .cmp(&v2._get_ipa_or_freq_headword_index())
                // 2. Sort by dictionary order
                .then_with(|| v1.dictionary_index().cmp(&v2.dictionary_index()))
                // 3. Sort by original index (default order)
                .then_with(|| v1.index().cmp(&v2.index()))
        });
    }

    /// Sorts a slice of `TermDefinition` structs in place.
    ///
    /// The sorting criteria are applied in the following order:
    /// 1. `frequency_order` (ascending).
    /// 2. `dictionary_index` (ascending).
    /// 3. `score` (descending).
    /// 4. `headword_indices`:
    ///    a. Length of `headword_indices` (descending).
    ///    b. Element-wise values of `headword_indices` (ascending), if lengths are equal.
    /// 5. Original `index` (ascending) as a final tie-breaker.
    fn _sort_term_dictionary_entry_definitions_mut(definitions: &mut [TermDefinition]) {
        definitions.sort_by(|v1, v2| {
            // 1. Sort by frequency_order (ascending)
            v1.frequency_order
                .cmp(&v2.frequency_order)
                // 2. Sort by dictionary_index (ascending)
                .then_with(|| v1.dictionary_index.cmp(&v2.dictionary_index))
                .then_with(|| v2.score.cmp(&v1.score))
                // 4. Sort by headword_indices
                .then_with(|| {
                    // 4a. Length of headword_indices (descending)
                    // Compare lengths: v2's length vs v1's length for descending order.
                    v2.headword_indices
                        .len()
                        .cmp(&v1.headword_indices.len())
                        // 4b. Element-wise values (ascending), if lengths are equal.
                        // Vec<T>::cmp (where T: Ord)
                        // compares lexicographically if lengths are equal.
                        .then_with(|| v1.headword_indices.cmp(&v2.headword_indices))
                })
                // 5. Sort by original index (ascending)
                .then_with(|| v1.index.cmp(&v2.index))
        });
    }

    fn _flag_redundant_definition_tags(definitions: &mut Vec<TermDefinition>) {
        if definitions.is_empty() {
            return;
        }

        let mut last_dictionary = None;
        let mut last_part_of_speech = "".to_string();
        let mut remove_categories_set: IndexSet<String> = IndexSet::new();

        for TermDefinition {
            dictionary, tags, ..
        } in definitions
        {
            let tag_names_with_category =
                Translator::_get_tag_names_with_category(tags, "partOfSpeech");
            let part_of_speech = Translator::_create_map_key(&tag_names_with_category);

            if last_dictionary.as_ref().is_some_and(|ld| *ld != dictionary) {
                last_dictionary = Some(dictionary);
                last_part_of_speech = "".into();
            }

            if last_part_of_speech == part_of_speech {
                remove_categories_set.insert("partOfSpeech".into());
            } else {
                last_part_of_speech = part_of_speech;
            }

            if !remove_categories_set.is_empty() {
                tags.iter_mut().for_each(|tag| {
                    if remove_categories_set.contains(&tag.category) {
                        tag.redundant = true;
                    }
                });
                // theres no need for this, it gets dropped
                // after this if block
                // remove_categories_set.clear();
            }
        }
    }

    fn _get_tag_names_with_category(tags: &[DictionaryTag], category: &str) -> Vec<String> {
        let mut res = tags
            .iter()
            .filter_map(|tag| {
                if tag.category != category {
                    return None;
                }
                Some(tag.name.clone())
            })
            .collect::<Vec<String>>();
        res.sort();
        res
    }

    fn _sort_term_dictionary_entries(
        &self,
        dictionary_entries: &mut [InternalTermDictionaryEntry],
    ) {
        let string_comparer = &self.string_comparer;

        dictionary_entries.sort_by(|v1, v2| {
            // 1. Sort by reading match (descending: true comes before false)
            // (v2.match_primary_reading ? 1 : 0) - (v1.match_primary_reading ? 1 : 0)
            // This means v2 true and v1 false -> 1 (v1 < v2, so v2 comes first)
            // v2 false and v1 true -> -1 (v1 > v2, so v1 comes first)
            // In Rust, .cmp orders bools as false < true. We want true first, so v2.cmp(&v1).
            let cmp = v2.match_primary_reading.cmp(&v1.match_primary_reading);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 2. Sort by length of source term (descending)
            // i = v2.max_original_text_length - v1.max_original_text_length;
            let cmp = v2
                .max_original_text_length
                .cmp(&v1.max_original_text_length);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 3. Sort by length of the shortest text processing chain (ascending)
            // i = self._get_shortest_text_processing_chain_length(v1.text_processor_rule_chain_candidates) - self._get_shortest_text_processing_chain_length(v2.text_processor_rule_chain_candidates);
            let len1 = self._get_shortest_text_processing_chain_length(
                &v1.text_processor_rule_chain_candidates,
            );
            let len2 = self._get_shortest_text_processing_chain_length(
                &v2.text_processor_rule_chain_candidates,
            );
            let cmp = len1.cmp(&len2);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 4. Sort by length of the shortest inflection chain (ascending)
            // i = self._get_shortest_inflection_chain_length(v1.inflection_rule_chain_candidates) - self._get_shortest_inflection_chain_length(v2.inflection_rule_chain_candidates);
            let len1 =
                self._get_shortest_inflection_chain_length(&v1.inflection_rule_chain_candidates);
            let len2 =
                self._get_shortest_inflection_chain_length(&v2.inflection_rule_chain_candidates);
            let cmp = len1.cmp(&len2);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 5. Sort by how many terms exactly match the source (descending)
            // i = v2.source_term_exact_match_count - v1.source_term_exact_match_count;
            let cmp = v2
                .source_term_exact_match_count
                .cmp(&v1.source_term_exact_match_count);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 6. Sort by frequency order (ascending)
            // i = v1.frequency_order - v2.frequency_order;
            let cmp = v1.frequency_order.cmp(&v2.frequency_order);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 7. Sort by dictionary order (ascending)
            // i = v1.dictionary_index - v2.dictionary_index;
            let cmp = v1.dictionary_index.cmp(&v2.dictionary_index);
            if cmp != Ordering::Equal {
                return cmp;
            }

            let cmp = v2.score.cmp(&v1.score);
            if cmp != Ordering::Equal {
                return cmp;
            }

            // 9. Sort by headword term text
            let headwords1 = &v1.headwords;
            let headwords2 = &v2.headwords;
            let min_len = headwords1.len().min(headwords2.len());

            for j in 0..min_len {
                let term1 = &headwords1[j].term;
                let term2 = &headwords2[j].term;

                // Sort by term length (descending)
                // i = term2.length - term1.length;
                let cmp_len = term2.len().cmp(&term1.len());
                if cmp_len != Ordering::Equal {
                    return cmp_len;
                }

                // Sort by string comparison (ascending)
                // i = stringComparer.compare(term1, term2);
                let cmp_str = string_comparer.compare(term1, term2);
                if cmp_str != Ordering::Equal {
                    return cmp_str;
                }
            }
            v2.definitions.len().cmp(&v1.definitions.len())
        });
    }

    fn _get_shortest_text_processing_chain_length(
        &self,
        candidates: &[TextProcessorRuleChainCandidate],
    ) -> usize {
        candidates
            .iter()
            .map(|chain| chain.len())
            .min()
            .unwrap_or(0)
    }

    fn _get_shortest_inflection_chain_length(
        &self,
        candidates: &[InternalInflectionRuleChainCandidate],
    ) -> usize {
        candidates
            .iter()
            .map(|candidate| candidate.inflection_rules.len())
            .min()
            .unwrap_or(0)
    }

    // Corrected _update_sort_frequencies
    fn _update_sort_frequencies(
        dictionary_entries: &mut [InternalTermDictionaryEntry],
        dictionary: &str,
        ascending: bool,
    ) {
        let mut frequency_map: IndexMap<usize, i128> = IndexMap::new();
        // Use .iter_mut() to get a mutable reference to each entry
        for entry in dictionary_entries.iter_mut() {
            let mut frequency_min = i128::MAX;
            let mut frequency_max = i128::MIN;
            for freq in &entry.frequencies {
                if freq.dictionary != dictionary {
                    continue;
                }
                frequency_map.insert(freq.headword_index, freq.frequency);
                frequency_min = frequency_min.min(freq.frequency);
                frequency_max = frequency_max.max(freq.frequency);
            }

            // Assign directly to the entry's field
            entry.frequency_order = match frequency_min <= frequency_max {
                true => match ascending {
                    true => frequency_min,
                    false => -frequency_max,
                },
                false => match ascending {
                    true => i128::MAX,
                    false => 0,
                },
            };

            for def in entry.definitions.iter_mut() {
                frequency_min = i128::MAX;
                frequency_max = i128::MIN;
                for headword_index in &def.headword_indices {
                    if let Some(frequency) = frequency_map.get(headword_index) {
                        frequency_min = frequency_min.min(*frequency);
                        frequency_max = frequency_max.max(*frequency);
                    }
                }
                // Assign directly to the definition's field
                def.frequency_order = match frequency_min <= frequency_max {
                    true => match ascending {
                        true => frequency_min,
                        false => -frequency_max,
                    },
                    false => match ascending {
                        true => i128::MAX,
                        false => 0,
                    },
                };
            }
            frequency_map.clear();
        }
    }

    // fn _update_sort_frequencies(
    //     dictionary_entries: &mut [InternalTermDictionaryEntry],
    //     dictionary: &str,
    //     ascending: bool,
    // ) {
    //     let mut frequency_map: IndexMap<usize, i128> = IndexMap::new();
    //     for InternalTermDictionaryEntry {
    //         definitions,
    //         frequencies,
    //         mut frequency_order,
    //         ..
    //     } in dictionary_entries.iter_mut()
    //     {
    //         let mut frequency_min = i128::MAX;
    //         let mut frequency_max = i128::MIN;
    //         for TermFrequency {
    //             headword_index,
    //             frequency,
    //             dictionary: term_freq_dictionary,
    //             ..
    //         } in frequencies
    //         {
    //             if term_freq_dictionary != dictionary {
    //                 continue;
    //             }
    //             // JS does this for some reason,
    //             // checking type TermFrequency, it's garunteed to be a number..idk
    //             // if (typeof frequency !== 'number') { continue; }
    //
    //             frequency_map.insert(*headword_index, *frequency);
    //             frequency_min = frequency_min.min(*frequency);
    //             frequency_max = frequency_max.max(*frequency);
    //         }
    //         frequency_order = match frequency_min <= frequency_max {
    //             true => match ascending {
    //                 true => frequency_min,
    //                 false => -frequency_max,
    //             },
    //             false => match ascending {
    //                 true => i128::MAX,
    //                 false => 0,
    //             },
    //         };
    //         for TermDefinition {
    //             headword_indices,
    //             mut frequency_order,
    //             ..
    //         } in definitions.iter_mut()
    //         {
    //             frequency_min = i128::MAX;
    //             frequency_max = i128::MIN;
    //             for headword_index in headword_indices {
    //                 // in js the type for the map is literally <number, number>
    //                 // idk what this is supposed to mean
    //                 //if (typeof frequency !== 'number') { continue; }
    //                 if let Some(frequency) = frequency_map.get(headword_index) {
    //                     frequency_min = frequency_min.min(*frequency);
    //                     frequency_max = frequency_max.max(*frequency);
    //                 }
    //             }
    //             frequency_order = match frequency_min <= frequency_max {
    //                 true => match ascending {
    //                     true => frequency_min,
    //                     false => -frequency_max,
    //                 },
    //                 false => match ascending {
    //                     true => i128::MAX,
    //                     false => 0,
    //                 },
    //             };
    //         }
    //         frequency_map.clear();
    //     }
    // }

    fn _expand_tag_groups_and_group(&mut self, tag_expansion_targets: &mut [TagExpansionTarget]) {
        self._expand_tag_groups_mut(tag_expansion_targets);
        self._group_tags_mut(tag_expansion_targets);
    }

    fn _group_tags_mut(&self, tag_targets: &mut [TagExpansionTarget]) {
        let string_comparer: &CollatorBorrowed<'a> = &self.string_comparer;
        for tag_expansion_target in tag_targets.iter_mut() {
            // 1. Skip if tags length is 1 or less
            if tag_expansion_target.tags.len() <= 1 {
                continue;
            }
            Translator::_merge_similar_tags_mut(&mut tag_expansion_target.tags);
            tag_expansion_target
                .tags
                .sort_by(|v1, v2| match v1.order.cmp(&v2.order) {
                    Ordering::Equal => string_comparer.compare(&v1.name, &v2.name),
                    non_eq => non_eq,
                });
        }
    }

    /// Merges similar tags within the given vector.
    /// Tags are considered similar if they have the same `name` and `category`.
    /// When merged, the first tag encountered (tag1) is updated:
    /// - `order` becomes the minimum of the two.
    /// - `score` becomes the maximum of the two.
    /// - `dictionaries` are combined.
    /// - `content` is combined uniquely using `_add_unique_simple`.
    ///   The second tag (tag2) is removed from the vector.
    pub fn _merge_similar_tags_mut(tags: &mut Vec<DictionaryTag>) {
        if tags.is_empty() {
            return;
        }

        let mut i = 0;
        while i < tags.len() {
            // `tags[i]` is our `tag1`.
            // We need to ensure that any data we read from `tags[i]` for comparison
            // is read before potentially creating mutable borrows for modification
            // if `tags[i]` and `tags[j]` were the same (not an issue here as j > i).

            let mut j = i + 1;
            while j < tags.len() {
                // `tags[j]` is our `tag2`.
                // We need to compare tags[i] and tags[j].
                // To avoid issues with multiple mutable borrows if Rust were stricter,
                // or complex borrow splitting, we can check names and categories first.
                // In this specific loop structure (i and j distinct, tags[i] modified
                // based on tags[j] which is then removed), it's generally manageable.

                if tags[j].name == tags[i].name && tags[j].category == tags[i].category {
                    // Match found, merge tags[j] into tags[i].

                    // Store values from tags[j] before it's removed or its data moved.
                    let tag2_order = tags[j].order;
                    let tag2_score = tags[j].score;
                    // The .dictionaries and .content will be moved from the removed tag.

                    // Update tag1 (tags[i])
                    tags[i].order = std::cmp::min(tags[i].order, tag2_order);
                    tags[i].score = tags[i].score.max(tag2_score); // For f64, .max() is fine.

                    // Remove tag2 (tags[j]) and get its owned data
                    let removed_tag = tags.remove(j);

                    // Merge dictionaries and content from the removed_tag into tags[i]
                    tags[i].dictionaries.extend(removed_tag.dictionaries); // Moves elements
                    Translator::_add_unique_simple(&mut tags[i].content, &removed_tag.content);

                    // `j` should NOT be incremented here, because `tags.remove(j)`
                    // shifted the next element into the current `j`th position.
                    // The loop condition `j < tags.len()` will correctly re-evaluate.
                } else {
                    // No match, move to the next `tags[j]`
                    j += 1;
                }
            }
            i += 1; // Move to the next `tags[i]`
        }
    }

    fn _expand_tag_groups_mut(&mut self, tag_targets: &mut [TagExpansionTarget]) {
        // `all_items` was an artifact of the initial incorrect cloning strategy.
        // Given our new approach where `target_map` is the source of truth
        // and we directly modify `tag_targets` in the final loop, `all_items` is
        // no longer needed. Removing it simplifies the code and avoids confusion.
        // let mut all_items: Vec<TagTargetItem> = Vec::new();

        let mut target_map: IndexMap<String, IndexMap<String, TagTargetItem>> = IndexMap::new();

        // First pass: Populate target_map and also store references to the original `tags` vectors.
        // We need to store a reference to the `tags` Vec<String> from the original `TagExpansionTarget`
        // so we can push the final `DictionaryTag` into it.
        // To do this, TagTargetItem needs to store a way to reference that specific `tags` vector.
        // Since `tags` is part of `tag_targets` (a mutable slice), and we iterate over `tag_targets`
        // with a mutable reference, we can pass `tags` as a mutable reference into the `TagTargetItem`.
        // However, `TagTargetItem` cannot directly hold `&'a mut Vec<String>` because its lifetime
        // would be tied to the outer `for target in tag_targets` loop.

        // A better way, as in the JS, is to store the `TagTargetItem` in `target_map`
        // and then in the final loop, *re-associate* with the original `tags` vector.
        // The `TagTargetItem.targets` field is currently `Vec<Vec<DictionaryTag>>`.
        // This is where the JS `item.targets.push(tags);` is interesting. In JS, `tags` is a reference
        // to the `tagTargets[x].tags` array. So `item.targets` stores references to the original arrays.
        // In Rust, `item.targets.push(tags.to_vec());` is pushing a *clone* of the current `tags` vector.
        // This implies that `item.targets` in Rust holds *snapshots* of `tags` at the time of creation.
        // This is problematic if the intent is to mutate the *original* `tags` vector in `tag_targets`.

        // Let's go back to the JS behavior:
        // `item.targets.push(tags);` means `item.targets` is `Array<Array<string>>` where the inner arrays
        // are *references* to the `tags` arrays from `tagTargets`.
        // When the final loop does `for (const tags of targets)`, `tags` here is a reference to the
        // original `tagTargets[x].tags` array, which is then mutated by `tags.push(...)`.

        // To mimic this without `Rc<RefCell>`, `TagTargetItem` cannot directly hold the `targets`
        // `Vec<Vec<DictionaryTag>>`. Instead, the final loop must explicitly find the `tags` vector
        // in the original `tag_targets` and populate it.

        // Let's modify TagTargetItem.targets to store a unique identifier for the original `tags` Vec.
        // Or, more simply, remove `TagTargetItem.targets` field.
        // The final loop will need to iterate through `tag_targets` and then
        // look up the corresponding `TagTargetItem` in `target_map`.

        // Let's simplify `TagTargetItem` and remove `targets` field, as it complicates things for Rust.
        // We'll rely on the final loop reconstructing the relationship.
        // Assuming TagTargetItem is updated to remove the `targets` field:
        // pub struct TagTargetItem {
        //     pub query: String,
        //     pub dictionary: String,
        //     pub tag_name: String,
        //     pub cache: Option<TagCache>, // This field is actually not needed in TagTargetItem with our current approach
        //     pub database_tag: Option<DatabaseTag>,
        // }

        // First pass: Populate target_map
        for target in tag_targets.iter() {
            // Iterate immutably to just read for map population
            let TagExpansionTarget {
                tags: _,
                tag_groups,
            } = target;
            for group in tag_groups {
                let TagGroup {
                    dictionary,
                    tag_names,
                } = group;
                let dictionary_items = target_map.entry(dictionary.clone()).or_default();
                for tag_name in tag_names {
                    dictionary_items.entry(tag_name.clone()).or_insert_with(|| {
                        let query = Translator::_get_base_name(tag_name);
                        TagTargetItem {
                            query,
                            dictionary: dictionary.clone(),
                            tag_name: tag_name.clone(),
                            // Starts as None, will be populated
                            // targets field removed from TagTargetItem
                            database_tag: None,
                            // you forgot to initialize these:
                            targets: vec![],
                            cache: None,
                        }
                    });
                }
            }
        }

        // Second pass: Identify non-cached items and populate their database_tag
        let mut non_cached_items_refs: Vec<&mut TagTargetItem> = Vec::new();
        let tag_cache_ref = &mut self.tag_cache;

        for (dictionary_name, dictionary_items) in target_map.iter_mut() {
            let cache_for_dict = tag_cache_ref.entry(dictionary_name.clone()).or_default();

            for item in dictionary_items.values_mut() {
                let database_tag_from_cache = cache_for_dict.get(item.query.as_str());
                match database_tag_from_cache {
                    Some(database_tag) => {
                        // database_tag is already an &Option<DatabaseTag>
                        item.database_tag = database_tag.to_owned();
                    }
                    None => {
                        non_cached_items_refs.push(item);
                    }
                }
            }
        }

        if !non_cached_items_refs.is_empty() {
            let non_cached_queries: Vec<GenericQueryRequest> = non_cached_items_refs
                .iter()
                // &&mut TagTargetItem
                // dereference back to OG, then reborrow
                .map(|item_ref| (&**item_ref).into())
                .collect();

            let database_tags = self
            .db
            .find_tag_meta_bulk(&non_cached_queries)
            .unwrap_or_else(|e| {
                eprintln!(
                    "`find_tag_meta_bulk` threw an error in Translator::_expand_tag_groups\nreason: {e}"
                );
                vec![]
            });

            for (item_ref, database_tag_option) in non_cached_items_refs
                .into_iter()
                .zip(database_tags.into_iter())
            {
                // `database_tag_option` is already an `Option<DatabaseTag>`
                // from `find_tag_meta_bulk`.
                // Assign it directly.
                item_ref.database_tag = database_tag_option.clone();
                if let Some(cache) = tag_cache_ref.get_mut(&item_ref.dictionary) {
                    /// if the cache exists, you can directly use the Option<DatabaseTag>
                    /// as cache: &mut IndexMap<String, Option<DatabaseTag>> already.
                    cache.insert(item_ref.query.clone(), database_tag_option);
                }
            }
        }

        // Final pass: Iterate over the original `tag_targets` and populate their `tags` vector.
        // The `database_tag` values are now correctly updated in `target_map`.
        for target in tag_targets {
            // Iterate mutably over the original input
            let TagExpansionTarget { tags, tag_groups } = target;
            for group in tag_groups {
                let TagGroup {
                    dictionary,
                    tag_names,
                } = group;
                if let Some(dictionary_items) = target_map.get(dictionary) {
                    for tag_name in tag_names {
                        if let Some(item) = dictionary_items.get(tag_name) {
                            // `item.database_tag` is already an Option<DatabaseTag>.
                            // Pass it directly to `_create_dictionary_tag`.
                            tags.push(Translator::_create_dictionary_tag(
                                item.database_tag.clone(), // Clone the Option<DatabaseTag>
                                tag_name.clone(),
                                dictionary.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// this is called _create_tag() in js
    fn _create_dictionary_tag(
        database_tag: Option<DatabaseTag>,
        name: String,
        dictionary: String,
    ) -> DictionaryTag {
        let Some(database_tag) = database_tag else {
            return DictionaryTag::new_default(name, dictionary);
        };
        let DatabaseTag {
            category,
            order,
            notes,
            score,
            ..
        } = database_tag;
        let category = match !category.is_empty() {
            true => category,
            false => String::from("default"),
        };
        let content = match !notes.is_empty() {
            true => vec![notes],
            false => vec![],
        };
        DictionaryTag {
            name,
            category,
            order: order as usize,
            score: score as usize,
            content,
            dictionaries: vec![dictionary],
            redundant: false,
        }
    }

    fn _get_base_name(name: &str) -> String {
        let Some(pos) = name.find(':') else {
            return name.to_string();
        };
        name[..pos].to_string()
    }

    fn _add_term_meta(
        &self,
        dictionary_entries: &mut [InternalTermDictionaryEntry],
        enabled_dictionary_map: &TermEnabledDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
    ) {
        let mut headword_map: IndexMap<String, IndexMap<String, Vec<TermMetaHeadword>>> =
            IndexMap::new();
        let mut headword_map_keys: IndexSet<String> = IndexSet::new();
        // In JS, headwordReadingMaps is an array of the inner maps.
        // We will populate this after the first loop to correctly capture all data.
        let mut headword_reading_maps: Vec<IndexMap<String, Vec<TermMetaHeadword>>> = Vec::new();
        for (entry_idx, dictionary_entry) in dictionary_entries.iter().enumerate() {
            let current_pronunciations = &dictionary_entry.pronunciations;
            let current_frequencies = &dictionary_entry.frequencies;
            for (i, headword) in dictionary_entry.headwords.iter().enumerate() {
                let term = &headword.term;
                let reading = &headword.reading;
                let reading_map_for_term = headword_map.entry(term.clone()).or_insert_with(|| {
                    headword_map_keys.insert(term.clone());
                    IndexMap::new()
                });
                let targets_for_reading = reading_map_for_term.entry(reading.clone()).or_default();
                targets_for_reading.push(TermMetaHeadword {
                    original_index: entry_idx,
                    headword_index: i,
                    pronunciations: current_pronunciations.clone(),
                    frequencies: current_frequencies.clone(),
                });
            }
        }
        // Construct headword_reading_maps
        //match JS structure after headword_map is fully populated.
        // This ensures that headword_reading_maps[i] corresponds to headword_map_keys[i].
        for key in &headword_map_keys {
            if let Some(inner_map) = headword_map.get(key) {
                headword_reading_maps.push(inner_map.clone());
            } else {
                // Should not happen if logic is correct
                headword_reading_maps.push(IndexMap::new());
            }
        }
        if headword_map_keys.is_empty() {
            return;
        }
        let metas_result = self
            .db
            .find_term_meta_bulk(&headword_map_keys, enabled_dictionary_map);
        let metas = match metas_result {
            Ok(m) => m,
            Err(e) => {
                // eprintln is a Rust convention for printing to stderr
                eprintln!("Failed to find term meta bulk: {e}");
                return;
            }
        };
        //dbg!("metas result len: {}", metas.len());
        for meta_entry in metas {
            let DatabaseTermMeta {
                // This is the index for headword_map_keys and headword_reading_maps
                index,
                mode,
                data,
                dictionary,
                ..
            } = meta_entry;

            let dictionary_index_val = Translator::_get_dictionary_order(
                &dictionary,
                &EnabledDictionaryMapType::Term(enabled_dictionary_map),
            );

            let dictionary_alias_val = Translator::_get_dictionary_alias(
                dictionary.clone(),
                &EnabledDictionaryMapType::Term(enabled_dictionary_map),
            );

            /// --- OLD CODE ---
            // JS: const map2 = headwordReadingMaps[index];
            if index >= headword_reading_maps.len() {
                eprintln!(
                    "Meta index {} out of bounds for headword_reading_maps (len {})",
                    index,
                    headword_reading_maps.len()
                );
                continue;
            }

            let map2 = &headword_reading_maps[index];
            /// --- END OLD CODE ---
            for (reading_key_str, targets_vec) in map2.iter() {
                match &data {
                    MetaDataMatchType::Frequency(ref freq_match_type) => {
                        if mode != TermMetaModeType::Freq {
                            continue;
                        }
                        // JS: hasReading
                        let mut has_reading_filter = false;
                        let frequency_data_value = match freq_match_type {
                            // JS: frequency
                            TermMetaFreqDataMatchType::WithReading(data_with_reading) => {
                                if &data_with_reading.reading != reading_key_str {
                                    continue;
                                }
                                has_reading_filter = true;
                                data_with_reading.frequency.clone()
                            }
                            TermMetaFreqDataMatchType::Generic(generic_data) => {
                                generic_data.clone()
                            }
                        };
                        let freq_info =
                            Translator::_get_frequency_info(frequency_data_value.clone());
                        let frequency_value_to_store = freq_info.frequency;
                        let display_value_str = freq_info.display_value;
                        let display_value_parsed_bool = freq_info.display_value_parsed;
                        for target_meta_headword in targets_vec.iter() {
                            // JS: targets loop
                            let dict_entry_to_update =
                                &mut dictionary_entries[target_meta_headword.original_index];
                            let new_term_freq = Translator::_create_term_frequency_(
                                dict_entry_to_update.frequencies.len(),
                                target_meta_headword.headword_index,
                                dictionary.clone(),
                                dictionary_index_val,
                                dictionary_alias_val.clone(),
                                has_reading_filter,
                                frequency_value_to_store,
                                display_value_str.clone(),
                                display_value_parsed_bool,
                            );
                            dict_entry_to_update.frequencies.push(new_term_freq);
                        }
                    }
                    MetaDataMatchType::Pitch(ref pitch_meta_data) => {
                        if mode != TermMetaModeType::Pitch {
                            continue;
                        }
                        if &pitch_meta_data.reading != reading_key_str {
                            continue;
                        }
                        // JS: pitches (array of PitchAccent)
                        // however _create_term_pronunciation takes the rust enum,
                        // js had multiple functions for this, so we combined it into one
                        let mut pitches_to_add: Vec<Pronunciation> = Vec::new();
                        // JS: data.pitches loop
                        for pitch_item_data in &pitch_meta_data.pitches {
                            // JS: tags2
                            let mut resolved_tags: Vec<DictionaryTag> = Vec::new();
                            if let Some(tags_from_data) = &pitch_item_data.tags {
                                // JS: if (Array.isArray(tags))
                                tag_aggregator.add_tags(
                                    &resolved_tags,
                                    &dictionary,
                                    tags_from_data,
                                );
                            }
                            let nasal_positions_vec = Translator::_vec_num_or_num_to_vec_u8(
                                pitch_item_data.nasal.as_ref(),
                            ); // JS: nasalPositions
                            let devoice_positions_vec = Translator::_vec_num_or_num_to_vec_u8(
                                pitch_item_data.devoice.as_ref(),
                            ); // JS: devoicePositions
                            pitches_to_add.push(Pronunciation::PitchAccent(PitchAccent {
                                term: TermPronunciationMatchType::PitchAccent,
                                position: pitch_item_data.position,
                                nasal_positions: nasal_positions_vec,
                                devoice_positions: devoice_positions_vec,
                                tags: resolved_tags,
                            }));
                        }
                        if pitches_to_add.is_empty() {
                            // Not explicitly in JS, but good practice
                            continue;
                        }
                        for target_meta_headword in targets_vec.iter() {
                            // JS: targets loop
                            let dict_entry_to_update =
                                &mut dictionary_entries[target_meta_headword.original_index];
                            let new_term_pron = Translator::_create_term_pronunciation(
                                dict_entry_to_update.pronunciations.len(),
                                target_meta_headword.headword_index,
                                dictionary.clone(),
                                dictionary_index_val,
                                dictionary_alias_val.clone(),
                                pitches_to_add.clone(),
                            );
                            dict_entry_to_update.pronunciations.push(new_term_pron);
                        }
                    }
                    MetaDataMatchType::Phonetic(ref phonetic_meta_data) => {
                        if mode != TermMetaModeType::Ipa {
                            continue;
                        }
                        if &phonetic_meta_data.reading != reading_key_str {
                            continue;
                        }
                        // JS: phoneticTranscriptions
                        let mut phonetic_transcriptions_to_add: Vec<Pronunciation> = Vec::new();
                        for transcription_item in &phonetic_meta_data.transcriptions {
                            // JS: data.transcriptions loop
                            let mut resolved_ipa_tags: Vec<DictionaryTag> = Vec::new(); // JS: tags2
                            let tag_names_for_aggregator: Vec<String> = transcription_item
                                .tags
                                .iter()
                                .map(|tag| tag.name.clone())
                                .collect();
                            if !tag_names_for_aggregator.is_empty() {
                                // JS: if (Array.isArray(tags))
                                tag_aggregator.add_tags(
                                    &resolved_ipa_tags,
                                    &dictionary,
                                    &tag_names_for_aggregator,
                                );
                            }
                            phonetic_transcriptions_to_add.push(
                                Pronunciation::PhoneticTranscription(PhoneticTranscription {
                                    match_type: TermPronunciationMatchType::PhoneticTranscription,
                                    ipa: transcription_item.ipa.clone(),
                                    tags: resolved_ipa_tags,
                                }),
                            );
                            // The JS code pushes the transcription_item itself again,
                            // which seems redundant or a mistake
                            // as it would use the original tags, not the resolved_ipa_tags.
                            // For a 1-to-1, if the JS truly does this, it should be replicated.
                            // However, it's more logical to use
                            // resolved_ipa_tags for all additions if tags are processed.
                            // This implies only one push per item with processed tags.
                        }
                        if phonetic_transcriptions_to_add.is_empty() {
                            // Not explicitly in JS, but good practice
                            continue;
                        }
                        for target_meta_headword in targets_vec.iter() {
                            // JS: targets loop
                            let dict_entry_to_update =
                                &mut dictionary_entries[target_meta_headword.original_index];
                            let new_term_pron = Translator::_create_term_pronunciation(
                                dict_entry_to_update.pronunciations.len(),
                                target_meta_headword.headword_index,
                                dictionary.clone(),
                                dictionary_index_val,
                                dictionary_alias_val.clone(),
                                phonetic_transcriptions_to_add.clone(),
                            );
                            dict_entry_to_update.pronunciations.push(new_term_pron);
                        }
                    }
                }
            }
        }
    }

    fn _vec_num_or_num_to_vec_u8(opt_vnon: Option<&VecNumOrNum>) -> Vec<u8> {
        match opt_vnon {
            Some(VecNumOrNum::Vec(vec)) => vec.clone(),
            Some(VecNumOrNum::Num(num)) => vec![*num],
            None => Vec::new(),
        }
    }
    fn _create_term_frequency_(
        index: usize,
        headword_index: usize,
        dictionary: String,
        dictionary_index: usize,
        dictionary_alias: String,
        has_reading: bool,
        frequency: i128,
        display_value: Option<String>,
        display_value_parsed: bool,
    ) -> TermFrequency {
        TermFrequency {
            index,
            headword_index,
            dictionary,
            dictionary_index,
            dictionary_alias,
            has_reading,
            frequency,
            display_value,
            display_value_parsed,
        }
    }
    fn _create_term_pronunciation(
        index: usize,
        headword_index: usize,
        dictionary: String,
        dictionary_index: usize,
        dictionary_alias: String,
        pronunciations: Vec<Pronunciation>,
    ) -> TermPronunciation {
        TermPronunciation {
            index,
            headword_index,
            dictionary,
            dictionary_index,
            dictionary_alias,
            pronunciations,
        }
    }
    fn _get_frequency_info(frequency_data: GenericFreqData) -> FrequencyInfo {
        match frequency_data {
            GenericFreqData::Object(obj) => FrequencyInfo {
                frequency: obj.value,
                display_value: obj.display_value,
                display_value_parsed: false,
            },
            GenericFreqData::Integer(num) => FrequencyInfo {
                frequency: num,
                display_value: None,
                display_value_parsed: false,
            },
            GenericFreqData::String(s_val) => {
                let numeric_value = Translator::_convert_string_to_number(&s_val);
                FrequencyInfo {
                    frequency: numeric_value,
                    display_value: Some(s_val),
                    display_value_parsed: true,
                }
            }
        }
    }
    fn _convert_string_to_number(s: &str) -> i128 {
        s.parse::<i128>().unwrap_or(-1)
    }
    fn find_terms_internal(
        &self,
        text: &mut String,
        opts: &FindTermsOptions,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> FindInternalTermsResult {
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
            return FindInternalTermsResult::default();
        }
        let deinflections = self._get_deinflections(text, opts);
        Translator::_get_dictionary_entries(
            &deinflections,
            enabled_dictionary_map,
            tag_aggregator,
            primary_reading,
        )
    }

    // Corrected logic for _remove_excluded_definitions
    fn _remove_excluded_definitions(
        dictionary_entries: &mut Vec<InternalTermDictionaryEntry>,
        exclude_dictionary_definitions: &IndexSet<String>,
    ) {
        dictionary_entries.retain_mut(|entry| {
            // Filter definitions and check if any were removed
            let original_def_len = entry.definitions.len();
            entry
                .definitions
                .retain(|def| !exclude_dictionary_definitions.contains(&def.dictionary));
            let definitions_were_removed = entry.definitions.len() != original_def_len;

            // Filter other fields
            entry
                .pronunciations
                .retain(|p| !exclude_dictionary_definitions.contains(&p.dictionary));
            entry
                .frequencies
                .retain(|f| !exclude_dictionary_definitions.contains(&f.dictionary));

            // Filter tags within definitions and headwords
            Translator::_remove_tag_groups_with_dictionary_mut(
                &mut entry.definitions,
                exclude_dictionary_definitions,
            );
            Translator::_remove_tag_groups_with_dictionary_mut(
                &mut entry.headwords,
                exclude_dictionary_definitions,
            );

            // FINAL DECISION:
            // If all definitions were removed, discard the entire entry.
            if entry.definitions.is_empty() {
                return false; // Discard
            }

            // If definitions were removed (but not all), clean up unused headwords.
            if definitions_were_removed {
                Translator::_remove_unused_headwords(entry);
            }

            true // Keep the entry
        });
    }

    // fn _remove_excluded_definitions(
    //     dictionary_entries: &mut Vec<InternalTermDictionaryEntry>,
    //     exclude_dictionary_definitions: &IndexSet<String>,
    // ) {
    //     dictionary_entries.retain_mut(|dictionary_entry| {
    //         // ---- definitions ----
    //         let definitions: Vec<TermType> = iter_type_to_iter_variant!(
    //             dictionary_entry.definitions.clone(),
    //             TermType::Definition
    //         )
    //         .collect();
    //         let filtered_definitions = Translator::_remove_array_items_with_dictionary(
    //             &definitions,
    //             exclude_dictionary_definitions,
    //         );
    //         let definitions_changed =
    //             dictionary_entry.definitions.len() == filtered_definitions.len();
    //         if (definitions_changed) {
    //             dictionary_entry.definitions =
    //                 iter_variant_to_iter_type!(filtered_definitions, TermType::Definition);
    //         }
    //         // ---- frequencies ----
    //         let frequencies: Vec<TermType> = iter_type_to_iter_variant!(
    //             dictionary_entry.frequencies.clone(),
    //             TermType::Frequency
    //         )
    //         .collect();
    //         let filtered_frequencies = Translator::_remove_array_items_with_dictionary(
    //             &frequencies,
    //             exclude_dictionary_definitions,
    //         );
    //         dictionary_entry.frequencies =
    //             iter_variant_to_iter_type!(filtered_definitions, TermType::Frequency);
    //         // ---- pronunciation ----
    //         let pronunciations: Vec<TermType> = iter_type_to_iter_variant!(
    //             dictionary_entry.pronunciations.clone(),
    //             TermType::Pronunciation
    //         )
    //         .collect();
    //         let filtered_pronunciations = Translator::_remove_array_items_with_dictionary(
    //             &pronunciations,
    //             exclude_dictionary_definitions,
    //         );
    //         dictionary_entry.pronunciations =
    //             iter_variant_to_iter_type!(filtered_definitions, TermType::Pronunciation);
    //         // ---- tags ----
    //         Translator::_remove_tag_groups_with_dictionary_mut(
    //             &mut dictionary_entry.definitions,
    //             exclude_dictionary_definitions,
    //         );
    //         Translator::_remove_tag_groups_with_dictionary_mut(
    //             &mut dictionary_entry.headwords,
    //             exclude_dictionary_definitions,
    //         );
    //         if !definitions_changed {
    //             return true;
    //         }
    //         // definitions_changed is true
    //         // check the current state of `dictionary_entry.definitions` (after all filtering).
    //         if dictionary_entry.definitions.is_empty() {
    //             // If all definitions were removed (by the first filter or tag filter),
    //             // then remove this entire dictionary entry.
    //             false // `retain_mut` will remove this item.
    //         } else {
    //             // Definitions were changed, but some remain.
    //             // Call `_remove_unused_headwords` to clean up.
    //             Translator::_remove_unused_headwords(dictionary_entry);
    //             // keep
    //             true
    //         }
    //     });
    // }

    fn _remove_unused_headwords(dictionary_entry: &mut InternalTermDictionaryEntry) {
        let mut remove_headword_indices: IndexSet<usize> = IndexSet::new();
        // Initially, mark all headword indices for removal.
        for i in 0..dictionary_entry.headwords.len() {
            remove_headword_indices.insert(i);
        }
        // Iterate through definitions to find used headword indices.
        // For any headword index found in a definition,
        // remove it from the `remove_headword_indices` set.
        for definition in &dictionary_entry.definitions {
            for &headword_index in &definition.headword_indices {
                remove_headword_indices.shift_remove_full(&headword_index);
            }
        }
        // If no headwords are marked for removal, there's nothing to do.
        if remove_headword_indices.is_empty() {
            return;
        }
        // Create a map to store the remapping of old indices to new indices.
        let mut index_remap: IndexMap<usize, usize> = IndexMap::new();
        let mut new_headwords: Vec<TermHeadword> = Vec::new();
        let mut current_new_index = 0;
        // Iterate through the original headwords along with their original indices.
        for old_index in 0..dictionary_entry.headwords.len() {
            // If the current old_index is NOT in the set of indices to remove,
            // it means this headword should be kept.
            if !remove_headword_indices.contains(&old_index) {
                // Add the headword to our new list of headwords.
                // We clone here because we're taking ownership of the string for the new Vec.
                new_headwords.push(dictionary_entry.headwords[old_index].clone());
                // Map the old_index to its new position (current_new_index).
                index_remap.insert(old_index, current_new_index);
                // Increment the new_index for the next kept headword.
                current_new_index += 1;
            }
        }
        // Replace the old headwords Vec with the new one.
        dictionary_entry.headwords = new_headwords;
        let mut definitions: Vec<TermType> =
            iter_type_to_iter_variant!(dictionary_entry.definitions.clone(), TermType::Definition)
                .collect();
        let mut frequencies: Vec<TermType> =
            iter_type_to_iter_variant!(dictionary_entry.frequencies.clone(), TermType::Frequency)
                .collect();
        let mut pronunciations: Vec<TermType> = iter_type_to_iter_variant!(
            dictionary_entry.pronunciations.clone(),
            TermType::Pronunciation
        )
        .collect();
        let mut updates = [definitions, frequencies, pronunciations];
        for mut update in &mut updates {
            Translator::_update_term_headword_indices_mut(update, &index_remap);
        }
        dictionary_entry.definitions = iter_variant_to_iter_type!(updates[0], TermType::Definition);
        dictionary_entry.frequencies = iter_variant_to_iter_type!(updates[1], TermType::Frequency);
        dictionary_entry.pronunciations =
            iter_variant_to_iter_type!(updates[2], TermType::Pronunciation);
    }
    /// Updates headword indices for a collection of
    /// `TermType` items based on an index remap.
    ///
    /// This function combines the logic of updating
    /// `headword_indices` within `TermDefinition`
    /// and updating `headword_index` for `TermPronunciation` and `TermFrequency`,
    /// removing items of the latter two types if their index cannot be remapped.
    ///
    /// # Arguments
    ///
    /// * `terms`: A mutable vector of `TermType` items to process. Items may be modified or removed in place.
    /// * `index_remap`: A map where the key is the old headword index and the value is the new headword index.
    fn _update_term_headword_indices_mut(
        terms: &mut Vec<TermType>,
        index_remap: &IndexMap<usize, usize>,
    ) {
        terms.retain_mut(|term_type| {
            match term_type {
                TermType::Definition(def) => {
                    // Update def.headword_indices in place.
                    // This creates a new Vec and replaces the old one, which is a clean way
                    // to handle removals and updates simultaneously for the inner Vec.
                    let mut updated_indices = Vec::with_capacity(def.headword_indices.len());
                    for &old_idx in &def.headword_indices {
                        if let Some(&new_idx) = index_remap.get(&old_idx) {
                            updated_indices.push(new_idx);
                        }
                    }
                    def.headword_indices = updated_indices;
                    // TermDefinition items are never removed from the outer `terms` Vec by this logic,
                    // only their internal `headword_indices` are modified.
                    true
                }
                TermType::Pronunciation(prono) => {
                    if let Some(&new_idx) = index_remap.get(&prono.headword_index) {
                        prono.headword_index = new_idx;
                        true // Keep the item as its index was successfully updated.
                    } else {
                        false // Remove the item as its headword_index could not be remapped.
                    }
                }
                TermType::Frequency(freq) => {
                    if let Some(&new_idx) = index_remap.get(&freq.headword_index) {
                        freq.headword_index = new_idx;
                        true // Keep the item as its index was successfully updated.
                    } else {
                        false // Remove the item as its headword_index could not be remapped.
                    }
                }
            }
        });
    }
    fn _remove_tag_groups_with_dictionary_mut(
        array: &mut [impl HasTags],
        exclude_dictionary_definitions: &IndexSet<String>,
    ) {
        for item in array {
            Translator::_remove_tag_items_with_dictionary(
                item._get_definition_tags_mut(),
                exclude_dictionary_definitions,
            );
        }
    }
    /// In JS: `_removeArrayItemsWithDictionary2`
    fn _remove_tag_items_with_dictionary(
        tags: &mut Vec<DictionaryTag>,
        exclude_dictionary_definitions: &IndexSet<String>,
    ) -> bool {
        let original_len = tags.len();
        tags.retain(|tag| {
            !Translator::_has_any(exclude_dictionary_definitions, tag.dictionaries.iter())
        });
        original_len != tags.len()
    }
    fn _has_any<'b, T: PartialEq + Eq + Hash + 'b, U: std::iter::Iterator<Item = &'b T>>(
        set: &IndexSet<T>,
        mut values: U,
    ) -> bool {
        values.any(|v| set.contains(v))
    }
    fn _remove_array_items_with_dictionary(
        terms: &[TermType],
        exclude_dictionary_definitions: &IndexSet<String>,
    ) -> Vec<TermType> {
        let mut changed = false;
        terms
            .iter()
            .filter_map(|term| {
                if !exclude_dictionary_definitions.contains(term.dictionary_and_alias().0) {
                    return Some(term.clone());
                }
                None
            })
            .collect()
    }
    fn _get_related_dictionary_entries(
        &self,
        dictionary_entries_input: &[InternalTermDictionaryEntry],
        options: &FindTermsOptions,
        tag_aggregator: &mut TranslatorTagAggregator,
    ) -> Vec<InternalTermDictionaryEntry> {
        let FindTermsOptions {
            main_dictionary,
            enabled_dictionary_map,
            language,
            primary_reading,
            ..
        } = options;
        /// in js this is `type SequenceQuery` but I simplified this to use enums
        /// instead of objects, to be generic.
        let mut sequence_list: Vec<GenericQueryRequest> = Vec::new();
        let mut grouped_dictionary_entries: Vec<DictionaryEntryGroup> = Vec::new();
        // Maps sequence (i128) to the index in `grouped_dictionary_entries` vector
        let mut grouped_dictionary_entries_map: IndexMap<i128, usize> = IndexMap::new();
        let mut ungrouped_dictionary_entries_map: IndexMap<String, InternalTermDictionaryEntry> =
            IndexMap::new();
        for dictionary_entry in dictionary_entries_input {
            // dictionary_entry is &InternalTermDictionaryEntry
            if let Some(first_definition) = dictionary_entry.definitions.first() {
                let id = &first_definition.id;
                let definition_dictionary = &first_definition.dictionary;
                if let Some(&sequence) = first_definition.sequences.first() {
                    // sequence is i128
                    if *definition_dictionary == *main_dictionary && sequence >= 0 {
                        let group_index_opt =
                            grouped_dictionary_entries_map.get(&sequence).copied();
                        let current_group_index = match group_index_opt {
                            Some(index) => index,
                            None => {
                                let new_group = DictionaryEntryGroup {
                                    ids: IndexSet::new(),
                                    dictionary_entries: Vec::new(),
                                };
                                grouped_dictionary_entries.push(new_group);
                                let new_index = grouped_dictionary_entries.len() - 1;
                                sequence_list.push(GenericQueryRequest {
                                    query_type: QueryType::Sequence(sequence),
                                    dictionary: definition_dictionary.clone(),
                                });
                                grouped_dictionary_entries_map.insert(sequence, new_index);
                                new_index
                            }
                        };
                        let group = &mut grouped_dictionary_entries[current_group_index];
                        group.dictionary_entries.push(dictionary_entry.clone());
                        group.ids.insert(id.clone());
                    } else {
                        ungrouped_dictionary_entries_map
                            .insert(id.clone(), dictionary_entry.clone());
                    }
                } else {
                    // If first_definition has no sequences,
                    // JS would error on destructuring [sequence]
                    // To align with entries that don't meet the criteria, add to ungrouped.
                    ungrouped_dictionary_entries_map.insert(id.clone(), dictionary_entry.clone());
                }
            } else {
                // If dictionary_entry has no definitions,
                // JS would error on destructuring definitions: [{...}]
                // Current Rust `if let` skips such entries. If it needs to be added to ungrouped,
                // an ID would be needed, but the current logic relies on first_definition.id.
            }
        }
        if !sequence_list.is_empty() {
            let secondary_search_dictionary_map =
                Translator::_get_secondary_search_dictionary_map(enabled_dictionary_map);
            self._add_related_dictionary_entries(
                &mut grouped_dictionary_entries,
                &mut ungrouped_dictionary_entries_map,
                sequence_list,
                enabled_dictionary_map,
                tag_aggregator,
                primary_reading,
            );
            for group in &mut grouped_dictionary_entries {
                Translator::_sort_term_dictionary_entries_by_id(&mut group.dictionary_entries);
            }
            if !ungrouped_dictionary_entries_map.is_empty()
                || !secondary_search_dictionary_map.is_empty()
            {
                self._add_secondary_related_dictionary_entries(
                    language,
                    &mut grouped_dictionary_entries,
                    &mut ungrouped_dictionary_entries_map,
                    enabled_dictionary_map,
                    &secondary_search_dictionary_map,
                    tag_aggregator,
                    primary_reading,
                )
            }
        }
        let mut new_dictionary_entries: Vec<InternalTermDictionaryEntry> = Vec::new();
        for group in &grouped_dictionary_entries {
            new_dictionary_entries.push(self._create_grouped_dictionary_entry(
                language,
                &group.dictionary_entries,
                true, // check_duplicate_definitions parameter in _create_grouped_dictionary_entry
                tag_aggregator,
                primary_reading,
            ));
        }
        let ungrouped_values_collected: Vec<InternalTermDictionaryEntry> =
            ungrouped_dictionary_entries_map.values().cloned().collect();
        new_dictionary_entries.extend(self._group_dictionary_entries_by_headword(
            language,
            &ungrouped_values_collected,
            tag_aggregator,
            primary_reading,
        ));
        new_dictionary_entries
    }
    fn _add_secondary_related_dictionary_entries(
        &self,
        language: &str,
        grouped_dictionary_entries: &mut [DictionaryEntryGroup],
        ungrouped_dictionary_entries_map: &mut IndexMap<String, InternalTermDictionaryEntry>,
        enabled_dictionary_map: &TermEnabledDictionaryMap,
        secondary_search_dictionary_map: &TermEnabledDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) {
        // Prepare grouping info
        // In JavaScript, termList and targetList are built up.
        // termList stores {term, reading} objects.
        // targetList stores references to objects in targetMap.
        // targetMap maps a key (derived from term and normalizedReading) to an object { groups: [] }.
        // The 'groups' array in targetMap's values will store references to the original groups
        // from grouped_dictionary_entries that share the same term/reading.
        // Rust equivalent:
        // term_list will store TermExactQueryRequest structs.
        // target_list_indices will store indices into `grouped_dictionary_entries`
        // that correspond to the term/reading at the same index in `term_list`.
        // target_map_for_grouping maps the string key (term + normalized_reading) to a list of indices
        // of groups in `grouped_dictionary_entries` that share this term/reading.
        let mut term_list: Vec<TermExactQueryRequest> = Vec::new();
        // In the JS, targetList stores references to the objects { groups: [] } which are also the values in targetMap.
        // In Rust, we need a way to associate a term/reading (from term_list) back to the
        // original groups in `grouped_dictionary_entries` that it came from,
        // and also to any *new* groups that might be formed or related.
        // The JS `target.groups.push(group)` effectively links a term/reading to one or more original groups.
        //
        // Let's rethink `target_list` and `target_map` for Rust.
        // `target_map_for_original_groups`: Maps a key (term+normalized_reading) to a list of indices
        // of groups in `grouped_dictionary_entries` that contain this term/reading.
        // This helps identify which original groups are associated with a unique term/reading.
        let mut target_map_for_original_groups: IndexMap<String, Vec<usize>> = IndexMap::new();
        // `term_list_to_original_group_indices`: Parallel to `term_list`. For each term in `term_list`,
        // this stores the list of indices from `grouped_dictionary_entries` that this term/reading corresponds to.
        let mut term_list_to_original_group_indices: Vec<Vec<usize>> = Vec::new();
        let reading_normalizer = self.reading_normalizers.get(language);
        // Iterate over the original grouped_dictionary_entries to populate
        // term_list and target_map_for_original_groups.
        for (group_idx, group) in grouped_dictionary_entries.iter().enumerate() {
            for dictionary_entry in &group.dictionary_entries {
                // Ensure headwords exist.
                let headword = dictionary_entry.headwords.first().unwrap_or_else(|| {
                    panic!(
                        "DictionaryEntry is missing headwords in _add_secondary_related_dictionary_entries for group processing, group_idx: {group_idx}"
                    )
                });
                let term = &headword.term;
                let reading = &headword.reading;
                let normalized_reading =
                    Translator::_get_or_default_normalized_reading(reading_normalizer, reading);
                let key =
                    Translator::_create_map_key(&(term.as_str(), normalized_reading.as_str()));
                // Update target_map_for_original_groups
                let groups_for_this_key = target_map_for_original_groups
                    .entry(key.clone())
                    .or_default();
                if !groups_for_this_key.contains(&group_idx) {
                    groups_for_this_key.push(group_idx);
                }
                // If this key is new for term_list, add it.
                // We need to ensure term_list only contains unique term/reading pairs.
                // A simple way is to check if the key is already a key in a map that tracks term_list entries.
                // Or, more directly, check if we've already added this term/reading to term_list.
                // The JS logic uses `targetMap.get(key)` and if undefined, adds to termList.
                // We can find the index of the key in `target_map_for_original_groups.keys()`
                // or maintain a separate set for quick lookups for `term_list`.
                if !term_list
                    .iter()
                    .any(|tr| tr.term == *term && tr.reading == *reading)
                {
                    term_list.push(TermExactQueryRequest {
                        term: term.clone(),
                        reading: reading.clone(),
                    });
                    // For the newly added term_list entry, associate it with the current group_idx.
                    // Since term_list_to_original_group_indices is parallel to term_list,
                    // we add a new vector for this term.
                    // This assumes that if a term/reading appears in multiple original groups,
                    // `target_map_for_original_groups` will correctly list all such group indices for that key.
                    // And `term_list_to_original_group_indices` will store these lists of group indices,
                    // corresponding to each unique term/reading in `term_list`.
                    // When a new unique term/reading is added to term_list,
                    // we fetch all group indices associated with its key from target_map_for_original_groups.
                    // This seems slightly off from the direct JS logic where targetList.push(target) happens
                    // when a new key is encountered. 'target' in JS is { groups: [] }, and this 'target'
                    // is then populated with original groups.
                    // Let's align more closely:
                    // `term_list_associated_group_indices`: Parallel to `term_list`.
                    // Each element is a Vec<usize> of indices from `grouped_dictionary_entries`.
                    // When a new term/reading is added to `term_list`, we initialize its entry
                    // in `term_list_associated_group_indices` with the current `group_idx`.
                    // If the term/reading was already in `term_list`, we find its index
                    // and add the current `group_idx` to the corresponding list in `term_list_associated_group_indices`.
                    // Let's refine:
                    // `term_list`: Stores unique TermExactQueryRequest.
                    // `term_to_group_indices_map`: Maps a key (term+normalized_reading) to Vec<usize> (indices in `grouped_dictionary_entries`).
                    // This map helps collect all original group associations for each unique term/reading.
                    // Populate term_list and term_to_group_indices_map
                    // The outer loop is already iterating `grouped_dictionary_entries`
                }
            }
        }
        // Rebuild term_list and a parallel list of associated original group indices
        // from target_map_for_original_groups to ensure term_list is unique.
        term_list.clear(); // Start fresh for unique entries
        let mut term_list_associated_original_group_indices: Vec<Vec<usize>> = Vec::new();
        for (key, original_group_indices) in target_map_for_original_groups.iter() {
            // We need to parse the key back to term and reading to populate term_list.
            // This is inefficient. It's better to populate term_list directly when a new key is first seen.
            // Let's retry the logic for populating term_list and its associated group indices:
            // `term_list`: Vec<TermExactQueryRequest> - unique term/reading pairs for DB query.
            // `target_list_data`: Vec<Vec<usize>> - parallel to `term_list`. Each Vec<usize> stores
            // indices of groups in `grouped_dictionary_entries` that correspond to the term/reading
            // at the same index in `term_list`.
            // `key_to_term_list_idx_map`: IndexMap<String, usize> - maps a term/reading key to its index in `term_list`.
        }
        // Reset and rebuild term_list and its parallel association list
        term_list.clear();
        let mut target_list_data: Vec<Vec<usize>> = Vec::new(); // Stores indices of groups from `grouped_dictionary_entries`
        let mut key_to_term_list_idx_map: IndexMap<String, usize> = IndexMap::new();
        for (group_idx, group) in grouped_dictionary_entries.iter().enumerate() {
            for dictionary_entry in &group.dictionary_entries {
                let headword = dictionary_entry.headwords.first().unwrap_or_else(|| {
                    panic!(
                        "DictionaryEntry is missing headwords (loop 2) for group_idx: {group_idx}"
                    )
                });
                let term = &headword.term;
                let reading = &headword.reading;
                let normalized_reading =
                    Translator::_get_or_default_normalized_reading(reading_normalizer, reading);
                let key =
                    Translator::_create_map_key(&(term.as_str(), normalized_reading.as_str()));
                if let Some(&existing_term_idx) = key_to_term_list_idx_map.get(&key) {
                    // Term/reading already in term_list. Add current group_idx if not already present.
                    if !target_list_data[existing_term_idx].contains(&group_idx) {
                        target_list_data[existing_term_idx].push(group_idx);
                    }
                } else {
                    // New term/reading. Add to term_list and map.
                    let new_term_idx = term_list.len();
                    term_list.push(TermExactQueryRequest {
                        term: term.clone(),
                        reading: reading.clone(),
                    });
                    key_to_term_list_idx_map.insert(key.clone(), new_term_idx);
                    target_list_data.push(vec![group_idx]);
                }
            }
        }
        // Group unsequenced dictionary entries with sequenced entries that have a matching [term, reading].
        let mut ids_to_remove_from_ungrouped: IndexSet<String> = IndexSet::new();
        for (id, dictionary_entry) in ungrouped_dictionary_entries_map.iter() {
            let headword = dictionary_entry.headwords.first().unwrap_or_else(|| {
                panic!(
                    "Ungrouped DictionaryEntry ID '{id}' is missing headwords in _add_secondary_related_dictionary_entries"
                )
            });
            let term = &headword.term;
            let reading = &headword.reading;
            let normalized_reading =
                Translator::_get_or_default_normalized_reading(reading_normalizer, reading);
            let key = Translator::_create_map_key(&(term.as_str(), normalized_reading.as_str()));
            // Check if this term/reading key exists in our `key_to_term_list_idx_map`,
            // which means it's associated with one or more original groups.
            if let Some(&term_idx_in_list) = key_to_term_list_idx_map.get(&key) {
                // This ungrouped entry matches a term/reading found in the original groups.
                // Add this dictionary_entry to all original groups associated with this term/reading.
                let original_group_indices_for_this_term = &target_list_data[term_idx_in_list];
                let mut entry_processed_and_moved = false;
                for &original_group_idx in original_group_indices_for_this_term {
                    let group_to_update = &mut grouped_dictionary_entries[original_group_idx];
                    if group_to_update.ids.contains(id) {
                        continue; // Already has this specific entry by ID
                    }
                    group_to_update
                        .dictionary_entries
                        .push(dictionary_entry.clone());
                    group_to_update.ids.insert(id.clone());
                    entry_processed_and_moved = true;
                }
                if entry_processed_and_moved {
                    ids_to_remove_from_ungrouped.insert(id.clone());
                }
            }
        }
        for id in ids_to_remove_from_ungrouped {
            ungrouped_dictionary_entries_map.shift_remove_full(&id);
        }
        // Search database for additional secondary terms
        if term_list.is_empty() || secondary_search_dictionary_map.is_empty() {
            return;
        }
        // Assuming self.db.find_terms_exact_bulk exists and matches this signature:
        // fn find_terms_exact_bulk(&self, terms: &[TermExactQueryRequest], dictionaries: &TermEnabledDictionaryMap) -> Result<Vec<TermEntry>, YourDbErrorType>
        // And that TermEntry has an `index` field corresponding to the index in the input `terms` slice.
        let mut database_entries = self
            .db
            .find_terms_exact_bulk(&term_list, secondary_search_dictionary_map)
            .unwrap_or_else(|e| {
                eprintln!("Error finding terms exact bulk: {e:?}");
                Vec::new()
            });
        dbg!(&database_entries);
        // this._sortDatabaseEntriesByIndex(databaseEntries);
        // Assuming TermEntry has an `index` field which is the original index from term_list
        database_entries.sort_by_key(|e| e.index);
        for database_entry in database_entries {
            // `database_entry.index` refers to the original index in `term_list`
            let original_req_index = database_entry.index; // This is the index in the `term_list`
            if original_req_index >= term_list.len() {
                eprintln!(
                    "Database returned out-of-bounds index {} for TermEntry id {}",
                    original_req_index, database_entry.id
                );
                continue;
            }
            let source_text = &term_list[original_req_index].term;
            // The groups to update are those associated with term_list[original_req_index].
            // These are found in target_list_data[original_req_index].
            let original_group_indices_for_this_db_entry = &target_list_data[original_req_index];
            for &group_idx_to_update in original_group_indices_for_this_db_entry {
                // Ensure group_idx_to_update is within bounds for `grouped_dictionary_entries`
                if group_idx_to_update >= grouped_dictionary_entries.len() {
                    eprintln!(
                        "Group index {} out of bounds for grouped_dictionary_entries (len {})",
                        group_idx_to_update,
                        grouped_dictionary_entries.len()
                    );
                    continue;
                }
                let group_to_update = &mut grouped_dictionary_entries[group_idx_to_update];
                if group_to_update.ids.contains(&database_entry.id) {
                    // This group already has this in the DB by id
                    continue;
                }
                let new_dictionary_entry =
                    Translator::_create_internal_term_dictionary_entry_from_database_entry(
                        database_entry.clone(),
                        source_text,
                        source_text,
                        source_text,
                        Vec::new(),
                        Vec::new(),
                        false,
                        enabled_dictionary_map,
                        tag_aggregator,
                        primary_reading,
                    );
                group_to_update
                    .dictionary_entries
                    .push(new_dictionary_entry);
                group_to_update.ids.insert(database_entry.id.clone());
                // If this ID was in ungrouped, remove it.
                // JS: ungroupedDictionaryEntriesMap.delete(id);
                ungrouped_dictionary_entries_map.shift_remove_full(&database_entry.id);
                // Use remove if order doesn't matter, or shift_remove if it does
            }
        }
    }

    fn _sort_term_dictionary_entries_by_id(dictionary_entries: &mut [InternalTermDictionaryEntry]) {
        if dictionary_entries.len() <= 1 {
            return;
        }
        dictionary_entries.sort_by(|a, b| a.definitions[0].id.cmp(&b.definitions[0].id));
    }
    fn _add_related_dictionary_entries(
        &self,
        grouped_dictionary_entries: &mut [DictionaryEntryGroup],
        ungrouped_dictionary_entries_map: &mut IndexMap<String, InternalTermDictionaryEntry>,
        sequence_list: Vec<GenericQueryRequest>,
        enabled_dictionary_map: &TermEnabledDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) {
        // should match result instead of empty vec but
        // this is how yomitan_js does
        let mut database_entries = self
            .db
            .find_terms_by_sequence_bulk(sequence_list)
            .unwrap_or_default();
        for db_entry in database_entries {
            let TermEntry {
                id, term, index, ..
            } = &db_entry;
            // direct access because yomitan does
            let DictionaryEntryGroup {
                ids,
                dictionary_entries,
            } = &mut grouped_dictionary_entries[*index];
            if ids.has(id) {
                continue;
            }
            let dictionary_entry =
                Translator::_create_internal_term_dictionary_entry_from_database_entry(
                    db_entry.clone(),
                    term,
                    term,
                    term,
                    vec![],
                    vec![],
                    false,
                    enabled_dictionary_map,
                    tag_aggregator,
                    primary_reading,
                );
            dictionary_entries.push(dictionary_entry);
            ids.insert(id.clone());
            // this could be optimized depending on if order matters
            // for now preserve order jic
            ungrouped_dictionary_entries_map.shift_remove_full(id);
        }
    }
    fn _get_secondary_search_dictionary_map(
        enabled_dictionary_map: &TermEnabledDictionaryMap,
    ) -> TermEnabledDictionaryMap {
        enabled_dictionary_map
            .iter()
            .filter_map(|(dictionary, details)| {
                if !details.allow_secondary_searches {
                    return None;
                }
                Some((dictionary.to_owned(), details.to_owned()))
            })
            .collect()
    }
    fn _group_dictionary_entries_by_headword(
        &self,
        language: &str,
        dictionary_entries: &[InternalTermDictionaryEntry],
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> Vec<InternalTermDictionaryEntry> {
        let mut groups: IndexMap<String, Vec<InternalTermDictionaryEntry>> = IndexMap::new();
        let reading_normalizer = self.reading_normalizers.get(language);
        for dictionary_entry in dictionary_entries {
            let InternalTermDictionaryEntry {
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
        dictionary_entries: &[InternalTermDictionaryEntry],
        mut check_duplicate_definitions: bool,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> InternalTermDictionaryEntry {
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
            let term_dictionary_entry_with_map = InternalTermDictionaryEntryWithIndexes {
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
        let mut inflections: Option<Vec<InternalInflectionRuleChainCandidate>> = None;
        let mut text_processes: Option<Vec<TextProcessorRuleChainCandidate>> = None;
        for definition_entry in &definition_entries {
            let InternalTermDictionaryEntryWithIndexes {
                dictionary_entry,
                headword_indexes,
                ..
            } = definition_entry;
            score = score.max(dictionary_entry.score);
            dictionary_index = dictionary_index.min(dictionary_entry.dictionary_index);

            // yomitan actually doesn't add the alias, this diverges from js,
            // the rest is the same after this line
            dictionary_alias = dictionary_entry.dictionary_alias.clone();

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
        Translator::_create_internal_term_dictionary_entry(
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
    ) -> FindInternalTermsResult {
        let mut original_text_length = 0;
        let mut dictionary_entries: Vec<InternalTermDictionaryEntry> = vec![];
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
                        Translator::_create_internal_term_dictionary_entry_from_database_entry(
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
                    Translator::_create_internal_term_dictionary_entry_from_database_entry(
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
        FindInternalTermsResult {
            dictionary_entries,
            original_text_length: original_text_length as i128,
        }
    }
    fn _merge_text_processor_rule_chains(
        existing_entry: &mut InternalTermDictionaryEntry,
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
        existing_entry: &mut InternalTermDictionaryEntry,
        inflection_rule_chain_candidates: &[InternalInflectionRuleChainCandidate],
    ) {
        for candidate in inflection_rule_chain_candidates {
            let InternalInflectionRuleChainCandidate {
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
                let new_rule_chain_candidate = InternalInflectionRuleChainCandidate {
                    source: *source,
                    inflection_rules: inflection_rules.clone(),
                };
                existing_entry
                    .inflection_rule_chain_candidates
                    .push(new_rule_chain_candidate);
            }
        }
    }

    fn _are_arrays_equal_ignore_order(x: &[impl AsRef<str>], y: &[impl AsRef<str>]) -> bool {
        if x.len() != y.len() {
            return false;
        }

        let mut frequency_counter = IndexMap::new();

        for element_ref in x {
            *frequency_counter.entry(element_ref.as_ref()).or_insert(0) += 1;
        }

        for element_ref in y {
            let count = frequency_counter.entry(element_ref.as_ref()).or_insert(0);
            if *count == 0 {
                return false; // Found an element in y that's not in x or has excess frequency
            }
            *count -= 1;
        }

        // All elements in y were accounted for in the frequency map.
        // Since the lengths are equal, the arrays are permutations of each other.
        true
    }

    fn _find_existing_entry(
        dictionary_entries: &[InternalTermDictionaryEntry],
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
    fn _create_internal_term_dictionary_entry_from_database_entry(
        database_entry: TermEntry,
        original_text: &str,
        transformed_text: &str,
        deinflected_text: &str,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InternalInflectionRuleChainCandidate>,
        is_primary: bool,
        enabled_dictionary_map: &FindTermDictionaryMap,
        tag_aggregator: &mut TranslatorTagAggregator,
        primary_reading: &str,
    ) -> InternalTermDictionaryEntry {
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
            .clone()
            .into_iter()
            .filter_map(|def| match def {
                TermGlossary::Content(c) => Some(c),
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
        Translator::_create_internal_term_dictionary_entry(
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
    fn _create_internal_term_dictionary_entry(
        is_primary: bool,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InternalInflectionRuleChainCandidate>,
        score: i128,
        dictionary_index: usize,
        dictionary_alias: String,
        source_term_exact_match_count: usize,
        match_primary_reading: bool,
        max_original_text_length: usize,
        headwords: Vec<TermHeadword>,
        definitions: Vec<TermDefinition>,
    ) -> InternalTermDictionaryEntry {
        InternalTermDictionaryEntry {
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
                for def in &entry.definitions {
                    if matches!(def, TermGlossary::Deinflection(_)) {
                        let incorrect_values_path = test_utils::TEST_PATHS
                            .tests_dir
                            .join("incorrect")
                            .with_extension("json");
                        let json = serde_json::to_string_pretty(&def).unwrap();
                        std::fs::write(incorrect_values_path, json);
                    }
                }
                entry
                    .definitions
                    .retain(|def| !matches!(def, TermGlossary::Deinflection(_)))
            }
            deinflection
                .database_entries
                .retain(|entry| !entry.definitions.is_empty());
        }
        //eprintln!("_get_deinflections: final deinflections list (after all processing & filtering): count={}, content={:#?}", deinflections.len(), deinflections);
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
                            form_of,
                            rules: inflection_rules,
                        } = term_glossary_deinflection;
                        if form_of.is_empty() {
                            continue;
                        }
                        let inflection_rule_chain_candidates: Vec<
                            InternalInflectionRuleChainCandidate,
                        > = algorithm_chains
                            .iter()
                            .map(|alg_chain_candidate| {
                                let alg_inflections = &alg_chain_candidate.inflection_rules;
                                let source = if alg_inflections.is_empty() {
                                    InflectionSource::Dictionary
                                } else {
                                    InflectionSource::Both
                                };
                                let inflection_rule_chain_candidates: Vec<String> = alg_inflections
                                    .iter()
                                    .cloned()
                                    .chain(inflection_rules.iter().cloned())
                                    .collect();
                                InternalInflectionRuleChainCandidate {
                                    source,
                                    inflection_rules: inflection_rule_chain_candidates,
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
            let text_replacements = Translator::_get_text_replacement_variants(opts);
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
                        let inflection_rule_chain_candidate =
                            InternalInflectionRuleChainCandidate {
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
        let has_bueno_candidate = db_deinflections
            .iter()
            .any(|d| d.deinflected_text == "bueno");
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

    // In impl<'a> Translator<'a>

    fn _add_entries_to_deinflections(
        &self,
        language: &str,
        deinflections: &mut [DatabaseDeinflection],
        enabled_dictionary_map: &FindTermDictionaryMap,
        match_type: TermSourceMatchType,
    ) {
        if deinflections.is_empty() {
            return;
        }

        // Step 1: Group mutable references to the original deinflections by their term.
        // This is the equivalent of JS `_groupDeinflectionsByTerm` but for mutable Rust.
        let mut unique_deinflections_map: IndexMap<String, Vec<&mut DatabaseDeinflection>> =
            IndexMap::new();
        for deinflection in deinflections.iter_mut() {
            // Do not process empty strings.
            if deinflection.deinflected_text.is_empty() {
                continue;
            }
            unique_deinflections_map
                .entry(deinflection.deinflected_text.clone())
                .or_default()
                .push(deinflection);
        }

        if unique_deinflections_map.is_empty() {
            return;
        }

        // Step 2: Get the unique terms for the DB query
        // and the corresponding groups of mutable references.
        let unique_deinflection_terms: Vec<String> =
            unique_deinflections_map.keys().cloned().collect();

        // Note: `into_values()` consumes the map.
        // This is fine as we don't need the map afterward.
        let mut unique_deinflection_groups: Vec<Vec<&mut DatabaseDeinflection>> =
            unique_deinflections_map.into_values().collect();

        // Step 3: Query the database.
        let database_entries = self
            .db
            .find_terms_bulk(
                &unique_deinflection_terms,
                enabled_dictionary_map,
                match_type,
            )
            .unwrap_or_default();
        // println!(
        //     "found {} for:\n unique_deinflections_terms: {:#?}, enabled_dictionary_map: {:#?}",
        //     database_entries.len(),
        //     &unique_deinflection_terms,
        //     &enabled_dictionary_map
        // );

        // Step 4: Match the results back to the original deinflections via the grouped mutable references.
        self._match_entries_to_deinflections(
            language,
            &database_entries,
            &mut unique_deinflection_groups,
            enabled_dictionary_map,
        );
    }

    // fn _add_entries_to_deinflections(
    //     &self,
    //     language: &str,
    //     deinflections: &mut [DatabaseDeinflection],
    //     enabled_dictionary_map: &FindTermDictionaryMap,
    //     match_type: TermSourceMatchType,
    // ) {
    //     if deinflections.is_empty() {
    //         return;
    //     }
    //
    //     // 1. Group deinflections by term, storing mutable references to the originals.
    //     // This is equivalent to `_groupDeinflectionsByTerm` in JS.
    //     let mut unique_deinflections_map: IndexMap<String, Vec<&mut DatabaseDeinflection>> =
    //         IndexMap::new();
    //     for deinflection in deinflections.iter_mut() {
    //         if deinflection.deinflected_text.is_empty() {
    //             continue;
    //         }
    //         unique_deinflections_map
    //             .entry(deinflection.deinflected_text.clone())
    //             .or_default()
    //             .push(deinflection);
    //     }
    //
    //     // 2. Get the unique terms for the DB query and the arrays of mutable references.
    //     let mut unique_deinflection_terms: Vec<String> =
    //         unique_deinflections_map.keys().cloned().collect();
    //     //unique_deinflection_terms.retain(|term| !term.is_empty());
    //     let mut unique_deinflection_arrays: Vec<Vec<&mut DatabaseDeinflection>> =
    //         unique_deinflections_map.into_values().collect();
    //
    //     // 3. Query the database with the unique terms.
    //     dbg!(
    //         "Calling findTermsBulk with the following terms:",
    //         &unique_deinflection_terms
    //     );
    //     let database_entries = self
    //         .db
    //         .find_terms_bulk(
    //             &unique_deinflection_terms,
    //             &enabled_dictionary_map,
    //             match_type,
    //         )
    //         .unwrap_or_default();
    //     //dbg!("term entries len: {}", database_entries.len());
    //
    //     // 4. Match the results back to the original deinflections via the grouped mutable references.
    //     self._match_entries_to_deinflections(
    //         language,
    //         &database_entries,
    //         &mut unique_deinflection_arrays,
    //         enabled_dictionary_map,
    //     );
    // }

    fn _match_entries_to_deinflections(
        &self,
        language: &str,
        database_entries: &[TermEntry],
        // The data structure now matches the JS: an array of arrays (of mutable refs).
        unique_deinflection_arrays: &mut [Vec<&mut DatabaseDeinflection>],
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

            let definition_conditions = self
                .mlt
                .get_condition_flags_from_parts_of_speech(language, &entry.rules);

            // Get the group of deinflections corresponding to this database entry's index.
            if let Some(deinflection_group) = unique_deinflection_arrays.get_mut(entry.index) {
                // Iterate through each deinflection in the group and push the entry.
                // This replicates the one-to-many mapping.
                for deinflection in deinflection_group.iter_mut() {
                    if !entry_dictionary.parts_of_speech_filter
                        || LanguageTransformer::conditions_match(
                            deinflection.conditions,
                            definition_conditions,
                        )
                    {
                        // `deinflection` is `&mut &mut DatabaseDeinflection`, so we dereference it
                        // to access the `database_entries` field and push the cloned entry.
                        deinflection.database_entries.push(entry.clone());
                    }
                }
            }
        }
    }

    // fn _match_entries_to_deinflections(
    //     &self,
    //     language: &str,
    //     database_entries: &[TermEntry],
    //     unique_deinflection_arrays: &mut Vec<&mut Vec<DatabaseDeinflection>>,
    //     enabled_dictionary_map: &FindTermDictionaryMap,
    // ) {
    //     for entry in database_entries {
    //         let entry_dictionary = enabled_dictionary_map
    //             .get(&entry.dictionary)
    //             .unwrap_or_else(|| {
    //                 panic!(
    //                     "{} was not found in enabled_dictionary_map",
    //                     &entry.dictionary
    //                 )
    //             });
    //         let parts_of_speech_filter = entry_dictionary.parts_of_speech_filter;
    //         if parts_of_speech_filter {
    //             eprintln!("parts_of_speech_filter for {entry_dictionary:?} is true, it will not push the entry to the deinflection.database_entries");
    //             continue;
    //         }
    //         let definition_conditions = self
    //             .mlt
    //             .get_condition_flags_from_parts_of_speech(language, &entry.rules);
    //         for deinflection in &mut **unique_deinflection_arrays
    //             .get_mut(entry.index)
    //             .unwrap_or_else(|| {
    //                 panic!(
    //                     "unique_deinflection_array.get({}) doesn't exist inside the array",
    //                     entry.index
    //                 )
    //             })
    //         {
    //             if LanguageTransformer::conditions_match(
    //                 deinflection.conditions,
    //                 definition_conditions,
    //             ) {
    //                 deinflection.database_entries.push(entry.clone());
    //             } else {
    //                 eprintln!("conditons_match returned false for deinflection conditions: {deinflection:#?}");
    //             }
    //         }
    //     }
    // }

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
    fn _get_text_replacement_variants(opts: &FindTermsOptions) -> FindTermsTextReplacements {
        opts.text_replacements.clone()
    }
    fn _create_deinflection(
        original_text: &str,
        transformed_text: &str,
        deinflected_text: &str,
        conditions: usize,
        text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
        inflection_rule_chain_candidates: Vec<InternalInflectionRuleChainCandidate>,
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

static GET_NEXT_SUBSTRING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^\p{L}][\p{L}\p{N}]*$").expect("Invalid get_next_substring_regex pattern")
});

#[derive(Clone, Debug)]
pub struct TagGroup {
    dictionary: String,
    tag_names: Vec<String>,
}

type TagCache = IndexMap<String, Option<DatabaseTag>>;

#[derive(Clone, Debug, PartialEq)]
pub struct TagTargetItem {
    pub query: String,
    pub dictionary: String,
    pub tag_name: String,
    pub cache: Option<TagCache>,
    pub database_tag: Option<DatabaseTag>,
    pub targets: Vec<Vec<DictionaryTag>>,
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
    /// Adds new tag names to a TagGroup, ensuring uniqueness.
    fn _add_unique_tags_to_group(tag_group: &mut TagGroup, new_tag_names_slice: &[String]) {
        for tag_name_to_add in new_tag_names_slice {
            if !tag_group.tag_names.contains(tag_name_to_add) {
                tag_group.tag_names.push(tag_name_to_add.clone());
            }
        }
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SequenceQuery {
    query: i128,
    dictionary: String,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum FindTermsMode {
    #[default]
    Simple,
    Group,
    Merge,
    Split,
}

trait HasTags {
    fn _get_definition_tags_mut(&mut self) -> &mut Vec<DictionaryTag>;
}
impl HasTags for TermDefinition {
    fn _get_definition_tags_mut(&mut self) -> &mut Vec<DictionaryTag> {
        &mut self.tags
    }
}
impl HasTags for TermHeadword {
    fn _get_definition_tags_mut(&mut self) -> &mut Vec<DictionaryTag> {
        &mut self.tags
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TermTypeError {
    #[error("could not convert from TermType::{}: to {target}", 
        value.name_as_str()
    )]
    TryFrom {
        value: TermType,
        target: &'static str,
    },
}
/// used to differentiate between the following:
/// [TermDefinition]
/// [TermPronunciation]
/// [TermFrequency]
#[derive(Clone, PartialEq, Debug, From)]
pub enum TermType {
    Definition(TermDefinition),
    Pronunciation(TermPronunciation),
    Frequency(TermFrequency),
}
impl TermType {
    /// returns the name of the variant
    fn name_as_str(&self) -> &'static str {
        match &self {
            Self::Definition(_) => "Definition",
            Self::Frequency(_) => "Frequency",
            Self::Pronunciation(_) => "Pronunciation",
        }
    }
}

/// helper to not need to match on enum variants to get fields
trait IsTermType {
    // TermDefinition sorting
    /// only exists on [TermDefinition]
    // TermFrequency/TermPronunciation sorting
    /// only exists on [TermFrequency] && [TermPronunciation]
    fn _get_ipa_or_freq_headword_index(&self) -> Option<usize>;
    // Fields common to all or used by both, assumed to be present
    fn dictionary_index(&self) -> usize;
    fn index(&self) -> usize;
    /// gets the term's (`.dictionary, .dictionary_alias`)
    fn dictionary_and_alias(&self) -> (&str, &str);
}
impl IsTermType for TermType {
    /// generic for all variants
    fn dictionary_and_alias(&self) -> (&str, &str) {
        match &self {
            Self::Definition(m) => (&m.dictionary, &m.dictionary_alias),
            Self::Frequency(m) => (&m.dictionary, &m.dictionary_alias),
            Self::Pronunciation(m) => (&m.dictionary, &m.dictionary_alias),
        }
    }
    fn dictionary_index(&self) -> usize {
        match self {
            TermType::Definition(d) => d.dictionary_index,
            TermType::Pronunciation(p) => p.dictionary_index,
            TermType::Frequency(f) => f.dictionary_index,
        }
    }
    fn index(&self) -> usize {
        match self {
            TermType::Definition(d) => d.index,
            TermType::Pronunciation(p) => p.index,
            TermType::Frequency(f) => f.index,
        }
    }
    /// only exists on [TermFrequency] && [TermPronunciation]
    fn _get_ipa_or_freq_headword_index(&self) -> Option<usize> {
        match self {
            TermType::Pronunciation(p) => Some(p.headword_index),
            TermType::Frequency(f) => Some(f.headword_index),
            _ => None,
        }
    }
}
impl TryFrom<TermType> for TermFrequency {
    type Error = TermTypeError;
    fn try_from(value: TermType) -> Result<Self, Self::Error> {
        match value {
            TermType::Frequency(f) => Ok(f),
            _ => Err(TermTypeError::TryFrom {
                value,
                target: "TermFrequency",
            }),
        }
    }
}
impl TryFrom<TermType> for TermDefinition {
    type Error = TermTypeError;
    fn try_from(value: TermType) -> Result<Self, Self::Error> {
        match value {
            TermType::Definition(d) => Ok(d),
            _ => Err(TermTypeError::TryFrom {
                value,
                target: "TermDefinition",
            }),
        }
    }
}
impl TryFrom<TermType> for TermPronunciation {
    type Error = TermTypeError;
    fn try_from(value: TermType) -> Result<Self, Self::Error> {
        match value {
            TermType::Pronunciation(p) => Ok(p),
            _ => Err(TermTypeError::TryFrom {
                value,
                target: "TermPronunciation",
            }),
        }
    }
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
    pub entry: InternalTermDictionaryEntry,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InternalTermDictionaryEntryWithIndexes {
    pub index: usize,
    pub dictionary_entry: InternalTermDictionaryEntry,
    pub headword_indexes: Vec<usize>,
}
#[derive(Clone, Debug)]
struct TermReadingItem {
    pub term: String,
    pub reading: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FindTermsResult {
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TermMetaHeadword {
    headword_index: usize,
    original_index: usize,
    pronunciations: Vec<TermPronunciation>,
    frequencies: Vec<TermFrequency>,
}
type TermMetaHeadwordMap = IndexMap<String, IndexMap<String, Vec<TermMetaHeadword>>>;

#[cfg(test)]
mod translator_tests {
    use super::{FindTermsMode, Translator};
    use crate::{test_utils::TEST_PATHS, translation::FindTermsOptions};

    // #[test]
    // fn find_terms() {
    // }
}
