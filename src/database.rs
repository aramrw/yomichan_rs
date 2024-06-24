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

// ! Javascript's `ArrayBuffer` most likely fundamentally differs from any Rust ds'
// ! Should look up how to map these before continuing
// ! So media can be displayed (or at least stored) within the lib
// TODO https://github.com/themoeway/yomitan/blob/8eea3661714c64857aa32a8662f7cca6674dd3a4/types/ext/dictionary-database.d.ts#L41
#[derive(Serialize, Deserialize, Debug)]
pub struct MediaDataBase<TContentType> {
    dictionary: String,
    path: String,
    media_type: String,
    width: u16,
    height: u16,
    content: TContentType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseTermEntry {
    expression: String,
    reading: String,
    expression_reverse: String,
    reading_reverse: String,
    definition_tags: Option<String>,
    tags: String,
    rules: String,
    score: u16,
    glossary: Vec<TermGlossary>,
    sequence: i64,
    term_tags: String,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseKanjiEntry {
    character: String,
    onyomi: String,
    kunyomi: String,
    tags: String,
    meanings: Vec<String>,
    dictionary: String,
    stats: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    name: String,
    category: String,
    order: i32,
    notes: String,
    score: i32,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseTermMetaFrequency {
    expression: String,
    /// Is of type `TermMetaModeType::Freq`
    mode: TermMetaModeType,
    data: TermMetaFrequencyDataType,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseTermMetaPitch {
    expression: String,
    /// Is of type `TermMetaModeType::Pitch`
    mode: TermMetaModeType,
    data: TermMetaPitchData,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseTermMetaPhoneticData {
    expression: String,
    /// Is of type `TermMetaModeType::Ipa`
    mode: TermMetaModeType,
    data: TermMetaPhoneticData,
    dictionary: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DatabaseTermMeta {
    Frequency(DatabaseTermMetaFrequency),
    Pitch(DatabaseTermMetaPitch),
    Phonetic(DatabaseTermMetaPhoneticData),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseKanjiMetaFrequency {
    index: u16,
    character: String,
    /// Is of type `TermMetaModeType::Frequency`
    mode: TermMetaModeType,
    data: GenericFrequencyData,
    dictionary: String,
}

pub type DictionaryCountGroup = HashMap<String, u16>;

#[derive(Serialize, Deserialize, Debug)]
pub struct DictionaryCounts {
    total: Option<DictionaryCountGroup>,
    counts: Vec<DictionaryCountGroup>,
}

/// Defines each `redb` store, containing serialized `Database` objects.
/// Each entry in the table is serialized into a byte slice (`&[u8]`) before storage.
pub mod db_stores {
    use redb::TableDefinition;

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
        let tx = self.db.begin_write()?;
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
        let tx = self.db.begin_read()?;
        let table = tx.open_table(db_stores::TERMS_STORE)?;

        if let Some(value_guard) = table.get(key)? {
            let stored_term: TermEntry = bincode::deserialize(value_guard.value())?;
            Ok(Some(stored_term))
        } else {
            Ok(None)
        }
    }
}
