use crate::backend::Backend;
use crate::database::dictionary_database::{
    DatabaseDictData, DatabaseKanjiEntry, DatabaseKanjiMeta, DatabaseMetaFrequency,
    DatabaseMetaMatchType, DatabaseMetaPhonetic, DatabaseMetaPitch, DatabaseTermEntry, KanjiEntry,
    MediaDataArrayBufferContent, TermEntry, DB_MODELS,
};
use crate::dictionary::{self, KanjiDictionaryEntry};
use crate::dictionary_data::{
    self, dictionary_data_util, DictionaryDataTag, Index, MetaDataMatchType, TermGlossaryImage,
    TermMeta, TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPitchData,
    TermV3, TermV4,
};
use crate::settings::{
    self, DictionaryDefinitionsCollapsible, DictionaryOptions, ProfileError, ProfileResult,
    YomichanOptions, YomichanProfile,
};
use crate::structured_content::{
    ContentMatchType, Element, LinkElement, StructuredContent, TermEntryItem,
};

use crate::errors::{DBError, DictionaryFileError, ImportError, ImportZipError};
use crate::{test_utils, Ptr, Yomichan};

use indexmap::IndexMap;
use native_db::transaction::RwTransaction;
use native_db::ToInput;
use native_db::{native_db, transaction::query::PrimaryScan, Builder as DBBuilder, ToKey};
use native_model::{native_model, Model};

use chrono::prelude::*;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use serde_untagged::UntaggedEnumVisitor;

use rayon::prelude::*;

use tempfile::tempdir;
use uuid::Uuid;

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use std::time::Instant;
use std::{fs, io, mem};

use super::dictionary_database::{DatabaseTag, DatabaseTermMeta, DictionaryDatabase};

//use chrono::{DateTime, Local};

impl Yomichan<'_> {
    pub fn import_dictionaries<P: AsRef<Path> + Send + Sync>(
        &mut self,
        zip_paths: &[P],
    ) -> Result<(), ImportError> {
        self.backend.import_dictionaries_internal(zip_paths);
        Ok(())
    }
}

impl Backend<'_> {
    pub fn import_dictionaries_internal<P: AsRef<Path> + Send + Sync>(
        &mut self,
        zip_paths: &[P],
    ) -> Result<(), ImportError> {
        let db = &self.db;
        let current_profile = self.get_current_profile()?;
        ImportZipError::check_zip_paths(zip_paths)?;

        let options: Vec<DictionaryOptions> = zip_paths
            .par_iter()
            .map(|path| import_dictionary(path, db, current_profile.clone()))
            .collect::<Result<Vec<DictionaryOptions>, ImportError>>()?;

        let mut dictionary_opts: IndexMap<String, DictionaryOptions> = options
            .into_iter()
            .map(|opt| (opt.name.clone(), opt))
            .collect();

        let global_options = self.options.clone();
        let res: Result<(), ProfileError> = global_options.with_ptr_mut(|gopts| {
            let current_profile_ptr: Ptr<YomichanProfile> = gopts.get_current_profile()?;
            current_profile_ptr.with_ptr_mut(|current_profile| {
                let mut main_dictionary = current_profile.get_main_dictionary();
                if main_dictionary.is_empty() {
                    let name = dictionary_opts
                        .get_index(0)
                        .expect("[unexpected] dictionary options created but len is 0");
                    current_profile.set_main_dictionary(name.0.to_string());
                }
                current_profile.extend_dictionaries(dictionary_opts);
            });
            Ok(())
        });
        res?;

        let rwtx = db.rw_transaction()?;
        db_rwriter(&rwtx, vec![global_options.read().clone()]);
        rwtx.commit()?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ImportSteps {
    Uninitialized,
    ValidateIndex,
    ValidateSchema,
    FormatDictionary,
    ImportMedia,
    ImportData,
    Completed,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CompiledSchemaNames {
    TermBank,
    /// Metadata & information for terms.
    ///
    /// This currently includes `frequency data` and `pitch accent` data.
    TermMetaBank,
    KanjiBank,
    KanjiMetaBank,
    /// Data file containing tag information for terms and kanji.
    TagBank,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportResult {
    result: Option<DictionarySummary>,
    //errors: Vec<ImportError>,
}

#[derive(Clone, Debug, PartialEq, Copy, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ImportDetails {
    prefix_wildcards_supported: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FrequencyMode {
    #[serde(rename = "occurrence-based")]
    OccurrenceBased,
    #[serde(rename = "rank-based")]
    RankBased,
}

// Final details about the Dictionary and it's import process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_db]
#[native_model(id = 1, version = 1)]
pub struct DictionarySummary {
    /// Name of the dictionary.
    #[primary_key]
    pub title: String,
    /// Revision of the dictionary.
    /// This value is only used for displaying information.
    pub revision: String,
    /// Whether or not this dictionary contains sequencing information for related terms.
    pub sequenced: Option<bool>,
    /// The minimum Yomitan version necessary for the dictionary to function
    pub minimum_yomitan_version: Option<String>,
    /// Format of data found in the JSON data files.
    pub version: Option<u8>,
    /// Date the dictionary was added to the db.
    pub import_date: DateTime<Local>,
    /// Whether or not wildcards can be used for the search query.
    ///
    /// Rather than searching for the source text exactly,
    /// the text will only be required to be a prefix of an existing term.
    /// For example, scanning `読み` will effectively search for `読み*`
    /// which may bring up additional results such as `読み方`.
    pub prefix_wildcards_supported: bool,
    pub counts: SummaryCounts,
    /// Creator of the dictionary.
    pub styles: String,
    pub is_updatable: bool,
    pub index_url: Option<String>,
    pub download_url: Option<String>,
    pub author: Option<String>,
    /// URL for the source of the dictionary.
    pub url: Option<String>,
    /// Description of the dictionary data.
    pub description: Option<String>,
    /// Attribution information for the dictionary data.
    pub attribution: Option<String>,
    /// Language of the terms in the dictionary.
    #[secondary_key]
    pub source_language: Option<String>,
    /// Main language of the definitions in the dictionary.
    #[secondary_key]
    pub target_language: Option<String>,
    /// (See: [FrequencyMode])
    pub frequency_mode: Option<FrequencyMode>,
}

#[derive(thiserror::Error, Debug)]
pub enum DictionarySummaryError {
    #[error("dictionary is incompatible with current version of Yomitan: (${yomitan_version}; minimum required: ${minimum_required_yomitan_version}); dictionary: {dictionary}")]
    IncompatibleYomitanVersion {
        yomitan_version: String,
        minimum_required_yomitan_version: String,
        dictionary: String,
    },
    #[error("invalid index data: `is_updatable` exists but is false")]
    InvalidIndexIsNotUpdatabale,
    #[error("index url: {url} is not a valid url\nreason: {err}")]
    InvalidIndexUrl { url: String, err: url::ParseError },
}

impl DictionarySummary {
    fn new(
        index: Index,
        prefix_wildcards_supported: bool,
        details: SummaryDetails,
    ) -> Result<Self, DictionarySummaryError> {
        let import_date: DateTime<Local> = Local::now();
        let SummaryDetails {
            prefix_wildcard_supported,
            counts,
            styles,
            yomitan_version,
        } = details;
        let Index {
            title,
            revision,
            sequenced,
            format,
            version,
            minimum_yomitan_version,
            is_updatable,
            index_url,
            download_url,
            author,
            url,
            description,
            attribution,
            source_language,
            target_language,
            frequency_mode,
            tag_meta,
        } = index;

        if yomitan_version == "0.0.0.0" {
            // running development version
        } else if let Some(minimum_yomitan_version) = &minimum_yomitan_version {
            if dictionary_data_util::compare_revisions(&yomitan_version, minimum_yomitan_version) {
                return Err(DictionarySummaryError::IncompatibleYomitanVersion {
                    yomitan_version,
                    minimum_required_yomitan_version: minimum_yomitan_version.clone(),
                    dictionary: title,
                });
            }
        }

        if let Some(is_updatable) = is_updatable {
            if !is_updatable {
                return Err(DictionarySummaryError::InvalidIndexIsNotUpdatabale);
            }
            if let Some(index_url) = &index_url {
                if let Err(err) = dictionary_data_util::validate_url(index_url) {
                    return Err(DictionarySummaryError::InvalidIndexUrl {
                        url: index_url.clone(),
                        err,
                    });
                }
            }
            if let Some(download_url) = &download_url {
                if let Err(err) = dictionary_data_util::validate_url(download_url) {
                    return Err(DictionarySummaryError::InvalidIndexUrl {
                        url: download_url.clone(),
                        err,
                    });
                }
            }
        }

        let res = Self {
            title,
            revision,
            sequenced,
            minimum_yomitan_version,
            version,
            import_date,
            prefix_wildcards_supported,
            counts,
            styles,
            is_updatable: is_updatable.unwrap_or_default(),
            index_url,
            download_url,
            author,
            url,
            description,
            attribution,
            source_language,
            target_language,
            frequency_mode,
        };
        Ok(res)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryDetails {
    pub prefix_wildcard_supported: bool,
    pub counts: SummaryCounts,
    // some kind of styles.css file stuff
    pub styles: String,
    pub yomitan_version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryCounts {
    pub terms: SummaryItemCount,
    pub term_meta: SummaryMetaCount,
    pub kanji: SummaryItemCount,
    pub kanji_meta: SummaryMetaCount,
    pub tag_meta: SummaryItemCount,
    pub media: SummaryItemCount,
}

impl SummaryCounts {
    fn new(
        term_len: usize,
        term_meta_len: usize,
        tag_len: usize,
        kanji_len: usize,
        kanji_meta_len: usize,
        term_meta_counts: MetaCounts,
        kanji_meta_counts: MetaCounts,
    ) -> Self {
        Self {
            terms: SummaryItemCount {
                total: term_len as u16,
            },
            term_meta: SummaryMetaCount {
                total: term_meta_len as u16,
                meta: term_meta_counts,
            },
            tag_meta: SummaryItemCount {
                total: tag_len as u16,
            },
            kanji_meta: SummaryMetaCount {
                total: kanji_meta_len as u16,
                meta: kanji_meta_counts,
            },
            kanji: SummaryItemCount {
                total: kanji_len as u16,
            },
            // Can't deserialize media (yet).
            media: SummaryItemCount { total: 0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryItemCount {
    pub total: u16,
}

impl SummaryItemCount {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryMetaCount {
    pub total: u16,
    pub meta: MetaCounts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct MetaCounts {
    pub freq: u32,
    pub pitch: u32,
    pub ipa: u32,
}

impl MetaCounts {
    fn count_kanji_metas(kanji_metas: &[DatabaseMetaFrequency]) -> Self {
        MetaCounts {
            freq: kanji_metas.len() as u32,
            ..Default::default()
        }
    }
    fn count_term_metas(metas: &[DatabaseMetaMatchType]) -> Self {
        let mut meta_counts = MetaCounts::default();

        for database_meta_match_type in metas.iter() {
            match database_meta_match_type {
                DatabaseMetaMatchType::Frequency(_) => meta_counts.freq += 1,
                DatabaseMetaMatchType::Pitch(_) => meta_counts.pitch += 1,
                DatabaseMetaMatchType::Phonetic(_) => meta_counts.ipa += 1,
            }
        }

        meta_counts
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImageImportMatchType {
    Image,
    StructuredContentImage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageImportRequirement {
    /// This is of type [`ImageImportType::Image`]
    image_type: ImageImportMatchType,
    target: TermGlossaryImage,
    source: TermGlossaryImage,
    entry: DatabaseTermEntry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuredContentImageImportRequirement {
    /// This is of type [`ImageImportType::StructuredContentImage`]
    image_type: ImageImportMatchType,
    target: TermGlossaryImage,
    source: TermGlossaryImage,
    entry: DatabaseTermEntry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportRequirementContext {
    //file_map: ArchiveFileMap,
    media: IndexMap<String, MediaDataArrayBufferContent>,
}
/// Deserializable type mapping a `term_bank_$i.json` file.
pub type TermBank = Vec<TermEntryItem>;
pub type TermMetaBank = Vec<TermMeta>;
pub type KanjiBank = Vec<DatabaseKanjiEntry>;

fn extract_dict_zip<P: AsRef<std::path::Path>>(
    zip_path: P,
) -> Result<std::path::PathBuf, ImportZipError> {
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path().to_owned();
    let temp_dir_path_clone = temp_dir_path.clone();

    {
        let file = fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let extract_handle = std::thread::spawn(move || archive.extract(temp_dir_path_clone));
        extract_handle.join().unwrap().unwrap();
    }

    temp_dir.close()?;
    Ok(temp_dir_path)
}

pub fn import_dictionary<P: AsRef<Path>>(
    zip_path: P,
    db: &DictionaryDatabase,
    current_profile: Ptr<YomichanProfile>,
) -> Result<DictionaryOptions, ImportError> {
    let data: DatabaseDictData = prepare_dictionary(zip_path, current_profile)?;
    let rwtx = db.rw_transaction()?;
    db_rwriter(&rwtx, data.term_list)?;
    db_rwriter(&rwtx, data.kanji_list)?;
    db_rwriter(&rwtx, data.tag_list)?;
    db_rwriter(&rwtx, data.kanji_meta_list)?;
    {
        for item in data.term_meta_list {
            match item {
                DatabaseMetaMatchType::Frequency(freq) => rwtx.insert(freq)?,
                DatabaseMetaMatchType::Pitch(pitch) => rwtx.insert(pitch)?,
                DatabaseMetaMatchType::Phonetic(ipa) => rwtx.insert(ipa)?,
            }
        }
    }
    db_rwriter(&rwtx, vec![data.summary])?;

    rwtx.commit()?;
    Ok(data.dictionary_options)
}

fn db_rwriter<L: ToInput>(
    rwtx: &RwTransaction,
    list: Vec<L>,
) -> Result<(), Box<native_db::db_type::Error>> {
    for item in list {
        rwtx.insert(item)?;
    }
    Ok(())
}

pub fn prepare_dictionary<P: AsRef<Path>>(
    zip_path: P,
    current_profile: Ptr<YomichanProfile>,
) -> Result<DatabaseDictData, ImportError> {
    //let instant = Instant::now();
    //let temp_dir_path = extract_dict_zip(zip_path)?;

    let mut index_path = PathBuf::new();
    let mut tag_bank_paths: Vec<PathBuf> = Vec::new();
    let mut kanji_meta_bank_paths: Vec<PathBuf> = Vec::new();
    let mut kanji_bank_paths: Vec<PathBuf> = Vec::new();
    let mut term_meta_bank_paths: Vec<PathBuf> = Vec::new();
    let mut term_bank_paths: Vec<PathBuf> = Vec::new();

    read_dir_helper(
        zip_path,
        &mut index_path,
        &mut tag_bank_paths,
        &mut kanji_meta_bank_paths,
        &mut kanji_bank_paths,
        &mut term_meta_bank_paths,
        &mut term_bank_paths,
    );

    let index: Index = convert_index_file(index_path)?;
    let dict_name = index.title.clone();
    // check if dict exists before continuing
    if current_profile
        .read()
        .get_dictionary_options_from_name(&dict_name)
        .is_some()
    {
        return Err(ImportError::DictionaryAlreadyExists(dict_name));
    }

    let tag_banks: Result<Vec<Vec<DatabaseTag>>, ImportError> =
        convert_tag_bank_files(tag_bank_paths, &dict_name);
    let tag_list: Vec<DatabaseTag> = match tag_banks {
        Ok(kml) => kml.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert tag banks | {e}"
            )))
        }
    };

    let term_banks: Result<Vec<Vec<DatabaseTermEntry>>, DictionaryFileError> = term_bank_paths
        .into_par_iter()
        .map(|path| convert_term_bank_file(path, &dict_name))
        .collect::<Result<Vec<Vec<DatabaseTermEntry>>, DictionaryFileError>>();
    let term_list: Vec<DatabaseTermEntry> = match term_banks {
        Ok(tl) => tl.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term banks | {e}"
            )));
        }
    };

    // ------------- TESTING ----------------
    // let jigoujitoku = term_list.iter().find(|term| term.expression == "自業自得");
    // let path = test_utils::TEST_PATHS
    //     .tests_dir
    //     .join("自業自得_rust")
    //     .with_extension("json");
    // if let Some(jt) = jigoujitoku {
    //     let vec = serde_json::to_vec_pretty(&[jt]).unwrap();
    //     std::fs::write(&path, vec).unwrap();
    // }
    // ------------- TESTING ----------------

    let kanji_meta_banks: Result<Vec<Vec<DatabaseMetaFrequency>>, DictionaryFileError> =
        kanji_meta_bank_paths
            .into_par_iter()
            .map(|path| DatabaseMetaMatchType::convert_kanji_meta_file(path, dict_name.clone()))
            .collect::<Result<Vec<Vec<DatabaseMetaFrequency>>, DictionaryFileError>>();

    let kanji_meta_list: Vec<DatabaseMetaFrequency> = match kanji_meta_banks {
        Ok(kml) => kml.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji_meta_banks | {e}"
            )))
        }
    };

    let term_meta_banks: Result<Vec<Vec<DatabaseMetaMatchType>>, DictionaryFileError> =
        term_meta_bank_paths
            .into_par_iter()
            .map(|path| DatabaseMetaMatchType::convert_term_meta_file(path, dict_name.clone()))
            .collect::<Result<Vec<Vec<DatabaseMetaMatchType>>, DictionaryFileError>>();

    let term_meta_list: Vec<DatabaseMetaMatchType> = match term_meta_banks {
        Ok(tml) => tml.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term_meta_banks | {e}"
            )))
        }
    };

    let kanji_banks: Result<Vec<Vec<DatabaseKanjiEntry>>, DictionaryFileError> = kanji_bank_paths
        .into_iter()
        .map(|path| convert_kanji_bank(path, &dict_name))
        .collect::<Result<Vec<Vec<DatabaseKanjiEntry>>, DictionaryFileError>>();

    let kanji_list: Vec<DatabaseKanjiEntry> = match kanji_banks {
        Ok(kl) => kl.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji banks | {e}"
            )))
        }
    };

    let term_meta_counts = MetaCounts::count_term_metas(&term_meta_list);
    let kanji_meta_counts = MetaCounts::count_kanji_metas(&kanji_meta_list);

    let counts = SummaryCounts::new(
        term_list.len(),
        term_meta_list.len(),
        tag_list.len(),
        kanji_meta_list.len(),
        kanji_list.len(),
        term_meta_counts,
        kanji_meta_counts,
    );

    let yomitan_version = env!("CARGO_PKG_VERSION").to_string();
    let prefix_wildcard_supported =
        current_profile.with_ptr(|prof| prof.options.general().prefix_wildcard_supported);
    let summary_details = SummaryDetails {
        prefix_wildcard_supported,
        counts,
        /// this is incorrect, it parses a 'styles.css' file
        /// need to do this later
        styles: "".to_string(),
        yomitan_version,
    };
    let summary = DictionarySummary::new(index, prefix_wildcard_supported, summary_details)?;
    let dictionary_options = DictionaryOptions::new(dict_name);

    Ok(DatabaseDictData {
        tag_list,
        kanji_meta_list,
        kanji_list,
        term_meta_list,
        term_list,
        summary,
        dictionary_options,
    })
}

fn convert_index_file(outpath: PathBuf) -> Result<Index, ImportError> {
    let index_str = fs::read_to_string(&outpath).map_err(|e| DictionaryFileError::File {
        outpath,
        reason: e.to_string(),
    })?;
    let index: Index = serde_json::from_str(&index_str)?;
    Ok(index)
}

// this one should probabaly be refactored to:
// 1. include the file and err if it throws like the rest of the converts
// 2. only handle one file and have the iteration be handled in the caller function
fn convert_tag_bank_files(
    outpaths: Vec<PathBuf>,
    dictionary: &str,
) -> Result<Vec<Vec<DatabaseTag>>, ImportError> {
    outpaths
        .into_iter()
        .map(|p| {
            let tag_str = fs::read_to_string(p)?;
            let mut tag: Vec<DictionaryDataTag> = serde_json::from_str(&tag_str)?;
            let res = tag
                .into_iter()
                .map(|tag| {
                    let DictionaryDataTag {
                        name,
                        category,
                        order,
                        notes,
                        score,
                    } = tag;
                    DatabaseTag {
                        id: Uuid::now_v7().to_string(),
                        name,
                        category,
                        order,
                        notes,
                        score,
                        dictionary: dictionary.to_string(),
                    }
                })
                .collect();
            Ok(res)
        })
        .collect()
}

/****************** Kanji Bank Functions ******************/

fn convert_kanji_bank(
    outpath: PathBuf,
    dict_name: &str,
) -> Result<Vec<DatabaseKanjiEntry>, DictionaryFileError> {
    let file = fs::File::open(&outpath).map_err(|reason| DictionaryFileError::FailedOpen {
        outpath: outpath.clone(),
        reason: reason.to_string(),
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<KanjiBank>();
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

    for item in &mut entries {
        item.dictionary = Some(dict_name.to_owned())
    }

    Ok(entries)
}

/****************** Term Bank Functions ******************/

fn convert_term_bank_file(
    outpath: PathBuf,
    dict_name: &str,
) -> Result<Vec<DatabaseTermEntry>, DictionaryFileError> {
    let file = fs::File::open(&outpath).map_err(|reason| DictionaryFileError::FailedOpen {
        outpath: outpath.clone(),
        reason: reason.to_string(),
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<Vec<TermEntryItem>>();
    let mut entries = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(reason)) => {
            eprintln!("{outpath:?} failed:\nreason: {reason}");
            return Err(crate::errors::DictionaryFileError::File {
                outpath,
                reason: reason.to_string(),
            });
        }
        None => return Err(DictionaryFileError::Empty(outpath)),
    };

    // Beginning of each word/phrase/expression (entry)
    // ie: ["headword","reading","","",u128,[{/* main */}]]];
    let terms: Vec<DatabaseTermEntry> = entries
        .into_iter()
        .map(|mut entry| {
            let TermEntryItem {
                expression,
                reading,
                def_tags,
                rules,
                score,
                structured_content,
                sequence,
                term_tags,
            } = entry;
            let id = uuid::Uuid::now_v7().to_string();
            let expression_reverse = rev_str(&expression);
            let reading_reverse = rev_str(&reading);
            let term = DatabaseTermEntry {
                id,
                expression,
                expression_reverse,
                reading,
                reading_reverse,
                definition_tags: def_tags,
                rules,
                score,
                sequence: Some(sequence),
                term_tags: Some(term_tags),
                file_path: outpath.clone().to_string_lossy().to_string(),
                dictionary: dict_name.to_owned(),
                // instead of pushing the entire tree as it is
                // create helper functions to parse the tree as a string or html
                glossary: structured_content.into_iter().map(|sc| sc.into()).collect(),
                ..Default::default()
            };
            term
        })
        .collect();
    Ok(terms)
}

fn rev_str(expression: &str) -> String {
    expression.chars().rev().collect()
}

// fn get_string_content(c_match_type: ContentMatchType) -> Vec<String> {
//     match c_match_type {
//         ContentMatchType::String(string) => vec![string],
//         ContentMatchType::Element(element) => handle_content_match_type(vec![*element]),
//         ContentMatchType::Content(vec) => handle_content_match_type(vec),
//     }
// }

// fn handle_content_match_type(content: Vec<ContentMatchType>) -> Vec<String> {
//     let mut content_strings: Vec<String> = Vec::new();
//
//     for e in content {
//         match e {
//             Element::UnknownString(string) => content_strings.push(string),
//             Element::Link(mut element) => {
//                 if let Some(content) = std::mem::take(&mut element.content) {
//                     content_strings.extend(get_string_content(content));
//                 }
//             }
//             Element::Styled(mut element) => {
//                 if let Some(content) = std::mem::take(&mut element.content) {
//                     content_strings.extend(get_string_content(content));
//                 }
//             }
//             Element::Unstyled(mut element) => {
//                 if let Some(content) = std::mem::take(&mut element.content) {
//                     content_strings.extend(get_string_content(content));
//                 }
//             }
//             Element::Table(mut element) => {
//                 if let Some(content) = std::mem::take(&mut element.content) {
//                     content_strings.extend(get_string_content(content));
//                 }
//             }
//             // img elements don't have children
//             Element::Image(_) => {}
//             // br elements don't have children
//             Element::LineBreak(_) => {}
//             _ => {
//                 panic!(
//                     "handle_content_match_type err: matched nothing! | line: {}",
//                     line!()
//                 )
//             }
//         }
//     }
//
//     content_strings
// }

/****************** Helper Functions ******************/

fn read_dir_helper<P: AsRef<Path>>(
    zip_path: P,
    index: &mut PathBuf,
    tag_banks: &mut Vec<PathBuf>,
    kanji_meta_banks: &mut Vec<PathBuf>,
    kanji_banks: &mut Vec<PathBuf>,
    term_meta_banks: &mut Vec<PathBuf>,
    term_banks: &mut Vec<PathBuf>,
) -> Result<(), io::Error> {
    fn contains(path: &[u8], substr: &[u8]) -> bool {
        path.windows(substr.len()).any(|w| w == substr)
    }

    fs::read_dir(&zip_path)?.try_for_each(|entry| -> Result<(), io::Error> {
        let entry = entry?;
        let outpath_buf = entry.path();
        let outpath = outpath_buf.as_os_str().as_encoded_bytes();

        if outpath.iter().last() != Some(&b'/') {
            if contains(outpath, b"term_bank") {
                term_banks.push(outpath_buf);
            } else if contains(outpath, b"index.json") {
                *index = outpath_buf;
            } else if contains(outpath, b"term_meta_bank") {
                term_meta_banks.push(outpath_buf);
            } else if contains(outpath, b"kanji_meta_bank") {
                kanji_meta_banks.push(outpath_buf);
            } else if contains(outpath, b"kanji_bank") {
                kanji_banks.push(outpath_buf);
            } else if contains(outpath, b"tag_bank") {
                tag_banks.push(outpath_buf);
            }
        }

        Ok(())
    })
}

#[cfg(test)]
mod importer_tests {
    use std::collections::HashSet;

    use crate::{
        database::{
            dictionary_database::Queries,
            dictionary_importer::{self, prepare_dictionary},
        },
        settings::YomichanOptions,
        test_utils, Yomichan,
    };

    #[test]
    fn dict() {
        #[cfg(target_os = "linux")]
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap();

        let options = YomichanOptions::default();
        let current_profile = options.get_current_profile().unwrap();
        let path = std::path::Path::new("./test_dicts/daijisen");
        prepare_dictionary(path, current_profile).unwrap();

        #[cfg(target_os = "linux")]
        if let Ok(report) = guard.report().build() {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}
