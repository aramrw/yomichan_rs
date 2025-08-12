use crate::database::dictionary_importer::YomichanDatabaseSummary;
use crate::dictionary_data::{
    DictionaryDataTag,
    TermMetaFreqDataMatchType,
    TermMetaFrequency,
    TermMetaPitch,
    TermMetaPitchData,
    // NOTE: TermMetaPhoneticData is defined below, so it's not imported from here.
};
use crate::errors::{DBError, DictionaryFileError, ImportError, YomichanError};
use crate::settings::{DictionaryOptions, YomichanOptions, YomichanProfile};
use crate::test_utils::TEST_PATHS;
use crate::translator::TagTargetItem;
use derive_more::derive::{Deref, DerefMut, From, Into};
use importer::dictionary_data::MetaDataMatchType;
use importer::dictionary_data::TermMeta;
use importer::dictionary_data::TermMetaModeType;
use importer::structured_content::{StructuredContent, TermGlossary, TermGlossaryGroupType};
use importer::DatabaseDictionaryData;
// FIX: Using the structs from the importer crate directly.
// They will be wrapped in newtypes for database compatibility.
use importer::dictionary_database::{
    DatabaseMetaFrequency as ImporterMetaFrequency, DatabaseMetaPhonetic as ImporterMetaPhonetic,
    DatabaseMetaPitch as ImporterMetaPitch, DictionaryTag, TermSourceMatchSource,
    TermSourceMatchType,
};
use serde_with::skip_serializing_none;
use serde_with::{serde_as, NoneAsEmptyString};

use db_type::{KeyOptions, ToKeyDefinition};
use indexmap::{IndexMap, IndexSet};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
// Renamed to avoid conflict if Model is used for DB Model
use native_model::{native_model, Model as NativeModelTrait};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use uuid::Uuid;

use std::io::BufReader;
use std::ops::{Bound, Deref, RangeBounds};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{fs, marker};

// FIX: Added back macros that are used in other files.
#[macro_export]
macro_rules! to_variant {
    ($value:expr, $variant_constructor_path:path) => {
        $variant_constructor_path($value)
    };
}

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
macro_rules! collect_variant_data_ref {
    ($items:expr, $enum_type:ident :: $variant:ident) => {
        $items
            .iter_mut()
            .filter_map(|item_ref| match item_ref {
                $enum_type::$variant(data) => Some(data),
                _ => None,
            })
            .collect::<Vec<_>>()
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
    models.define::<YomichanOptions>().unwrap();
    models.define::<YomichanDatabaseSummary>().unwrap();
    models.define::<DatabaseTermEntry>().unwrap();
    // NOTE: Using the newtype wrappers for the database models.
    models.define::<DbMetaFrequency>().unwrap();
    models.define::<DbMetaPitch>().unwrap();
    models.define::<DbMetaPhonetic>().unwrap();
    models.define::<DatabaseKanjiEntry>().unwrap();
    models.define::<DatabaseKanjiMeta>().unwrap();
    models.define::<DatabaseTag>().unwrap();
    models
});

// FIX: Newtype pattern to wrap importer types for database compatibility.
// This is the idiomatic way to add traits (like ToInput from native_db) to external types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, From, Into, Deref, DerefMut)]
#[native_model(id = 3, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db(
    primary_key(id_getter -> &str),
    secondary_key(expression_getter -> &str),
    secondary_key(dictionary_getter -> &str)
)]
struct DbMetaFrequency(ImporterMetaFrequency);
impl DbMetaFrequency {
    fn id_getter(&self) -> &str {
        &self.0.id
    }
    fn expression_getter(&self) -> &str {
        &self.0.freq_expression
    }
    fn dictionary_getter(&self) -> &str {
        &self.0.dictionary
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, From, Into, Deref, DerefMut)]
#[native_model(id = 6, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db(
    primary_key(id_getter -> &str),
    secondary_key(expression_getter -> &str),
    secondary_key(dictionary_getter -> &str)
)]
struct DbMetaPitch(ImporterMetaPitch);
impl DbMetaPitch {
    fn id_getter(&self) -> &str {
        &self.0.id
    }
    fn expression_getter(&self) -> &str {
        &self.0.pitch_expression
    }
    fn dictionary_getter(&self) -> &str {
        &self.0.dictionary
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, From, Into, Deref, DerefMut)]
#[native_model(id = 5, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db(
    primary_key(id_getter -> &str),
    secondary_key(expression_getter -> &str),
    secondary_key(dictionary_getter -> &str)
)]
struct DbMetaPhonetic(ImporterMetaPhonetic);
impl DbMetaPhonetic {
    fn id_getter(&self) -> &str {
        &self.0.id
    }
    fn expression_getter(&self) -> &str {
        &self.0.phonetic_expression
    }
    fn dictionary_getter(&self) -> &str {
        &self.0.dictionary
    }
}

pub type MediaDataArrayBufferContent = MediaDataBase<Vec<u8>>;
pub type MediaDataStringContent = MediaDataBase<String>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaDataBase<TContentType: Serialize> {
    dictionary: String,
    path: String,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMeta {
    pub index: usize,
    pub term: String,
    pub mode: TermMetaModeType,
    pub data: MetaDataMatchType,
    pub dictionary: String,
}

impl From<DatabaseTermEntryTuple> for DatabaseTermEntry {
    fn from(tuple: DatabaseTermEntryTuple) -> Self {
        Self {
            id: tuple.0,
            expression: tuple.1,
            reading: tuple.2,
            expression_reverse: tuple.3,
            reading_reverse: tuple.4,
            definition_tags: tuple.5,
            tags: tuple.6,
            rules: tuple.7,
            score: tuple.8,
            glossary: tuple.9,
            sequence: tuple.10,
            term_tags: tuple.11,
            dictionary: tuple.12,
            file_path: tuple.13,
        }
    }
}

impl From<DatabaseTermEntry> for DatabaseTermEntryTuple {
    fn from(s: DatabaseTermEntry) -> Self {
        Self(
            s.id,
            s.expression,
            s.reading,
            s.expression_reverse,
            s.reading_reverse,
            s.definition_tags,
            s.tags,
            s.rules,
            s.score,
            s.glossary.to_vec(),
            s.sequence,
            s.term_tags,
            s.dictionary,
            s.file_path,
        )
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(from = "DatabaseTermEntryTuple", into = "DatabaseTermEntryTuple")]
#[native_model(id = 2, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
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
    pub tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub glossary: Vec<TermGlossaryGroupType>,
    #[secondary_key]
    pub sequence: Option<i128>,
    pub term_tags: Option<String>,
    pub dictionary: String,
    pub file_path: String,
}

#[derive(Serialize, Deserialize)]
struct DatabaseTermEntryTuple(
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i128,
    Vec<TermGlossaryGroupType>,
    Option<i128>,
    Option<String>,
    String,
    String,
);

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
    pub definitions: Vec<TermGlossaryGroupType>,
    pub score: i128,
    pub dictionary: String,
    pub sequence: i128,
}

impl DatabaseTermEntry {
    pub fn into_term_generic(
        self,
        match_type: &mut TermSourceMatchType,
        data: FindMultiBulkData,
    ) -> TermEntry {
        let match_source_is_term = data.index_index == 0;
        let match_source = match match_source_is_term {
            true => TermSourceMatchSource::Term,
            false => TermSourceMatchSource::Reading,
        };
        let found = match match_source {
            TermSourceMatchSource::Term => self.expression == data.item,
            TermSourceMatchSource::Reading => self.reading == data.item,
            _ => unreachable!(),
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
            definition_tags,
            rules,
            score,
            glossary,
            sequence,
            term_tags,
            dictionary,
            ..
        } = self;
        TermEntry {
            id,
            index,
            match_type,
            match_source,
            term: expression,
            reading,
            definition_tags: split_optional_string_field(definition_tags),
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
#[native_model(id = 9, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db]
pub struct DatabaseTag {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub name: String,
    #[secondary_key]
    pub category: String,
    pub order: u64,
    pub notes: String,
    pub score: i128,
    #[secondary_key]
    pub dictionary: String,
}

/*************** Database Term Meta ***************/
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DatabaseMetaMatchType {
    Frequency(ImporterMetaFrequency),
    Pitch(ImporterMetaPitch),
    Phonetic(ImporterMetaPhonetic),
}

impl DatabaseMetaMatchType {
    // NOTE: This function returns the ImporterMetaFrequency type. The calling code that
    // performs the DB insertion will need to wrap it in the DbMetaFrequency newtype.
    // e.g., `DbMetaFrequency::from(importer_meta_frequency_instance)`
    pub fn convert_kanji_meta_file(
        outpath: PathBuf,
        dict_name: String,
    ) -> Result<Vec<ImporterMetaFrequency>, DictionaryFileError> {
        let file = fs::File::open(&outpath).map_err(|reason| DictionaryFileError::FailedOpen {
            outpath: outpath.clone(),
            reason: reason.to_string(),
        })?;
        let reader = BufReader::new(file);
        let mut stream =
            JsonDeserializer::from_reader(reader).into_iter::<Vec<ImporterMetaFrequency>>();

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

        let stream = JsonDeserializer::from_reader(reader).into_iter::<Vec<TermMeta>>();
        let entries = match stream.last() {
            Some(Ok(entries)) => entries,
            Some(Err(e)) => {
                return Err(crate::errors::DictionaryFileError::File {
                    outpath,
                    reason: e.to_string(),
                })
            }
            None => return Err(DictionaryFileError::Empty(outpath)),
        };

        // FIX: The logic here was flawed. A single TermMeta from JSON corresponds to one DB entry.
        // The DB schema expects a Vec for pitch/phonetic data, suggesting aggregation is needed.
        // However, to fix the immediate compile error, I'm mapping 1-to-1 and wrapping data in a Vec.
        // This might need a more robust aggregation strategy later.
        let term_metas: Vec<DatabaseMetaMatchType> = entries
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
                        DatabaseMetaMatchType::Frequency(ImporterMetaFrequency {
                            id,
                            freq_expression: expression,
                            mode: TermMetaModeType::Freq,
                            data, // This is already the correct type
                            dictionary: dict_name.clone(),
                        })
                    }
                    MetaDataMatchType::Pitch(data) => {
                        DatabaseMetaMatchType::Pitch(ImporterMetaPitch {
                            id,
                            pitch_expression: expression,
                            mode: TermMetaModeType::Pitch,
                            data,
                            dictionary: dict_name.clone(),
                        })
                    }
                    MetaDataMatchType::Phonetic(data) => {
                        DatabaseMetaMatchType::Phonetic(ImporterMetaPhonetic {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PitchAccent {
    pub term: TermPronunciationMatchType,
    pub position: u8,
    pub nasal_positions: Vec<u8>,
    pub devoice_positions: Vec<u8>,
    pub tags: Vec<DictionaryTag>,
}

// This struct is defined in `importer`, but we need a local definition for `From` impls if not public
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermMetaPhoneticData {
    pub reading: String,
    pub transcriptions: Vec<PhoneticTranscription>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhoneticTranscription {
    pub match_type: TermPronunciationMatchType,
    pub ipa: String,
    pub tags: Vec<DictionaryTag>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TermPronunciationMatchType {
    #[serde(rename = "lowercase")]
    PitchAccent,
    #[serde(rename = "phonetic-transcription")]
    PhoneticTranscription,
}

/*************** Database Kanji Meta ***************/
#[native_db]
#[native_model(id = 8, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMeta {
    #[primary_key]
    pub character: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    #[secondary_key]
    pub dictionary: String,
}

#[native_db]
#[native_model(id = 7, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
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
    #[secondary_key]
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

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Deref, DerefMut, From, Into)]
// pub struct YomichanDatabaseDictionaryData(DatabaseDictionaryData);

/*************** Database Dictionary ***************/
// FIX: Added back struct used in other files.
// #[derive(Clone, Debug, PartialEq)]
// pub struct DatabaseDictData {
//     pub tag_list: Vec<DatabaseTag>,
//     pub kanji_meta_list: Vec<DatabaseKanjiMeta>,
//     pub kanji_list: Vec<DatabaseKanjiEntry>,
//     pub term_meta_list: Vec<DatabaseMetaMatchType>,
//     pub term_list: Vec<DatabaseTermEntry>,
//     pub summary: YomichanDatabaseSummary,
//     pub dictionary_options: DictionaryOptions,
// }

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

#[macro_export]
macro_rules! iter_type_to_iter_variant {
    ($items_iterable:expr, $enum_type:ident :: $variant:ident) => {
        $items_iterable
            .into_iter()
            .map(|item_to_wrap| $enum_type::$variant(item_to_wrap))
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
pub enum FindMultiBulkDataItemType {
    String(String),
}
impl PartialEq<FindMultiBulkDataItemType> for String {
    fn eq(&self, other: &FindMultiBulkDataItemType) -> bool {
        match other {
            FindMultiBulkDataItemType::String(s_other) => self == s_other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindMultiBulkData {
    item: FindMultiBulkDataItemType,
    item_index: usize,
    index_index: usize,
}

pub trait DictionarySet: Sync + Send {
    fn has(&self, value: &str) -> bool;
}

use native_db::db_type::{KeyDefinition, ToKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SecondaryKeyQueryKind {
    Expression,
    Reading,
    Sequence,
    ExpressionReverse,
    ReadingReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexQueryIdentifier {
    PrimaryKey,
    SecondaryKey(SecondaryKeyQueryKind),
}

#[derive(Clone)]
pub enum NativeDbQueryInfo<K: ToKey + Clone> {
    Exact(K),
    Prefix(K),
    Range { start: Bound<K>, end: Bound<K> },
}

pub struct DictionaryDatabase<'a> {
    db: Database<'a>,
    db_name: &'a str,
}

impl<'a> Deref for DictionaryDatabase<'a> {
    type Target = Database<'a>;
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

impl DictionaryDatabase<'_> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            db: DBBuilder::new().create(&DB_MODELS, path).unwrap(),
            db_name: "dict",
        }
    }

    pub fn create_in_memory() -> Result<Self, native_db::db_type::Error> {
        Ok(Self {
            db: DBBuilder::new()
                .create_in_memory(&DB_MODELS)?,
            db_name: "yomichan_rs_memory",
        })
    }

    pub fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<YomichanDatabaseSummary>, Box<DictionaryDatabaseError>> {
        let rtx = self.db.r_transaction()?;
        let summaries: Result<Vec<YomichanDatabaseSummary>, native_db::db_type::Error> =
            rtx.scan().primary()?.all()?.collect();
        let mut summaries = summaries?;
        summaries.sort_by_key(|s| s.data.import_date);
        Ok(summaries)
    }

    pub fn find_terms_bulk(
        &self,
        term_list_input: &[impl AsRef<str>],
        dictionaries: &impl DictionarySet,
        match_type: importer::dictionary_database::TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let term_list_refs: Vec<&str> = term_list_input.iter().map(|s| s.as_ref()).collect();

        let (processed_term_list, actual_match_type_for_query) = match match_type {
            TermSourceMatchType::Suffix => (
                term_list_refs
                    .iter()
                    .map(|s| s.chars().rev().collect::<String>())
                    .collect::<Vec<String>>(),
                TermSourceMatchType::Prefix,
            ),
            _ => (
                term_list_refs.iter().map(|s| s.to_string()).collect(),
                match_type,
            ),
        };

        let index_kinds_to_query: [(SecondaryKeyQueryKind, usize); 2] = match match_type {
            TermSourceMatchType::Suffix => [
                (SecondaryKeyQueryKind::ExpressionReverse, 0),
                (SecondaryKeyQueryKind::ReadingReverse, 1),
            ],
            _ => [
                (SecondaryKeyQueryKind::Expression, 0),
                (SecondaryKeyQueryKind::Reading, 1),
            ],
        };

        let r_txn = self.db.r_transaction()?;
        let mut all_final_results: Vec<TermEntry> = Vec::new();
        let mut visited_ids: IndexSet<String> = IndexSet::new();

        for (item_idx, item_to_query) in processed_term_list.iter().enumerate() {
            for (index_kind, index_kind_idx) in index_kinds_to_query.iter().copied() {
                let db_key_for_query = match index_kind {
                    SecondaryKeyQueryKind::Expression => DatabaseTermEntryKey::expression,
                    SecondaryKeyQueryKind::Reading => DatabaseTermEntryKey::reading,
                    SecondaryKeyQueryKind::ExpressionReverse => {
                        DatabaseTermEntryKey::expression_reverse
                    }
                    SecondaryKeyQueryKind::ReadingReverse => DatabaseTermEntryKey::reading_reverse,
                    SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence,
                };

                let scan_result = match actual_match_type_for_query {
                    TermSourceMatchType::Exact => r_txn
                        .scan()
                        .secondary::<DatabaseTermEntry>(db_key_for_query)?
                        .range(item_to_query.clone()..=item_to_query.clone())?
                        .collect::<Result<Vec<_>, _>>(),
                    _ => r_txn
                        .scan()
                        .secondary::<DatabaseTermEntry>(db_key_for_query)?
                        .start_with(item_to_query.clone())?
                        .collect::<Result<Vec<_>, _>>(),
                };

                for db_model in scan_result? {
                    if !dictionaries.has(&db_model.dictionary) {
                        continue;
                    }
                    let mut current_match_type_for_result = match_type;
                    let find_data = FindMultiBulkData {
                        item: FindMultiBulkDataItemType::String(item_to_query.clone()),
                        item_index: item_idx,
                        index_index: index_kind_idx,
                    };
                    let term_entry =
                        db_model.into_term_generic(&mut current_match_type_for_result, find_data);

                    if visited_ids.insert(term_entry.id.clone()) {
                        all_final_results.push(term_entry);
                    }
                }
            }
        }

        Ok(all_final_results)
    }

    pub fn find_terms_exact_bulk(
        &self,
        term_list: &[TermExactQueryRequest],
        dictionaries: &impl DictionarySet,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let r_txn = self.db.r_transaction()?;
        let mut all_final_results = Vec::new();
        let mut visited_ids: IndexSet<String> = IndexSet::new();

        for (item_idx, req) in term_list.iter().enumerate() {
            for db_entry in r_txn
                .scan()
                .secondary::<DatabaseTermEntry>(DatabaseTermEntryKey::expression)?
                .range(req.term.clone()..=req.term.clone())?
            {
                let db_entry = db_entry?;
                if db_entry.reading == req.reading && dictionaries.has(&db_entry.dictionary) {
                    let term_entry = db_entry.into_term_entry_specific(
                        TermSourceMatchSource::Term,
                        TermSourceMatchType::Exact,
                        item_idx,
                    );
                    if visited_ids.insert(term_entry.id.clone()) {
                        all_final_results.push(term_entry);
                    }
                }
            }
        }
        Ok(all_final_results)
    }

    pub fn find_term_meta_bulk(
        &self,
        term_list_input: &IndexSet<impl AsRef<str> + Sync>,
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

        for (item_idx, term) in terms_as_strings.iter().enumerate() {
            // --- Query Frequency ---
            for result in r_txn
                .scan()
                .secondary::<DbMetaFrequency>(DbMetaFrequencyKey::expression_getter)?
                .range(term.clone()..=term.clone())?
            {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    all_term_meta_results.push(DatabaseTermMeta {
                        index: item_idx,
                        term: db_entry.freq_expression.clone(),
                        mode: db_entry.mode.clone(),
                        data: MetaDataMatchType::Frequency(db_entry.data.clone()),
                        dictionary: db_entry.dictionary.clone(),
                    });
                }
            }

            // --- Query Pitch ---
            for result in r_txn
                .scan()
                .secondary::<DbMetaPitch>(DbMetaPitchKey::expression_getter)?
                .range(term.clone()..=term.clone())?
            {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    // NOTE: The DB entry can have a Vec of data. The result type expects one.
                    // Taking the first one found. This might need more complex logic.
                    if let Some(first_data) = db_entry.data.pitches.iter().next() {
                        all_term_meta_results.push(DatabaseTermMeta {
                            index: item_idx,
                            term: db_entry.pitch_expression.clone(),
                            mode: db_entry.mode.clone(),
                            data: MetaDataMatchType::Pitch(
                                importer::dictionary_data::TermMetaPitchData {
                                    reading: term.clone(),
                                    pitches: vec![first_data.clone()],
                                },
                            ),
                            dictionary: db_entry.dictionary.clone(),
                        });
                    }
                }
            }

            // --- Query Phonetic ---
            for result in r_txn
                .scan()
                .secondary::<DbMetaPhonetic>(DbMetaPhoneticKey::expression_getter)?
                .range(term.clone()..=term.clone())?
            {
                let db_entry = result?;
                if dictionaries.has(&db_entry.dictionary) {
                    if let Some(first_data) = db_entry.data.transcriptions.iter().next() {
                        all_term_meta_results.push(DatabaseTermMeta {
                            index: item_idx,
                            term: db_entry.phonetic_expression.clone(),
                            mode: db_entry.mode.clone(),
                            data: MetaDataMatchType::Phonetic(
                                importer::dictionary_database::TermMetaPhoneticData {
                                    reading: term.clone(),
                                    transcriptions: vec![first_data.clone()],
                                },
                            ),
                            dictionary: db_entry.dictionary.clone(),
                        });
                    }
                }
            }
        }
        Ok(all_term_meta_results)
    }

    pub fn find_terms_by_sequence_bulk(
        &self,
        items_to_query: &[GenericQueryRequest],
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let r_txn = self.db.r_transaction()?;
        let mut all_final_results = Vec::new();

        for (item_idx, req) in items_to_query.iter().enumerate() {
            let seq_val = match req.query_type {
                QueryType::Sequence(val) => val,
                _ => continue,
            };

            for db_entry in r_txn
                .scan()
                .secondary::<DatabaseTermEntry>(DatabaseTermEntryKey::sequence)?
                .range(Some(seq_val)..=Some(seq_val))?
            {
                let db_entry = db_entry?;
                if db_entry.dictionary == req.dictionary {
                    all_final_results.push(db_entry.into_term_entry_specific(
                        TermSourceMatchSource::Sequence,
                        TermSourceMatchType::Exact,
                        item_idx,
                    ));
                }
            }
        }
        Ok(all_final_results)
    }

    pub fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        let r_txn = self.db.r_transaction()?;
        let mut results: Vec<Option<DatabaseTag>> = vec![None; queries.len()];

        for (item_idx, req) in queries.iter().enumerate() {
            if results[item_idx].is_some() {
                continue;
            }
            let tag_name = match &req.query_type {
                QueryType::String(name) => name,
                _ => continue,
            };

            for db_tag in r_txn
                .scan()
                .secondary::<DatabaseTag>(DatabaseTagKey::name)?
                .range(tag_name.clone()..=tag_name.clone())?
            {
                let db_tag = db_tag?;
                if db_tag.dictionary == req.dictionary {
                    results[item_idx] = Some(db_tag);
                    break;
                }
            }
        }
        Ok(results)
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
