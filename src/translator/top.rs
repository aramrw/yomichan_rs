use crate::database::DictionaryService;
use crate::translator::{ReadingNormalizerMap, TagCache, TextProcessorMap};

// macro_rules! iter_type_to_iter_variant {
//     ($v:expr, $variant:path) => {
//         $v.into_iter().map(|item| $variant(item))
//     };
// }

// macro_rules! iter_variant_to_iter_type {
//     ($v:expr, $variant:path) => {
//         $v.into_iter()
//             .filter_map(|item| {
//                 if let $variant(inner) = item {
//                     Some(inner)
//                 } else {
//                     None
//                 }
//             })
//             .collect()
//     };
// }

use deinflector::multi_language_transformer::MultiLanguageTransformer;
use fancy_regex::Regex;
use indexmap::IndexMap;
use std::sync::Arc;

use parking_lot::RwLock;

/// class which finds term and kanji dictionary entries for text.
pub struct Translator {
    pub db: Arc<dyn DictionaryService>,
    pub mlt: MultiLanguageTransformer,
    pub tag_cache: RwLock<IndexMap<String, TagCache>>,
    pub number_regex: &'static Regex,
    pub text_processors: TextProcessorMap,
    pub reading_normalizers: ReadingNormalizerMap,
}
