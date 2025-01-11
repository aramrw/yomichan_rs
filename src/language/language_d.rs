use std::sync::Arc;

use crate::language::transformer_d::LanguageTransformDescriptor;

/// This is the following function type in yomitan:
/// export type TextProcessorFunction<T = unknown> = (str: string, setting: T) => string;
trait TextProcessable<T> {
    fn process(str: &str, options: Vec<T>) -> String;
}

// impl TextProcessable<bool> for TextProcessor<'_, bool> {
//     fn process(text: &str, setting: bool) -> String {
//         if setting {
//             let mut chars = text.chars();
//             if let Some(first_char) = chars.next() {
//                 format!("{}{}", first_char.to_uppercase(), chars.as_str())
//             } else {
//                 String::new()
//             }
//         } else {
//             text.to_string()
//         }
//     }
// }

/// Text `pre-` & `post-`processors are used during the translation process to
/// create alternate versions of the input text to search for.
///
/// This can be helpful when the input text don't exactly
/// match the term or expression found in the database.
///
/// When a language has multiple processors, the translator will generate
/// variants of the text by applying all combinations of the processors.
#[derive(Clone)]
pub struct TextProcessor<'a, O, S> {
    pub name: &'a str,
    pub description: &'a str,
    pub options: &'a [O],
    pub process: TextProcessorFP<S>,
}

pub type TextProcessorFP<T> = fn(&str, T) -> String;

/// Helper function to normalize .
pub type ReadingNormalizer = fn(&str) -> String;

#[derive(Clone)]
pub enum BidirectionalPreProcessorOptions {
    Off,
    Direct,
    Inverse,
}

pub type BidirectionalConversionPreProcessor<'a> =
    TextProcessor<'a, BidirectionalPreProcessorOptions, BidirectionalPreProcessorOptions>;

pub struct LanguageAndProcessors<'a, O, S> {
    pub iso: String,
    pub text_preprocessors: Option<Vec<TextProcessorWithId<'a, O, S>>>,
    pub text_postprocessors: Option<Vec<TextProcessorWithId<'a, O, S>>>,
}

pub struct LanguageAndReadingNormalizer {
    pub iso: String,
    pub reading_normalizer: ReadingNormalizer,
}

pub struct LanguageAndTransforms<'a> {
    pub iso: String,
    pub language_transforms: &'a LanguageTransformDescriptor<'a>,
}

pub struct TextProcessorWithId<'a, O, S> {
    pub id: String,
    pub text_processor: TextProcessor<'a, O, S>,
}

#[derive(Debug, Clone)]
pub struct LanguageSummary {
    pub name: String,
    pub iso: String,
    pub iso639_3: String,
    pub example_text: String,
}
