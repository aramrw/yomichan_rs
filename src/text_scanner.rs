use std::{cmp, collections::HashSet, sync::Arc};

use anki_direct::{
    decks::DeckConfig,
    error::AnkiResult,
    model::FullModelDetails,
    notes::{Note, NoteBuilder},
    ReqwestClient,
};
use indexmap::IndexMap;
use parking_lot::{ArcRwLockReadGuard, RawRwLock};

use crate::{
    backend::FindTermsDetails,
    database::dictionary_database::DictionaryDatabase,
    dictionary::{TermDictionaryEntry, TermSource, TermSourceMatchType},
    settings::{AnkiOptions, DecksMap, ProfileOptions, YomichanProfile},
    translator::{FindTermsMode, FindTermsResult, Translator},
    Ptr, Yomichan,
};

impl Yomichan<'_> {
    pub fn search(&mut self, text: &str) -> Option<Vec<TermSearchResultsSegment>> {
        let profile = self.backend.get_current_profile().ok()?;
        let profile = profile.read();
        let opts = profile.options();
        let res = self.backend.text_scanner.search_sentence(text, opts)?;
        Some(SentenceParser::parse(res))
    }
}

/// Represents one chunk of a parsed sentence, ready for display.
///
/// A segment can either be a known term (with dictionary entries) or
#[derive(Debug, Clone)]
pub struct TermSearchResultsSegment {
    pub text: String,
    pub results: Option<Arc<TermSearchResults>>,
}

struct SentenceParser {}
impl SentenceParser {
    /// Parses the flat list of dictionary entries from `TermSearchResults`
    /// into a structured `Vec<TermSearchResultsSegment>` using a "longest match" algorithm.
    ///
    /// This is the "frontend" logic that constructs the clickable sentence view.
    ///
    /// # Arguments
    /// * `results` - The output from `TextScanner::_search_internal`.
    ///
    /// # Returns
    /// A vector of `TermSearchResultsSegment`s that represent the sentence, broken down
    pub fn parse(results: TermSearchResults) -> Vec<TermSearchResultsSegment> {
        if results.dictionary_entries.is_empty() {
            return vec![TermSearchResultsSegment {
                text: results.sentence.text,
                results: None,
            }];
        }

        // --- Step 1: Group dictionary entries by their original source text ---
        let mut grouped_by_source = IndexMap::<String, Vec<TermDictionaryEntry>>::new();
        for entry in results.dictionary_entries {
            if let Some(source) = Self::find_primary_source(&entry) {
                grouped_by_source
                    .entry(source.original_text.clone())
                    .or_default()
                    .push(entry);
            }
        }

        // --- Step 2: Segment the sentence using the "longest match" algorithm ---
        let sentence_text = &results.sentence.text;
        let mut parsed_sentence: Vec<TermSearchResultsSegment> = Vec::new();
        let mut current_pos = 0;

        let mut match_keys: Vec<_> = grouped_by_source.keys().cloned().collect();
        match_keys.sort_by_key(|b| std::cmp::Reverse(b.len()));

        while current_pos < sentence_text.len() {
            let remaining_text = &sentence_text[current_pos..];

            let best_match = match_keys
                .iter()
                .find(|key| remaining_text.starts_with(*key));

            if let Some(found_key) = best_match {
                let entries_for_key = grouped_by_source.get(found_key).unwrap();

                // --- CORRECT FIX LOCATION ---
                // Deduplicate the entries for this specific segment.
                let mut seen_entries = HashSet::new();
                let unique_entries: Vec<TermDictionaryEntry> = entries_for_key
                    .iter()
                    .filter(|entry| {
                        if let (Some(headword), Some(definition)) =
                            (entry.headwords.first(), entry.definitions.first())
                        {
                            let key = (headword.term.clone(), definition.id.clone());
                            // `insert` returns true if the key was new. We keep the entry only if it's new.
                            seen_entries.insert(key)
                        } else {
                            // Don't keep malformed entries
                            false
                        }
                    })
                    .cloned()
                    .collect();
                // --- END FIX ---

                let segment_results = TermSearchResults {
                    // Use the clean, unique list of entries
                    dictionary_entries: unique_entries,
                    sentence: results.sentence.clone(),
                };

                parsed_sentence.push(TermSearchResultsSegment {
                    text: found_key.clone(),
                    results: Some(Arc::new(segment_results)),
                });

                current_pos += found_key.len();
            } else {
                let char_str = remaining_text.chars().next().unwrap().to_string();
                parsed_sentence.push(TermSearchResultsSegment {
                    text: char_str.clone(),
                    results: None,
                });
                current_pos += char_str.len();
            }
        }

        parsed_sentence
    }

    /// Helper to find the primary source text for a dictionary entry.
    /// Yomitan's data structure is complex, so we navigate it to find the
    /// `original_text` that this entry was found from.
    fn find_primary_source(entry: &TermDictionaryEntry) -> Option<&TermSource> {
        entry
            .headwords
            .iter()
            .find_map(|hw| hw.sources.iter().find(|s| s.is_primary))
    }
}

/// Represents the full sentence context for a search result.
///
/// * text the full unchanged string looked up
/// * offset: The character offset of the original search text within the full sentence text.
#[derive(Debug, Clone)]
pub struct Sentence {
    pub text: String,
    pub offset: usize,
}

/// The final, structured result of a term search.
///
/// # Parsing Examples
/// ```
/// todo!()
/// ```
/// See [TermDictionaryEntry] for all fields
#[derive(Debug, Clone)]
pub struct TermSearchResults {
    pub dictionary_entries: Vec<TermDictionaryEntry>,
    pub sentence: Sentence,
}

#[derive(thiserror::Error, Debug)]
pub enum BuildNoteError {
    #[error("current profile has no anki deck selected")]
    NoDeckSelected,
}
impl TermSearchResults {
    //Returns a Note future
    // pub async fn build_note(
    //     &self,
    //     model: &FullModelDetails,
    //     deck_name: String,
    //     anki_opts: ArcRwLockReadGuard<RawRwLock, AnkiOptions>,
    //     tags: Vec<String>,
    //     client: Option<&ReqwestClient>,
    // ) -> AnkiResult<Note> {
    //     let fields = &model.fields;
    //     NoteBuilder::default()
    //         .model_name(model.name.clone())
    //         .deck_name(deck_name)
    //         .field(field_name, value)
    //         .tags(tags)
    //         .build(client)
    //         .await
    // }
}

/// Scans text to find dictionary terms and sentence context.
/// Inspired by [YomitanJS's TextScanner](https://github.com/yomidevs/yomitan/blob/2fc09f9b2d2f130ea18ae117be15f5683bc13440/ext/js/language/text-scanner.js#L33)
pub struct TextScanner<'a> {
    /// A mutable reference to the core translator engine.
    translator: Translator<'a>,

    /// The max number of characters to scan (initially).
    /// Corresponds to `_scanLength` in JS.
    scan_len: usize,

    /// How far back and forward to scan for sentence boundaries.
    /// Corresponds to `_sentenceScanExtent` in JS.
    sentence_scan_extent: usize,

    /// Whether a newline character should terminate a sentence.
    /// Corresponds to `_sentenceTerminateAtNewlines` in JS.
    sentence_terminate_at_newlines: bool,

    /// The match type to use for finding terms (e.g., exact or prefix).
    /// Corresponds to `_matchTypePrefix` in JS.
    match_type: TermSourceMatchType,
    // Note: The terminator/quote maps from JS can be added here later if needed.
    // For now, we can start with simpler sentence extraction logic.
}

impl<'a> TextScanner<'a> {
    /// Creates a new TextScanner with default or provided configuration.
    pub fn new(db: &Arc<DictionaryDatabase<'a>>) -> Self {
        TextScanner {
            translator: Translator::new(db.clone()),
            scan_len: 20,
            sentence_scan_extent: 50,
            sentence_terminate_at_newlines: true,
            // Default to exact matching
            match_type: TermSourceMatchType::Exact,
        }
    }

    /// Scans an entire sentence to find all possible dictionary terms within it.
    ///
    /// This method implements the core logic for the full-sentence analysis view.
    /// It uses language-specific strategies to find all potential terms.
    ///
    /// - For non-spaced languages (ja, zh, ko), it performs a "sliding window"
    ///   search from every character position.
    /// - For spaced languages, it optimizes by only starting a search from the
    ///   beginning of each word, while still preserving the full sentence context
    ///   for the parser.
    ///
    /// The final, flat list of all found dictionary entries is then returned,
    /// ready to be consumed by the `SentenceParser`.
    pub fn search_sentence(
        &mut self,
        sentence_text: &str,
        options: &ProfileOptions,
    ) -> Option<TermSearchResults> {
        let mut all_entries: Vec<TermDictionaryEntry> = Vec::new();

        // A helper closure to avoid duplicating the API call logic.
        let mut find_and_extend = |text_slice: &str| {
            if let Some(find_result) = self.find_term_dictionary_entries(text_slice, options) {
                all_entries.extend(find_result.dictionary_entries);
            }
        };

        // --- Core Logic Change ---
        // Use a language-specific scanning strategy.
        match options.general.language.as_str() {
            // For non-spaced languages, a search must be started from every position.
            "ja" | "zh" | "ko" => {
                for (i, _) in sentence_text.char_indices() {
                    find_and_extend(&sentence_text[i..]);
                }
            }

            // For spaced languages, we can optimize by only searching from the start of words.
            // This preserves the original string slicing, which is vital for the SentenceParser.
            _ => {
                let mut last_char_was_whitespace = true;
                for (i, c) in sentence_text.char_indices() {
                    let is_whitespace = c.is_whitespace();
                    if !is_whitespace && last_char_was_whitespace {
                        // This character is the start of a new word.
                        // We search the rest of the string from this point.
                        find_and_extend(&sentence_text[i..]);
                    }
                    last_char_was_whitespace = is_whitespace;
                }
            }
        }

        // If no terms were found anywhere in the sentence, return None.
        if all_entries.is_empty() {
            return None;
        }

        // The input text IS the sentence. Construct the final result.
        // The `SentenceParser` will use this complete list to build the segmented view.
        Some(TermSearchResults {
            dictionary_entries: all_entries,
            sentence: Sentence {
                text: sentence_text.to_string(),
                offset: 0,
            },
        })
    }

    // --- Public API Method ---

    /// The main entry point for scanning text.
    /// Takes a full text block and a starting character position.
    ///
    /// # Arguments
    /// * `full_text` - The entire string to be scanned (e.g., a paragraph).
    /// * `start_position` - The character index in `full_text` where the scan should begin.
    /// * `options` - Search options, see [ProfileOptions].
    ///
    /// # Returns
    /// An option [TermSearchResults] containing the sorted dictionary entries and sentence context.
    pub fn _search_internal(
        &mut self,
        full_text: &str,
        start_position: usize,
        options: &ProfileOptions,
        search_entire_text: bool,
    ) -> Option<TermSearchResults> {
        // This method will orchestrate the calls to the private helpers below.
        // It's the public "do everything" function.

        // 1. Get the initial search text.
        let search_text_ref = self.get_text_source_content(full_text, start_position);
        if search_text_ref.is_empty() {
            return None;
        }
        let search_text = search_text_ref.to_string();
        // 2. Find the terms using the translator.
        let find_result = self.find_term_dictionary_entries(&search_text, options)?;

        if search_entire_text {
            return self.search_sentence(full_text, options);
        }

        let sentence = self.extract_sentence(
            full_text,
            start_position,
            find_result.original_text_length as usize,
        );
        Some(TermSearchResults {
            dictionary_entries: find_result.dictionary_entries,
            sentence,
        })
    }

    /// Gets the initial chunk of text to be searched.
    fn get_text_source_content<'b>(&'a self, full_text: &'b str, start_position: usize) -> &'b str {
        // you cannot just index into the string with "start_position",
        // as that's a character index, not a byte index.
        // `char_indices()` gives an iterator of `(byte_index, char)`.

        // find the starting byte index that corresponds to the `start_position` character index.
        // skip to the desired character and then take its byte index.
        let start_byte = match full_text.char_indices().nth(start_position) {
            Some((byte_index, _)) => byte_index,
            // If start_position is out of bounds, there's no text to scan. Return an empty slice.
            None => return "",
        };

        // now, find the ending byte index. we start iterating from our `start_position`
        // and go forward `self.scan_len` characters.
        let end_byte = match full_text
            .char_indices()
            .skip(start_position)
            .nth(self.scan_len)
        {
            // If we find the character at that position, its byte index is our end boundary.
            Some((byte_index, _)) => byte_index,
            // If `nth()` returns `None`, it means the scan length goes past the end
            // of the string. In that case, the end boundary is simply the total length of the string.
            None => full_text.len(),
        };

        // Safely slice the string using the calculated byte indices.
        &full_text[start_byte..end_byte]
    }

    /// Calls the core translator to find dictionary entries.
    fn find_term_dictionary_entries(
        &mut self,
        search_text: &str,
        options: &ProfileOptions,
    ) -> Option<FindTermsResult> {
        let mut details = FindTermsDetails::default();

        // 2. Conditionally set the `match_type`, just like the JS code.
        //    We use the `match_type` from our scanner's configuration.
        //    `_get_translator_find_terms_options` will handle the case where it's None.
        if self.match_type == TermSourceMatchType::Prefix {
            details.match_type = Some(TermSourceMatchType::Prefix);
        }
        // Note: If more scanner-specific overrides are needed in the future (like deinflection toggles),
        // you would set them on `details` here.

        // 3. use `FindTermsMode::Group` as a sensible default for this kind of scanning.
        //    This is because most users would want frequencies and pronunciations to be included.
        const MODE: FindTermsMode = FindTermsMode::Group;

        let find_terms_options =
            Translator::_get_translator_find_terms_options(MODE, &details, options);

        let find_result = self
            .translator
            .find_terms(MODE, search_text, &find_terms_options);

        // 6. Check if any dictionary entries were found and return an Option.
        if find_result.dictionary_entries.is_empty() {
            None
        } else {
            Some(find_result)
        }
    }

    /// Extracts the full sentence surrounding the found term.
    /// Mirrors `extractSentence` in yomitan.
    ///
    /// # Arguments
    /// * start_position: The character index where the scan started.
    /// * parsed_length: The number of characters that were parsed.
    fn extract_sentence(
        &self,
        full_text: &str,
        start_position: usize,
        parsed_length: usize,
    ) -> Sentence {
        // establish a wide "Context Window" around the parsed text
        // This mirrors the JS `setStartOffset` and `setEndOffset` logic.

        // Calculate the start of our context window by scanning backwards.
        let context_start_char = start_position.saturating_sub(self.sentence_scan_extent);

        // Calculate the end of the anchor text (the text that was actually parsed).
        let parsed_end_char = start_position + parsed_length;

        // Calculate the end of our context window by scanning forwards.
        let context_end_char = cmp::min(
            full_text.chars().count(),
            parsed_end_char + self.sentence_scan_extent,
        );

        // Scan backwards from the anchor to find the true sentence start
        // We start looking for a terminator from the character just before our anchor.
        let mut sentence_start_char = start_position;
        // We create an iterator for the characters *before* the anchor, within our context window.
        for (i, c) in full_text
            .char_indices()
            .rev()
            .skip(full_text.chars().count() - start_position)
        {
            // Stop if we go past our context window's boundary.
            if i < context_start_char {
                break;
            }

            // Check for terminators. In JS, this is `terminatorMap.get(c)`.
            if ".!?。？！\n".contains(c) && self.sentence_terminate_at_newlines {
                // The sentence starts *after* the terminator.
                // We use the character index `i` and add 1 to get the start.
                sentence_start_char = full_text[..i].chars().count() + 1;
                break;
            }
        }
        // If no terminator was found, the sentence start is the start of our context window.
        sentence_start_char = cmp::max(context_start_char, sentence_start_char);

        // --- Phase 3: Scan forwards from the anchor to find the true sentence end ---

        let mut sentence_end_char = parsed_end_char;
        // Create an iterator for the characters *after* the anchor.
        for (i, c) in full_text.char_indices().skip(parsed_end_char) {
            // Stop if we go past our context window's boundary.
            if i >= context_end_char {
                break;
            }

            if ".!?。？！\n".contains(c) && self.sentence_terminate_at_newlines {
                // The sentence ends *at* the terminator (inclusive).
                sentence_end_char = full_text[..i].chars().count() + 1;
                break;
            }
        }
        // If no terminator was found, the sentence end is the end of our context window.
        sentence_end_char = cmp::min(context_end_char, sentence_end_char);

        // --- Phase 4: Create the sentence slice and trim whitespace (JS `_isWhitespace` loops) ---

        // Get the byte indices for our character-based start and end.
        let sentence_start_byte = full_text
            .char_indices()
            .nth(sentence_start_char)
            .map_or(0, |(b, _)| b);
        let sentence_end_byte = full_text
            .char_indices()
            .nth(sentence_end_char)
            .map_or(full_text.len(), |(b, _)| b);

        let sentence_slice = &full_text[sentence_start_byte..sentence_end_byte];
        let trimmed_sentence = sentence_slice.trim();

        // --- Phase 5: Calculate the final offset ---
        // The offset is the number of characters from the start of the *trimmed* sentence
        // to the start of our original search position.

        // First, find how many characters were trimmed from the start.
        let leading_whitespace_len = sentence_slice.len() - sentence_slice.trim_start().len();
        let leading_whitespace_chars = sentence_slice[..leading_whitespace_len].chars().count();

        // The offset is the original start position minus the start of the (untrimmed) sentence,
        // adjusted for any whitespace we trimmed from the front.
        let offset = start_position
            .saturating_sub(sentence_start_char)
            .saturating_sub(leading_whitespace_chars);

        // --- Phase 6: Assemble and return the final Sentence struct ---
        Sentence {
            text: trimmed_sentence.to_string(),
            offset,
        }
    }
}

#[cfg(test)]
mod textscanner {
    use crate::{
        structured_content::TermGlossary,
        test_utils::{self, YCD},
        Yomichan,
    };
    use std::{
        fs::OpenOptions,
        io::Write,
        sync::{LazyLock, RwLock},
    };

    #[test]
    fn search_dbg() {
        let mut ycd = YCD.write().unwrap();
        ycd.set_language("ja");
        let res = ycd.search("晩餐会をやっているみたいですか");
        let Some(res) = res else {
            panic!("search test failed");
        };
        dbg!(res);
    }

    #[test]
    fn search() {
        let mut ycd = YCD.write().unwrap();
        ycd.set_language("ja");
        let res = ycd.search("晩餐");
        let Some(res) = res else {
            panic!("search test failed");
        };
        for segment in &res {
            let Some(results) = &segment.results else {
                continue;
            };
            let entries = &results.dictionary_entries;
            for entry in entries {
                let defs = &entry.definitions;
                for def in defs {
                    let gloss = def.entries.clone();
                    for content in gloss.iter() {
                        let path = test_utils::TEST_PATHS
                            .tests_dir
                            .join("search")
                            .with_extension("json");
                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(path)
                            .unwrap();
                        file.write_all(content.plain_text.as_bytes()).unwrap();
                    }
                }
            }
        }
    }
}
