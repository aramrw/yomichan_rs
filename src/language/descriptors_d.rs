use super::{
    cjk_utils::is_code_point_in_ranges,
    ja::text_preprocessors::{
        ALPHABETIC_TO_HIRAGANA, ALPHANUMERIC_WIDTH_VARIANTS, CONVERT_HALF_WIDTH_CHARACTERS,
        NORMALIZE_COMBINING_CHARACTERS,
    },
    language_d::{
        BidirectionalConversionPreProcessor, BidirectionalPreProcessorOptions, ReadingNormalizer,
        TextProcessor,
    },
    transformer_d::LanguageTransformDescriptor,
};

use unicode_segmentation::UnicodeSegmentation;

use std::collections::HashMap;

pub fn collect_graphemes(text: &str) -> Vec<&str> {
    text.graphemes(true).collect::<Vec<&str>>()
}

type IsTextLookupWorthyFP = fn(text: &str) -> bool;

pub struct LanguageDescriptor<'a, Pre, Post> {
    pub iso: String,
    pub iso639_3: String,
    pub name: String,
    pub example_text: String,
    pub is_text_lookup_worthy: Option<IsTextLookupWorthyFP>,
    pub reading_normalizer: Option<ReadingNormalizer>,
    pub text_processors: PreAndPostProcessors<Pre, Post>,
    pub language_transforms: Option<&'a LanguageTransformDescriptor<'a>>,
}

type TextProcessorDescriptor<'a, T, F> = HashMap<String, TextProcessor<'a, T, F>>;

struct CapitalizationPreProcessors<'a, F> {
    capitalize_first_letter: TextProcessor<'a, bool, F>,
    decapitalize: TextProcessor<'a, bool, F>,
}

struct AlphabeticDiacriticsProcessor<'a, F> {
    remove_alphabetic_diacritics: TextProcessor<'a, bool, F>,
}

/// This is a mapping of the iso tag to all of the text processors for that language.
/// Any new language should be added to this struct.
pub struct AllTextProcessors<'a> {
    ja: PreAndPostProcessors<JapanesePreProcessors<'a>, ()>,
}

pub struct PreAndPostProcessors<Pre, Post> {
    pub pre: Pre,
    pub post: Option<Post>,
}

// Language Processor structs get created here
pub struct JapanesePreProcessors<'a> {
    pub convert_half_width_characters: TextProcessor<'a, bool, fn(&str, bool) -> String>,
    pub alphabetic_to_hiragana: TextProcessor<'a, bool, fn(&str, bool) -> String>,
    pub normalize_combining_characters: TextProcessor<'a, bool, fn(&str, bool) -> String>,
    pub alphanumeric_width_variants: BidirectionalConversionPreProcessor<'a>,
    pub convert_hiragana_to_katakana: BidirectionalConversionPreProcessor<'a>,
    pub collapse_emphatic_sequences: TextProcessor<'a, [bool; 2], fn(&str, &[bool; 2]) -> String>,
}
