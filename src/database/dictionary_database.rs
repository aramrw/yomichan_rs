use crate::database::dictionary_importer::DictionarySummary;
use crate::dictionary_importer::CHUNKS;
use crate::translator::core::TagTargetItem;
use yomichan_importer::dictionary_data::{TermMetaFreqDataMatchType, TermMetaModeType, TermMetaPitchData};
use yomichan_importer::dictionary_database::{TermEntry, TermMetaPhoneticData};
use yomichan_importer::dictionary_database::{TermSourceMatchSource, TermSourceMatchType};
use yomichan_importer::structured_content::TermGlossaryGroupType;
use serde_with::{serde_as, NoneAsEmptyString};

use indexmap::{IndexMap, IndexSet};
use native_model::{decode, native_model};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

use log;
use serde::{Deserialize, Serialize};

use std::path::Path;

pub trait DictionarySet: Sync + Send {
    fn has(&self, value: &str) -> bool;
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

pub type MediaDataArrayBufferContent = MediaDataBase<Vec<u8>>;

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
pub struct DatabaseTermMeta {
    pub index: usize,
    pub term: String,
    pub mode: TermMetaModeType,
    pub data: DatabaseMetaMatchType,
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

// WARNING: Never use `#[skip_serializing_none]` or similar serde macros that omit fields from serialization on this struct.
// `postcard` is a strict, positional binary format. Omitting a field (like `None`) from the byte stream breaks positional
// deserialization and causes issues like "Found an Option discriminant that wasn't 0 or 1" or "Hit the end of buffer".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[native_model(id = 2, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DatabaseTermEntry {
    pub id: String,
    pub expression: String,
    pub reading: String,
    pub expression_reverse: String,
    pub reading_reverse: String,
    pub definition_tags: Option<String>,
    pub tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub glossary: Vec<TermGlossaryGroupType>,
    pub sequence: Option<i128>,
    pub term_tags: Option<String>,
    pub dictionary: String,
    pub file_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseTermEntryTuple(
    pub String,                     // id
    pub String,                     // expression
    pub String,                     // reading
    pub String,                     // expression_reverse
    pub String,                     // reading_reverse
    pub Option<String>,             // definition_tags
    pub Option<String>,             // tags
    pub String,                     // rules
    pub i128,                       // score
    pub Vec<TermGlossaryGroupType>, // glossary
    pub Option<i128>,               // sequence
    pub Option<String>,             // term_tags
    pub String,                     // dictionary
    pub String,                     // file_path
);

impl DatabaseTermEntry {
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
#[native_model(id = 9, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DatabaseTag {
    pub id: String,
    pub name: String,
    pub category: String,
    pub order: i64,
    pub notes: String,
    pub score: i128,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DatabaseMetaMatchType {
    Frequency(DatabaseMetaFrequency),
    Pitch(DatabaseMetaPitch),
    Phonetic(DatabaseMetaPhonetic),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 3, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DatabaseMetaFrequency {
    pub id: String,
    pub freq_expression: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 4, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DatabaseMetaPitch {
    pub id: String,
    pub pitch_expression: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaPitchData,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 5, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DatabaseMetaPhonetic {
    pub id: String,
    pub phonetic_expression: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaPhoneticData,
    pub dictionary: String,
}

#[native_model(id = 8, version = 1, with = native_model::postcard_1_0::PostCard)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMeta {
    pub character: String,
    pub mode: TermMetaModeType,
    pub data: TermMetaFreqDataMatchType,
    pub dictionary: String,
}

#[native_model(id = 7, version = 1, with = native_model::postcard_1_0::PostCard)]
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseKanjiEntry {
    pub character: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub onyomi: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub kunyomi: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub tags: Option<String>,
    pub meanings: Vec<String>,
    pub stats: Option<IndexMap<String, String>>,
    pub dictionary: Option<String>,
}

#[derive(thiserror::Error, Debug)]
#[error("queries returned None:\n {queries:#?}\n reason: {reason}")]
pub struct QueryRequestError {
    queries: Vec<QueryRequestMatchType>,
    reason: Box<rusqlite::Error>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum QueryRequestMatchType {
    TermExactQueryRequest(TermExactQueryRequest),
    GenericQueryRequest(GenericQueryRequest),
}

#[derive(thiserror::Error, Debug)]
pub enum DictionaryDatabaseError {
    #[error("database error: {0}")]
    Database(#[from] Box<rusqlite::Error>),
    #[error("failed to find terms: {0}")]
    QueryRequest(#[from] QueryRequestError),
    #[error("incorrect variant(s) passed: {wrong:#?}\nexpected: {expected:#?}")]
    WrongQueryRequestMatchType {
        wrong: QueryRequestMatchType,
        expected: QueryRequestMatchType,
    },
}

impl From<rusqlite::Error> for DictionaryDatabaseError {
    fn from(e: rusqlite::Error) -> Self {
        DictionaryDatabaseError::Database(Box::new(e))
    }
}

// Ensure row conversion methods handle errors correctly by converting to DictionaryDatabaseError
impl From<rusqlite::Error> for Box<DictionaryDatabaseError> {
    fn from(e: rusqlite::Error) -> Self {
        Box::new(DictionaryDatabaseError::Database(Box::new(e)))
    }
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
pub struct FindMulitBulkData {
    pub item: FindMultiBulkDataItemType,
    pub item_index: usize,
    pub index_index: usize,
}

pub struct DictionaryDatabase {
    pub conn: Arc<Mutex<Connection>>,
}

impl DictionaryDatabase {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if path.exists() {
            if let Ok(conn) = Connection::open(path) {
                let version_result: Result<i32, _> =
                    conn.query_row("PRAGMA user_version", [], |row| row.get(0));
                if version_result.is_err() {
                    let _ = std::fs::remove_file(path);
                }
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
        let conn = Connection::open(path).expect("Failed to open SQLite database");

        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
        ",
        )
        .expect("Failed to set PRAGMAs");

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.setup_tables().expect("Failed to setup tables");
        db
    }

    fn setup_tables(&self) -> Result<(), rusqlite::Error> {
        self.conn.lock().execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value BLOB
            );
            CREATE TABLE IF NOT EXISTS summaries (
                title TEXT PRIMARY KEY,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS terms (
                id TEXT PRIMARY KEY,
                expression TEXT,
                reading TEXT,
                expression_reverse TEXT,
                reading_reverse TEXT,
                sequence INTEGER,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_terms_expression ON terms(expression);
            CREATE INDEX IF NOT EXISTS idx_terms_reading ON terms(reading);
            CREATE INDEX IF NOT EXISTS idx_terms_expression_reverse ON terms(expression_reverse);
            CREATE INDEX IF NOT EXISTS idx_terms_reading_reverse ON terms(reading_reverse);
            CREATE INDEX IF NOT EXISTS idx_terms_dictionary ON terms(dictionary);

            CREATE TABLE IF NOT EXISTS term_meta (
                id TEXT PRIMARY KEY,
                term TEXT,
                mode TEXT,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_term_meta_term ON term_meta(term);
            CREATE INDEX IF NOT EXISTS idx_term_meta_dictionary ON term_meta(dictionary);

            CREATE TABLE IF NOT EXISTS kanji (
                character TEXT PRIMARY KEY,
                dictionary TEXT,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS kanji_meta (
                character TEXT PRIMARY KEY,
                dictionary TEXT,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
        ",
        )
    }

    pub fn begin_import_session(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -200000;
        ",
        )
    }

    pub fn end_import_session(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "
            PRAGMA temp_store = DEFAULT;
            PRAGMA cache_size = -2000;
        ",
        )
    }

    pub fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT data FROM summaries")?;
        let summaries = stmt
            .query_map([], |row| {
                let data: Vec<u8> = row.get(0)?;
                decode::<DictionarySummary>(data)
                    .map(|(t, _)| t)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })
            })?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;

        let mut summaries = summaries;
        summaries.sort_by_key(|s| s.import_date);
        Ok(summaries)
    }

    pub fn get_settings(&self) -> Result<Option<Vec<u8>>, Box<DictionaryDatabaseError>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'options'")?;
        let mut rows = stmt.query_map([], |row| row.get(0))?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn remove_dictionary_by_name(
        &self,
        name: &str,
    ) -> Result<(), Box<DictionaryDatabaseError>> {
        log::info!("Database: Removing dictionary '{}'...", name);
        let conn = self.conn.lock();
        conn.execute("DELETE FROM summaries WHERE title = ?", [name])?;
        conn.execute("DELETE FROM terms WHERE dictionary = ?", [name])?;
        conn.execute("DELETE FROM term_meta WHERE dictionary = ?", [name])?;
        conn.execute("DELETE FROM kanji WHERE dictionary = ?", [name])?;
        conn.execute("DELETE FROM kanji_meta WHERE dictionary = ?", [name])?;
        conn.execute("DELETE FROM tags WHERE dictionary = ?", [name])?;
        Ok(())
    }

    pub fn find_terms_bulk(
        &self,
        term_list_input: &[impl AsRef<str>],
        dictionaries: &dyn DictionarySet,
        match_type: TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let term_list_refs: Vec<&str> = term_list_input.iter().map(|s| s.as_ref()).collect();
        let (processed_term_list, actual_column) = match match_type {
            TermSourceMatchType::Suffix => (
                term_list_refs
                    .iter()
                    .map(|s| s.chars().rev().collect::<String>())
                    .collect::<Vec<String>>(),
                "expression_reverse",
            ),
            _ => (
                term_list_refs.iter().map(|s| s.to_string()).collect(),
                "expression",
            ),
        };
        let actual_reading_column = if match_type == TermSourceMatchType::Suffix {
            "reading_reverse"
        } else {
            "reading"
        };
        let mut all_final_results: Vec<TermEntry> = Vec::new();
        let mut visited_ids: IndexSet<String> = IndexSet::new();
        if processed_term_list.is_empty() {
            return Ok(all_final_results);
        }
        let conn = self.conn.lock();
        for chunk in processed_term_list.chunks(CHUNKS) {
            let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT data, expression, reading FROM terms WHERE ({} IN ({}) OR {} IN ({}))",
                actual_column, placeholders, actual_reading_column, placeholders
            );
            dbg!(&query, &chunk);
            let mut stmt = conn.prepare(&query)?;
            let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for term in chunk {
                params_vec.push(term);
            }
            for term in chunk {
                params_vec.push(term);
            }
            let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
                let data: Vec<u8> = row.get(0)?;
                let expression: String = row.get(1)?;
                let reading: String = row.get(2)?;
                let (db_model, _) = match decode::<DatabaseTermEntry>(data.clone()) {
                    Ok(val) => val,
                    Err(e) => {
                        println!("DEBUG: Failed to decode expression: {}, reading: {}, data len: {}, data prefix: {:?}", expression, reading, data.len(), &data.get(0..16));
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        ));
                    }
                };
                Ok((db_model, expression, reading))
            })?;
            for row_result in rows {
                let (db_model, expression, reading) = row_result?;
                dbg!(&db_model.dictionary);
                if !dictionaries.has(&db_model.dictionary) {
                    continue;
                }

                for (item_idx, item_to_query) in chunk.iter().enumerate() {
                    let is_match = match match_type {
                        TermSourceMatchType::Exact | TermSourceMatchType::Suffix => {
                            if match_type == TermSourceMatchType::Suffix {
                                let rev_expr: String = expression.chars().rev().collect();
                                let rev_read: String = reading.chars().rev().collect();
                                rev_expr == *item_to_query || rev_read == *item_to_query
                            } else {
                                expression == *item_to_query || reading == *item_to_query
                            }
                        }
                        _ => {
                            expression.starts_with(item_to_query)
                                || reading.starts_with(item_to_query)
                        }
                    };
                    if is_match {
                        let mut current_match_type_for_result = match_type;
                        let index_kind_idx = if expression.contains(item_to_query) {
                            0
                        } else {
                            1
                        };
                        let find_data = FindMulitBulkData {
                            item: FindMultiBulkDataItemType::String(item_to_query.clone()),
                            item_index: item_idx,
                            index_index: index_kind_idx,
                        };
                        let term_entry = db_model
                            .into_term_generic(&mut current_match_type_for_result, find_data);
                        if visited_ids.insert(term_entry.id.clone()) {
                            all_final_results.push(term_entry);
                        }
                        break;
                    }
                }
            }
        }
        Ok(all_final_results)
    }

    pub fn find_terms_exact_bulk(
        &self,
        term_list: &[TermExactQueryRequest],
        dictionaries: &dyn DictionarySet,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let mut results = Vec::new();
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT data FROM terms WHERE expression = ? AND reading = ?")?;
        for (idx, req) in term_list.iter().enumerate() {
            let rows = stmt.query_map([&req.term, &req.reading], |row| {
                let data: Vec<u8> = row.get(0)?;
                decode::<DatabaseTermEntry>(data)
                    .map(|(t, _)| t)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })
            })?;
            for row in rows {
                let entry = row?;
                if dictionaries.has(&entry.dictionary) {
                    results.push(entry.into_term_entry_specific(
                        TermSourceMatchSource::Term,
                        TermSourceMatchType::Exact,
                        idx,
                    ));
                }
            }
        }
        Ok(results)
    }

    pub fn find_term_meta_bulk(
        &self,
        term_list_input: &IndexSet<impl AsRef<str> + Sync>,
        dictionaries: &dyn DictionarySet,
    ) -> Result<Vec<DatabaseTermMeta>, Box<DictionaryDatabaseError>> {
        let terms_as_strings: Vec<String> = term_list_input
            .iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        if terms_as_strings.is_empty() {
            return Ok(Vec::new());
        }
        let mut all_term_meta_results: Vec<DatabaseTermMeta> = Vec::new();
        let conn = self.conn.lock();
        for chunk in terms_as_strings.chunks(CHUNKS) {
            let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT term, mode, data, dictionary FROM term_meta WHERE term IN ({})",
                placeholders
            );
            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
                let term: String = row.get(0)?;
                let mode_str: String = row.get(1)?;
                let data_blob: Vec<u8> = row.get(2)?;
                let dictionary: String = row.get(3)?;
                let mode = match mode_str.as_str() {
                    "freq" => TermMetaModeType::Freq,
                    "pitch" => TermMetaModeType::Pitch,
                    "ipa" => TermMetaModeType::Ipa,
                    _ => panic!("Unknown term meta mode: {}", mode_str),
                };
                let meta_data = match mode {
                    TermMetaModeType::Freq => DatabaseMetaMatchType::Frequency(
                        decode::<DatabaseMetaFrequency>(data_blob)
                            .map(|(t, _)| t)
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Blob,
                                    Box::new(e),
                                )
                            })?,
                    ),
                    TermMetaModeType::Pitch => DatabaseMetaMatchType::Pitch(
                        decode::<DatabaseMetaPitch>(data_blob)
                            .map(|(t, _)| t)
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Blob,
                                    Box::new(e),
                                )
                            })?,
                    ),
                    TermMetaModeType::Ipa => DatabaseMetaMatchType::Phonetic(
                        decode::<DatabaseMetaPhonetic>(data_blob)
                            .map(|(t, _)| t)
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Blob,
                                    Box::new(e),
                                )
                            })?,
                    ),
                };
                Ok((term, mode, meta_data, dictionary))
            })?;
            for row_result in rows {
                let (term, mode, data, dictionary) = row_result?;
                if dictionaries.has(&dictionary) {
                    let index = chunk.iter().position(|t| *t == term).unwrap_or(0);
                    all_term_meta_results.push(DatabaseTermMeta {
                        index,
                        term,
                        mode,
                        data,
                        dictionary,
                    });
                }
            }
        }
        Ok(all_term_meta_results)
    }

    pub fn find_terms_by_sequence_bulk(
        &self,
        items_to_query_vec: Vec<GenericQueryRequest>,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        let mut results = Vec::new();
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT data FROM terms WHERE sequence = ? AND dictionary = ?")?;
        for (idx, req) in items_to_query_vec.iter().enumerate() {
            let seq_val = match req.query_type {
                QueryType::Sequence(seq) => seq,
                _ => panic!("QueryType for sequence search must be Sequence"),
            };
            let rows = stmt.query_map(params![seq_val as i64, req.dictionary], |row| {
                let data: Vec<u8> = row.get(0)?;
                decode::<DatabaseTermEntry>(data)
                    .map(|(t, _)| t)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Blob,
                            Box::new(e),
                        )
                    })
            })?;
            for row in rows {
                let entry = row?;
                results.push(entry.into_term_entry_specific(
                    TermSourceMatchSource::Sequence,
                    TermSourceMatchType::Exact,
                    idx,
                ));
            }
        }
        Ok(results)
    }

    pub fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        let mut results: Vec<Option<DatabaseTag>> = vec![None; queries.len()];
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT data FROM tags WHERE name = ? AND dictionary = ?")?;
        for (idx, req) in queries.iter().enumerate() {
            let name = match &req.query_type {
                QueryType::String(name) => name,
                _ => continue,
            };
            let mut rows = stmt.query_map([name, &req.dictionary], |row| {
                let data: Vec<u8> = row.get(0)?;
                decode::<DatabaseTag>(data).map(|(t, _)| t).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })
            })?;
            if let Some(row) = rows.next() {
                let tag = row?;
                results[idx] = Some(tag);
            }
        }
        Ok(results)
    }
}

impl crate::database::DictionaryService for DictionaryDatabase {
    fn get_settings(&self) -> Result<Option<Vec<u8>>, Box<DictionaryDatabaseError>> {
        self.get_settings()
    }

    fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>> {
        self.get_dictionary_summaries()
    }

    fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>> {
        self.find_tag_meta_bulk(queries)
    }

    fn find_term_meta_bulk(
        &self,
        keys: &indexmap::IndexSet<String>,
        enabled_dictionaries: &dyn DictionarySet,
    ) -> Result<Vec<DatabaseTermMeta>, Box<DictionaryDatabaseError>> {
        self.find_term_meta_bulk(keys, enabled_dictionaries)
    }

    fn find_terms_exact_bulk(
        &self,
        terms: &[TermExactQueryRequest],
        enabled_dictionaries: &dyn DictionarySet,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>> {
        self.find_terms_exact_bulk(terms, enabled_dictionaries)
    }

    fn find_terms_by_sequence_bulk(
        &self,
        queries: Vec<GenericQueryRequest>,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>> {
        self.find_terms_by_sequence_bulk(queries)
    }

    fn find_terms_bulk(
        &self,
        term_list: &[String],
        dictionaries: &dyn DictionarySet,
        match_type: TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, Box<DictionaryDatabaseError>> {
        self.find_terms_bulk(term_list, dictionaries, match_type)
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
    use super::*;
    use crate::utils::test_utils;

    #[test]
    fn find_terms_sequence_bulk() {
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let queries = &[QueryType::Sequence(0)];
        let queries_generic_req =
            GenericQueryRequest::from_query_type_slice_to_vec(queries, "小学館 西和中辞典 第2版");
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
    fn verify_data_integrity() {
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let conn = ycd.conn.lock();
        let data: Vec<u8> = conn
            .query_row("SELECT data FROM terms LIMIT 1", [], |row| row.get(0))
            .expect("Failed to get data from terms");
        let decoded = decode::<DatabaseTermEntry>(data);
        match decoded {
            Ok((entry, _)) => {
                println!("Successfully decoded entry: {:?}", entry.id);
            }
            Err(e) => {
                panic!("Failed to decode DatabaseTermEntry: {:?}", e);
            }
        }
    }
}
