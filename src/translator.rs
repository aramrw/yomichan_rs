use std::collections::HashMap;

use icu::{
    collator::{options::CollatorOptions, Collator, CollatorBorrowed},
    locale::locale,
};
use language_transformer::{
    descriptors::{PreAndPostProcessors, PreAndPostProcessorsWithId},
    language_d::ReadingNormalizer,
    multi_language_transformer::MultiLanguageTransformer,
};
use native_db::*;
use native_model::{native_model, Model};
use regex::Regex;

use crate::{
    database::dictionary_database::DictionaryDatabaseTag,
    dictionary::{Tag, TermDictionaryEntry},
};

type TagCache = HashMap<&'static str, Option<DictionaryDatabaseTag>>;

struct TagTargetItem {
    query: &'static str,
    dictionary: String,
    tag_name: String,
    cache: Option<TagCache>,
    database_tag: DictionaryDatabaseTag,
    targets: Vec<Vec<Tag>>,
}

struct SequenceQuery {
    query: u128,
    dictionary: String,
}

enum FindTermsMode {
    Simple,
    Group,
    Merge,
    Split,
}

struct TermReadingItem {
    term: String,
    reading: Option<String>,
}

struct FindTermResult {
    dictionary_entries: Vec<TermDictionaryEntry>,
    original_text_length: u64,
}

type TextProcessorMap = HashMap<&'static str, PreAndPostProcessorsWithId>;
type ReadingNormalizerMap = HashMap<&'static str, ReadingNormalizer>;

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
            mlt: MultiLanguageTransformer::new(),
            tag_cache: HashMap::new(),
            string_comparer: Collator::try_new(locale!("en-US").into(), CollatorOptions::default())
                .unwrap(),
            number_regex: Regex::new(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?").unwrap(),
            text_processors: HashMap::new(),
            reading_normalizers: HashMap::new(),
        }
    }

    fn prepare() {
        let all_language_text_processors = getlal
    }
}
