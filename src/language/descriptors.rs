use super::{
    cjk_utils::is_code_point_in_ranges,
    language_d::{BidirectionalPreprocessorOptions, TextProcessor},
};

use unicode_segmentation::UnicodeSegmentation;

use std::collections::HashMap;

pub fn collect_graphemes(text: &str) -> Vec<&str> {
    text.graphemes(true).collect::<Vec<&str>>()
}

//type TextProcessorDescriptor<T> = HashMap<String, TextProcessor<T>>;

trait TextProcessorDescriptor {}

struct DefaultTextProcessorDescriptor;
impl TextProcessorDescriptor for DefaultTextProcessorDescriptor {}

trait LookupWorthy {
    fn is_lookup_worthy(&self, text: &str) -> bool;
}

trait ReadingNormalizer {
    fn normalize(&self, text: &str) -> String;
}

trait LanguageTransformDescriptor {
    fn transform(&self, text: &str) -> String;
}

struct LanguageDescriptor<
    TIso: AsRef<str>,
    TTextPreprocessorDescriptor: TextProcessorDescriptor = DefaultTextProcessorDescriptor,
    TTextPostprocessorDescriptor: TextProcessorDescriptor = DefaultTextProcessorDescriptor,
> {
    iso: TIso,
    iso639_3: String,
    name: String,
    example_text: String,
    reading_normalizer: Option<Box<dyn ReadingNormalizer>>,
    text_preprocessors: TTextPreprocessorDescriptor,
    text_postprocessors: TTextPostprocessorDescriptor,
    language_transforms: Option<Box<dyn LanguageTransformDescriptor>>,
}

struct CollapseEmphaticOptions {
    collapse_emphatic: bool,
    collapse_emphatic_full: bool,
}

// This is a mapping of the iso tag to all of the text processors for that language.
// Any new language should be added to this object.
// pub struct AllTextProcessors {
//     ja: JaTextProcessors,
// }
