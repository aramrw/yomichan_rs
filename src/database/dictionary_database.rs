use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    DictionaryDataTag, GenericFreqData, TermGlossary, TermGlossaryContent, TermMetaDataMatchType,
    TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPhoneticData,
    TermMetaPitch, TermMetaPitchData,
};

use crate::database::dictionary_importer::{Summary, TermMetaBank};
use crate::dictionary_data::KANA_MAP;
use crate::errors::{DBError, ImportError};
use crate::settings::{DictionaryOptions, Options, Profile};
use crate::Yomichan;

//use lindera::{LinderaError, Token, Tokenizer};

use db_type::{KeyOptions, ToKeyDefinition};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use native_model::{native_model, Model};

use rayon::collections::hash_set;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer as JsonDeserializer;

use transaction::RTransaction;
//use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use uuid::Uuid;

use std::cell::LazyCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{fs, marker};

pub static DB_MODELS: LazyLock<Models> = LazyLock::new(|| {
    let mut models = Models::new();
    models.define::<DatabaseTermEntry>().unwrap();
    models.define::<DatabaseMetaFrequency>().unwrap();
    models.define::<DatabaseMetaPitch>().unwrap();
    models.define::<DatabaseMetaPhonetic>().unwrap();
    models
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaDataBase<TContentType> {
    dictionary: String,
    path: String,
    media_type: String,
    width: u16,
    height: u16,
    content: TContentType,
}

pub type MediaDataArrayBufferContent = MediaDataBase<Vec<u8>>;
pub type MediaDataStringContent = MediaDataBase<String>;

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

pub trait HasExpression {
    fn expression(&self) -> &str;
}

impl HasExpression for DatabaseTermEntry {
    fn expression(&self) -> &str {
        &self.expression
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermEntry {
    pub id: String,
    pub index: u32,
    pub term: String,
    pub reading: String,
    pub sequence: Option<i128>,
    pub match_type: TermSourceMatchType,
    pub match_source: TermSourceMatchSource,
    pub definition_tags: Option<String>,
    pub term_tags: Option<Vec<String>>,
    pub rules: Vec<String>,
    pub definitions: Vec<TermGlossary>,
    pub score: i8,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DictionaryDatabaseTag {
    name: String,
    category: String,
    order: i32,
    notes: String,
    score: i32,
    dictionary: String,
}

/*************** Database Term Meta ***************/

pub trait DBMetaType {
    fn mode(&self) -> &TermMetaModeType;
    fn expression(&self) -> &str;
}

/// A custom `Yomichan_rs`-unique, generic Database Meta model.
///
/// May contain `any` or `all` of the values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseMeta {
    pub frequency: Option<DatabaseMetaFrequency>,
    pub pitch: Option<DatabaseMetaPitch>,
    pub phonetic: Option<DatabaseMetaPhonetic>,
}

impl DatabaseMeta {
    pub fn convert_kanji_meta_file(
        outpath: PathBuf,
        dict_name: String,
    ) -> Result<Vec<DatabaseMeta>, ImportError> {
        let file = fs::File::open(&outpath).map_err(|e| {
            ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
        })?;
        let reader = BufReader::new(file);

        let mut stream =
            JsonDeserializer::from_reader(reader).into_iter::<Vec<TermMetaFrequency>>();
        let entries = match stream.next() {
            Some(Ok(entries)) => entries,
            Some(Err(e)) => {
                return Err(ImportError::Custom(format!(
                    "File: {} | Err: {e}",
                    &outpath.to_string_lossy(),
                )))
            }
            None => {
                return Err(ImportError::Custom(String::from(
                    "no data in term_meta_bank stream",
                )))
            }
        };

        let kanji_metas: Vec<DatabaseMeta> = entries
            .into_iter()
            .map(|entry| {
                let dbkmf = DatabaseMetaFrequency {
                    id: Uuid::new_v4().to_string(),
                    expression: entry.expression,
                    mode: TermMetaModeType::Freq,
                    data: entry.data,
                    dictionary: dict_name.clone(),
                };

                DatabaseMeta {
                    frequency: Some(dbkmf),
                    pitch: None,
                    phonetic: None,
                }
            })
            .collect();
        Ok(kanji_metas)
    }

    pub fn convert_term_meta_file(
        outpath: PathBuf,
        dict_name: String,
    ) -> Result<Vec<DatabaseMeta>, ImportError> {
        let file = fs::File::open(&outpath)?;
        let reader = BufReader::new(file);

        let mut stream = JsonDeserializer::from_reader(reader).into_iter::<TermMetaBank>();
        let entries: TermMetaBank = match stream.next() {
            Some(Ok(entries)) => entries,
            Some(Err(e)) => {
                return Err(ImportError::Custom(format!(
                    "File: {} | Err: {e}",
                    &outpath.to_string_lossy(),
                )))
            }
            None => {
                return Err(ImportError::Custom(String::from(
                    "no data in term_meta_bank stream",
                )))
            }
        };

        let term_metas: Vec<DatabaseMeta> = entries
            .into_iter()
            .map(|entry| {
                let mut meta = DatabaseMeta {
                    frequency: None,
                    pitch: None,
                    phonetic: None,
                };

                let id = Uuid::new_v4().to_string();
                let dictionary = dict_name.clone();
                let expression = entry.expression;

                match entry.mode {
                    TermMetaModeType::Freq => {
                        if let TermMetaDataMatchType::Frequency(data) = entry.data {
                            meta.frequency = Some(DatabaseMetaFrequency {
                                id,
                                expression,
                                mode: TermMetaModeType::Freq,
                                data,
                                dictionary,
                            });
                        }
                    }
                    TermMetaModeType::Pitch => {
                        if let TermMetaDataMatchType::Pitch(data) = entry.data {
                            meta.pitch = Some(DatabaseMetaPitch {
                                id,
                                expression,
                                mode: TermMetaModeType::Pitch,
                                data,
                                dictionary,
                            });
                        }
                    }
                    TermMetaModeType::Ipa => {
                        if let TermMetaDataMatchType::Phonetic(data) = entry.data {
                            meta.phonetic = Some(DatabaseMetaPhonetic {
                                id,
                                expression,
                                mode: TermMetaModeType::Freq,
                                data,
                                dictionary,
                            });
                        }
                    }
                }

                meta
            })
            .collect();
        Ok(term_metas)
    }
}

/// Used to store the frequency metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 2, version = 1, with = native_model::rmp_serde_1_3::RmpSerde)]
#[native_db]
pub struct DatabaseMetaFrequency {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub expression: String,
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
        &self.expression
    }
}

impl HasExpression for DatabaseMetaFrequency {
    fn expression(&self) -> &str {
        &self.expression
    }
}

/// Used to store the pitch metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 3, version = 1)]
#[native_db]
pub struct DatabaseMetaPitch {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub expression: String,
    /// Is of type [`TermMetaModeType::Pitch`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPitchData,
    pub dictionary: String,
}

impl DBMetaType for DatabaseMetaPitch {
    fn mode(&self) -> &TermMetaModeType {
        &self.mode
    }
    fn expression(&self) -> &str {
        &self.expression
    }
}

/// Used to store the phonetic metadata of a term in the db.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 4, version = 1)]
#[native_db]
pub struct DatabaseMetaPhonetic {
    #[primary_key]
    pub id: String,
    #[secondary_key]
    pub expression: String,
    /// Is of type [`TermMetaModeType::Ipa`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPhoneticData,
    pub dictionary: String,
}

impl DBMetaType for DatabaseMetaPhonetic {
    fn mode(&self) -> &TermMetaModeType {
        &self.mode
    }
    fn expression(&self) -> &str {
        &self.expression
    }
}

/*************** Database Kanji Meta ***************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMetaFrequency {
    pub character: String,
    /// Is of type [`TermMetaModeType::Freq`]
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiEntry {
    pub character: String,
    // all of these are most likely empty strings-
    // its better dx to if let them then use .is_empty()
    // so add this serde macro later
    //#[serde_as(as = "NoneAsEmptyString")]
    pub onyomi: Option<String>,
    pub kunyomi: Option<String>,
    pub tags: Option<String>,
    pub definitions: Vec<String>,
    /// The kanji dictionary name.
    ///
    /// Does not exist within the JSON, gets added _after_ deserialization.
    pub stats: Option<HashMap<String, String>>,
    #[serde(skip_deserializing)]
    pub dictionary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiEntry {
    pub index: i32,
    pub character: String,
    pub onyomi: Vec<String>,
    pub kunyomi: Vec<String>,
    pub tags: Vec<String>,
    pub definitions: Vec<String>,
    pub stats: HashMap<String, String>,
    pub dictionary: String,
}

/*************** Database Dictionary ***************/

pub type DictionaryCountGroup = HashMap<String, u16>;

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
    stores_processed: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum QueryMatchType {
    Str(String),
    Num(i64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DictionaryAndQueryRequest {
    query: QueryMatchType,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermExactRequest {
    term: String,
    reading: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaRequest {
    path: String,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindMulitBulkData<TItem> {
    item: TItem,
    item_index: u64,
    index_index: u64,
}

trait DBReadWrite {
    fn rw_insert(&self, db: Database) -> Result<(), DBError>;
}

/// Vec<[TermEntry]>
pub type VecTermEntry = Vec<TermEntry>;
/// Vec<[DatabaseTermEntry]>
pub type VecDBTermEntry = Vec<DatabaseTermEntry>;
/// Vec<[DatabaseMeta]>
pub type VecDBTermMeta = Vec<DatabaseMeta>;
/// Vec<[DatabaseMetaFrequency]>
pub type VecDBMetaFreq = Vec<DatabaseMetaFrequency>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseDictData {
    pub tag_list: Vec<Vec<DictionaryDataTag>>,
    pub kanji_meta_list: Vec<DatabaseMeta>,
    pub kanji_list: Vec<DatabaseKanjiEntry>,
    pub term_meta_list: Vec<DatabaseMeta>,
    pub term_list: VecDBTermEntry,
    pub summary: Summary,
    pub dictionary_options: DictionaryOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Queries<'a, Q: AsRef<str>> {
    Exact(&'a [Q]),
    StartsWith(&'a [Q]),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[native_model(id = 1, version = 1)]
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
    pub score: i8,
    pub glossary: TermGlossary,
    #[secondary_key]
    pub sequence: Option<i128>,
    pub term_tags: Option<String>,
    pub dictionary: String,
    pub file_path: OsString,
}

pub trait DictionarySet: Sync + Send {
    fn has(&self, value: &str) -> bool;
}

use native_db::{
    db_type::KeyRange, // From redb, used by native_db
    db_type::{KeyDefinition, ToKey},
    native_model::Model as NativeDbModelTrait, // Renamed to avoid conflict
    // For scan operations and key types:
    Key,
};
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

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
#[derive(Clone)]
struct DictionaryDatabase {
    db: Arc<Database<'static>>,
    db_name: &'static str,
}

impl DictionaryDatabase {
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            db: Arc::new(DBBuilder::new().open(&DB_MODELS, path).unwrap()),
            db_name: "dict",
        }
    }

    // Translates the JavaScript `findTermsBulk` function.
    ///
    /// Queries for a list of terms, matching against expression or reading fields,
    /// with support for exact, prefix, or suffix matching.
    /// Handles deduplication of results based on term ID.
    pub fn find_terms_bulk(
        &self,
        term_list: &[String],
        dictionaries: &impl DictionarySet, // Your DictionarySet trait
        match_type: TermSourceMatchType,   // Your TermSourceMatchType enum
    ) -> Result<Vec<TermEntry>, DBError> {
        // TermEntry is your Rust struct

        // 1. Handle term list pre-processing for suffix searches
        let (processed_term_list, actual_match_type_for_query) = match match_type {
            TermSourceMatchType::Suffix => (
                term_list
                    .iter()
                    .map(|s| s.chars().rev().collect::<String>())
                    .collect::<Vec<String>>(),
                TermSourceMatchType::Prefix, // Suffix on normal string becomes prefix on reversed string
            ),
            _ => (term_list.to_vec(), match_type),
        };

        // 2. Determine index_query_identifiers based on original match_type
        let index_query_identifiers: Vec<IndexQueryIdentifier> =
            if match_type == TermSourceMatchType::Suffix {
                vec![
                    IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ExpressionReverse),
                    IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ReadingReverse),
                ]
            } else {
                vec![
                    IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Expression),
                    IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Reading),
                ]
            };

        // 3. Define `resolve_secondary_key_fn`
        // This now needs to include reverse keys if they exist.
        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| -> DatabaseTermEntryKey {
            match kind {
                SecondaryKeyQueryKind::Expression => DatabaseTermEntryKey::expression,
                SecondaryKeyQueryKind::Reading => DatabaseTermEntryKey::reading,
                SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence, // Keep if used by IndexQueryIdentifier
                // Add these if DatabaseTermEntryKey has them:
                SecondaryKeyQueryKind::ExpressionReverse => {
                    DatabaseTermEntryKey::expression_reverse
                }
                SecondaryKeyQueryKind::ReadingReverse => DatabaseTermEntryKey::reading_reverse,
            }
        };

        // 4. Define `create_query_fn`
        // `item_from_list` is from `processed_term_list` (so it's already reversed for suffix)
        let create_query_fn = |item_from_list: &String, _idx_identifier: IndexQueryIdentifier| {
            match actual_match_type_for_query {
                // Use the adjusted match type
                TermSourceMatchType::Exact => NativeDbQueryInfo::Exact(item_from_list.clone()),
                TermSourceMatchType::Prefix => NativeDbQueryInfo::Prefix(item_from_list.clone()),
                // Suffix was converted to Prefix on reversed string
                TermSourceMatchType::Suffix => {
                    // This case should ideally not be hit if actual_match_type_for_query is used correctly.
                    // Defaulting to Prefix as a safeguard, assuming item_from_list is already reversed.
                    NativeDbQueryInfo::Prefix(item_from_list.clone())
                } // Handle other match types if they exist in your TermSourceMatchType enum
            }
        };

        // 5. Define `predicate_fn` (for find_multi_bulk - dictionary check only)
        // The JS `visited` logic will be applied *after* this call.
        let find_multi_bulk_predicate =
            |row: &DatabaseTermEntry, _item_to_query: &String| dictionaries.has(&row.dictionary);

        // 6. Define `create_result_fn` (for find_multi_bulk)
        // This will construct the `TermEntry`.
        let create_result_fn = |db_entry: DatabaseTermEntry,
                                 _processed_item: &String, // Item from processed_term_list
                                 item_idx: usize,          // Index in processed_term_list (maps to original term_list)
                                 index_kind_idx: usize|    // Index in index_query_identifiers
         -> TermEntry {
            let current_index_identifier = index_query_identifiers[index_kind_idx];
            let match_source = match current_index_identifier {
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Expression) |
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ExpressionReverse) => {
                    TermSourceMatchSource::Term
                }
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Reading) |
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ReadingReverse) => {
                    TermSourceMatchSource::Reading
                }
                _ => TermSourceMatchSource::Term, // Default or error
            };

            TermEntry {
                id: db_entry.id,
                index: item_idx as u32, // Use item_idx from the (potentially reversed) term list
                term: db_entry.expression,
                reading: db_entry.reading,
                sequence: db_entry.sequence,
                match_type, // The original match_type passed to find_terms_bulk_rust
                match_source,
                definition_tags: db_entry.definition_tags,
                term_tags: DictionaryDatabase::split_optional_string_field(db_entry.term_tags),
                rules: DictionaryDatabase::split_string_field(db_entry.rules),
                definitions: vec![db_entry.glossary], // Assumes one glossary per db_entry maps to one definition entry
                score: db_entry.score,
                dictionary: db_entry.dictionary,
            }
        };

        // 7. Call the generic find_multi_bulk function
        let mut potential_term_entries = self.find_multi_bulk::<
            String,                // ItemQueryType (type of elements in processed_term_list)
            DatabaseTermEntry,     // M (Model)
            String,                // ModelKeyType (type for query values like exact term, prefix)
            DatabaseTermEntryKey,  // SecondaryKeyEnumType (actual native_db key)
            TermEntry,             // QueryResultType
            _, _, _, _             // Infer closures
        >(
            &index_query_identifiers,
            &processed_term_list,
            create_query_fn,
            resolve_secondary_key_fn,
            find_multi_bulk_predicate,
            create_result_fn,
        )?;

        // 8. Deduplication based on ID (mimicking the JavaScript `visited` set)
        let mut visited_ids: HashSet<String> = HashSet::new();
        potential_term_entries.retain(|term_entry| visited_ids.insert(term_entry.id.clone()));

        Ok(potential_term_entries)
    }

    /// Performs a bulk query against the database for multiple items across multiple specified indexes.
    ///
    /// This function is highly generic and relies on several closures to define specific behaviors
    /// such as query creation, secondary key resolution, result filtering, and final result transformation.
    /// It's designed to handle cases where the native database key enums (like `SecondaryKeyEnumType`)
    /// might not be `Clone`, by using a `Copy`-able `IndexQueryIdentifier` and a resolver function.
    ///
    /// # Arguments
    ///
    /// * `index_query_identifiers`: A slice of `IndexQueryIdentifier` indicating which primary or secondary key kinds to query.
    ///   These identifiers are `Copy`-able.
    /// * `items_to_query`: A slice of items that will be queried against the database.
    /// * `create_query_fn`: A closure that generates the specific `NativeDbQueryInfo` (e.g., exact match, prefix, range)
    ///   for a given `ItemQueryType` and `IndexQueryIdentifier`.
    /// * `resolve_secondary_key_fn`: A closure that takes a `SecondaryKeyQueryKind` (from `IndexQueryIdentifier`)
    ///   and returns an owned value of the actual `native_db` generated `SecondaryKeyEnumType`. This is crucial
    ///   for working with `native_db` key enums that are not `Clone`.
    /// * `predicate_fn`: A closure that filters the records fetched from the database. It receives a reference to the
    ///   database model instance (`M`) and the original `ItemQueryType`.
    /// * `create_result_fn`: A closure that transforms a successfully filtered database model instance (`M`) into the desired
    ///   `QueryResultType`. It also receives the original `ItemQueryType` and the indices of the item and query identifier.
    ///
    /// # Generic Parameters
    ///
    /// * `ItemQueryType`: The type of items in `items_to_query` (e.g., `String` for a search term).
    ///   Must be `Sync + Send`.
    /// * `M`: The database model struct (e.g., `DatabaseTermEntry`). Must implement `NativeDbModelTrait` (likely `native_model::Model`),
    ///   `ToInput` (for deserialization from `native_db`), `Clone`, `Send`, `Sync`, and be `'static`.
    /// * `ModelKeyType`: The type of the keys used for querying within `NativeDbQueryInfo` (e.g., `String`).
    ///   Must implement `ToKey`, `Clone`, `Send`, `Sync`, and be `'static`.
    /// * `SecondaryKeyEnumType`: The actual `native_db` generated key enum for secondary keys (e.g., `DatabaseTermEntryKey`).
    ///   This type is NOT required to be `Clone`. Must implement `ToKeyDefinition<KeyOptions>`, `Send`, `Sync`, and be `'static`.
    /// * `QueryResultType`: The type of items that will be present in the final returned vector.
    ///   Must be `Send + 'static`.
    ///
    /// # Closures
    ///
    /// * `CreateQueryFn`: `Fn(&ItemQueryType, IndexQueryIdentifier) -> NativeDbQueryInfo<ModelKeyType> + Sync + Send`
    /// * `ResolveSecondaryKeyFn`: `Fn(SecondaryKeyQueryKind) -> SecondaryKeyEnumType + Sync + Send`
    /// * `PredicateFn`: `Fn(&M, &ItemQueryType) -> bool + Sync + Send`
    /// * `CreateResultFn`: `Fn(M, &ItemQueryType, usize, usize) -> QueryResultType + Sync + Send`
    ///
    /// # Returns
    ///
    /// A `Result` containing either a `Vec<QueryResultType>` with all successfully fetched, filtered, and transformed
    /// results, or a `DBError` if any database operation fails.
    pub fn find_multi_bulk<
        ItemQueryType: Sync + Send,
        M: NativeDbModelTrait + ToInput + Clone + Send + Sync + 'static,
        ModelKeyType: ToKey + Clone + Send + Sync + 'static,
        SecondaryKeyEnumType: ToKeyDefinition<KeyOptions> + Send + Sync + 'static,
        QueryResultType: Send + 'static,
        CreateQueryFn: Fn(&ItemQueryType, IndexQueryIdentifier) -> NativeDbQueryInfo<ModelKeyType> + Sync + Send,
        ResolveSecondaryKeyFn: Fn(SecondaryKeyQueryKind) -> SecondaryKeyEnumType + Sync + Send,
        PredicateFn: Fn(&M, &ItemQueryType) -> bool + Sync + Send,
        CreateResultFn: Fn(M, &ItemQueryType, usize, usize) -> QueryResultType + Sync + Send,
    >(
        &self,
        index_query_identifiers: &[IndexQueryIdentifier],
        items_to_query: &[ItemQueryType],
        create_query_fn: CreateQueryFn,
        resolve_secondary_key_fn: ResolveSecondaryKeyFn,
        predicate_fn: PredicateFn,
        create_result_fn: CreateResultFn,
    ) -> Result<Vec<QueryResultType>, DBError> {
        let r_txn = self.db.r_transaction()?;
        let mut all_final_results = Vec::new();

        for (item_idx, item_to_query) in items_to_query.iter().enumerate() {
            for (index_kind_idx, query_identifier_ref) in index_query_identifiers.iter().enumerate()
            {
                let query_identifier = *query_identifier_ref; // Dereference to get a copy (IndexQueryIdentifier is Copy)

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
                        // Use the new closure to get an owned, fresh native_db key enum value
                        let actual_native_db_key: SecondaryKeyEnumType =
                            resolve_secondary_key_fn(secondary_kind);

                        let scan = r_txn.scan().secondary::<M>(actual_native_db_key)?; // Pass the owned value

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

    pub fn split_optional_string_field(field: Option<String>) -> Option<Vec<String>> {
        field.map(|s| {
            s.split(' ')
                .map(String::from)
                .filter(|part| !part.is_empty())
                .collect()
        })
    }

    pub fn split_string_field(field: String) -> Vec<String> {
        field
            .split(' ')
            .map(String::from)
            .filter(|part| !part.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod dictdbtests {
    use super::{
        DatabaseTermEntry,
        DatabaseTermEntryKey, // The actual native_db generated key enum
        DictionaryDatabase,
        IndexQueryIdentifier, // Your new copyable enum
        NativeDbQueryInfo,
        SecondaryKeyQueryKind, // Your new copyable enum
    };
    use crate::{
        database::dictionary_database::DictionarySet, dictionary::TermSourceMatchType,
        yomichan_test_utils,
    }; // Your test utilities
    use std::path::PathBuf; // Assuming f_path might be PathBuf

    #[test]
    fn find_multi_bulk_daijoubu_expression_test() {
        let (f_path, _handle) = yomichan_test_utils::copy_test_db();
        let ycd = DictionaryDatabase::new(f_path);

        let search_term = "大丈夫".to_string();
        let items_to_query: Vec<String> = vec![search_term.clone()];

        let index_query_identifiers: Vec<IndexQueryIdentifier> =
            vec![IndexQueryIdentifier::SecondaryKey(
                SecondaryKeyQueryKind::Expression,
            )];

        let create_query_fn = |_item: &String, _idx_query_identifier: IndexQueryIdentifier| {
            NativeDbQueryInfo::Exact("大丈夫".to_string())
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

        let predicate_fn = |db_entry: &DatabaseTermEntry, original_item: &String| {
            db_entry.expression == *original_item
        };

        let create_result_fn =
            |db_entry: DatabaseTermEntry,
             _original_item: &String,
             _item_idx: usize,
             _index_idx: usize| { db_entry.expression };

        let result = ycd.find_multi_bulk::<
            String,                // ItemQueryType
            DatabaseTermEntry,     // M (Model)
            String,                // ModelKeyType (for query values)
            DatabaseTermEntryKey,  // SecondaryKeyEnumType (the actual native_db key)
            String,                // QueryResultType
            _, _, _, _             // Infer closure types
        >(
            &index_query_identifiers,
            &items_to_query,
            create_query_fn,
            resolve_secondary_key_fn,
            predicate_fn,
            create_result_fn,
        );

        match result {
            Ok(found_expressions) => {
                assert!(
                    !found_expressions.is_empty(),
                    "Expected to find entries for '{search_term}'"
                );
                for expr in found_expressions {
                    dbg!(&expr);
                    assert_eq!(expr, search_term);
                }
            }
            Err(e) => {
                panic!("Test failed: find_multi_bulk returned an error: {e:?}");
            }
        }
    }
    #[test]
    fn find_terms_bulk_daijoubu_exact_match_test() {
        let (f_path, _handle) = yomichan_test_utils::copy_test_db();
        let ycd = DictionaryDatabase::new(f_path);

        let term_list = vec!["大丈夫".to_string()];

        let mut dictionaries_set = std::collections::HashSet::new();
        // Assuming your test DB has terms from a dictionary named "JMDict" or similar.
        // Adjust this if your test dictionary has a different name.
        // If the dictionary name doesn't matter for this specific test and all terms should be included,
        // the `has` method of your DictionarySet impl could always return true for testing.
        dictionaries_set.insert("大辞林\u{3000}第四版".to_string()); // Example dictionary name

        struct TestDictionarySet(std::collections::HashSet<String>);
        impl DictionarySet for TestDictionarySet {
            fn has(&self, value: &str) -> bool {
                self.0.contains(value)
            }
        }
        let dictionaries = TestDictionarySet(dictionaries_set);

        let match_type = TermSourceMatchType::Exact;

        let result = ycd.find_terms_bulk(&term_list, &dictionaries, match_type);

        match result {
            Ok(term_entries) => {
                assert!(
                    !term_entries.is_empty(),
                    "Expected to find TermEntry for '大丈夫'"
                );
                let mut found_match = false;
                for entry in term_entries {
                    dbg!(&entry);
                    if entry.term == "大丈夫" {
                        assert_eq!(entry.reading, "だいじょうぶ"); // Or another common reading
                        assert_eq!(entry.match_type, TermSourceMatchType::Exact);
                        found_match = true;
                        break;
                    }
                }
                assert!(
                    found_match,
                    "Did not find '大丈夫' with expected reading in results."
                );
            }
            Err(e) => {
                panic!("find_terms_bulk_rust test failed: {e:?}");
            }
        }
    }
}
