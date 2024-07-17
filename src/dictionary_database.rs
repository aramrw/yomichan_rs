use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFrequencyData, TermGlossary, TermMetaFrequencyDataType, TermMetaModeType,
    TermMetaPhoneticData, TermMetaPitchData,
};
use crate::errors;
use crate::Yomichan;

//use redb::TableDefinition;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermEntry {
    expression: String,
    reading: String,
    expression_reverse: Option<String>,
    reading_reverse: Option<String>,
    definition_tags: Option<String>,
    tags: Option<String>,
    rules: String,
    score: u16,
    glossary: Vec<TermGlossary>,
    sequence: Option<i64>,
    term_tags: Option<String>,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermEntry {
    index: u16,
    match_type: TermSourceMatchType,
    match_source: TermSourceMatchSource,
    term: String,
    reading: String,
    definition_tags: String,
    term_tags: Vec<String>,
    rules: Vec<String>,
    definitions: Vec<TermGlossary>,
    score: u16,
    dictionary: String,
    id: u128,
    sequence: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiEntry {
    character: String,
    onyomi: String,
    kunyomi: String,
    tags: String,
    meanings: Vec<String>,
    dictionary: String,
    stats: Option<std::collections::HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiEntry {
    index: i32,
    character: String,
    onyomi: Vec<String>,
    kunyomi: Vec<String>,
    tags: Vec<String>,
    definitions: Vec<String>,
    stats: std::collections::HashMap<String, String>,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    name: String,
    category: String,
    order: i32,
    notes: String,
    score: i32,
    dictionary: String,
}

/*************** Database Term Meta ***************/

/// A custom `Yomichan_rs`-unique Database Term Meta model.  
///
/// May contain `any` or `all` of the values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMeta {
    pub frequency: Option<DatabaseTermMetaFrequency>,
    pub pitch: Option<DatabaseTermMetaPitch>,
    pub phonetic: Option<DatabaseTermMetaPhonetic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMetaFrequency {
    expression: String,
    /// Is of type `TermMetaModeType::Freq`
    mode: TermMetaModeType,
    data: TermMetaFrequencyDataType,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMetaPitch {
    expression: String,
    /// Is of type `TermMetaModeType::Pitch`
    mode: TermMetaModeType,
    data: TermMetaPitchData,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseTermMetaPhoneticData {
    expression: String,
    /// Is of type [`TermMetaModeType::Ipa`]
    mode: TermMetaModeType,
    data: TermMetaPhoneticData,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DatabaseTermMeta {
    Frequency(DatabaseTermMetaFrequency),
    Pitch(DatabaseTermMetaPitch),
    Phonetic(DatabaseTermMetaPhoneticData),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMetaFrequency {
    index: u16,
    character: String,
    /// Is of type `TermMetaModeType::Frequency`
    mode: TermMetaModeType,
    data: GenericFrequencyData,
    dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiEntry {
    character: String,
    onyomi: String,
    kunyomi: String,
    tags: String,
    meanings: Vec<String>,
    dictionary: String,
    stats: Option<std::collections::HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanjiEntry {
    index: i32,
    character: String,
    onyomi: Vec<String>,
    kunyomi: Vec<String>,
    tags: Vec<String>,
    definitions: Vec<String>,
    stats: std::collections::HashMap<String, String>,
    dictionary: String,
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

pub trait DictionarySet {
    fn has(&self, value: &str) -> bool;
}

/// Defines each `redb` store, containing serialized `Database` objects.
/// Each entry in the table is serialized into a byte slice (`&[u8]`) before storage.
pub mod db_stores {
    use redb::TableDefinition;

    /// Mapped to [`dictionary_importer::Summary`].
    ///
    /// [`dictionary_importer::Summary`]: dictionary_importer::Summary
    pub const DICTIONARIES_STORE: TableDefinition<&str, &[u8]> =
        TableDefinition::new("dictionaries");
    /// Mapped to [`DatabaseTermEntry`].
    ///
    /// [`DatabaseTermEntry`]: DatabaseTermEntry
    pub const TERMS_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("terms");
    /// Mapped to [`DatabaseTermMeta`].
    ///
    /// [`DatabaseTermMeta`]: DatabaseTermMeta
    pub const TERM_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("term_meta");
    /// Mapped to [`DatabaseKanjiEntry`].
    ///
    /// [`DatabaseKanjiEntry`]: DatabaseKanjiEntry
    pub const KANJI_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("kanji");
    /// Mapped to [`DatabaseKanjiMeta`].
    ///
    /// [`DatabaseKanjiMeta`]: DatabaseKanjiMeta
    pub const KANJI_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("kanji_meta");
    /// Mapped to [`Tag`].
    ///
    /// [`Tag`]: Tag
    pub const TAG_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("tag_meta");
    /// Mapped to [`MediaDataArrayBufferContent`].
    ///
    /// [`MediaDataArrayBufferContent`]: MediaDataArrayBufferContent
    pub const MEDIA: TableDefinition<&str, &[u8]> = TableDefinition::new("media");
}

impl Yomichan {
    /// Adds a term entry to the database
    pub fn add_term(&self, key: &str, term: TermEntry) -> Result<(), errors::DBError> {
        let tx = self.ycdatabase.db.begin_write()?;
        {
            let mut table = tx.open_table(db_stores::TERMS_STORE)?;

            let term_bytes = bincode::serialize(&term)?;
            table.insert(key, &*term_bytes)?;
        }
        tx.commit()?;

        Ok(())
    }

    /// Looks up a term in the database
    pub fn lookup_term(&self, key: &str) -> Result<Option<TermEntry>, errors::DBError> {
        let tx = self.ycdatabase.db.begin_read()?;
        let table = tx.open_table(db_stores::TERMS_STORE)?;

        if let Some(value_guard) = table.get(key)? {
            let stored_term: TermEntry = bincode::deserialize(value_guard.value())?;
            Ok(Some(stored_term))
        } else {
            Ok(None)
        }
    }
}


