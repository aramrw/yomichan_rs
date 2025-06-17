use std::{cmp, sync::Arc};

use crate::{
    backend::FindTermsDetails,
    database::dictionary_database::DictionaryDatabase,
    dictionary::{TermDictionaryEntry, TermSourceMatchType},
    settings::ProfileOptions,
    translator::{FindTermsMode, FindTermsResult, Translator},
    Yomichan,
};

impl<'a> Yomichan<'a> {
    fn search(&mut self, text: &str) -> Option<TermSearchResults> {
        let opts = &self.backend.options.get_current_profile().options;
        self.backend.text_scanner._search_internal(text, 0, opts)
    }
}

/// Represents the full sentence context for a search result.
///
/// * offset: The character offset of the original search text within the full sentence text.
#[derive(Debug, Clone)]
pub struct Sentence {
    pub text: String,
    pub offset: usize,
}

/// The final, structured result of a term search.
#[derive(Debug, Clone)]
pub struct TermSearchResults {
    pub dictionary_entries: Vec<TermDictionaryEntry>,
    pub sentence: Sentence,
}

/// A simplified text scanner to find dictionary terms and sentence context,
/// inspired by Yomitan's core logic.
pub struct TextScanner<'a> {
    /// A mutable reference to the core translator engine.
    translator: Translator<'a>,

    /// The maximum number of characters to scan for terms initially.
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
    pub fn new(db: Arc<DictionaryDatabase<'a>>) -> Self {
        TextScanner {
            translator: Translator::new(db),
            scan_len: 20,
            sentence_scan_extent: 50,
            sentence_terminate_at_newlines: true,
            // Default to exact matching
            match_type: TermSourceMatchType::Exact,
        }
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

        // 3. Extract the sentence context.
        let sentence = self.extract_sentence(
            full_text,
            start_position,
            find_result.original_text_length as usize,
        );

        // 4. Assemble and return the final result.
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
    use std::sync::{LazyLock, RwLock};

    use crate::{test_utils::TEST_PATHS, Yomichan};

    static YCD: LazyLock<RwLock<Yomichan>> = LazyLock::new(|| {
        let mut ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        // no need to set language, you do it in the test functions
        RwLock::new(ycd)
    });

    #[test]
    fn search() {
        let ycd = YCD.read().unwrap();
        //dbg!(res);
    }
}
