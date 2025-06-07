use std::{cmp::Ordering, collections::VecDeque, path::Path};

use anki_direct::AnkiClient;
use fancy_regex::Regex;
use indexmap::{IndexMap, IndexSet};
use language_transformer::{
    ja::japanese::{distribute_furigana_inflected, is_code_point_japanese, FuriganaSegment},
    language_d::FindTermsTextReplacement,
};
use serde::{Deserialize, Serialize};

use crate::{
    dictionary::{TermDictionaryEntry, TermSource, TermSourceMatchType},
    environment::{EnvironmentInfo, CACHED_ENVIRONMENT_INFO},
    settings::{
        DictionaryOptions, GeneralOptions, Options, ProfileOptions, ScanningOptions,
        TranslationOptions, TranslationTextReplacementGroup, TranslationTextReplacementOptions,
    },
    translation::{
        FindTermDictionary, FindTermsMatchType, FindTermsOptions, TermEnabledDictionaryMap,
    },
    translator::{EnabledDictionaryMapType, FindTermsMode, FindTermsResult, Translator},
    Yomichan,
};

pub struct Backend {
    environment: &'static EnvironmentInfo,
    anki: AnkiClient,
    pub translator: Translator,
}

impl Yomichan {
    pub fn set_language(&mut self, language_iso: &str) {
        self.options
            .get_current_profile_mut()
            .options
            .general
            .language = language_iso.to_string();
    }
    pub fn parse_text(&mut self, text: &str, scan_length: usize) -> Vec<LocatedTerm> {
        let current_profile = self.options.get_current_profile();
        self.backend
            ._parse_text_terms(text, &current_profile.options)
    }
    pub fn find_terms(&mut self, text: &str, details: FindTermsDetails) -> FindTermsResult {
        let current_profile = self.options.get_current_profile();
        let ProfileOptions {
            general:
                GeneralOptions {
                    result_output_mode: mode,
                    max_results,
                    ..
                },
            ..
        } = current_profile.options;
        let find_terms_options =
            Backend::_get_translator_find_terms_options(mode, &details, &current_profile.options);
        //dbg!(&find_terms_options);
        self.backend
            .translator
            .find_terms(mode, text, &find_terms_options)
    }
}

// A more useful output struct that includes positional data and the full dictionary entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocatedTerm {
    /// The starting byte index of the match in the original text.
    pub start: usize,
    /// The byte length of the matched text.
    pub length: usize,
    /// The actual text that was matched from the input string.
    pub text: String,
    /// The full dictionary entry, containing definitions, readings, score, etc.
    pub entry: TermDictionaryEntry,
}

// A temporary struct to hold the term and its "covered" status for sorting.
#[derive(Clone)]
struct ScoredTerm {
    term: LocatedTerm,
    is_covered: bool,
}

impl Backend {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            environment: &CACHED_ENVIRONMENT_INFO,
            anki: AnkiClient::default(),
            translator: Translator::new(path),
        }
    }

    /// Finds all possible dictionary terms and ranks them by relevance.
    ///
    /// This function finds all matching dictionary terms and then performs a
    /// sophisticated sort to rank them. The ranking prioritizes longer,
    /// non-overlapping terms, while still retaining shorter, valid sub-terms
    /// but giving them a lower priority.
    ///
    /// # Returns
    /// A vector of `LocatedTerm`s, sorted by a relevance score that considers
    /// whether a term is a sub-component of a larger match, its length, and its position.
    pub fn _parse_text_terms(&mut self, text: &str, options: &ProfileOptions) -> Vec<LocatedTerm> {
        const MODE: FindTermsMode = FindTermsMode::Simple;
        let details = FindTermsDetails {
            match_type: Some(TermSourceMatchType::Exact),
            deinflect: Some(true),
            primary_reading: None,
        };
        let find_terms_options =
            Backend::_get_translator_find_terms_options(MODE, &details, options);

        let mut all_found_terms: Vec<LocatedTerm> = Vec::new();
        let text_chars: Vec<char> = text.chars().collect();

        // Pass 1: Find all possible terms.
        for i in 0..text_chars.len() {
            let search_slice = &text[text_chars
                .get(..i)
                .map_or(0, |s| s.iter().map(|c| c.len_utf8()).sum())..];
            if search_slice.is_empty() {
                continue;
            }

            let find_result = self
                .translator
                .find_terms(MODE, search_slice, &find_terms_options);

            for entry in find_result.dictionary_entries {
                if let Some(source) = entry.headwords.first().and_then(|hw| hw.sources.first()) {
                    let matched_text = &source.original_text;
                    if !matched_text.is_empty() && search_slice.starts_with(matched_text) {
                        all_found_terms.push(LocatedTerm {
                            start: i, // Use character index for start
                            length: matched_text.chars().count(),
                            text: matched_text.clone(),
                            entry: entry.clone(),
                        });
                    }
                }
            }
        }

        // Remove true duplicates before scoring.
        all_found_terms.sort_unstable_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then_with(|| b.length.cmp(&a.length))
                .then_with(|| a.entry.score.cmp(&b.entry.score))
        });
        all_found_terms.dedup_by(|a, b| {
            a.start == b.start
                && a.text == b.text
                && a.entry.headwords.first().map(|h| (&h.term, &h.reading))
                    == b.entry.headwords.first().map(|h| (&h.term, &h.reading))
        });

        // Pass 2: Identify which terms are "covered" by longer terms.
        let mut scored_terms: Vec<ScoredTerm> = Vec::new();
        for term_a in &all_found_terms {
            let mut is_covered = false;
            for term_b in &all_found_terms {
                if std::ptr::eq(term_a, term_b) {
                    continue;
                }

                // A term is "covered" if its character span is a proper subset of another, longer term's span.
                let a_end = term_a.start + term_a.length;
                let b_end = term_b.start + term_b.length;

                if term_b.length > term_a.length && term_b.start <= term_a.start && b_end >= a_end {
                    is_covered = true;
                    break;
                }
            }
            scored_terms.push(ScoredTerm {
                term: term_a.clone(),
                is_covered,
            });
        }

        // Pass 3: Sort based on the new, more nuanced ranking logic.
        scored_terms.sort_unstable_by(|a, b| {
            // Primary sort: uncovered terms come before covered ones.
            a.is_covered
                .cmp(&b.is_covered)
                // Secondary sort: prioritize longer terms.
                .then_with(|| b.term.length.cmp(&a.term.length))
                // Tertiary sort: for ties, use start position.
                .then_with(|| a.term.start.cmp(&b.term.start))
                // Final tie-breaker: use dictionary score.
                .then_with(|| a.term.entry.score.cmp(&b.term.entry.score))
        });

        // Map back to the original `LocatedTerm` struct for the final result.
        scored_terms.into_iter().map(|st| st.term).collect()
    }

    // /// Finds all possible dictionary terms for every substring in the input text.
    // ///
    // /// This function iterates through the text, and for each starting position, it
    // /// finds all matching dictionary terms and returns them with their position
    // /// and full entry data. It does NOT attempt to create a single segmented path,
    // /// but instead provides a comprehensive list of all possibilities.
    // ///
    // /// # Returns
    // /// A vector of `LocatedTerm`s, sorted by start position, then by longest match,
    // /// and finally by dictionary score. This provides a clean, ordered list of
    // /// all terms found in the text.
    // pub fn _parse_text_terms(&mut self, text: &str, options: &ProfileOptions) -> Vec<LocatedTerm> {
    //     const MODE: FindTermsMode = FindTermsMode::Simple;
    //     let details = FindTermsDetails {
    //         match_type: Some(TermSourceMatchType::Exact),
    //         deinflect: Some(true),
    //         primary_reading: None,
    //     };
    //     let find_terms_options =
    //         Backend::_get_translator_find_terms_options(MODE, &details, options);
    //
    //     let mut all_found_terms: Vec<LocatedTerm> = Vec::new();
    //
    //     // Iterate through each character's byte position as a potential starting point.
    //     for (start_index, _) in text.char_indices() {
    //         // *** FIX: Search the ENTIRE remaining slice of the string. ***
    //         // This gives find_terms full context to find the longest possible words.
    //         let search_slice = &text[start_index..];
    //
    //         if search_slice.is_empty() {
    //             continue;
    //         }
    //
    //         let find_result = self
    //             .translator
    //             .find_terms(MODE, search_slice, &find_terms_options);
    //
    //         // Iterate through every dictionary entry it found for this slice.
    //         for entry in find_result.dictionary_entries {
    //             if let Some(source) = entry.headwords.first().and_then(|hw| hw.sources.first()) {
    //                 let matched_text = &source.original_text;
    //                 // Ensure the match starts at the beginning of our search slice.
    //                 if !matched_text.is_empty() && search_slice.starts_with(matched_text) {
    //                     all_found_terms.push(LocatedTerm {
    //                         start: start_index,
    //                         length: matched_text.len(),
    //                         text: matched_text.clone(),
    //                         // Clone the entry to take ownership of the data.
    //                         entry: entry.clone(),
    //                     });
    //                 }
    //             }
    //         }
    //     }
    //
    //     // Sort the results into a "sane order":
    //     // 1. Group by starting position (ascending).
    //     // 2. Within each group, list longest matches first (descending length).
    //     // 3. For matches with the same start/length, use dictionary score (lower is better).
    //     all_found_terms.sort_unstable_by(|a, b| {
    //         a.start
    //             .cmp(&b.start)
    //             .then_with(|| b.length.cmp(&a.length))
    //             .then_with(|| a.entry.score.cmp(&b.entry.score))
    //     });
    //
    //     // Remove duplicates, which can occur if the same term (e.g., same term/reading)
    //     // is found in multiple dictionaries.
    //     all_found_terms.dedup_by(|a, b| {
    //         let headword_a = a.entry.headwords.first();
    //         let headword_b = b.entry.headwords.first();
    //
    //         a.start == b.start
    //             && a.text == b.text
    //             && headword_a.map(|h| (&h.term, &h.reading))
    //                 == headword_b.map(|h| (&h.term, &h.reading))
    //     });
    //
    //     all_found_terms
    // }
    //
    // /// Parses text by finding the longest possible dictionary term at each position.
    // ///
    // /// This function implements the "Maximal Match" (or "Longest Match") algorithm,
    // /// a common and effective heuristic for tokenizing unspaced languages like Japanese.
    // ///
    // /// # Parameters
    // /// * `text` - The input string to parse.
    // /// * `scan_length` - The maximum length of the substring to search at each step.
    // ///   This acts as a performance optimization to limit the search space. A value
    // ///   like 32 is usually sufficient, as Japanese words rarely exceed this length.
    // /// * `options` - Contextual options for parsing.
    // ///
    // /// # Returns
    // /// A `Result` containing a vector of `ParseTextLine`s, where each `ParseTextLine`
    // /// represents a found word or a series of ungrouped characters.
    // pub fn _text_parse_maximal_match(
    //     &mut self,
    //     text: &str,
    //     scan_length: usize,
    //     options: &ProfileOptions,
    // ) -> Vec<ParseTextLine> {
    //     const MODE: FindTermsMode = FindTermsMode::Simple;
    //     let details = FindTermsDetails {
    //         match_type: Some(TermSourceMatchType::Exact),
    //         deinflect: Some(true),
    //         primary_reading: None,
    //     };
    //     let find_terms_options =
    //         Backend::_get_translator_find_terms_options(MODE, &details, options);
    //
    //     let mut results: Vec<ParseTextLine> = Vec::new();
    //     let mut i = 0; // Current byte position in the text
    //     let text_len = text.len();
    //
    //     // Create a vector of character start indices to correctly handle multi-byte characters.
    //     let char_indices: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    //
    //     while i < text_len {
    //         // Define the slice to search. Using scan_length prevents looking too far ahead.
    //         let scan_end = (i + scan_length).min(text_len);
    //         let search_slice = &text[i..scan_end];
    //
    //         let find_result = self
    //             .translator
    //             .find_terms(MODE, search_slice, &find_terms_options);
    //
    //         // Find the best match. The criteria are:
    //         // 1. Longest matched text length (primary).
    //         // 2. Best dictionary score (tie-breaker). Your scores seem to be lower-is-better.
    //         let best_match = find_result
    //             .dictionary_entries
    //             .iter()
    //             .filter_map(|entry| {
    //                 // The actual matched text from the input is in the headword's source.
    //                 entry
    //                     .headwords
    //                     .first()
    //                     .and_then(|hw| hw.sources.first())
    //                     .map(|source| (entry, source.original_text.len()))
    //             })
    //             .filter(|(_, len)| *len > 0) // Ensure we only consider actual matches.
    //             .max_by(|(entry_a, len_a), (entry_b, len_b)| {
    //                 // Compare by length first. If lengths are equal, compare by score.
    //                 match len_a.cmp(len_b) {
    //                     Ordering::Equal => entry_a.score.cmp(&entry_b.score).reverse(), // Lower score is better
    //                     other => other,
    //                 }
    //             });
    //
    //         if let Some((entry, length)) = best_match {
    //             // A dictionary word was found, so we use it.
    //             let headword = entry.headwords.first().unwrap(); // Safe due to previous filters.
    //             let source_text = &text[i..i + length];
    //
    //             let text_segments = distribute_furigana_inflected(
    //                 headword.term.clone(),
    //                 headword.reading.clone(),
    //                 source_text.to_string(),
    //             );
    //             results.push(text_segments.into_iter().map(|fs| fs.into()).collect());
    //
    //             // Advance the cursor by the length of the matched word.
    //             i += length;
    //         } else {
    //             // No dictionary entry found at this position.
    //             // Treat the current character as an ungrouped segment and advance.
    //             let current_char = text[i..].chars().next().unwrap_or('\u{FFFD}');
    //             let char_len = current_char.len_utf8();
    //
    //             // Check if the previous segment was also ungrouped so we can merge them.
    //             let mut merged = false;
    //             if let Some(last_line) = results.last_mut() {
    //                 // Heuristic for "is ungrouped": it has one segment and no reading.
    //                 if last_line.len() == 1 && last_line[0].reading.is_empty() {
    //                     last_line[0].text.push(current_char);
    //                     merged = true;
    //                 }
    //             }
    //
    //             if !merged {
    //                 // Otherwise, create a new line for the ungrouped character.
    //                 let new_segment = ParseTextSegment {
    //                     text: current_char.to_string(),
    //                     reading: String::new(),
    //                 };
    //                 results.push(vec![new_segment]);
    //             }
    //
    //             // Advance the cursor by the length of the single character.
    //             i += char_len;
    //         }
    //     }
    //
    //     results
    // }

    /// Parses text by scanning for dictionary terms.
    ///
    /// # Parameters
    /// * `text` - The input string to parse.
    /// * `scan_length` - The length of the substring to scan at each step.
    /// * `options_context` - Contextual options for parsing.
    ///
    /// # Returns
    /// A `Result` containing a vector of `ParseTextLine`s, where each `ParseTextLine`
    /// is a vector of `TextSegment`s representing a word or a series of ungrouped characters.
    pub fn _text_parse_scanning(
        &mut self,
        text: &str,
        scan_length: usize,
        options: &ProfileOptions,
    ) -> Vec<ParseTextLine> {
        const MODE: FindTermsMode = FindTermsMode::Simple;
        let details = FindTermsDetails {
            match_type: Some(TermSourceMatchType::Exact),
            deinflect: Some(true),
            primary_reading: None,
        };
        let find_terms_options =
            Backend::_get_translator_find_terms_options(MODE, &details, options);

        let mut results: Vec<ParseTextLine> = Vec::new();
        // This flag tracks if the most recently processed segment was ungrouped (a plain character).
        // It replaces the JavaScript approach of holding a mutable reference, which is complex in Rust.
        let mut was_previous_segment_ungrouped = false;

        let text_bytes = text.as_bytes();
        let mut i = 0;
        while i < text_bytes.len() {
            // Find the character at the current byte position `i`.
            // We use `char_indices` to correctly handle multi-byte UTF-8 characters.
            let character = text[i..].chars().next().unwrap_or('\u{FFFD}'); // Fallback to replacement char

            // Define the substring to scan. Ensure it doesn't exceed the text length.
            let scan_end = (i + scan_length).min(text_bytes.len());
            let scan_slice = &text[i..scan_end];

            let find_result = self
                .translator
                .find_terms(MODE, scan_slice, &find_terms_options);

            let found_entry = find_result
                .dictionary_entries
                .first()
                .and_then(|e| e.headwords.first());

            if let (Some(headword), true) = (found_entry, find_result.original_text_length > 0) {
                // Additional condition from the original JS logic.
                let char_byte_len = character.len_utf8() as i128;
                if find_result.original_text_length != char_byte_len
                    || is_code_point_japanese(character as u32)
                {
                    was_previous_segment_ungrouped = false;

                    let source = &text[i..i + find_result.original_text_length as usize];
                    let text_segments = distribute_furigana_inflected(
                        headword.term.clone(),
                        headword.reading.clone(),
                        source.to_string(),
                    );
                    results.push(text_segments.into_iter().map(|fs| fs.into()).collect());

                    i += find_result.original_text_length as usize;
                    continue;
                }
            }

            // --- Else branch: No valid dictionary entry found ---
            if was_previous_segment_ungrouped {
                // If the previous segment was also a plain character, append to it.
                if let Some(last_line) = results.last_mut() {
                    if let Some(ungrouped_segment) = last_line.first_mut() {
                        ungrouped_segment.text.push(character);
                    }
                }
            } else {
                // Otherwise, create a new ungrouped segment.
                was_previous_segment_ungrouped = true;
                let new_segment = ParseTextSegment {
                    text: character.to_string(),
                    reading: String::new(),
                };
                results.push(vec![new_segment]);
            }
            i += character.len_utf8();
        }

        results
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

        let mut enabled_dictionary_map = Backend::_get_translator_enabled_dictionary_map(opts);
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

        let text_replacements = Backend::_get_translator_text_replacements(text_replacements_opts);
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

    fn _get_translator_enabled_dictionary_map(opts: &ProfileOptions) -> TermEnabledDictionaryMap {
        let mut enabled_dictionary_map: TermEnabledDictionaryMap = IndexMap::new();
        for dictionary in &opts.dictionaries {
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
            } = dictionary.clone();
            let new = FindTermDictionary {
                index: enabled_dictionary_map.len(),
                alias: alias.clone(),
                allow_secondary_searches,
                parts_of_speech_filter,
                use_deinflections,
            };
            enabled_dictionary_map.insert(name, new);
        }
        enabled_dictionary_map
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindTermsDetails {
    pub match_type: Option<FindTermsMatchType>,
    pub deinflect: Option<bool>,
    pub primary_reading: Option<String>,
}
impl Default for FindTermsDetails {
    fn default() -> Self {
        Self {
            match_type: Some(FindTermsMatchType::Exact),
            deinflect: Some(true),
            primary_reading: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParseTextSegment {
    text: String,
    reading: String,
}
impl From<FuriganaSegment> for ParseTextSegment {
    fn from(value: FuriganaSegment) -> Self {
        Self {
            text: value.text,
            reading: value.reading.unwrap_or_default(),
        }
    }
}
type ParseTextLine = Vec<ParseTextSegment>;

mod ycd_tests {
    use crate::{test_utils::TEST_PATHS, Yomichan};

    use super::{Backend, FindTermsDetails};

    #[test]
    fn ycd_find() {
        let mut ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        ycd.set_language("ja");
        let details = FindTermsDetails::default();
        let res = ycd.find_terms("お前", details);
        dbg!(res);
    }

    #[test]
    fn text_match() {
        let mut ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        ycd.set_language("ja");
        let res = ycd.parse_text("出来て", 20);
        //dbg!(res);
        let txt = std::fs::write(
            TEST_PATHS.tests_dir.join("output.json"),
            serde_json::to_vec_pretty(&res).unwrap(),
        );
    }
}
