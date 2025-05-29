use std::{collections::HashMap, fmt::Display, str::FromStr};

use icu::{
    collator::{options::CollatorOptions, Collator, CollatorBorrowed},
    locale::locale,
};
use language_transformer::{
    descriptors::{PreAndPostProcessors, PreAndPostProcessorsWithId},
    ja::japanese::is_code_point_japanese,
    language_d::{LanguageAndProcessors, LanguageAndReadingNormalizer, ReadingNormalizer},
    languages::{get_all_language_reading_normalizers, get_all_language_text_processors},
    multi_language_transformer::MultiLanguageTransformer,
    zh::chinese::is_code_point_chinese,
};
use native_db::*;
use native_model::{native_model, Model};
use regex::Regex;

use crate::{
    database::dictionary_database::DictionaryDatabaseTag,
    dictionary::{DictionaryTag, TermDictionaryEntry},
    translation::FindTermsOptions, translation_internal::DatabaseDeinflection,
};

type TagCache = HashMap<&'static str, Option<DictionaryDatabaseTag>>;

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
struct FindTermResult {
    dictionary_entries: Vec<TermDictionaryEntry>,
    original_text_length: u64,
}

type TextProcessorMap = HashMap<&'static str, PreAndPostProcessorsWithId>;
type ReadingNormalizerMap = HashMap<&'static str, ReadingNormalizer>;

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
    fn new(db: Database<'static>) -> Self {
        Self {
            db,
            mlt: MultiLanguageTransformer::default(),
            tag_cache: HashMap::new(),
            string_comparer: Collator::try_new(locale!("en-US").into(), CollatorOptions::default())
                .unwrap(),
            number_regex: Regex::new(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?").unwrap(),
            text_processors: HashMap::new(),
            reading_normalizers: HashMap::new(),
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
    fn find_terms(mode: FindTermsMode, text: &str, opts: FindTermsOptions) {
        let FindTermsOptions {
            enabled_dictionary_map,
            exclude_dictionary_definitions,
            sort_frequency_dictionary,
            sort_frequency_dictionary_order,
            language,
            primary_reading,
            ..
        } = opts;
        let tag_aggregator = TranslatorTagAggregator::default();
    }

    fn find_terms_internal(
        &self,
        text: &mut String,
        opts: FindTermsOptions,
        tag_aggregator: TranslatorTagAggregator,
        primary_reading: &str,
    ) -> FindTermResult {
        let FindTermsOptions {
            remove_non_japanese_characters,
            enabled_dictionary_map,
            ..
        } = opts;
        if remove_non_japanese_characters && ["ja", "zh", "yue"].contains(opts.language.as_str()) {
            *text = Translator::get_japanese_chinese_only_text(&text);
        }
        if text.is_empty() {
            return FindTermResult {
                dictionary_entries: vec![],
                original_text_length: 0,
            };
        }
        let deinflections = 
    }

    fn _get_deinflections(text: &str, opts: FindTermsOptions) -> DatabaseDeinflection {
        let deinflections = (
        if opts.deinflect {
            Translation::_get_algorithm_deinflections
        }
    );
        
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
    tag_expansion_target_map: HashMap<Vec<DictionaryTag>, Vec<TagGroup>>,
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
