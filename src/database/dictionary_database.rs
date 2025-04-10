use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFreqData, Tag as DictDataTag, TermGlossary, TermGlossaryContent, TermMetaDataMatchType,
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
use std::sync::LazyLock;
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
    pub expression_reverse: String,
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
pub struct Tag {
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

pub trait DictionarySet {
    fn has(&self, value: &str) -> bool;
}

trait DBReadWrite {
    fn rw_insert(&self, db: Database) -> Result<(), DBError>;
}

pub type VecTermEntry = Vec<TermEntry>;
pub type VecDBTermEntry = Vec<DatabaseTermEntry>;
pub type VecDBTermMeta = Vec<DatabaseMeta>;
pub type VecDBMetaFreq = Vec<DatabaseMetaFrequency>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseDictData {
    pub tag_list: Vec<Vec<DictDataTag>>,
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
