use std::{collections::HashMap, fmt::Display, str::FromStr, sync::LazyLock};

use fancy_regex::Regex;
use icu::{
    collator::{options::CollatorOptions, Collator, CollatorBorrowed},
    locale::locale,
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
    transformer::TransformedText,
    zh::chinese::is_code_point_chinese,
};
use native_db::*;
use native_model::{native_model, Model};

use crate::{
    database::dictionary_database::DictionaryDatabaseTag,
    dictionary::{
        DictionaryTag, InflectionRuleChainCandidate, InflectionSource, TermDictionaryEntry,
        TermSourceMatchType,
    },
    regex_util::apply_text_replacement,
    settings::SearchResolution,
    translation::{FindTermDictionary, FindTermsOptions},
    translation_internal::{
        DatabaseDeinflection, TextCache, TextProcessorRuleChainCandidate,
        VariantAndTextProcessorRuleChainCandidatesMap,
    },
};

type TagCache = HashMap<&'static str, Option<DictionaryDatabaseTag>>;

static GET_NEXT_SUBSTRING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^\p{L}][\p{L}\p{N}]*$").expect("Invalid get_next_substring_regex pattern")
});

#[derive(Clone, Debug)]
struct SequenceQuery {
    query: u128,
    dictionary: String,
}

#[derive(Clone, Copy, Debug)]
enum FindTermsMode {
    Simple,
    Group,
    Merge,
    Split,
}

#[derive(Clone, Debug)]
struct TermReadingItem {
    term: String,
    reading: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FindTermResult {
    dictionary_entries: Vec<TermDictionaryEntry>,
    original_text_length: u64,
}

type TextProcessorMap = HashMap<&'static str, PreAndPostProcessorsWithId>;
type ReadingNormalizerMap = HashMap<&'static str, ReadingNormalizer>;

#[derive(thiserror::Error, Debug)]
pub enum TranslatorError {
    #[error("Unsupported Language: {0}")]
    UnsupportedLanguage(String),
}

/// class which finds term and kanji dictionary entries for text.
struct Translator {
    db: Database<'static>,
    mlt: MultiLanguageTransformer,
    tag_cache: HashMap<&'static str, TagCache>,
    /// Invariant Locale
    /// Default: "en-US"
    string_comparer: CollatorBorrowed<'static>,
    number_regex: Regex,
    text_processors: TextProcessorMap,
    reading_normalizers: ReadingNormalizerMap,
}

impl Translator {
//     fn new(db: Database<'static>) -> Self {
//         Self {
//             db,
//             mlt: MultiLanguageTransformer::default(),
//             tag_cache: HashMap::new(),
//             string_comparer: Collator::try_new(locale!("en-US").into(), CollatorOptions::default())
//                 .unwrap(),
//             number_regex: Regex::new(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?").unwrap(),
//             text_processors: HashMap::new(),
//             reading_normalizers: HashMap::new(),
//         }
//     }
//
//     /// Initializes the instance for use.
//     /// The public API should not be used until this function has been called.
//     fn prepare(&mut self) {
//         let language_processors = get_all_language_text_processors();
//         for processor in language_processors {
//             let LanguageAndProcessors { iso, pre, post } = processor;
//             let pre_and_post_processors = PreAndPostProcessorsWithId { pre, post };
//             self.text_processors.insert(iso, pre_and_post_processors);
//         }
//         let reading_normalizers = get_all_language_reading_normalizers();
//         for normalizer in reading_normalizers {
//             let LanguageAndReadingNormalizer {
//                 iso,
//                 reading_normalizer,
//             } = normalizer;
//             self.reading_normalizers.insert(iso, reading_normalizer);
//         }
//     }
//
//     /// Clears the database tag cache.
//     /// This should be called if the database is changed.
//     fn clear_dbtag_caches(&mut self) {
//         self.tag_cache.clear();
//     }
//
//     /// Finds term definitions for the given text.
//     ///
//     /// This function operates asynchronously.
//     ///
//     /// # Parameters
//     ///
//     /// `mode`: The [FindTermsMode] to use for finding terms.
//     /// Determines the format of the resulting array.
//     /// Must be one of: `Group`, `Merge`, `Split`, or `Simple`.
//     ///
//     ///  `text`: The `&str` text to find terms for.
//     ///
//     ///  `options`: A reference to `FindTermsOptions`, a struct describing settings
//     ///   for the lookup.
//     ///
//     /// # Returns
//     ///
//     /// A `Result` which, upon success, yields a struct containing:
//     ///  `dictionary_entries`: A `Vec<TermDictionaryEntry>` with the found terms.
//     ///  `original_text_length`: The `usize` length of the original source text.
//     ///
//     /// # Errors
//     ///
//     /// Returns an error if the term lookup process fails for any reason.
//     fn find_terms(mode: FindTermsMode, text: &str, opts: FindTermsOptions) {
//         let FindTermsOptions {
//             enabled_dictionary_map,
//             exclude_dictionary_definitions,
//             sort_frequency_dictionary,
//             sort_frequency_dictionary_order,
//             language,
//             primary_reading,
//             ..
//         } = opts;
//         let tag_aggregator = TranslatorTagAggregator::default();
//     }
//
//     fn find_terms_internal(
//         &self,
//         text: &mut String,
//         opts: FindTermsOptions,
//         tag_aggregator: TranslatorTagAggregator,
//         primary_reading: &str,
//     ) -> FindTermResult {
//         let FindTermsOptions {
//             remove_non_japanese_characters,
//             enabled_dictionary_map,
//             ..
//         } = opts;
//         if remove_non_japanese_characters && ["ja", "zh", "yue"].contains(&opts.language.as_str()) {
//             *text = Translator::get_japanese_chinese_only_text(&text);
//         }
//         if text.is_empty() {
//             return FindTermResult {
//                 dictionary_entries: vec![],
//                 original_text_length: 0,
//             };
//         }
//         let deinflections = self._get_deinflections(text, opts);
//     }
//
//     fn _get_deinflections(&self, text: &str, opts: FindTermsOptions) -> Vec<DatabaseDeinflection> {
//         let deinflections = if opts.deinflect {
//             self._get_algorithm_deinflections(text, &opts).unwrap()
//         } else {
//             vec![Translator::_create_deinflection(
//                 text,
//                 text,
//                 text,
//                 0,
//                 vec![],
//                 vec![],
//             )]
//         };
//         if deinflections.is_empty() {
//             return vec![];
//         }
//
//         let FindTermsOptions {
//             match_type,
//             language,
//             enabled_dictionary_map,
//             ..
//         } = opts;
//
//         Translator::deinflections
//     }
//
//     fn _get_algorithm_deinflections(
//         &self,
//         text: &str,
//         opts: &FindTermsOptions,
//     ) -> Result<Vec<DatabaseDeinflection>, TranslatorError> {
//         let language = opts.language.clone();
//         let Some(processors_for_lang) = self.text_processors.get(language.as_str()) else {
//             return Err(TranslatorError::UnsupportedLanguage(language));
//         };
//         let PreAndPostProcessorsWithId { pre, post } = processors_for_lang;
//         let mut db_deinflections: Vec<DatabaseDeinflection> = Vec::new();
//         // for reusing text processor's outputs
//         let mut source_cache = HashMap::new();
//         let mut raw_source = text.to_string();
//         while !raw_source.is_empty() {
//             let text_replacements = Translator::_get_text_replacement_variants(opts.clone());
//             let pre_processed_text_variants = Translator::_get_text_variants(
//                 &raw_source,
//                 pre,
//                 text_replacements,
//                 &mut source_cache,
//             );
//             for pre_processed_variant in pre_processed_text_variants {
//                 let (source, preprocessor_rule_chain_candidates) = pre_processed_variant;
//                 let deinflections = self.mlt.transform(&language, &source);
//                 for deinflection in deinflections {
//                     let TransformedText {
//                         trace, conditions, ..
//                     } = deinflection;
//                     let postprocessed_text_variants = Translator::_get_text_variants(
//                         &deinflection.text,
//                         post,
//                         vec![None],
//                         &mut source_cache,
//                     );
//                     for post_processed_variant in postprocessed_text_variants {
//                         let (transformed_text, postprocessor_rule_chain_candidates) =
//                             post_processed_variant;
//                         let inflection_rule_chain_candidate = InflectionRuleChainCandidate {
//                             source: InflectionSource::Algorithm,
//                             inflection_rules: trace.iter().map(|frame| frame.transform).collect(),
//                         };
//
//                         // every combination of preprocessor rule candidates
//                         // and postprocessor rule candidates
//                         let text_processor_rule_chain_candidates: Vec<Vec<String>> =
//                             preprocessor_rule_chain_candidates
//                                 .iter()
//                                 .flat_map(|pre_candidate_slice| {
//                                     postprocessor_rule_chain_candidates.iter().map(
//                                         move |post_candidate_slice| {
//                                             pre_candidate_slice
//                                                 .iter()
//                                                 .cloned()
//                                                 .chain(post_candidate_slice.iter().cloned())
//                                                 .collect::<Vec<String>>()
//                                         },
//                                     )
//                                 })
//                                 .collect();
//                         let new_deinflection = Translator::_create_deinflection(
//                             &raw_source,
//                             &source,
//                             &transformed_text,
//                             conditions,
//                             text_processor_rule_chain_candidates,
//                             vec![inflection_rule_chain_candidate],
//                         );
//                         db_deinflections.push(new_deinflection);
//                     }
//                 }
//             }
//             raw_source = Translator::_get_next_substring(opts.search_resolution, &raw_source);
//         }
//         Ok(db_deinflections)
//     }
//
//     fn _get_text_variants(
//         text: &str,
//         text_processors: &[TextProcessorWithId],
//         text_replacements: FindTermsTextReplacements,
//         text_cache: &mut TextCache,
//     ) -> VariantAndTextProcessorRuleChainCandidatesMap {
//         let mut variants_map: VariantAndTextProcessorRuleChainCandidatesMap = HashMap::new();
//         variants_map.insert(text.to_string(), vec![vec![]]);
//         for (id, replacement) in text_replacements.iter().enumerate() {
//             let Some(replacement) = replacement else {
//                 continue;
//             };
//             let k = Translator::_apply_text_replacements(text, replacement);
//             let v = vec![vec![format!("Text Replacement {id}")]];
//             variants_map.insert(k, v);
//         }
//         for processor in text_processors {
//             let TextProcessorWithId { id, processor } = processor;
//             let TextProcessor {
//                 options, process, ..
//             } = processor;
//             let mut new_variants_map: VariantAndTextProcessorRuleChainCandidatesMap =
//                 HashMap::new();
//             for variant in variants_map.iter() {
//                 let (variant, current_preprocessor_rule_chain_candidates) = variant;
//                 for opt in options.into_iter() {
//                     let processed = Translator::_get_processed_text(
//                         text_cache,
//                         variant.clone(),
//                         id.to_string(),
//                         opt.clone(),
//                         *process,
//                     );
//                     let existing_candidates = new_variants_map.get(&processed);
//                     let mapped_current_preprocessor_rule_chain_candidates: Vec<Vec<String>> =
//                         current_preprocessor_rule_chain_candidates
//                             .clone()
//                             .into_iter()
//                             .map(|mut candidate: Vec<String>| {
//                                 candidate.push(id.to_string());
//                                 candidate
//                             })
//                             .collect();
//                     // ignore if applying text_processor !change source
//                     if processed == *variant {
//                         if let Some(existing_candidates) = existing_candidates {
//                             new_variants_map.insert(processed, existing_candidates.clone());
//                         } else {
//                             new_variants_map.insert(
//                                 processed,
//                                 current_preprocessor_rule_chain_candidates.clone(),
//                             );
//                         }
//                     } else if let Some(existing_candidates) = existing_candidates {
//                         let concat = existing_candidates
//                             .clone()
//                             .into_iter()
//                             .chain(mapped_current_preprocessor_rule_chain_candidates.into_iter())
//                             .collect();
//                         new_variants_map.insert(processed, concat);
//                     } else {
//                         new_variants_map
//                             .insert(processed, mapped_current_preprocessor_rule_chain_candidates);
//                     }
//                 }
//             }
//             variants_map = new_variants_map;
//         }
//         variants_map
//     }
//
//     fn _add_entries_to_deinflections(
//         &self
//         language: &str,
//         deinflections: Vec<DatabaseDeinflection>,
//         enabled_dictionary_map: FindTermDictionary,
//         match_type: TermSourceMatchType,
//     ) {
//         let unique_deinflections_map = Translator::_group_deinflections_by_term(deinflections);
//         let uniqe_deinflection_values_vec: Vec<Vec<DatabaseDeinflection>> =
//             unique_deinflections_map.values().cloned().collect();
//         let uniqe_deinflection_keys_vec: Vec<String> =
//             unique_deinflections_map.keys().cloned().collect();
//
//         let database_entries = self.db
//     }
//
//     fn _group_deinflections_by_term(
//         deinflections: Vec<DatabaseDeinflection>,
//     ) -> HashMap<String, Vec<DatabaseDeinflection>> {
//         let mut result: HashMap<String, Vec<DatabaseDeinflection>> = HashMap::new();
//         for deinflection in deinflections {
//             let key = deinflection.deinflected_text.clone();
//             result
//                 .entry(key)
//                 .or_insert_with(Vec::new)
//                 .push(deinflection);
//         }
//         result
//     }
//
//     /// helper function to return (opts: FindTermOptions).text_replacements
//     fn _get_text_replacement_variants(opts: FindTermsOptions) -> FindTermsTextReplacements {
//         opts.text_replacements
//     }
//
//     fn _create_deinflection(
//         original_text: &str,
//         transformed_text: &str,
//         deinflected_text: &str,
//         conditions: usize,
//         text_processor_rule_chain_candidates: Vec<TextProcessorRuleChainCandidate>,
//         inflection_rule_chain_candidates: Vec<InflectionRuleChainCandidate>,
//     ) -> DatabaseDeinflection {
//         DatabaseDeinflection {
//             original_text: original_text.to_string(),
//             transformed_text: transformed_text.to_string(),
//             deinflected_text: deinflected_text.to_string(),
//             conditions,
//             text_processor_rule_chain_candidates,
//             inflection_rule_chain_candidates,
//             database_entries: vec![],
//         }
//     }
//
//     fn _apply_text_replacements(text: &str, replacements: &[FindTermsTextReplacement]) -> String {
//         let mut text = text.to_string();
//         for replacement in replacements {
//             let FindTermsTextReplacement {
//                 pattern,
//                 replacement,
//                 is_global,
//             } = replacement;
//             text = apply_text_replacement(&text, &pattern, &replacement, is_global);
//         }
//         text
//     }
//
//     fn _get_processed_text(
//         text_cache: &mut TextCache,
//         text_key: String,
//         id_key: String,
//         setting: TextProcessorSetting,
//         process: fn(&str, TextProcessorSetting) -> String,
//     ) -> String {
//         // Level 1: Access or create the cache for the given `text_key`.
//         // `entry` API gets a mutable reference to the value if key exists,
//         // or inserts a new HashMap and returns a mutable reference to it.
//         let level1_map = text_cache
//             .entry(text_key.clone())
//             .or_insert_with(HashMap::new);
//
//         // Level 2: Access or create the cache for the given `id_key` within level1_map.
//         let level2_map = level1_map.entry(id_key).or_insert_with(HashMap::new);
//
//         // Level 3: Check if the (setting -> processed_text) mapping exists in level2_map.
//         if let Some(cached_processed_text_ref) = level2_map.get(&setting) {
//             // Cache hit: `cached_processed_text_ref` is `&&'static str`.
//             cached_processed_text_ref.to_string()
//         } else {
//             // Cache miss: process the text, store it in the cache, and then return it.
//             let processed_text_string: String = process(&text_key, setting.clone());
//             level2_map.insert(setting, processed_text_string.clone());
//             processed_text_string
//         }
//     }
//
//     fn _get_next_substring(search_resolution: SearchResolution, current_str: &str) -> String {
//         let end_byte_index: usize;
//
//         if search_resolution == SearchResolution::Word {
//             if let Some(mat) = GET_NEXT_SUBSTRING_REGEX.find(current_str).unwrap() {
//                 end_byte_index = mat.start();
//             } else {
//                 end_byte_index = 0;
//             }
//         } else {
//             let char_count = current_str.chars().count();
//
//             if char_count <= 1 {
//                 end_byte_index = 0;
//             } else {
//                 end_byte_index = current_str.char_indices().nth(char_count - 1).unwrap().0;
//             }
//         }
//
//         String::from(&current_str[0..end_byte_index])
//     }
//
//     /// Returns the initial portion of a string containing only Japanese or Chinese characters.
//     ///
//     /// It scans the input string and returns a slice ending just before the first
//     /// character that is not considered Japanese or Chinese. If all characters
//     /// are Japanese or Chinese, the entire input string slice is returned.
//     ///
//     /// # Arguments
//     /// * `text` - The input string slice to process.
//     ///
//     /// # Returns
//     /// A string slice containing only the leading Japanese/Chinese characters.
//     fn get_japanese_chinese_only_text(text: &str) -> String {
//         // .char_indices() iterates, giving the starting byte index and the char.
//         for (byte_index, c) in text.char_indices() {
//             // `c` *is* the character (Unicode Scalar Value).
//             let code_point = c as u32;
//
//             // Check if the character is *not* Japanese and *not* Chinese.
//             if !is_code_point_japanese(code_point) && !is_code_point_chinese(code_point) {
//                 // If it's not, we've found the boundary.
//                 // We return a slice from the beginning (0) up to the
//                 // *start* of the current non-matching character (byte_index).
//                 return text[..byte_index].to_string();
//             }
//         }
//
//         // If the loop finished without returning, it means all characters
//         // were either Japanese or Chinese. Return the whole string.
//         text.to_string()
//     }
// }
//
// #[derive(Clone, Debug)]
// pub struct TagGroup {
//     dictionary: String,
//     tag_names: Vec<String>,
// }
//
// #[derive(Clone, Debug)]
// struct TagTargetItem {
//     query: &'static str,
//     dictionary: String,
//     tag_name: String,
//     cache: Option<TagCache>,
//     database_tag: DictionaryDatabaseTag,
//     targets: Vec<Vec<DictionaryTag>>,
// }
//
// #[derive(Clone, Debug)]
// struct TagExpansionTarget {
//     tags: Vec<DictionaryTag>,
//     tag_groups: Vec<TagGroup>,
// }
//
// #[derive(Clone, Debug, Default)]
// struct TranslatorTagAggregator {
//     tag_expansion_target_map: HashMap<Vec<DictionaryTag>, Vec<TagGroup>>,
// }
//
// impl TranslatorTagAggregator {
//     pub fn new() -> Self {
//         Self::default()
//     }
//
//     /// Adds tags to a specific dictionary group associated with a primary list of tags.
//     ///
//     /// # Arguments
//     /// * `tags_key` - The primary list of DictionaryTags (the key).
//     /// * `dictionary_name` - The name of the dictionary to associate the new tag names with.
//     /// * `tag_names_to_add` - A slice of tag names to add to the specified dictionary group.
//     pub fn add_tags(
//         &mut self,
//         tags_key: &[DictionaryTag],
//         dictionary_name: &str,
//         tag_names_to_add: &[String],
//     ) {
//         if tag_names_to_add.is_empty() {
//             return;
//         }
//
//         let target_collection_of_tag_groups = self._get_or_create_tag_groups(tags_key);
//         let specific_tag_group = Self::_get_or_create_tag_group_in_collection(
//             target_collection_of_tag_groups,
//             dictionary_name,
//         );
//         Self::_add_unique_tags_to_group(specific_tag_group, tag_names_to_add);
//     }
//
//     /// Retrieves all tag expansion targets.
//     /// Each target consists of a list of primary tags and their associated grouped tags.
//     pub fn get_tag_expansion_targets(&self) -> Vec<TagExpansionTarget> {
//         self.tag_expansion_target_map
//             .iter()
//             .map(|(tags_vec, tag_groups_vec)| TagExpansionTarget {
//                 tags: tags_vec.clone(),
//                 tag_groups: tag_groups_vec.clone(),
//             })
//             .collect()
//     }
//
//     /// Merges tag groups from a source entry (identified by `tags_key_source`)
//     /// into a target entry (identified by `tags_key_target`).
//     pub fn merge_tags(
//         &mut self,
//         tags_key_target: &[DictionaryTag], // The key for the destination entry
//         tags_key_source: &[DictionaryTag], // The key for the source entry
//     ) {
//         // Get the source tag groups.
//         // We clone them here to avoid borrowing issues,
//         // as we'll need mutable access to
//         // `self.tag_expansion_target_map` shortly after for the target.
//         let source_groups_list_option = self.tag_expansion_target_map.get(tags_key_source).cloned(); // Clones Option<Vec<TagGroup>>
//
//         if let Some(source_groups_list) = source_groups_list_option {
//             // Now `source_groups_list` is an owned Vec<TagGroup>.
//             // Get or create the target collection of tag groups.
//             // This mutably borrows `self`.
//             let target_collection_of_tag_groups = self._get_or_create_tag_groups(tags_key_target);
//
//             for source_tag_group_item in source_groups_list {
//                 // For each TagGroup from the source,
//                 // find/create a corresponding one in the target's collection
//                 // and add its tags.
//                 let specific_target_tag_group = Self::_get_or_create_tag_group_in_collection(
//                     target_collection_of_tag_groups,
//                     &source_tag_group_item.dictionary,
//                 );
//                 Self::_add_unique_tags_to_group(
//                     specific_target_tag_group,
//                     &source_tag_group_item.tag_names,
//                 );
//             }
//         }
//         // If `tags_key_source` is not in the map,
//         // `source_groups_list_option` is None, and nothing happens,
//         // which matches the original JavaScript logic.
//     }
//
//     // --- Helper Methods (private by default) ---
//
//     /// Gets or creates a mutable reference to the list of TagGroups for a given key.
//     /// The key `tags_key_slice` is cloned to
//     /// create an owned `Vec<DictionaryTag>` for map insertion if needed.
//     fn _get_or_create_tag_groups(
//         &mut self,
//         tags_key_slice: &[DictionaryTag],
//     ) -> &mut Vec<TagGroup> {
//         self.tag_expansion_target_map
//             // Clones the slice into an owned Vec for the key
//             .entry(tags_key_slice.to_vec())
//             .or_default()
//     }
//
//     /// Finds or creates a specific TagGroup within a given collection of
//     /// TagGroups, based on dictionary name.
//     /// Returns a mutable reference to the TagGroup.
//     fn _get_or_create_tag_group_in_collection<'cl>(
//         collection_of_tag_groups: &'cl mut Vec<TagGroup>,
//         dictionary_name: &str,
//     ) -> &'cl mut TagGroup {
//         // 1. Search for the index using an *immutable* borrow first.
//         //    This borrow ends as soon as `position` gets its value.
//         let position = collection_of_tag_groups
//             .iter()
//             .position(|group| group.dictionary == dictionary_name);
//
//         // 2. Check if we found an index.
//         if let Some(pos) = position {
//             // If yes, *now* we create a mutable borrow using the index
//             // and return it immediately. The compiler knows this is safe
//             // because we don't proceed to the `push` part.
//             return &mut collection_of_tag_groups[pos];
//         }
//
//         // 3. If we didn't find it, we reach here. No borrows are active.
//         //    Create the new group.
//         let new_tag_group = TagGroup {
//             dictionary: dictionary_name.to_string(),
//             tag_names: Vec::new(),
//         };
//
//         // 4. Perform a mutable borrow for `push`. This is safe.
//         collection_of_tag_groups.push(new_tag_group);
//
//         // 5. Perform a mutable borrow for `last_mut`. This is safe.
//         //    We `unwrap` because we *know* we just pushed an element.
//         //    This borrow is then returned.
//         collection_of_tag_groups.last_mut().unwrap()
//     }
//
//     /// Adds new tag names to a TagGroup, ensuring uniqueness.
//     fn _add_unique_tags_to_group(tag_group: &mut TagGroup, new_tag_names_slice: &[String]) {
//         for tag_name_to_add in new_tag_names_slice {
//             if !tag_group.tag_names.contains(tag_name_to_add) {
//                 tag_group.tag_names.push(tag_name_to_add.clone());
//             }
//         }
//     }
}
