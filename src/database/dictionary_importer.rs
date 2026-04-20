use crate::backend::Backend;
use crate::database::dictionary_database::{
    DatabaseKanjiEntry, DatabaseMetaFrequency, DatabaseMetaMatchType, DatabaseMetaPhonetic,
    DatabaseMetaPitch, DatabaseTag, DatabaseTermEntry, DatabaseTermEntryTuple, DictionaryDatabase,
};
use crate::errors::{ImportError, ImportZipError};
use crate::settings::{DictionaryDefinitionsCollapsible, DictionaryOptions, YomichanProfile};
use crate::Ptr;
use crate::Yomichan;

use importer::dictionary_data::{
    dictionary_data_util, FreqObjectData, GenericFreqData, Pitch as DictionaryPitch,
    TermGlossaryImage, TermMeta, TermMetaFreqDataMatchType, TermMetaFreqDataWithReading,
    TermMetaModeType, TermMetaPitchData, VecNumOrNum, YomichanIndexFile,
};
use importer::dictionary_database::{
    DictionaryTag, PhoneticTranscription, TermMetaPhoneticData, TermPronunciationMatchType,
};
use importer::structured_content::{
    TermEntryItem, TermGlossaryContentGroup, TermGlossaryDeinflection, TermGlossaryGroupType,
};
use indexmap::IndexMap;
use native_db::{native_db, ToKey};
use native_db::{transaction::RwTransaction, ToInput};
use native_model::{native_model, Model};

use chrono::prelude::*;

use serde::{Deserialize, Serialize};

use rayon::prelude::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::{fs, io};

//use chrono::{DateTime, Local};

impl Yomichan<'_> {
    pub fn import_dictionaries<P: AsRef<Path> + Send + Sync>(
        &self,
        zip_paths: &[P],
    ) -> Result<(), ImportError> {
        Backend::import_dictionaries_internal(
            zip_paths,
            self.options().read().get_current_profile()?,
            self.db.clone(),
        )?;

        let rwtx = self.db.rw_transaction()?;
        db_rwriter(&rwtx, vec![self.options().read().clone()])?;
        rwtx.commit()?;

        Ok(())
    }
}

impl Backend<'_> {
    pub fn import_dictionaries_internal<P: AsRef<Path> + Send + Sync>(
        zip_paths: &[P],
        current_profile: Ptr<YomichanProfile>,
        db: Arc<DictionaryDatabase>,
    ) -> Result<(), ImportError> {
        ImportZipError::check_zip_paths(zip_paths)?;
        let options: Vec<DictionaryOptions> = zip_paths
            .par_iter()
            .map(|path| import_dictionary(path, db.clone(), current_profile.clone()))
            .collect::<Result<Vec<DictionaryOptions>, ImportError>>()?;

        let dictionary_opts: IndexMap<String, DictionaryOptions> = options
            .into_iter()
            .map(|opt| (opt.name.clone(), opt))
            .collect();

        current_profile.with_ptr_mut(|current_profile| {
            let main_dictionary = current_profile.get_main_dictionary();
            if main_dictionary.is_empty() {
                let name = dictionary_opts
                    .get_index(0)
                    .expect("[unexpected] dictionary options created but len is 0");
                current_profile.set_main_dictionary(name.0.to_string());
            }
            current_profile.extend_dictionaries(dictionary_opts);
        });

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

// Overwrites importers DictionaryImporter with nativedb
// Final details about the Dictionary and it's import process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_db]
#[native_model(id = 1, version = 1, with = native_model::postcard_1_0::PostCard)]
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
    // some kind of styles.css file stuff
    pub styles: String,
}

#[derive(thiserror::Error, Debug)]
pub enum DictionarySummaryError {
    #[error("dictionary is incompatible with current version of Yomitan: (minimum required: ${minimum_required_yomitan_version}); dictionary: {dictionary}")]
    IncompatibleYomitanVersion {
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
        index: YomichanIndexFile,
        prefix_wildcards_supported: bool,
        details: SummaryDetails,
    ) -> Result<Self, DictionarySummaryError> {
        let import_date: DateTime<Local> = Local::now();
        let SummaryDetails {
            prefix_wildcard_supported,
            counts,
            styles,
        } = details;
        let YomichanIndexFile {
            title,
            revision,
            sequenced,
            format: _,
            version,
            minimum_yomitan_version,
            is_updatable,
            index_url,
            download_url,
            author: _,
            url: _,
            description: _,
            attribution: _,
            source_language: _,
            target_language: _,
            frequency_mode: _,
            tag_meta: _,
        } = index;

        let yomitan_version = env!("CARGO_PKG_VERSION").to_string();

        if yomitan_version == "0.0.0.0" {
            // running development version
        } else if let Some(minimum_yomitan_version) = &minimum_yomitan_version as &Option<String> {
            if dictionary_data_util::compare_revisions(&yomitan_version, minimum_yomitan_version) {
                return Err(DictionarySummaryError::IncompatibleYomitanVersion {
                    minimum_required_yomitan_version: minimum_yomitan_version.clone(),
                    dictionary: title,
                });
            }
        }

        if let Some(is_updatable) = is_updatable {
            if !is_updatable {
                return Err(DictionarySummaryError::InvalidIndexIsNotUpdatabale);
            }
            if let Some(index_url) = &index_url as &Option<String> {
                if let Err(err) = dictionary_data_util::validate_url(index_url) {
                    return Err(DictionarySummaryError::InvalidIndexUrl {
                        url: index_url.clone(),
                        err,
                    });
                }
            }
            if let Some(download_url) = &download_url as &Option<String> {
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

// // this is not used
// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub struct ImportRequirementContext {
//     //file_map: ArchiveFileMap,
//     media: IndexMap<String, MediaDataArrayBufferContent>,
// }
//
/// Deserializable type mapping a `term_bank_$i.json` file.
pub type TermBank = Vec<TermEntryItem>;
pub type TermMetaBank = Vec<TermMeta>;
pub type KanjiBank = Vec<DatabaseKanjiEntry>;

pub fn import_dictionary<P: AsRef<Path>>(
    zip_path: P,
    db: Arc<DictionaryDatabase>,
    _current_profile: Ptr<YomichanProfile>,
) -> Result<DictionaryOptions, ImportError> {
    let external_data = importer::import_dictionary(&zip_path)?;
    tracing::info!("Mapping dictionary data for: {}", external_data.summary.title);
    let dict_name = external_data.summary.title.clone();

    // Parallelize the most intensive mapping parts using rayon
    let tag_list: Vec<DatabaseTag> = external_data
        .tag_list
        .into_par_iter()
        .map(|t| DatabaseTag {
            id: t.id,
            name: t.name,
            category: t.category,
            order: t.order,
            notes: t.notes,
            score: t.score,
            dictionary: t.dictionary,
        })
        .collect();

    let kanji_meta_list: Vec<DatabaseMetaFrequency> = external_data
        .kanji_meta_list
        .into_par_iter()
        .map(|m| DatabaseMetaFrequency {
            id: m.id,
            freq_expression: m.freq_expression,
            mode: match m.mode {
                importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
            },
            data: match m.data {
                importer::dictionary_data::TermMetaFreqDataMatchType::WithReading(wr) => {
                    TermMetaFreqDataMatchType::WithReading(TermMetaFreqDataWithReading {
                        reading: wr.reading,
                        frequency: match wr.frequency {
                            importer::dictionary_data::GenericFreqData::Object(obj) => {
                                GenericFreqData::Object(FreqObjectData {
                                    value: obj.value,
                                    display_value: obj.display_value,
                                })
                            }
                            importer::dictionary_data::GenericFreqData::Integer(i) => {
                                GenericFreqData::Integer(i)
                            }
                            importer::dictionary_data::GenericFreqData::String(s) => {
                                GenericFreqData::String(s)
                            }
                        },
                    })
                }
                importer::dictionary_data::TermMetaFreqDataMatchType::Generic(g) => {
                    TermMetaFreqDataMatchType::Generic(match g {
                        importer::dictionary_data::GenericFreqData::Object(obj) => {
                            GenericFreqData::Object(FreqObjectData {
                                value: obj.value,
                                display_value: obj.display_value,
                            })
                        }
                        importer::dictionary_data::GenericFreqData::Integer(i) => {
                            GenericFreqData::Integer(i)
                        }
                        importer::dictionary_data::GenericFreqData::String(s) => {
                            GenericFreqData::String(s)
                        }
                    })
                }
            },
            dictionary: m.dictionary,
        })
        .collect();

    let kanji_list: Vec<DatabaseKanjiEntry> = external_data
        .kanji_list
        .into_par_iter()
        .map(|k| DatabaseKanjiEntry {
            character: k.character,
            onyomi: k.onyomi,
            kunyomi: k.kunyomi,
            tags: k.tags,
            meanings: k.meanings,
            stats: k.stats,
            dictionary: k.dictionary,
        })
        .collect();

    let term_meta_list: Vec<DatabaseMetaMatchType> = external_data
        .term_meta_list
        .into_par_iter()
        .map(|m| match m {
            importer::dictionary_database::DatabaseMetaMatchType::Frequency(f) => {
                DatabaseMetaMatchType::Frequency(DatabaseMetaFrequency {
                    id: f.id,
                    freq_expression: f.freq_expression,
                    mode: match f.mode {
                        importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                        importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                        importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
                    },
                    data: match f.data {
                        importer::dictionary_data::TermMetaFreqDataMatchType::WithReading(wr) => {
                            TermMetaFreqDataMatchType::WithReading(TermMetaFreqDataWithReading {
                                reading: wr.reading,
                                frequency: match wr.frequency {
                                    importer::dictionary_data::GenericFreqData::Object(obj) => {
                                        GenericFreqData::Object(FreqObjectData {
                                            value: obj.value,
                                            display_value: obj.display_value,
                                        })
                                    }
                                    importer::dictionary_data::GenericFreqData::Integer(i) => {
                                        GenericFreqData::Integer(i)
                                    }
                                    importer::dictionary_data::GenericFreqData::String(s) => {
                                        GenericFreqData::String(s)
                                    }
                                },
                            })
                        }
                        importer::dictionary_data::TermMetaFreqDataMatchType::Generic(g) => {
                            TermMetaFreqDataMatchType::Generic(match g {
                                importer::dictionary_data::GenericFreqData::Object(obj) => {
                                    GenericFreqData::Object(FreqObjectData {
                                        value: obj.value,
                                        display_value: obj.display_value,
                                    })
                                }
                                importer::dictionary_data::GenericFreqData::Integer(i) => {
                                    GenericFreqData::Integer(i)
                                }
                                importer::dictionary_data::GenericFreqData::String(s) => {
                                    GenericFreqData::String(s)
                                }
                            })
                        }
                    },
                    dictionary: f.dictionary,
                })
            }
            importer::dictionary_database::DatabaseMetaMatchType::Pitch(p) => {
                DatabaseMetaMatchType::Pitch(DatabaseMetaPitch {
                    id: p.id,
                    pitch_expression: p.pitch_expression,
                    mode: match p.mode {
                        importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                        importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                        importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
                    },
                    data: TermMetaPitchData {
                        reading: p.data.reading,
                        pitches: p
                            .data
                            .pitches
                            .into_iter()
                            .map(|p| DictionaryPitch {
                                position: p.position,
                                nasal: p.nasal.map(|n| match n {
                                    VecNumOrNum::Vec(v) => VecNumOrNum::Vec(v),
                                    VecNumOrNum::Num(n) => VecNumOrNum::Num(n),
                                }),
                                devoice: p.devoice.map(|d| match d {
                                    VecNumOrNum::Vec(v) => VecNumOrNum::Vec(v),
                                    VecNumOrNum::Num(n) => VecNumOrNum::Num(n),
                                }),
                                tags: p.tags,
                            })
                            .collect(),
                    },
                    dictionary: p.dictionary,
                })
            }
            importer::dictionary_database::DatabaseMetaMatchType::Phonetic(p) => {
                DatabaseMetaMatchType::Phonetic(DatabaseMetaPhonetic {
                    id: p.id,
                    phonetic_expression: p.phonetic_expression,
                    mode: match p.mode {
                        importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                        importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                        importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
                    },
                    data: TermMetaPhoneticData {
                        reading: p.data.reading,
                        transcriptions: p
                            .data
                            .transcriptions
                            .into_iter()
                            .map(|t| PhoneticTranscription {
                                match_type: match t.match_type {
                                    importer::dictionary_database::TermPronunciationMatchType::PitchAccent => {
                                        TermPronunciationMatchType::PitchAccent
                                    }
                                    importer::dictionary_database::TermPronunciationMatchType::PhoneticTranscription => {
                                        TermPronunciationMatchType::PhoneticTranscription
                                    }
                                },
                                ipa: t.ipa,
                                tags: t
                                    .tags
                                    .into_iter()
                                    .map(|tag| DictionaryTag {
                                        name: tag.name,
                                        category: tag.category,
                                        order: tag.order,
                                        score: tag.score,
                                        content: tag.content,
                                        dictionaries: tag.dictionaries,
                                        redundant: tag.redundant,
                                    })
                                    .collect(),
                            })
                            .collect(),
                    },
                    dictionary: p.dictionary,
                })
            }
        })
        .collect();

    let term_list: Vec<DatabaseTermEntry> = external_data
        .term_list
        .into_par_iter()
        .map(|t| {
            DatabaseTermEntry::from(DatabaseTermEntryTuple(
                t.0,
                t.1,
                t.2,
                t.3,
                t.4,
                t.5.map(|s| s.to_string()),
                t.6.map(|s| s.to_string()),
                t.7.to_string(),
                t.8,
                t.9.into_iter()
                    .map(|g| match g {
                        importer::structured_content::TermGlossaryGroupType::Content(c) => {
                            TermGlossaryGroupType::Content(TermGlossaryContentGroup {
                                plain_text: c.plain_text,
                                html: c.html,
                            })
                        }
                        importer::structured_content::TermGlossaryGroupType::Deinflection(d) => {
                            TermGlossaryGroupType::Deinflection(TermGlossaryDeinflection {
                                form_of: d.form_of,
                                rules: d.rules.iter().map(|s| s.to_owned()).collect(),
                            })
                        }
                    })
                    .collect(),
                t.10,
                t.11.as_ref().map(|s| s.to_string()),
                t.12,
                t.13,
            ))
        })
        .collect();

    let summary = DictionarySummary {
        title: external_data.summary.title,
        revision: external_data.summary.revision,
        sequenced: external_data.summary.sequenced,
        minimum_yomitan_version: external_data.summary.minimum_yomitan_version,
        version: external_data.summary.version,
        import_date: external_data.summary.import_date,
        prefix_wildcards_supported: external_data.summary.prefix_wildcards_supported,
        counts: SummaryCounts {
            terms: SummaryItemCount {
                total: external_data.summary.counts.terms.total,
            },
            term_meta: SummaryMetaCount {
                total: external_data.summary.counts.term_meta.total,
                meta: MetaCounts {
                    freq: external_data.summary.counts.term_meta.meta.freq,
                    pitch: external_data.summary.counts.term_meta.meta.pitch,
                    ipa: external_data.summary.counts.term_meta.meta.ipa,
                },
            },
            kanji: SummaryItemCount {
                total: external_data.summary.counts.kanji.total,
            },
            kanji_meta: SummaryMetaCount {
                total: external_data.summary.counts.kanji_meta.total,
                meta: MetaCounts {
                    freq: external_data.summary.counts.kanji_meta.meta.freq,
                    pitch: external_data.summary.counts.kanji_meta.meta.pitch,
                    ipa: external_data.summary.counts.kanji_meta.meta.ipa,
                },
            },
            tag_meta: SummaryItemCount {
                total: external_data.summary.counts.tag_meta.total,
            },
            media: SummaryItemCount {
                total: external_data.summary.counts.media.total,
            },
        },
        styles: external_data.summary.styles,
    };

    let dictionary_options = DictionaryOptions {
        name: dict_name,
        alias: "".to_string(),
        enabled: true,
        allow_secondary_searches: true,
        definitions_collapsible: DictionaryDefinitionsCollapsible::default(),
        parts_of_speech_filter: false,
        use_deinflections: true,
        styles: None,
    };

    tracing::info!("Finished mapping dictionary data. Starting batched inserts...");

    // Batch size of 100,000 might be better for very large imports
    const BATCH_SIZE: usize = 100_000;

    tracing::info!("Inserting {} terms in batches of {}...", term_list.len(), BATCH_SIZE);
    db_insert_batched(db.clone(), term_list, BATCH_SIZE)?;

    tracing::info!("Inserting kanji, tags, and metas in batches...");
    db_insert_batched(db.clone(), kanji_list, BATCH_SIZE)?;
    db_insert_batched(db.clone(), tag_list, BATCH_SIZE)?;
    db_insert_batched(db.clone(), kanji_meta_list, BATCH_SIZE)?;

    let mut term_meta_iter = term_meta_list.into_iter();
    loop {
        let mut batch_count = 0;
        let rwtx = db.rw_transaction()?;
        for item in term_meta_iter.by_ref().take(BATCH_SIZE) {
            match item {
                DatabaseMetaMatchType::Frequency(freq) => {
                    rwtx.insert(freq)?;
                }
                DatabaseMetaMatchType::Pitch(pitch) => {
                    rwtx.insert(pitch)?;
                }
                DatabaseMetaMatchType::Phonetic(ipa) => {
                    rwtx.insert(ipa)?;
                }
            }
            batch_count += 1;
        }
        if batch_count == 0 {
            break;
        }
        rwtx.commit()?;
        if batch_count < BATCH_SIZE {
            break;
        }
    }

    let rwtx = db.rw_transaction()?;
    rwtx.upsert(summary)?; // Keep upsert for summary just in case
    rwtx.commit()?;

    tracing::info!("Import finished for dictionary: {}", dictionary_options.name);
    Ok(dictionary_options)
}

fn db_insert_batched<L: ToInput + Send>(
    db: Arc<DictionaryDatabase>,
    list: Vec<L>,
    batch_size: usize,
) -> Result<(), Box<native_db::db_type::Error>> {
    let mut iter = list.into_iter();
    loop {
        let mut batch_count = 0;
        let rwtx = db.rw_transaction()?;
        for item in iter.by_ref().take(batch_size) {
            rwtx.insert(item)?;
            batch_count += 1;
        }
        if batch_count == 0 {
            break;
        }
        rwtx.commit()?;
        if batch_count < batch_size {
            break;
        }
    }
    Ok(())
}


fn db_rwriter<L: ToInput>(
    rwtx: &RwTransaction,
    list: Vec<L>,
) -> Result<(), Box<native_db::db_type::Error>> {
    for item in list {
        rwtx.upsert(item)?;
    }
    Ok(())
}

fn read_dir_helper<P: AsRef<Path>>(
    path: P,
    index: &mut PathBuf,
    tag_banks: &mut Vec<PathBuf>,
    kanji_meta_banks: &mut Vec<PathBuf>,
    kanji_banks: &mut Vec<PathBuf>,
    term_meta_banks: &mut Vec<PathBuf>,
    term_banks: &mut Vec<PathBuf>,
) -> Result<(), io::Error> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            read_dir_helper(
                &path,
                index,
                tag_banks,
                kanji_meta_banks,
                kanji_banks,
                term_meta_banks,
                term_banks,
            )?;
        } else {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if file_name.contains("term_bank") {
                term_banks.push(path);
            } else if file_name.contains("index.json") {
                *index = path;
            } else if file_name.contains("term_meta_bank") {
                term_meta_banks.push(path);
            } else if file_name.contains("kanji_meta_bank") {
                kanji_meta_banks.push(path);
            } else if file_name.contains("kanji_bank") {
                kanji_banks.push(path);
            } else if file_name.contains("tag_bank") {
                tag_banks.push(path);
            }
        }
    }
    Ok(())
}
