use crate::dictionary::{DictionaryTag, TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    DictionaryDataTag, MetaDataMatchType, TermGlossary, TermGlossaryContent, TermMeta,
    TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPitch,
    TermMetaPitchData,
};
use crate::test_utils::TEST_PATHS;
use crate::translator::TagTargetItem;
use serde_with::{serde_as, NoneAsEmptyString};

use crate::database::dictionary_importer::{DictionarySummary, TermMetaBank};
// KANA_MAP is unused, consider removing if not used elsewhere in this module or submodules
// use crate::dictionary_data::KANA_MAP;
use crate::errors::{DBError, DictionaryFileError, ImportError};
use crate::settings::{DictionaryOptions, Options, Profile};
// Yomichan is unused, consider removing if not used elsewhere in this module or submodules
// use crate::Yomichan;

//use lindera::{LinderaError, Token, Tokenizer};

use db_type::{KeyOptions, ToKeyDefinition};
use indexmap::{IndexMap, IndexSet};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use native_model::{native_model, Model as NativeModelTrait}; // Renamed to avoid conflict if Model is used for DB Model

// rayon::collections::hash_set is unused
// use rayon::collections::hash_set;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer as JsonDeserializer;

// RTransaction is unused directly in this snippet, but likely used by self.db.r_transaction()
// use transaction::RTransaction;
//use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use uuid::Uuid;

// std::cell::LazyCell is unused
// use std::cell::LazyCell;
// std::error::Error is unused directly
// use std::error::Error;
use std::ffi::OsString;
// std::fmt::{Debug, Display} are unused directly, but derived often
// use std::fmt::{Debug, Display};
// std::hash::Hash is unused directly, but derived often
// use std::hash::Hash;
use std::io::BufReader;
use std::path::{Path, PathBuf};
// std::sync::{Arc, LazyLock} - LazyLock is used, Arc is not
use std::sync::LazyLock;
use std::{fs, marker}; // marker is unused

// Helper macro for creating enum variants like NativeDbQueryInfo::Exact(value)
// Exporting in case it's useful in other modules of this crate.
// If only used in this file, #[macro_export] can be omitted.
#[macro_export]
macro_rules! to_variant {
    ($value:expr, $variant_constructor_path:path) => {
        $variant_constructor_path($value)
    };
}

impl DictionarySet for IndexSet<String> {
    fn has(&self, value: &str) -> bool {
        self.contains(value)
    }
}
impl DictionarySet for &IndexSet<String> {
    fn has(&self, value: &str) -> bool {
        self.contains(value)
    }
}
impl<V: Send + Sync> DictionarySet for IndexMap<String, V> {
    fn has(&self, value: &str) -> bool {
        self.contains_key(value)
    }
}
impl<V: Send + Sync> DictionarySet for &IndexMap<String, V> {
    fn has(&self, value: &str) -> bool {
        self.contains_key(value)
    }
}

pub static DB_MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<Options>().unwrap();
    models.define::<DictionarySummary>().unwrap();
    models.define::<DatabaseTermEntry>().unwrap();
    /// in js, freq, pitch, and phonetic are grouped under an enum
    /// native_model doesn't support this you can only have a single primary key
    /// so we add all 3 types
    models.define::<DatabaseMetaFrequency>().unwrap();
    models.define::<DatabaseMetaPitch>().unwrap();
    models.define::<DatabaseMetaPhonetic>().unwrap();
    models.define::<DatabaseKanjiEntry>().unwrap();
    models.define::<DatabaseKanjiMeta>().unwrap();
    models.define::<DatabaseTag>().unwrap();
    /// serialization is not implemented for this yet
    /// native_db doesn't like generics for the model struct
    /// until then don't serialize
    //models.define::<MediaDataArrayBufferContent>().unwrap();
    models
});

pub type MediaDataArrayBufferContent = MediaDataBase<Vec<u8>>;
pub type MediaDataStringContent = MediaDataBase<String>;

// #[native_db]
// #[native_model(id = 11, version = 1)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaDataBase<TContentType: Serialize> {
    //#[primary_key]
    dictionary: String,
    //#[secondary_key]
    path: String,
    //#[secondary_key]
    media_type: String,
    width: u16,
    height: u16,
    content: TContentType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MediaType {
    ArrayBuffer(Vec<u8>),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Media<T = MediaType> {
    index: usize,
    data: T,
}

/// Represents a single term metadata entry found by find_term_meta_bulk.
/// This structure matches the output of the JavaScript _createTermMeta function.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMeta {
    /// Index of the original query term in the input term_list_input.
    pub index: usize,
    /// The term expression. (Corresponds to JS row.expression, named 'term' in JS output)
    //#[primary_key]
    pub term: String,
    /// The type of metadata (e.g., Freq, Pitch, Ipa). (Corresponds to JS row.mode)
    //#[secondary_key]
    pub mode: TermMetaModeType,
    /// The actual metadata content. (Corresponds to JS row.data)
    pub data: MetaDataMatchType,
    /// The name of the dictionary this metadata belongs to.
    //#[secondary_key]
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct DatabaseTermEntry {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub expression: String,
    #[secondary_key]
    pub reading: String,
    #[secondary_key]
    pub expression_reverse: String,
    #[secondary_key]
    pub reading_reverse: String,
    pub definition_tags: Option<String>,
    /// Legacy alias for the `definitionTags` field.
    pub tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub glossary: Vec<TermGlossary>,
    #[secondary_key]
    pub sequence: Option<i128>,
    pub term_tags: Option<String>,
    pub dictionary: String,
    pub file_path: OsString,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermEntry {
    pub id: String,
    pub index: usize,
    pub match_type: TermSourceMatchType,
    pub match_source: TermSourceMatchSource,
    pub term: String,
    pub reading: String,
    pub definition_tags: Vec<String>,
    pub term_tags: Vec<String>,
    pub rules: Vec<String>,
    pub definitions: Vec<TermGlossary>,
    pub score: i128,
    pub dictionary: String,
    pub sequence: i128,
}

impl DatabaseTermEntry {
    pub fn into_term_generic(
        self,
        match_type: &mut TermSourceMatchType,
        data: FindMulitBulkData,
    ) -> TermEntry {
        let match_source_is_term = data.index_index == 0; // In JS, 0 is expression, 1 is reading
        let match_source = match match_source_is_term {
            true => TermSourceMatchSource::Term,
            false => TermSourceMatchSource::Reading,
        };
        let found = match match_source {
            TermSourceMatchSource::Term => self.expression == data.item,
            TermSourceMatchSource::Reading => self.reading == data.item,
            _ => unreachable!(
                "DictionaryDatabase::into_term_generic does not expect the match_to be a sequence."
            ),
        };
        if found {
            *match_type = TermSourceMatchType::Exact;
        }
        self.into_term_entry_specific(match_source, *match_type, data.item_index)
    }
    pub fn into_term_entry_specific(
        self,
        match_source: TermSourceMatchSource,
        match_type: TermSourceMatchType,
        index: usize,
    ) -> TermEntry {
        let DatabaseTermEntry {
            id,
            expression,
            reading,
            expression_reverse: _expression_reverse, // Mark unused if not used in this function
            reading_reverse: _reading_reverse,       // Mark unused
            definition_tags,
            tags: _tags, // Mark unused (or use if it's the fallback for definition_tags)
            rules,
            score,
            glossary,
            sequence,
            term_tags,
            dictionary,
            file_path: _file_path, // Mark unused
        } = self;
        TermEntry {
            id,
            index,
            match_type,
            match_source,
            term: expression,
            reading,
            definition_tags: split_optional_string_field(definition_tags), // Consider fallback to _tags
            term_tags: split_optional_string_field(term_tags),
            rules: split_optional_string_field(Some(rules)),
            definitions: glossary,
            score,
            dictionary,
            sequence: sequence.unwrap_or(-1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[native_model(id = 9, version = 1)]
#[native_db]
pub struct DatabaseTag {
    /// id field doesn't exist in JS
    /// need it because primary keys must be unique
    //#[serde(skip_deserializing, default)]
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub name: String,
    #[secondary_key]
    pub category: String,
    pub order: u64,
    pub notes: String,
    pub score: i128,
    /// dictionary gets added afterwards
    /// it doesn't exist in any yomitan dictionary
    //#[serde(skip_deserializing, default)]
    #[secondary_key]
    pub dictionary: String,
}

/*************** Database Term Meta ***************/

pub trait DBMetaType {
    fn mode(&self) -> &TermMetaModeType;
    fn expression(&self) -> &str;
}

// /// A custom `Yomichan_rs`-unique, generic Database Meta model.
// ///
// /// May contain `any` or `all` of the values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DatabaseMetaMatchType {
    Frequency(DatabaseMetaFrequency),
    Pitch(DatabaseMetaPitch),
    Phonetic(DatabaseMetaPhonetic),
}

impl DatabaseMetaMatchType {
    pub fn convert_kanji_meta_file(
        outpath: PathBuf,
        dict_name: String,
    ) -> Result<Vec<DatabaseMetaFrequency>, DictionaryFileError> {
        let file = fs::File::open(&outpath).map_err(|reason| DictionaryFileError::FailedOpen {
            outpath: outpath.clone(),
            reason: reason.to_string(),
        })?;
        let reader = BufReader::new(file);

        // Kanji metas are only frequencies
        let mut stream =
            JsonDeserializer::from_reader(reader).into_iter::<Vec<DatabaseMetaFrequency>>();

        let mut entries = match stream.next() {
            Some(Ok(entries)) => entries,
            Some(Err(reason)) => {
                return Err(crate::errors::DictionaryFileError::File {
                    outpath,
                    reason: reason.to_string(),
                })
            }
            None => return Err(DictionaryFileError::Empty(outpath)),
        };
        entries.iter_mut().for_each(|entry| {
            entry.id = Uuid::now_v7().to_string();
            entry.dictionary = dict_name.clone();
        });
        Ok(entries)
    }

    pub fn convert_term_meta_file(
        outpath: PathBuf,
        dict_name: String,
    ) -> Result<Vec<DatabaseMetaMatchType>, DictionaryFileError> {
        let file = fs::File::open(&outpath).map_err(|reason| DictionaryFileError::FailedOpen {
            outpath: outpath.clone(),
            reason: reason.to_string(),
        })?;
        let reader = BufReader::new(file);

        let mut stream = JsonDeserializer::from_reader(reader).into_iter::<Vec<TermMeta>>();
        let mut entries = match stream.next() {
            Some(Ok(entries)) => entries,
            Some(Err(reason)) => {
                return Err(crate::errors::DictionaryFileError::File {
                    outpath,
                    reason: reason.to_string(),
                })
            }
            None => return Err(DictionaryFileError::Empty(outpath)),
        };

        let term_metas: Vec<DatabaseMetaMatchType> = entries
            // entries is TermMetaBank which is Vec<TermMetaData>
            .into_iter()
            .map(|entry| {
                let id = Uuid::now_v7().to_string();
                let TermMeta {
                    expression,
                    mode,
                    data,
                } = entry;

                match data {
                    MetaDataMatchType::Frequency(data) => {
                        DatabaseMetaMatchType::Frequency(DatabaseMetaFrequency {
                            id,
                            freq_expression: expression,
                            mode: TermMetaModeType::Freq,
                            data,
                            dictionary: dict_name.clone(),
                        })
                    }
                    MetaDataMatchType::Pitch(data) => {
                        DatabaseMetaMatchType::Pitch(DatabaseMetaPitch {
                            id,
                            pitch_expression: expression,
                            mode: TermMetaModeType::Pitch,
                            data,
                            dictionary: dict_name.clone(),
                        })
                    }
                    MetaDataMatchType::Phonetic(data) => {
                        DatabaseMetaMatchType::Phonetic(DatabaseMetaPhonetic {
                            id,
                            phonetic_expression: expression,
                            mode: TermMetaModeType::Ipa,
                            data,
                            dictionary: dict_name.clone(),
                        })
                    }
                }
            })
            .collect();
        Ok(term_metas)
    }
}

/// Used to store the frequency metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 3, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db]
pub struct DatabaseMetaFrequency {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub freq_expression: String,
    /// Is of type [`TermMetaModeType::Freq`]
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    pub dictionary: String,
}
impl DBMetaType for DatabaseMetaFrequency {
    fn mode(&self) -> &TermMetaModeType {
        &self.mode
    }
    fn expression(&self) -> &str {
        &self.freq_expression
    }
}

/// Used to store the pitch metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 4, version = 1)]
#[native_db]
pub struct DatabaseMetaPitch {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub pitch_expression: String,
    /// Is of type [`TermMetaModeType::Pitch`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPitchData,
    pub dictionary: String,
}
/// Pitch accent information for a term, represented as the position of the downstep.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PitchAccent {
    /// Type of the pronunciation, for disambiguation between union type members.
    /// Should be `"pitch-accent"` in the json.
    pub term: TermPronunciationMatchType,
    /// Position of the downstep, as a number of mora.
    pub position: u8,
    /// Positions of morae with a nasal sound.
    pub nasal_positions: Vec<u8>,
    /// Positions of morae with a devoiced sound.
    pub devoice_positions: Vec<u8>,
    /// Tags for the pitch accent.
    pub tags: Vec<DictionaryTag>,
}
impl DBMetaType for DatabaseMetaPitch {
    fn mode(&self) -> &TermMetaModeType {
        &self.mode
    }
    fn expression(&self) -> &str {
        &self.pitch_expression
    }
}

/// Used to store the phonetic metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 5, version = 1)]
#[native_db]
pub struct DatabaseMetaPhonetic {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub phonetic_expression: String,
    /// Is of type [`TermMetaModeType::Ipa`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPhoneticData,
    pub dictionary: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaPhoneticData {
    pub reading: String,
    /// List of different IPA transcription information for the term and reading combination.
    pub transcriptions: Vec<PhoneticTranscription>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhoneticTranscription {
    /// Type of the pronunciation, for disambiguation between union type members.
    /// Should be `"phonetic-transcription"` in the json.
    pub match_type: TermPronunciationMatchType,
    /// IPA transcription for the term.
    pub ipa: String,
    /// List of tags for this IPA transcription.
    pub tags: Vec<DictionaryTag>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TermPronunciationMatchType {
    #[serde(rename = "lowercase")]
    PitchAccent,
    #[serde(rename = "phonetic-transcription")]
    PhoneticTranscription,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Pronunciation {
    PitchAccent(PitchAccent),
    PhoneticTranscription(PhoneticTranscription),
}
impl DBMetaType for DatabaseMetaPhonetic {
    fn mode(&self) -> &TermMetaModeType {
        &self.mode
    }
    fn expression(&self) -> &str {
        &self.phonetic_expression
    }
}

/*************** Database Kanji Meta ***************/

/// Kanji Meta's only have frequency data
#[native_db]
#[native_model(id = 8, version = 1)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMeta {
    #[primary_key]
    pub character: String,
    /// Is of type [TermMetaModeType::Freq]
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    #[secondary_key]
    pub dictionary: String,
}

#[native_db]
#[native_model(id = 7, version = 1)]
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseKanjiEntry {
    #[primary_key]
    pub character: String,
    #[secondary_key]
    #[serde_as(as = "NoneAsEmptyString")]
    pub onyomi: Option<String>,
    #[secondary_key]
    #[serde_as(as = "NoneAsEmptyString")]
    pub kunyomi: Option<String>,
    #[secondary_key]
    #[serde_as(as = "NoneAsEmptyString")]
    pub tags: Option<String>,
    pub meanings: Vec<String>,
    pub stats: Option<IndexMap<String, String>>,
    /// The kanji dictionary name.
    /// Does not exist within the JSON, gets added _after_ deserialization.
    #[secondary_key]
    //#[serde(skip_deserializing)]
    pub dictionary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiEntry {
    pub index: usize,
    pub character: String,
    pub onyomi: Vec<String>,
    pub kunyomi: Vec<String>,
    pub tags: Vec<String>,
    pub definitions: Vec<String>,
    pub stats: IndexMap<String, String>,
    pub dictionary: String,
}

/*************** Database Dictionary ***************/

pub type DictionaryCountGroup = IndexMap<String, u16>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DictionaryCounts {
    total: Option<DictionaryCountGroup>,
    counts: Vec<DictionaryCountGroup>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeleteDictionaryProgressData {
    count: u64,
    processed: u64,
    store_count: u16,
    stores_processed: u64, // Corrected typo: stores_processed
}

#[derive(thiserror::Error, Debug)]
#[error("queries returned None:\n {queries:#?}\n reason: {reason}")]
pub struct QueryRequestError {
    queries: Vec<QueryRequestMatchType>,
    reason: Box<native_db::db_type::Error>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum QueryRequestMatchType {
    TermExactQueryRequest(TermExactQueryRequest),
    GenericQueryRequest(GenericQueryRequest),
}
/// converts any `IntoIter<Enum::Variant(T)>` to a `IntoIter<Item = T>`
#[macro_export]
macro_rules! iter_variant_to_iter_type {
    ($items:expr, $enum_type:ident :: $variant:ident) => {
        $items
            .iter()
            .filter_map(|item| {
                if let $enum_type::$variant(data) = item {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .collect()
    };
}
#[macro_export]
macro_rules! iter_type_to_iter_variant {
    ($items_iterable:expr, $enum_type:ident :: $variant:ident) => {
        $items_iterable
            .into_iter()
            .map(|item_to_wrap| $enum_type::$variant(item_to_wrap))
    };
}
// Collects mutable references to data within a specific enum variant from an iterable.
/// Input `$items` is expected to have an `.iter_mut()` method (e.g., a `Vec<MyEnum>`).
/// The output is a `Vec<&mut InnerDataType>`.
#[macro_export]
macro_rules! collect_variant_data_ref {
    ($items:expr, $enum_type:ident :: $variant:ident) => {
        $items
            .iter_mut() // Iterates over &mut EnumType
            .filter_map(|item_ref| {
                // item_ref_mut is &mut EnumType
                match item_ref {
                    // `ref mut data` borrows the data mutably from within the enum variant
                    $enum_type::$variant(ref data) => Some(data), // data is &mut InnerDataType
                    _ => None,                                    // Ignore other variants
                }
            })
            .collect::<Vec<_>>() // Collects into Vec<&mut InnerDataType>
    };
}
/// Converts an iterable of items into a Vec of enums, where each enum variant
/// holds a mutable reference to an original item.
/// Input `$items_iterable` is expected to have an `.iter_mut()` method (e.g., a `Vec<MyData>`).
/// The enum variant specified must be capable of holding a mutable reference
/// (e.g., defined as `enum MyWrapper<'a> { MyVariant(&'a mut MyData) }`).
/// The output is a `Vec<EnumType<'a>::Variant(&'a mut MyData)>`.
#[macro_export]
macro_rules! variant_to_generic_vec_mut {
    ($items_iterable:expr, $enum_type:ident :: $variant:ident) => {
        $items_iterable
            .iter_mut() // Iterates over &mut MyData
            .map(|item_ref_mut| {
                // item_ref_mut is &mut MyData
                // The enum variant constructor takes the mutable reference
                $enum_type::$variant(item_ref_mut)
            })
            .collect::<Vec<_>>() // Collects into Vec<EnumType::Variant(&mut MyData)>
    };
}

#[derive(thiserror::Error, Debug)]
pub enum DictionaryDatabaseError {
    #[error("database error: {0}")]
    Database(#[from] Box<native_db::db_type::Error>),
    #[error("failed to find terms: {0}")]
    QueryRequest(#[from] QueryRequestError),
    #[error("incorrect variant(s) passed: {wrong:#?}\nexpected: {expected:#?}")]
    WrongQueryRequestMatchType {
        wrong: QueryRequestMatchType,
        expected: QueryRequestMatchType,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TermExactQueryRequest {
    pub term: String,
    pub reading: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord, Hash)]
pub enum QueryType {
    String(String),
    Sequence(i128),
}
/// so far it seems this can be refactored to use references
/// for now keep owned so don't have to deal with lifetimes
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord, Hash)]
pub struct GenericQueryRequest {
    pub query_type: QueryType,
    pub dictionary: String,
}
impl From<&TagTargetItem> for GenericQueryRequest {
    fn from(value: &TagTargetItem) -> Self {
        Self {
            query_type: QueryType::String(value.query.clone()),
            dictionary: value.dictionary.clone(),
        }
    }
}
impl From<TagTargetItem> for GenericQueryRequest {
    fn from(value: TagTargetItem) -> Self {
        Self {
            query_type: QueryType::String(value.query),
            dictionary: value.dictionary,
        }
    }
}
impl GenericQueryRequest {
    pub fn new(query_type: QueryType, dictionary: &str) -> Self {
        Self {
            query_type,
            dictionary: dictionary.to_string(),
        }
    }
    pub fn from_query_type_slice_to_vec(queries: &[QueryType], dictionary: &str) -> Vec<Self> {
        queries
            .iter()
            .map(|q| Self::new(q.clone(), dictionary))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaRequest {
    path: String,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FindMultiBulkDataItemType {
    String(String),
    // Consider adding other types if `item` in JS can be non-string for into_term_generic
}
impl PartialEq<FindMultiBulkDataItemType> for String {
    fn eq(&self, other: &FindMultiBulkDataItemType) -> bool {
        match other {
            FindMultiBulkDataItemType::String(s_other) => self == s_other,
            // _ => false, // If other variants are added
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindMulitBulkData {
    item: FindMultiBulkDataItemType, // This is the original query item (e.g. a string from term_list)
    item_index: usize,               // Index of this item in the input items_to_query list
    index_index: usize, // Index of the IndexQueryIdentifier used (e.g. 0 for expression, 1 for reading)
}

trait DBReadWrite {
    fn rw_insert(&self, db: Database) -> Result<(), DBError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryDataDetails {
    pub summary: DictionarySummary,
    pub dictionary_options: DictionaryOptions,
}

pub struct DatabaseDictData {
    pub tag_list: Vec<DatabaseTag>,
    pub kanji_meta_list: Vec<DatabaseMetaFrequency>,
    pub kanji_list: Vec<DatabaseKanjiEntry>,
    pub term_meta_list: Vec<DatabaseMetaMatchType>,
    pub term_list: Vec<DatabaseTermEntry>,
    pub summary: DictionarySummary,
    pub dictionary_options: DictionaryOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Queries<'a, Q: AsRef<str>> {
    Exact(&'a [Q]),
    StartsWith(&'a [Q]),
}

pub trait DictionarySet: Sync + Send {
    fn has(&self, value: &str) -> bool;
}

// native_db imports are fine
use native_db::{
    db_type::KeyRange, // KeyRange unused
    db_type::{KeyDefinition, ToKey},
    native_model::Model as NativeDbModelTrait, // Already aliased above as NativeModelTrait
    Key,                                       // Key unused
};
// std::marker::PhantomData is unused
// use std::marker::PhantomData;
use std::ops::{Bound, Deref, RangeBounds};

use super::dictionary_importer::DictionarySummaryKey; // RangeBounds unused

/// Describes the kind of secondary key to query.
/// This enum IS `Clone` and `Copy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SecondaryKeyQueryKind {
    Expression,
    Reading,
    Sequence,
    ExpressionReverse,
    ReadingReverse,
}

/// Identifies whether to scan the primary key or a specific kind of secondary key.
/// This enum IS `Clone` and `Copy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexQueryIdentifier {
    PrimaryKey,
    SecondaryKey(SecondaryKeyQueryKind),
}

/// Represents the type of query to perform on an index.
/// K is the type of the value used for querying (e.g., String).
#[derive(Clone)] // ModelKeyType is Clone, so this can be Clone
pub enum NativeDbQueryInfo<K: ToKey + Clone> {
    Exact(K),
    Prefix(K),
    Range { start: Bound<K>, end: Bound<K> },
}

// Definition of the CreateQueryFn type alias using dyn Fn
type CreateQueryFn<Item, KeyVal> =
    dyn Fn(&Item, IndexQueryIdentifier) -> NativeDbQueryInfo<KeyVal> + Sync + Send;

pub struct DictionaryDatabase {
    db: Database<'static>,
    db_name: &'static str,
}

impl Deref for DictionaryDatabase {
    type Target = Database<'static>;
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl From<native_db::db_type::Error> for DictionaryDatabaseError {
    fn from(e: native_db::db_type::Error) -> Self {
        DictionaryDatabaseError::Database(Box::new(e))
    }
}
impl From<native_db::db_type::Error> for Box<DictionaryDatabaseError> {
    fn from(e: native_db::db_type::Error) -> Self {
        Box::new(DictionaryDatabaseError::Database(Box::new(e)))
    }
}
impl From<Box<native_db::db_type::Error>> for Box<DictionaryDatabaseError> {
    fn from(e: Box<native_db::db_type::Error>) -> Self {
        Box::new(DictionaryDatabaseError::Database(e))
    }
}

impl DictionaryDatabase {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            // if the file does not exist, or is an empty file,
            // a new database will be initialized in it
            // if the file is a valid native_db database, it will be opened
            // otherwise this function will return an error
            db: DBBuilder::new().create(&DB_MODELS, path).unwrap(),
            db_name: "dict",
        }
    }

    pub fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>> {
        let rtx = self.db.r_transaction()?;
        let summaries: Result<Vec<DictionarySummary>, native_db::db_type::Error> =
            rtx.scan().primary()?.all()?.collect();
        let mut summaries = summaries?;
        summaries.sort_by_key(|s| s.import_date);
        Ok(summaries)
    }

    // Translates the JavaScript `findTermsBulk` function.
    ///
    /// Queries for a list of terms, matching against expression or reading fields,
    /// with support for exact, prefix, or suffix matching.
    /// Handles deduplication of results based on term ID.
    pub fn find_terms_bulk(
        &self,
        term_list_input: &[impl AsRef<str>], // Renamed to avoid conflict
        dictionaries: &impl DictionarySet,
        match_type: TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let term_list_refs: Vec<&str> = term_list_input.iter().map(|s| s.as_ref()).collect();

        // 1. Handle term list pre-processing for suffix searches
        let (processed_term_list, actual_match_type_for_query) = match match_type {
            TermSourceMatchType::Suffix => (
                term_list_refs
                    .iter()
                    .map(|s| s.chars().rev().collect::<String>())
                    .collect::<Vec<String>>(),
                TermSourceMatchType::Prefix, // Suffix searches become prefix searches on reversed strings
            ),
            _ => (
                term_list_refs.iter().map(|s| s.to_string()).collect(),
                match_type,
            ),
        };

        let index_names: [IndexQueryIdentifier; 2] = match match_type {
            TermSourceMatchType::Suffix => [
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ExpressionReverse),
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ReadingReverse),
            ],
            _ => [
                // Exact, Prefix
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Expression),
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Reading),
            ],
        };

        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| -> DatabaseTermEntryKey {
            match kind {
                SecondaryKeyQueryKind::Expression => DatabaseTermEntryKey::expression,
                SecondaryKeyQueryKind::Reading => DatabaseTermEntryKey::reading,
                SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence,
                SecondaryKeyQueryKind::ExpressionReverse => {
                    DatabaseTermEntryKey::expression_reverse
                }
                SecondaryKeyQueryKind::ReadingReverse => DatabaseTermEntryKey::reading_reverse,
            }
        };

        let create_query_fn_closure = Box::new(
            move |item_from_list: &String, _idx_identifier: IndexQueryIdentifier| {
                match actual_match_type_for_query {
                    // Use the adjusted match type
                    TermSourceMatchType::Exact => {
                        to_variant!(item_from_list.clone(), NativeDbQueryInfo::Exact)
                    }
                    TermSourceMatchType::Prefix => {
                        to_variant!(item_from_list.clone(), NativeDbQueryInfo::Prefix)
                    }
                    TermSourceMatchType::Suffix => {
                        // This case should not be hit if
                        // actual_match_type_for_query is Prefix for suffix searches
                        // but as a safeguard, treat it as prefix on the (already reversed) string.
                        to_variant!(item_from_list.clone(), NativeDbQueryInfo::Prefix)
                    }
                }
            },
        );

        let find_multi_bulk_predicate =
            |row: &DatabaseTermEntry, _item_to_query: &String| dictionaries.has(&row.dictionary);

        let create_result_fn = |db_entry: DatabaseTermEntry,
                                item_from_list: &String,
                                item_idx: usize,
                                index_kind_idx: usize|
         -> TermEntry {
            let mut current_match_type_for_result = match_type; // Original match_type from function args

            let find_data = FindMulitBulkData {
                item: FindMultiBulkDataItemType::String(item_from_list.clone()),
                item_index: item_idx,
                index_index: index_kind_idx,
            };
            db_entry.into_term_generic(&mut current_match_type_for_result, find_data)
        };

        let mut potential_term_entries = self.find_multi_bulk::<
            String, // ItemQueryType (type of elements in processed_term_list)
            DatabaseTermEntry, // M (Model)
            String, // ModelKeyType (type for query values like exact term, prefix)
            DatabaseTermEntryKey, // SecondaryKeyEnumType
            TermEntry, // QueryResultType
            _, // ResolveSecondaryKeyFnParamType (inferred)
            _, // PredicateFnParamType (inferred)
            _  // CreateResultFnParamType (inferred)
        >(
            &index_names,
            &processed_term_list,
            create_query_fn_closure, // Pass the boxed closure
            resolve_secondary_key_fn,
            find_multi_bulk_predicate,
            create_result_fn,
        )?;

        let mut visited_ids: IndexSet<String> = IndexSet::new();
        potential_term_entries.retain(|term_entry| visited_ids.insert(term_entry.id.clone()));

        Ok(potential_term_entries)
    }

    pub fn find_terms_exact_bulk(
        &self,
        term_list: &[TermExactQueryRequest],
        dictionaries: &impl DictionarySet,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let index_query_identifiers = [IndexQueryIdentifier::SecondaryKey(
            SecondaryKeyQueryKind::Expression,
        )];

        let create_query_fn_closure = Box::new(
            |req: &TermExactQueryRequest, _idx_identifier: IndexQueryIdentifier| {
                to_variant!(req.term.clone(), NativeDbQueryInfo::Exact)
            },
        );

        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| match kind {
            SecondaryKeyQueryKind::Expression => DatabaseTermEntryKey::expression,
            _ => unreachable!(
                "Only SecondaryKeyQueryKind::Expression is expected in find_terms_exact_bulk"
            ),
        };

        let predicate_fn = |row: &DatabaseTermEntry, item_request: &TermExactQueryRequest| {
            row.reading == item_request.reading && dictionaries.has(&row.dictionary)
        };

        let create_result_fn = |db_entry: DatabaseTermEntry,
                                _req: &TermExactQueryRequest,
                                item_idx: usize,
                                _index_kind_idx: usize|
         -> TermEntry {
            db_entry.into_term_entry_specific(
                TermSourceMatchSource::Term,
                TermSourceMatchType::Exact,
                item_idx,
            )
        };

        match self.find_multi_bulk::<
            TermExactQueryRequest, // ItemQueryType
            DatabaseTermEntry,     // M (Model)
            String,                // ModelKeyType (for term, which is String)
            DatabaseTermEntryKey,  // SecondaryKeyEnumType
            TermEntry,             // QueryResultType
            _, _, _                // Infer other closure types
        >(
            &index_query_identifiers,
            term_list,
            create_query_fn_closure, // Pass boxed closure
            resolve_secondary_key_fn,
            predicate_fn,
            create_result_fn,
        ) {
            Ok(results) => Ok(results),
            Err(reason) => {
                Err(Box::new(DictionaryDatabaseError::QueryRequest(
                    QueryRequestError {
                        queries: iter_type_to_iter_variant!(term_list.to_vec(), QueryRequestMatchType::TermExactQueryRequest).collect(),
                        reason,
                    },
                )))
            }
        }
    }

    pub fn find_term_meta_bulk(
        &self,
        term_list_input: &[impl AsRef<str> + Sync],
        dictionaries: &(impl DictionarySet),
    ) -> Result<Vec<DatabaseTermMeta>, Box<DictionaryDatabaseError>> {
        let terms_as_strings: Vec<String> = term_list_input
            .iter()
            .map(|s| s.as_ref().to_string())
            .collect();

        if terms_as_strings.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_term_meta_results: Vec<DatabaseTermMeta> = Vec::new();
        let r_txn = self.db.r_transaction()?;

        // Iterate through each term we need to find.
        for (item_idx, term) in terms_as_strings.iter().enumerate() {
            // --- 1. Query Frequency Metadata ---
            let freq_scan = r_txn
                .scan()
                .secondary::<DatabaseMetaFrequency>(DatabaseMetaFrequencyKey::freq_expression)?;
            for result in freq_scan.range(term.clone()..=term.clone())? {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    all_term_meta_results.push(DatabaseTermMeta {
                        index: item_idx,
                        term: db_entry.freq_expression,
                        mode: db_entry.mode,
                        data: MetaDataMatchType::Frequency(db_entry.data),
                        dictionary: db_entry.dictionary,
                    });
                }
            }

            // --- 2. Query Pitch Metadata ---
            let pitch_scan = r_txn
                .scan()
                .secondary::<DatabaseMetaPitch>(DatabaseMetaPitchKey::pitch_expression)?;
            for result in pitch_scan.range(term.clone()..=term.clone())? {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    all_term_meta_results.push(DatabaseTermMeta {
                        index: item_idx,
                        term: db_entry.pitch_expression,
                        mode: db_entry.mode,
                        data: MetaDataMatchType::Pitch(db_entry.data),
                        dictionary: db_entry.dictionary,
                    });
                }
            }

            // --- 3. Query Phonetic Metadata ---
            let phonetic_scan = r_txn
                .scan()
                .secondary::<DatabaseMetaPhonetic>(DatabaseMetaPhoneticKey::phonetic_expression)?;
            for result in phonetic_scan.range(term.clone()..=term.clone())? {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    all_term_meta_results.push(DatabaseTermMeta {
                        index: item_idx,
                        term: db_entry.phonetic_expression,
                        mode: db_entry.mode,
                        data: MetaDataMatchType::Phonetic(db_entry.data),
                        dictionary: db_entry.dictionary,
                    });
                }
            }
        }

        Ok(all_term_meta_results)
    }
    pub fn find_terms_by_sequence_bulk(
        &self,
        items_to_query_vec: Vec<GenericQueryRequest>, // Renamed
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let index_query_identifiers = [IndexQueryIdentifier::SecondaryKey(
            SecondaryKeyQueryKind::Sequence,
        )];

        let create_query_fn_closure = Box::new(
            |req: &GenericQueryRequest, _idx_identifier: IndexQueryIdentifier| match req.query_type
            {
                QueryType::Sequence(seq_val) => {
                    to_variant!(Some(seq_val), NativeDbQueryInfo::Exact)
                }
                _ => {
                    // This panic is fine for now, but proper error handling might be better
                    panic!("QueryType for sequence search must be Num (i.e. Sequence)");
                }
            },
        );

        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| match kind {
            SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence,
            _ => unreachable!(
                "Only SecondaryKeyQueryKind::Sequence is expected in find_terms_by_sequence_bulk"
            ),
        };

        let predicate_fn = |row: &DatabaseTermEntry, current_item_request: &GenericQueryRequest| {
            row.dictionary == current_item_request.dictionary
        };

        let create_result_fn = |db_entry: DatabaseTermEntry,
                                _req: &GenericQueryRequest,
                                item_idx: usize,
                                _index_kind_idx: usize| {
            db_entry.into_term_entry_specific(
                TermSourceMatchSource::Sequence,
                TermSourceMatchType::Exact,
                item_idx,
            )
        };

        match self.find_multi_bulk::<
            GenericQueryRequest, // ItemQueryType
            DatabaseTermEntry,   // M (Model)
            Option<i128>,        // ModelKeyType (value for 'sequence' key)
            DatabaseTermEntryKey,// SecondaryKeyEnumType
            TermEntry,           // QueryResultType
            _, _, _              // Infer other closure types
        >(
            &index_query_identifiers,
            &items_to_query_vec,
            create_query_fn_closure, // Pass boxed closure
            resolve_secondary_key_fn,
            predicate_fn,
            create_result_fn,
        ) {
            Ok(results) => Ok(results),
            Err(reason) => {
                Err(Box::new(DictionaryDatabaseError::QueryRequest(
                    QueryRequestError {
                        queries: iter_type_to_iter_variant!(items_to_query_vec, QueryRequestMatchType::GenericQueryRequest).collect(),
                        reason
                    }
                )))
            }
        }
    }

    pub fn find_multi_bulk<
        ItemQueryType: Sync + Send,
        M: NativeDbModelTrait + ToInput + Clone + Send + Sync + 'static,
        ModelKeyType: ToKey + Clone + Send + Sync + 'static,
        SecondaryKeyEnumType: ToKeyDefinition<KeyOptions> + Send + Sync + 'static,
        QueryResultType: Send + 'static,
        // CreateQueryFnParamType is replaced by Box<CreateQueryFn<...>>
        ResolveSecondaryKeyFnParamType: Fn(SecondaryKeyQueryKind) -> SecondaryKeyEnumType + Sync + Send,
        PredicateFnParamType: Fn(&M, &ItemQueryType) -> bool + Sync + Send,
        CreateResultFnParamType: Fn(M, &ItemQueryType, usize, usize) -> QueryResultType + Sync + Send,
    >(
        &self,
        index_query_identifiers: &[IndexQueryIdentifier],
        items_to_query: &[ItemQueryType],
        create_query_fn: Box<CreateQueryFn<ItemQueryType, ModelKeyType>>, // MODIFIED: Use Boxed Trait Object
        resolve_secondary_key_fn: ResolveSecondaryKeyFnParamType,
        predicate_fn: PredicateFnParamType,
        create_result_fn: CreateResultFnParamType,
    ) -> Result<Vec<QueryResultType>, Box<native_db::db_type::Error>> {
        let r_txn = self.db.r_transaction()?;
        let mut all_final_results = Vec::new();

        for (item_idx, item_to_query) in items_to_query.iter().enumerate() {
            for (index_kind_idx, query_identifier_ref) in index_query_identifiers.iter().enumerate()
            {
                let query_identifier = *query_identifier_ref;

                // Call the boxed closure directly
                let query_info = create_query_fn(item_to_query, query_identifier);
                let mut current_batch_models: Vec<M> = Vec::new();

                match query_identifier {
                    IndexQueryIdentifier::PrimaryKey => {
                        let scan = r_txn.scan().primary::<M>()?;
                        match query_info {
                            NativeDbQueryInfo::Exact(key_val) => {
                                current_batch_models = scan
                                    .range(key_val.clone()..=key_val.clone())?
                                    .collect::<Result<Vec<M>, _>>()?;
                            }
                            NativeDbQueryInfo::Prefix(prefix_key) => {
                                current_batch_models = scan
                                    .start_with(prefix_key)?
                                    .collect::<Result<Vec<M>, _>>()?;
                            }
                            NativeDbQueryInfo::Range { start, end } => {
                                current_batch_models =
                                    scan.range((start, end))?.collect::<Result<Vec<M>, _>>()?;
                            }
                        }
                    }
                    IndexQueryIdentifier::SecondaryKey(secondary_kind) => {
                        let actual_native_db_key: SecondaryKeyEnumType =
                            resolve_secondary_key_fn(secondary_kind);
                        let scan = r_txn.scan().secondary::<M>(actual_native_db_key)?;

                        match query_info {
                            NativeDbQueryInfo::Exact(key_val) => {
                                current_batch_models = scan
                                    .range(key_val.clone()..=key_val.clone())?
                                    .collect::<Result<Vec<M>, _>>()?;
                            }
                            NativeDbQueryInfo::Prefix(prefix_key) => {
                                current_batch_models = scan
                                    .start_with(prefix_key)?
                                    .collect::<Result<Vec<M>, _>>()?;
                            }
                            NativeDbQueryInfo::Range { start, end } => {
                                current_batch_models =
                                    scan.range((start, end))?.collect::<Result<Vec<M>, _>>()?;
                            }
                        }
                    }
                }

                for db_model in current_batch_models {
                    if predicate_fn(&db_model, item_to_query) {
                        all_final_results.push(create_result_fn(
                            db_model,
                            item_to_query,
                            item_idx,
                            index_kind_idx,
                        ));
                    }
                }
            }
        }
        Ok(all_final_results)
    }

    /// Finds tag metadata for a list of tag names and their respective dictionaries.
    ///
    /// For each query in `queries`, this function attempts to find a matching tag.
    /// The result is a `Vec` of `Option<DatabaseTag>` where each element
    /// corresponds to the query at the same index. `Some(tag)` if found, `None` otherwise.
    /// [GenericQueryRequest] is `DictionaryAndQueryRequest` in yomitan JS.
    pub fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        // We will query by the 'name' secondary key of DbTagDefinition.
        // We use SecondaryKeyQueryKind::Expression
        // as a convention for the "main name/text" query.
        let index_query_identifiers = [IndexQueryIdentifier::SecondaryKey(
            SecondaryKeyQueryKind::Expression,
        )];

        // create_query_fn: Given a GenericQueryRequest,
        // create an Exact query for its 'name'.
        let create_query_fn_closure = Box::new(
            |req: &GenericQueryRequest, _idx_identifier: IndexQueryIdentifier| {
                let tag_name = match &req.query_type {
                    QueryType::String(name) => name,
                    _ => unreachable!("you cannot pass sequences to this function"),
                };
                NativeDbQueryInfo::Exact(tag_name.clone())
            },
        );

        // resolve_secondary_key_fn:
        // Map SecondaryKeyQueryKind::Expression to DbTagDefinitionKey::name.
        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| match kind {
            SecondaryKeyQueryKind::Expression => DatabaseTagKey::name,
            _ => unreachable!(
                "Only Expression-like key query is expected for tag name in find_tag_meta_bulk"
            ),
        };

        let predicate_fn =
            |db_tag: &DatabaseTag, req: &GenericQueryRequest| db_tag.dictionary == req.dictionary;

        let create_result_fn = |db_tag: DatabaseTag,
                                _req: &GenericQueryRequest,
                                item_idx: usize,
                                _index_kind_idx: usize|
         -> (usize, DatabaseTag) {
            (
                item_idx,
                DatabaseTag {
                    id: db_tag.id,
                    name: db_tag.name,
                    category: db_tag.category,
                    order: db_tag.order,
                    notes: db_tag.notes,
                    score: db_tag.score,
                    dictionary: db_tag.dictionary,
                },
            )
        };

        match self.find_multi_bulk::<
            GenericQueryRequest,
            DatabaseTag,
            String,
            DatabaseTagKey,
            (usize, DatabaseTag),
            _,
            _,
            _,
        >(
            &index_query_identifiers,
            queries,
            create_query_fn_closure,
            resolve_secondary_key_fn,
            predicate_fn,
            create_result_fn,
        ) {
            Ok(found_tags_with_indices) => {
                // Reconstruct the Vec<Option<DatabaseTag>> in the correct order.
                // Initialize with Nones, then fill in found tags at their original query indices.
                let mut results: Vec<Option<DatabaseTag>> = vec![None; queries.len()];
                for (original_idx, tag) in found_tags_with_indices {
                // Should always be true if item_idx is correct
                    if original_idx < results.len() {
                        results[original_idx] = Some(tag);
                    }
                }
                Ok(results)
            }
            Err(reason) => {
                Err(Box::new(DictionaryDatabaseError::QueryRequest(
                    QueryRequestError {
                        queries: iter_type_to_iter_variant!(
                            queries.to_vec(),
                            QueryRequestMatchType::GenericQueryRequest
                        )
                        .collect(),
                        reason,
                    },
                )))
            }
        }
    }
}

pub fn split_optional_string_field(field: Option<String>) -> Vec<String> {
    field
        .map(|s| {
            s.split(' ')
                .map(String::from)
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn split_string_field(field: String) -> Vec<String> {
    field
        .split(' ')
        .map(String::from)
        .filter(|part| !part.is_empty())
        .collect()
}

#[cfg(test)]
mod ycd {
    use indexmap::IndexSet;
    use pretty_assertions::assert_eq;

    // Assuming to_variant! is available in this scope if dictionary_database is the parent module
    // or if it's exported from the crate root and imported here.
    // If dictionary_database.rs is a file module, to_variant! is in scope.
    // use crate::to_variant; // Only if to_variant! is exported from crate root and not in this module.

    use super::{
        DatabaseTermEntry,
        DatabaseTermEntryKey,
        DictionaryDatabase,
        GenericQueryRequest,
        IndexQueryIdentifier,
        NativeDbQueryInfo,
        QueryRequestMatchType,
        QueryType,
        SecondaryKeyQueryKind,
        TermExactQueryRequest, // Added TermExactQueryRequest
    };
    use crate::{
        database::dictionary_database::DictionarySet,
        dictionary::TermSourceMatchType,
        test_utils::{self, TEST_PATHS}, // TEST_PATHS unused
    };

    #[test]
    fn find_terms_sequence_bulk() {
        //let (f_path, _handle) = test_utils::copy_test_db();
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let queries = &[
            // 大丈夫
            QueryType::Sequence(9635800000),
            // 奉迎
            QueryType::Sequence(14713900000),
        ];
        let queries_generic_req =
            GenericQueryRequest::from_query_type_slice_to_vec(queries, "大辞林\u{3000}第四版");
        let entries = ycd.find_terms_by_sequence_bulk(queries_generic_req);
        match entries {
            Ok(entries) => {
                assert!(
                    !entries.is_empty(),
                    "Expected entries for sequence bulk search"
                );
                entries.into_iter().for_each(|entry| {
                    dbg!(&entry);
                });
            }
            Err(e) => panic!("find_terms_sequence_bulk_failed: {e}"),
        };
    }

    #[test]
    fn find_term_meta_bulk_() {
        //let (_f_path, _handle) = test_utils::copy_test_db();
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let term_list = vec!["自業自得".to_string()];
        let mut dictionaries = IndexSet::new();
        dictionaries.insert("Anime & J-drama".to_string());
        let result = ycd.find_term_meta_bulk(&term_list, &dictionaries).unwrap();
        dbg!(result);
    }

    #[test]
    fn find_terms_bulk_exact_match_test() {
        //let (_f_path, _handle) = test_utils::copy_test_db();
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let term_list = vec!["日本語".to_string()];
        let mut dictionaries_set = IndexSet::new();
        dictionaries_set.insert("大辞林\u{3000}第四版".to_string());

        struct TestDictionarySet(IndexSet<String>);
        impl DictionarySet for TestDictionarySet {
            fn has(&self, value: &str) -> bool {
                self.0.contains(value)
            }
        }
        let dictionaries = TestDictionarySet(dictionaries_set);
        let match_type = TermSourceMatchType::Exact;
        // Pass term_list directly as it implements AsRef<str> for String
        let result = ycd.find_terms_bulk(&term_list, &dictionaries, match_type);
        dbg!(result);
    }
}

#[cfg(test)]
mod init_db {
    use std::fs::remove_dir_all;

    use crate::{test_utils, Yomichan};

    #[test]
    #[ignore]
    /// Initializes the repo's yomichan database with specified dicts.
    fn init_db() {
        let td = &*test_utils::TEST_PATHS.tests_dir;
        let yomichan_rs_folder = td.join("yomichan_rs");
        if yomichan_rs_folder.exists() {
            remove_dir_all(yomichan_rs_folder);
        }
        let tdcs = &*test_utils::TEST_PATHS.test_dicts_dir;
        let mut ycd = Yomichan::new(td).unwrap();
        let paths = [
            // tdcs.join("daijirin"),
            // tdcs.join("ajdfreq"),
            // tdcs.join("pitch_accent"),
            tdcs.join("kotobankesjp"),
        ];
        match ycd.import_dictionaries(&paths) {
            Ok(_) => {}
            Err(e) => {
                let db_path = td.join("db.ycd");
                if db_path.exists() && db_path.is_file() {
                    if let Some(ext) = db_path.extension() {
                        if ext == "ycd" {
                            std::fs::remove_file(db_path).unwrap();
                        }
                    }
                }
                panic!("failed init_db test: {e}");
            }
        }
    }
}
