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
use native_model::{encode, native_model, Model};
use rusqlite::params;
use tracing;

use chrono::prelude::*;

use serde::{Deserialize, Serialize};

use rayon::prelude::*;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::{fs, io};

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

        // Update options in DB
        let options = self.options().read().clone();
        let data_blob = encode(&options).expect("Failed to encode YomichanOptions");

        self.db
            .conn
            .lock()
            .execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
                params!["options", data_blob],
            )
            .expect("Failed to update options");

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[native_model(id = 1, version = 1, with = native_model::postcard_1_0::PostCard)]
pub struct DictionarySummary {
    pub title: String,
    pub revision: String,
    pub sequenced: Option<bool>,
    pub minimum_yomitan_version: Option<String>,
    pub version: Option<u8>,
    pub import_date: DateTime<Local>,
    pub prefix_wildcards_supported: bool,
    pub counts: SummaryCounts,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryItemCount {
    pub total: u16,
}

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

pub fn import_dictionary<P: AsRef<Path>>(
    zip_path: P,
    db: Arc<DictionaryDatabase>,
    _current_profile: Ptr<YomichanProfile>,
) -> Result<DictionaryOptions, ImportError> {
    let external_data = importer::import_dictionary(&zip_path)?;
    tracing::info!(
        "Mapping dictionary data for: {}",
        external_data.summary.title
    );
    let dict_name = external_data.summary.title.clone();

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

    let term_meta_list: Vec<DatabaseMetaMatchType> = external_data.term_meta_list.into_par_iter().map(|m| match m {
        importer::dictionary_database::DatabaseMetaMatchType::Frequency(f) => DatabaseMetaMatchType::Frequency(DatabaseMetaFrequency {
            id: f.id, freq_expression: f.freq_expression, mode: match f.mode {
                importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
            },
            data: match f.data {
                importer::dictionary_data::TermMetaFreqDataMatchType::WithReading(wr) => TermMetaFreqDataMatchType::WithReading(TermMetaFreqDataWithReading {
                    reading: wr.reading, frequency: match wr.frequency {
                        importer::dictionary_data::GenericFreqData::Object(obj) => GenericFreqData::Object(FreqObjectData { value: obj.value, display_value: obj.display_value }),
                        importer::dictionary_data::GenericFreqData::Integer(i) => GenericFreqData::Integer(i),
                        importer::dictionary_data::GenericFreqData::String(s) => GenericFreqData::String(s),
                    },
                }),
                importer::dictionary_data::TermMetaFreqDataMatchType::Generic(g) => TermMetaFreqDataMatchType::Generic(match g {
                    importer::dictionary_data::GenericFreqData::Object(obj) => GenericFreqData::Object(FreqObjectData { value: obj.value, display_value: obj.display_value }),
                    importer::dictionary_data::GenericFreqData::Integer(i) => GenericFreqData::Integer(i),
                    importer::dictionary_data::GenericFreqData::String(s) => GenericFreqData::String(s),
                }),
            },
            dictionary: f.dictionary,
        }),
        importer::dictionary_database::DatabaseMetaMatchType::Pitch(p) => DatabaseMetaMatchType::Pitch(DatabaseMetaPitch {
            id: p.id, pitch_expression: p.pitch_expression, mode: match p.mode {
                importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
            },
            data: TermMetaPitchData {
                reading: p.data.reading, pitches: p.data.pitches.into_iter().map(|p| DictionaryPitch {
                    position: p.position, nasal: p.nasal.map(|n| match n { VecNumOrNum::Vec(v) => VecNumOrNum::Vec(v), VecNumOrNum::Num(n) => VecNumOrNum::Num(n) }),
                    devoice: p.devoice.map(|d| match d { VecNumOrNum::Vec(v) => VecNumOrNum::Vec(v), VecNumOrNum::Num(n) => VecNumOrNum::Num(n) }),
                    tags: p.tags,
                }).collect(),
            },
            dictionary: p.dictionary,
        }),
        importer::dictionary_database::DatabaseMetaMatchType::Phonetic(p) => DatabaseMetaMatchType::Phonetic(DatabaseMetaPhonetic {
            id: p.id, phonetic_expression: p.phonetic_expression, mode: match p.mode {
                importer::dictionary_data::TermMetaModeType::Freq => TermMetaModeType::Freq,
                importer::dictionary_data::TermMetaModeType::Pitch => TermMetaModeType::Pitch,
                importer::dictionary_data::TermMetaModeType::Ipa => TermMetaModeType::Ipa,
            },
            data: TermMetaPhoneticData {
                reading: p.data.reading, transcriptions: p.data.transcriptions.into_iter().map(|t| PhoneticTranscription {
                    match_type: match t.match_type {
                        importer::dictionary_database::TermPronunciationMatchType::PitchAccent => TermPronunciationMatchType::PitchAccent,
                        importer::dictionary_database::TermPronunciationMatchType::PhoneticTranscription => TermPronunciationMatchType::PhoneticTranscription,
                    },
                    ipa: t.ipa, tags: t.tags.into_iter().map(|tag| DictionaryTag {
                        name: tag.name, category: tag.category, order: tag.order, score: tag.score, content: tag.content, dictionaries: tag.dictionaries, redundant: tag.redundant,
                    }).collect(),
                }).collect(),
            },
            dictionary: p.dictionary,
        }),
    }).collect();

    tracing::info!(
        "Mapping and inserting {} terms in batches of {BATCH_SIZE}",
        external_data.term_list.len()
    );
    const BATCH_SIZE: usize = 10_000_000;
    let total_terms = external_data.term_list.len();
    let mut processed_terms = 0;
    let mut term_iter = external_data.term_list.into_iter();
    loop {
        let mut batch_count = 0;
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock
            .unchecked_transaction()
            .expect("Failed to start transaction");
        {
            let mut stmt = conn.prepare("INSERT OR REPLACE INTO terms (id, expression, reading, expression_reverse, reading_reverse, sequence, dictionary, data) VALUES (?, ?, ?, ?, ?, ?, ?, ?)").expect("Failed to prepare stmt");
            for t in term_iter.by_ref().take(BATCH_SIZE) {
                let entry = DatabaseTermEntry {
                    id: t.0.clone(),
                    expression: t.1.clone(),
                    reading: t.2.clone(),
                    expression_reverse: t.3.clone(),
                    reading_reverse: t.4.clone(),
                    definition_tags: t.5.map(|s| s.to_string()),
                    tags: t.6.map(|s| s.to_string()),
                    rules: t.7.to_string(),
                    score: t.8,
                    glossary: t
                        .9
                        .into_iter()
                        .map(|g| match g {
                            importer::structured_content::TermGlossaryGroupType::Content(c) => {
                                TermGlossaryGroupType::Content(TermGlossaryContentGroup {
                                    plain_text: c.plain_text,
                                    html: c.html,
                                })
                            }
                            importer::structured_content::TermGlossaryGroupType::Deinflection(
                                d,
                            ) => TermGlossaryGroupType::Deinflection(TermGlossaryDeinflection {
                                form_of: d.form_of,
                                rules: d.rules.iter().map(|s| s.to_owned()).collect(),
                            }),
                        })
                        .collect(),
                    sequence: t.10,
                    term_tags: t.11.as_ref().map(|s| s.to_string()),
                    dictionary: t.12.clone(),
                    file_path: t.13.clone(),
                };
                let data_blob = encode(&entry).expect("Failed to encode");
                stmt.execute(params![
                    entry.id,
                    entry.expression,
                    entry.reading,
                    entry.expression_reverse,
                    entry.reading_reverse,
                    entry.sequence.map(|s| s as i64),
                    entry.dictionary,
                    data_blob
                ])
                .expect("Failed to execute");
                batch_count += 1;
            }
        }
        if batch_count == 0 {
            break;
        }
        conn.commit().expect("Failed to commit");
        processed_terms += batch_count;
        tracing::info!(
            "  - Progress: {}/{} terms ({:.1}%)",
            processed_terms,
            total_terms,
            (processed_terms as f64 / total_terms as f64) * 100.0
        );
        if batch_count < BATCH_SIZE {
            break;
        }
    }

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

    {
        let data_blob = encode(&summary).expect("Failed to encode summary");
        db.conn
            .lock()
            .execute(
                "INSERT OR REPLACE INTO summaries (title, data) VALUES (?, ?)",
                params![summary.title, data_blob],
            )
            .expect("Failed to insert summary");
    }

    tracing::info!("Inserting kanji, tags, and metas in batches...");
    insert_kanji_batched(db.clone(), kanji_list, BATCH_SIZE)?;
    insert_tags_batched(db.clone(), tag_list, BATCH_SIZE)?;
    insert_kanji_meta_batched(db.clone(), kanji_meta_list, BATCH_SIZE)?;

    let total_term_metas = term_meta_list.len();
    let mut processed_term_metas = 0;
    let mut term_meta_iter = term_meta_list.into_iter();
    loop {
        let mut batch_count = 0;
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock
            .unchecked_transaction()
            .expect("Failed to start transaction");
        {
            let mut stmt = conn.prepare("INSERT OR REPLACE INTO term_meta (id, term, mode, dictionary, data) VALUES (?, ?, ?, ?, ?)").expect("Failed to prepare stmt");
            for item in term_meta_iter.by_ref().take(BATCH_SIZE) {
                let (id, term, mode, dictionary, data_blob) = match item {
                    DatabaseMetaMatchType::Frequency(freq) => {
                        let data_blob = encode(&freq).unwrap();
                        (
                            freq.id,
                            freq.freq_expression,
                            "freq",
                            freq.dictionary,
                            data_blob,
                        )
                    }
                    DatabaseMetaMatchType::Pitch(pitch) => {
                        let data_blob = encode(&pitch).unwrap();
                        (
                            pitch.id,
                            pitch.pitch_expression,
                            "pitch",
                            pitch.dictionary,
                            data_blob,
                        )
                    }
                    DatabaseMetaMatchType::Phonetic(ipa) => {
                        let data_blob = encode(&ipa).unwrap();
                        (
                            ipa.id,
                            ipa.phonetic_expression,
                            "ipa",
                            ipa.dictionary,
                            data_blob,
                        )
                    }
                };
                stmt.execute(params![id, term, mode, dictionary, data_blob])
                    .expect("Failed to execute stmt");
                batch_count += 1;
            }
        }
        if batch_count == 0 {
            break;
        }
        conn.commit().expect("Failed to commit");
        processed_term_metas += batch_count;
        tracing::info!(
            "  - Progress: {}/{} term metas ({:.1}%)",
            processed_term_metas,
            total_term_metas,
            (processed_term_metas as f64 / total_term_metas as f64) * 100.0
        );
        if batch_count < BATCH_SIZE {
            break;
        }
    }

    tracing::info!(
        "Import finished for dictionary: {}",
        dictionary_options.name
    );
    Ok(dictionary_options)
}

fn insert_kanji_batched(
    db: Arc<DictionaryDatabase>,
    list: Vec<DatabaseKanjiEntry>,
    batch_size: usize,
) -> Result<(), rusqlite::Error> {
    let mut iter = list.into_iter();
    loop {
        let mut batch_count = 0;
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock.unchecked_transaction()?;
        {
            let mut stmt = conn.prepare(
                "INSERT OR REPLACE INTO kanji (character, dictionary, data) VALUES (?, ?, ?)",
            )?;
            for item in iter.by_ref().take(batch_size) {
                let data_blob = encode(&item).unwrap();
                stmt.execute(params![item.character, item.dictionary, data_blob])?;
                batch_count += 1;
            }
        }
        if batch_count == 0 {
            break;
        }
        conn.commit()?;
        if batch_count < batch_size {
            break;
        }
    }
    Ok(())
}

fn insert_tags_batched(
    db: Arc<DictionaryDatabase>,
    list: Vec<DatabaseTag>,
    batch_size: usize,
) -> Result<(), rusqlite::Error> {
    let mut iter = list.into_iter();
    loop {
        let mut batch_count = 0;
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock.unchecked_transaction()?;
        {
            let mut stmt = conn.prepare(
                "INSERT OR REPLACE INTO tags (id, name, dictionary, data) VALUES (?, ?, ?, ?)",
            )?;
            for item in iter.by_ref().take(batch_size) {
                let data_blob = encode(&item).unwrap();
                stmt.execute(params![item.id, item.name, item.dictionary, data_blob])?;
                batch_count += 1;
            }
        }
        if batch_count == 0 {
            break;
        }
        conn.commit()?;
        if batch_count < batch_size {
            break;
        }
    }
    Ok(())
}

fn insert_kanji_meta_batched(
    db: Arc<DictionaryDatabase>,
    list: Vec<DatabaseMetaFrequency>,
    batch_size: usize,
) -> Result<(), rusqlite::Error> {
    let mut iter = list.into_iter();
    loop {
        let mut batch_count = 0;
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock.unchecked_transaction()?;
        {
            let mut stmt = conn.prepare(
                "INSERT OR REPLACE INTO kanji_meta (character, dictionary, data) VALUES (?, ?, ?)",
            )?;
            for item in iter.by_ref().take(batch_size) {
                let data_blob = encode(&item).unwrap();
                stmt.execute(params![item.freq_expression, item.dictionary, data_blob])?;
                batch_count += 1;
            }
        }
        if batch_count == 0 {
            break;
        }
        conn.commit()?;
        if batch_count < batch_size {
            break;
        }
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
