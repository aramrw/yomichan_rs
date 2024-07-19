use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFrequencyData, Tag as DictDataTag, TermGlossary, TermGlossaryContent,
    TermMetaFrequencyDataType, TermMetaModeType, TermMetaPhoneticData, TermMetaPitchData,
};
use crate::dictionary_importer::{prepare_dictionary, Summary};
use crate::errors::DBError;
use crate::Yomichan;

use bincode::Error;
use lindera::{LinderaError, Token, Tokenizer};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use native_model::{native_model, Model};
use once_cell::sync::Lazy;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};

pub static DB_MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<DatabaseTermEntry>().unwrap();
    models
});

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::Path;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct DatabaseTermEntry {
    #[secondary_key]
    pub expression: String,
    #[secondary_key]
    pub reading: String,
    pub expression_reverse: String,
    pub reading_reverse: String,
    pub definition_tags: Option<String>,
    /// Legacy alias for the `definitionTags` field.
    pub tags: Option<String>,
    pub rules: String,
    pub score: i8,
    pub glossary: TermGlossaryContent,
    #[primary_key]
    pub sequence: Option<i128>,
    pub term_tags: Option<String>,
    pub dictionary: String,
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
pub struct Tag {
    name: String,
    category: String,
    order: i32,
    notes: String,
    score: i32,
    dictionary: String,
}

/*************** Database Term Meta ***************/

/// A custom `Yomichan_rs`-unique, generic Database Meta model.
///
/// May contain `any` or `all` of the values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseMeta {
    pub frequency: Option<DatabaseMetaFrequency>,
    pub pitch: Option<DatabaseMetaPitch>,
    pub phonetic: Option<DatabaseMetaPhonetic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseMetaFrequency {
    pub expression: String,
    /// Is of type [`TermMetaModeType::Freq`]
    pub mode: TermMetaModeType,
    pub data: TermMetaFrequencyDataType,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseMetaPitch {
    pub expression: String,
    /// Is of type [`TermMetaModeType::Pitch`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPitchData,
    pub dictionary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseMetaPhonetic {
    pub expression: String,
    /// Is of type [`TermMetaModeType::Ipa`]
    pub mode: TermMetaModeType,
    pub data: TermMetaPhoneticData,
    pub dictionary: String,
}

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum DatabaseTermMetaMatchType {
//     Frequency(DatabaseTermMetaFrequency),
//     Pitch(DatabaseTermMetaPitch),
//     Phonetic(DatabaseTermMetaPhonetic),
// }

/*************** Database Kanji Meta ***************/

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseKanjiMetaFrequency {
    pub character: String,
    /// Is of type [`TermMetaModeType::Freq`]
    pub mode: TermMetaModeType,
    pub data: TermMetaFrequencyDataType,
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

pub trait DictionarySet {
    fn has(&self, value: &str) -> bool;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseDictData {
    pub tag_list: Vec<Vec<DictDataTag>>,
    pub kanji_meta_list: Vec<DatabaseMeta>,
    pub kanji_list: Vec<DatabaseKanjiEntry>,
    pub term_meta_list: Vec<DatabaseMeta>,
    pub term_list: Vec<DatabaseTermEntry>,
    pub summary: Summary,
}

/// Defines each [`redb`] store, containing serialized `Database` objects.
/// Each entry in the table is serialized into a byte slice _(`&[u8]`)_ before storage.
// pub mod db_stores {
//     use redb::TableDefinition;
//
//     /// Mapped to [`dictionary_importer::Summary`].
//     ///
//     /// [`dictionary_importer::Summary`]: dictionary_importer::Summary
//     pub const DICTIONARIES_STORE: TableDefinition<&str, &[u8]> =
//         TableDefinition::new("dictionaries");
//     /// Mapped to [`DatabaseTermEntry`].
//     ///
//     /// [`DatabaseTermEntry`]: DatabaseTermEntry
//     pub const TERMS_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("terms");
//     /// Mapped to [`DatabaseTermMeta`].
//     ///
//     /// [`DatabaseTermMeta`]: DatabaseTermMeta
//     pub const TERM_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("term_meta");
//     /// Mapped to [`DatabaseKanjiEntry`].
//     ///
//     /// [`DatabaseKanjiEntry`]: DatabaseKanjiEntry
//     pub const KANJI_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("kanji");
//     /// Mapped to [`DatabaseKanjiMeta`].
//     ///
//     /// [`DatabaseKanjiMeta`]: DatabaseKanjiMeta
//     pub const KANJI_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("kanji_meta");
//     /// Mapped to [`Tag`].
//     ///
//     /// [`Tag`]: Tag
//     pub const TAG_META_STORE: TableDefinition<&str, &[u8]> = TableDefinition::new("tag_meta");
//     /// Mapped to [`MediaDataArrayBufferContent`].
//     ///
//     /// [`MediaDataArrayBufferContent`]: MediaDataArrayBufferContent
//     pub const MEDIA: TableDefinition<&str, &[u8]> = TableDefinition::new("media");
// }

impl Yomichan {
    pub fn import_dictionary<P: AsRef<Path>>(&mut self, zip_path: P) -> Result<(), DBError> {
        let data = prepare_dictionary(zip_path, &mut self.options)?;
        let terms = data.term_list;
        let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        {
            let rwtx = db.rw_transaction()?;
            for t in terms {
                rwtx.insert(t)?;
            }
            rwtx.commit()?;
        }

        Ok(())
    }

    /// Looks up a term in the database
    pub fn bulk_lookup<Q: AsRef<str>>(&self, query: Q) -> Result<Vec<DatabaseTermEntry>, DBError> {
        let tokenizer = init_tokenizer()?;
        let tokens = tokenizer.tokenize(query.as_ref())?;
        let tokens: Vec<&str> = process_tokens(tokens);
        let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;

        let rtx = db.r_transaction()?;

        let entries: Result<Vec<Vec<DatabaseTermEntry>>, native_db::db_type::Error> = tokens
            .iter()
            .map(|tok| {
                rtx.scan()
                    .secondary(DatabaseTermEntryKey::expression)?
                    .start_with(query.as_ref())
                    .collect::<Result<Vec<DatabaseTermEntry>, native_db::db_type::Error>>()
            })
            .collect();

        let entries = match entries {
            Ok(ent) => {
                if ent.is_empty() {
                    return Err(DBError::Query(format!(
                        "no entries found for: {}",
                        query.as_ref()
                    )));
                }
                ent.into_iter().flatten().collect()
            }
            Err(e) => return Err(DBError::Query(format!("bulk query err: | {}", e))),
        };

        Ok(entries)
    }
}

fn process_tokens(tokens: Vec<Token>) -> Vec<&str> {
    tokens.iter().map(|t| t.text).collect()
}

fn init_tokenizer() -> Result<Tokenizer, LinderaError> {
    use lindera::{
        DictionaryConfig, DictionaryKind, LinderaResult, Mode, Tokenizer, TokenizerConfig,
    };

    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };

    let config = TokenizerConfig {
        dictionary,
        user_dictionary: None,
        mode: Mode::Normal,
    };

    let tokenizer = Tokenizer::from_config(config)?;
    //let tokens = tokenizer.tokenize(query.as_ref())?;

    Ok(tokenizer)
}
