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
pub struct TextProcessor<'a, T, F> {
    pub name: &'a str,
    pub description: &'a str,
    pub options: &'a [T],
    pub process: F,
}

/// Helper function to normalize .
pub type ReadingNormalizer = dyn Fn(&str) -> String;

pub enum BidirectionalPreprocessorOptions {
    Off,
    Direct,
    Inverse,
}

pub type BidirectionalConversionPreprocessor<'a> = TextProcessor<
    'a,
    BidirectionalPreprocessorOptions,
    fn(&str, BidirectionalPreprocessorOptions) -> String,
>;

pub struct LanguageAndProcessors<'a, T, F> {
    iso: String,
    text_preprocessors: Option<Vec<TextProcessorWithId<'a, T, F>>>,
    text_postprocessors: Option<Vec<TextProcessorWithId<'a, T, F>>>,
}

pub struct LanguageAndReadingNormalizer {
    iso: String,
    reading_normalizer: ReadingNormalizer,
}

pub struct LanguageAndTransforms<'a>
// where
//     F: Fn(&str, &str, &str) -> String,
{
    iso: String,
    language_transforms: LanguageTransformDescriptor<'a>,
}

pub struct TextProcessorWithId<'a, T, F> {
    id: String,
    text_processor: TextProcessor<'a, T, F>,
}

pub struct LanguageSummary {
    name: String,
    iso: String,
    iso639_3: String,
    example_text: String,
}
