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
use indexmap::{IndexMap, IndexSet};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use native_model::{native_model, Model};

use rayon::collections::hash_set;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer as JsonDeserializer;

use transaction::RTransaction;
//use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use uuid::Uuid;

use std::cell::LazyCell;
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{fs, marker};

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
    // pub fn predicate(
    //     &self,
    //     dictionaries: impl DictionarySet,
    //     visited_ids: &mut IndexSet<String>,
    // ) -> bool {
    //     if !dictionaries.has(&self.dictionary) {
    //         return false;
    //     }
    //     if visited_ids.contains(&self.id) {
    //         return false;
    //     }
    //     visited_ids.insert(self.id.clone());
    //     true
    // }
    pub fn into_term_generic(
        self,
        match_type: &mut TermSourceMatchType,
        data: FindMulitBulkData,
    ) -> TermEntry {
        let match_source_is_term = data.index_index == 0;
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
            expression_reverse,
            reading_reverse,
            definition_tags,
            tags,
            rules,
            score,
            glossary,
            sequence,
            term_tags,
            dictionary,
            file_path,
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
    pub stats: Option<IndexMap<String, String>>,
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
    stores_processed: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum QueryMatchType {
    Str(String),
    Num(i128),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DictionaryAndQueryRequest {
    query_type: QueryMatchType,
    dictionary: String,
}
impl DictionaryAndQueryRequest {
   pub fn new(query_type: QueryMatchType, dictionary: &str) -> Self {
      Self {
            query_type,
            dictionary: dictionary.to_string(),
        } 
   }
   pub fn from_slice(queries: &[QueryMatchType], dictionary: &str) -> Vec<Self> {
        queries.iter().map(|q| {
            Self::new(q.clone(), dictionary)
        }).collect()
   } 
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
pub enum FindMultiBulkDataItemType {
    String(String),
}
impl PartialEq<FindMultiBulkDataItemType> for String {
    fn eq(&self, other: &FindMultiBulkDataItemType) -> bool {
        match other {
            FindMultiBulkDataItemType::String(other) => self == other,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindMulitBulkData {
    item: FindMultiBulkDataItemType,
    item_index: usize,
    index_index: usize,
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
pub struct DictionaryDatabase {
    db: &'static Database<'static>,
    db_name: &'static str,
}

impl DictionaryDatabase {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            db: Box::leak(Box::new(DBBuilder::new().open(&DB_MODELS, path).unwrap())),
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
        term_list: &[impl AsRef<str>],
        dictionaries: &impl DictionarySet,
        match_type: TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, DBError> {
        let term_list: Vec<&str> = term_list.iter().map(|s| s.as_ref()).collect();

        // 1. Handle term list pre-processing for suffix searches
        let (processed_term_list, actual_match_type_for_query) = match match_type {
            TermSourceMatchType::Suffix => (
                term_list
                    .iter()
                    .map(|s| s.chars().rev().collect::<String>())
                    .collect::<Vec<String>>(),
                TermSourceMatchType::Prefix,
            ),
            _ => (
                term_list.iter().map(|s| s.to_string()).collect(),
                match_type,
            ),
        };

        let index_names: [IndexQueryIdentifier; 2] = match match_type {
            TermSourceMatchType::Suffix => [
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ExpressionReverse),
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::ReadingReverse),
            ],
            _ => [
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Expression),
                IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Reading),
            ],
        };

        // 3. Define `resolve_secondary_key_fn`
        // This now needs to include reverse keys if they exist.
        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| -> DatabaseTermEntryKey {
            match kind {
                SecondaryKeyQueryKind::Expression => DatabaseTermEntryKey::expression,
                SecondaryKeyQueryKind::Reading => DatabaseTermEntryKey::reading,
                // Keep if used by IndexQueryIdentifier
                SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence,
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
                    // This case should ideally not be hit if
                    // actual_match_type_for_query is used correctly.
                    // Defaulting to Prefix as a safeguard,
                    // assuming item_from_list is already reversed.
                    NativeDbQueryInfo::Prefix(item_from_list.clone())
                }
            }
        };

        // 5. Define `predicate_fn` (for find_multi_bulk - dictionary check only)
        // The JS `visited` logic will be applied *after* this call.
        let find_multi_bulk_predicate =
            |row: &DatabaseTermEntry, _item_to_query: &String| dictionaries.has(&row.dictionary);

        let create_result_fn = |
            db_entry: DatabaseTermEntry,      
            // The string from `processed_term_list` that was used for the query.
            item_from_list: &String,          
            // The index of `item_from_list` in `processed_term_list`.
            item_idx: usize,                  
            // The index into `index_names` that was used for this 
            // specific query part (0 for expression-related, 1 for reading-related).
            index_kind_idx: usize             
        | -> TermEntry {
            // `match_type` is captured from the find_terms_bulk function's parameters.
            // This will be the initial match type, 
            // which `into_term_generic` can then refine to `Exact` if applicable.
            let mut current_match_type_for_result = match_type; 

            // Construct the `FindMulitBulkData` required by `into_term_generic`.
            // This structure bundles information about the 
            // query item and how it was matched.
            let find_data = FindMulitBulkData {
                // The item used in the query.
                item: FindMultiBulkDataItemType::String(item_from_list.clone()), 
                // The original index of the item in the input list.
                item_index: item_idx,             
                // Identifies if the match was against term (0) or reading (1).
                index_index: index_kind_idx,      
            };

            // `into_term_generic` handles the logic of determining the precise
            // `match_source` and updates 
            // `current_match_type_for_result` to `TermSourceMatchType::Exact`
            // if the criteria are met.
            db_entry.into_term_generic(&mut current_match_type_for_result, find_data)
        };

        let mut potential_term_entries = self.find_multi_bulk::<
            // ItemQueryType (type of elements in processed_term_list)
            String,                
            // M (Model)
            DatabaseTermEntry,     
            // ModelKeyType (type for query values like exact term, prefix)
            String,                
            // SecondaryKeyEnumType (actual native_db key)
            DatabaseTermEntryKey,  
            // QueryResultType
            TermEntry,             
            // Infer closures
            _, _, _, _             
        >(
            &index_names,
            &processed_term_list,
            create_query_fn,
            resolve_secondary_key_fn,
            find_multi_bulk_predicate,
            create_result_fn,
        )?;

        // 8. Deduplication based on ID (mimicking the JavaScript `visited` set)
        let mut visited_ids: IndexSet<String> = IndexSet::new();
        potential_term_entries.retain(|term_entry| visited_ids.insert(term_entry.id.clone()));

        Ok(potential_term_entries)
    }

    fn find_terms_by_sequence_bulk(&self, item_request: Vec<DictionaryAndQueryRequest>) -> Vec<TermEntry> {
        let index_query_identifiers = 
        [IndexQueryIdentifier::SecondaryKey(SecondaryKeyQueryKind::Sequence)];
        let items_to_query_vec = item_request.clone(); 

        let create_query_fn = 
        |req: &DictionaryAndQueryRequest, _idx_identifier: IndexQueryIdentifier| {
            match req.query_type {
                QueryMatchType::Num(seq_val) => NativeDbQueryInfo::Exact(Some(seq_val)),
                _ => {
                    panic!("QueryMatchType for sequence search must be Num");
                }
            }
        };

        let resolve_secondary_key_fn = |kind: SecondaryKeyQueryKind| {
            match kind {
                SecondaryKeyQueryKind::Sequence => DatabaseTermEntryKey::sequence,
                // This function is specific to sequence, so other kinds are unexpected.
                _ => unreachable!(
                "Only SecondaryKeyQueryKind::Sequence is expected in find_terms_by_sequence_bulk"
            ),
            }
        };
        
        // The predicate provided by the user, adapted for the correct ItemQueryType.
        let predicate_fn = 
        |row: &DatabaseTermEntry, current_item_request: &DictionaryAndQueryRequest| {
            row.dictionary == current_item_request.dictionary
        };

        let create_result_fn = 
        |db_entry: DatabaseTermEntry, 
        _req: &DictionaryAndQueryRequest, 
        item_idx: usize, 
        _index_kind_idx: usize
        | {
            db_entry.into_term_entry_specific(
                TermSourceMatchSource::Sequence, TermSourceMatchType::Exact, item_idx
            )
        };

        match self.find_multi_bulk::<
            // ItemQueryType (type of elements in items_to_query_vec)
            DictionaryAndQueryRequest, 
            // M (Model)
            DatabaseTermEntry,         
            // ModelKeyType (value for the 'sequence' key, e.g., i128)
            Option<i128>,
            // SecondaryKeyEnumType (DatabaseTermEntryKey::sequence)
            DatabaseTermEntryKey,
            // QueryResultType
            TermEntry,
            // Infer closure types
            _, _, _, _
        >(
            &index_query_identifiers,
            &items_to_query_vec,
            create_query_fn,
            resolve_secondary_key_fn,
            predicate_fn,
            create_result_fn,
        ) {
            Ok(results) => results,
            Err(e) => {
                // Handle or log the error appropriately
                eprintln!("Error finding terms by sequence: {:?}", e);
                Vec::new()
            }
        }
    }

    /// Performs a bulk query against the database for multiple items 
    /// across multiple specified indexes.
    ///
    /// This function is highly generic and relies on 
    /// several closures to define specific behaviors
    /// such as query creation, secondary key resolution, 
    /// result filtering, and final result transformation.
    ///
    /// It's designed to handle cases where the 
    /// native_db key enums (like `SecondaryKeyEnumType`)
    /// might not be `Clone`, by using a `Copy`-able `IndexQueryIdentifier` 
    /// and a resolver function.
    ///
    /// # Arguments
    ///
    /// * `index_query_identifiers`: A slice of `IndexQueryIdentifier` 
    ///   indicating which primary or secondary key kinds to query.
    ///   These identifiers are `Copy`-able.
    ///
    /// * `items_to_query`: A slice of items that will be queried against the database.
    ///
    /// * `create_query_fn`: A closure that generates the specific `NativeDbQueryInfo` 
    ///   (e.g., exact match, prefix, range) for a given `ItemQueryType` and `IndexQueryIdentifier`
    ///
    /// * `resolve_secondary_key_fn`: A closure that takes a 
    ///   `SecondaryKeyQueryKind` (from `IndexQueryIdentifier`)
    ///   and returns an owned value of the actual `native_db` generated `SecondaryKeyEnumType`. 
    ///   Crucial for working with `native_db` key enums that are not `Clone`.
    ///
    /// * `predicate_fn`: A closure that filters the records fetched from the database. 
    ///   It receives a reference to the database model instance (`M`) 
    ///   and the original `ItemQueryType`.
    ///
    /// * `create_result_fn`: A closure that transforms a 
    ///   filtered database model instance (`M`) into the desired type 
    ///
    /// * `QueryResultType`. It also receives the original `ItemQueryType` 
    ///   and the indices of the item and query identifier.
    ///
    /// # Generic Parameters
    ///
    /// * `ItemQueryType`: The type of items in `items_to_query` 
    ///   (e.g., `String` for a search term) -- Must be `Sync + Send`.
    ///
    /// * `M`: The database model struct (e.g., `DatabaseTermEntry`). 
    ///   Must implement `NativeDbModelTrait` (likely `native_model::Model`),
    ///   
    /// * `ToInput` (for deserialization from `native_db`), 
    ///   `Clone`, `Send`, `Sync`, and be `'static`.
    ///
    /// * `ModelKeyType`: The type of the keys used for querying within `NativeDbQueryInfo` 
    ///   (e.g., `String`).
    ///   Must implement `ToKey`, `Clone`, `Send`, `Sync`, and be `'static`.
    ///
    /// * `SecondaryKeyEnumType`: The actual `native_db` generated key enum for secondary keys 
    ///   (e.g., `DatabaseTermEntryKey`).
    ///   This type is NOT required to be `Clone`. 
    ///   Must implement `ToKeyDefinition<KeyOptions>`, `Send`, `Sync`, and be `'static`.
    ///
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
    /// A `Result` containing either a `Vec<QueryResultType>` 
    /// with all successfully fetched, filtered, and transformed
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
                // Dereference to get a copy (IndexQueryIdentifier is Copy)
                let query_identifier = *query_identifier_ref; 

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
    use pretty_assertions::{assert_eq};

    use super::{
        DatabaseTermEntry, DatabaseTermEntryKey, DictionaryAndQueryRequest, DictionaryDatabase, IndexQueryIdentifier, NativeDbQueryInfo, QueryMatchType, SecondaryKeyQueryKind 
    };
    use crate::{
        database::dictionary_database::DictionarySet, dictionary::TermSourceMatchType,
        yomichan_test_utils::{self, TEST_PATHS},
    }; 
    use std::path::PathBuf; 
    
    #[test]
    fn find_terms_sequence_bulk() {
        //let (f_path, _handle) = yomichan_test_utils::copy_test_db();
        let ycd = &yomichan_test_utils::SHARED_DB_INSTANCE;
        let queries = 
        &[
            // 大丈夫
            QueryMatchType::Num(9635800000), 
            // 奉迎
            QueryMatchType::Num(14713900000), 
        ];
        let queries = DictionaryAndQueryRequest::from_slice(queries, "大辞林\u{3000}第四版");
        let entries = ycd.find_terms_by_sequence_bulk(queries);
        entries.into_iter().for_each(|entry| {
            dbg!(&entry);
        });
    }

    #[test]
    fn find_terms_bulk_daijoubu_exact_match_test() {
        let (f_path, _handle) = yomichan_test_utils::copy_test_db();
        let ycd = &yomichan_test_utils::SHARED_DB_INSTANCE;
        let term_list = vec!["大丈夫".to_string()];
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
        let result = ycd.find_terms_bulk(&term_list, &dictionaries, match_type);

        match result {
            Ok(term_entries) => {
                assert!(
                    !term_entries.is_empty(),
                    "Expected to find TermEntry for '大丈夫'"
                );
                let mut found_match = false;
                for entry in term_entries {
                    //dbg!(&entry);
                    if entry.term == "大丈夫" {
                        assert_eq!(entry.reading, "だいじょうぶ"); 
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
