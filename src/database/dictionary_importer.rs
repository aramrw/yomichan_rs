use crate::database::dictionary_database::{
    DatabaseDictData, DatabaseKanjiEntry, DatabaseKanjiMetaFrequency, DatabaseMeta,
    DatabaseMetaFrequency, DatabaseMetaPhonetic, DatabaseMetaPitch, DatabaseTermEntry, KanjiEntry,
    MediaDataArrayBufferContent, TermEntry, DB_MODELS,
};
use crate::dictionary::KanjiDictionaryEntry;
use crate::dictionary_data::{
    GenericFreqData, Index, Tag as DictDataTag, TermGlossary, TermGlossaryContent,
    TermGlossaryImage, TermMeta, TermMetaDataMatchType, TermMetaFreqDataMatchType,
    TermMetaFrequency, TermMetaModeType, TermMetaPitchData, TermV3, TermV4,
};
use crate::settings::{
    self, DictionaryDefinitionsCollapsible, DictionaryOptions, Options, Profile,
};
use crate::structured_content::{ContentMatchType, Element, LinkElement};

use crate::errors::{DBError, ImportError};
use crate::Yomichan;

use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use transaction::RwTransaction;

use chrono::prelude::*;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use serde_untagged::UntaggedEnumVisitor;

use rayon::prelude::*;

use tempfile::tempdir;

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use std::time::Instant;
use std::{fs, io, mem};

//use chrono::{DateTime, Local};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    result: Option<Summary>,
    //errors: Vec<ImportError>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportDetails {
    prefix_wildcards_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrequencyMode {
    #[serde(rename = "occurrence-based")]
    OccurrenceBased,
    #[serde(rename = "rank-based")]
    RankBased,
}

// Final details about the Dictionary and it's import process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Name of the dictionary.
    pub title: String,
    /// Revision of the dictionary. This value is only used for displaying information.
    pub revision: String,
    /// Whether or not this dictionary contains sequencing information for related terms.
    pub sequenced: Option<bool>,
    /// Format of data found in the JSON data files.
    pub version: Option<u8>,
    /// Date the dictionary was added to the db.
    pub import_date: String,
    /// Whether or not wildcards can be used for the search query.
    ///
    /// Rather than searching for the source text exactly,
    /// the text will only be required to be a prefix of an existing term.
    /// For example, scanning `読み` will effectively search for `読み*`
    /// which may bring up additional results such as `読み方`.
    pub prefix_wildcards_supported: bool,
    pub counts: SummaryCounts,
    /// Creator of the dictionary.
    pub author: Option<String>,
    /// URL for the source of the dictionary.
    pub url: Option<String>,
    /// Description of the dictionary data.
    pub description: Option<String>,
    /// Attribution information for the dictionary data.
    pub attribution: Option<String>,
    /// Language of the terms in the dictionary.
    pub source_language: Option<String>,
    /// Main language of the definitions in the dictionary.
    pub target_language: Option<String>,
    /// (See: [`FrequencyMode`])
    pub frequency_mode: Option<FrequencyMode>,
}

impl Summary {
    fn new(index: Index, prefix_wildcards_supported: bool, counts: SummaryCounts) -> Self {
        let local: DateTime<Local> = Local::now();
        let formatted = local
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            .to_string()
            .rsplit_once('T')
            .unwrap()
            .0
            .to_string();

        Self {
            title: index.title,
            revision: index.revision,
            sequenced: index.sequenced,
            version: index.version,
            import_date: formatted,
            prefix_wildcards_supported,
            counts,
            author: index.author,
            url: index.url,
            description: index.description,
            attribution: index.attribution,
            source_language: index.source_language,
            target_language: index.target_language,
            frequency_mode: index.frequency_mode,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryDetails {
    pub prefix_wildcard_supported: bool,
    pub counts: SummaryCounts,
    // I dont know what this is
    // some kind of styles.css file stuff
    //pub styles: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryItemCount {
    pub total: u16,
}

impl SummaryItemCount {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryMetaCount {
    pub total: u16,
    pub meta: MetaCounts,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MetaCounts {
    freq: u32,
    pitch: u32,
    ipa: u32,
}

impl MetaCounts {
    fn new(metas: &Vec<DatabaseMeta>) -> Self {
        let mut meta_counts = MetaCounts::default();

        for mt in metas {
            if mt.frequency.is_some() {
                meta_counts.freq += 1;
            }
            if mt.pitch.is_some() {
                meta_counts.pitch += 1;
            }
            if mt.phonetic.is_some() {
                meta_counts.ipa += 1;
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
    media: HashMap<String, MediaDataArrayBufferContent>,
}

/// An `untagged` match type to generically match
/// the `header`, `reading`, and `structured-content`
/// of a `term_bank_$i.json` entry item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
enum EntryItemMatchType {
    String(String),
    Integer(i128),
    /// The array holding the main `structured-content` object.
    /// There is only 1 per entry.
    StructuredContentVec(Vec<StructuredContent>),
}

impl<'de> Deserialize<'de> for EntryItemMatchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        UntaggedEnumVisitor::new()
            .string(|single| Ok(EntryItemMatchType::String(single.to_string())))
            .i128(|int| Ok(EntryItemMatchType::Integer(int)))
            .seq(|seq| {
                seq.deserialize()
                    .map(EntryItemMatchType::StructuredContentVec)
            })
            .deserialize(deserializer)
    }
}

/// Deserializable type mapping a `term_bank_$i.json` file.
pub type TermBank = Vec<TermEntryItem>;
pub type TermMetaBank = Vec<TermMeta>;
pub type KanjiBank = Vec<DatabaseKanjiEntry>;

/// The 'header', and `structured-content`
/// of a `term_bank_${i}.json` entry item.
#[derive(Deserialize)]
pub struct TermEntryItem {
    pub expression: String,
    pub reading: String,
    pub def_tags: Option<String>,
    pub rules: String,
    pub score: i8,
    pub structured_content: Vec<StructuredContent>,
    pub sequence: i128,
    pub term_tags: String,
}

/// The object holding all html & information about an entry.
/// _There is only 1 per entry_.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    /// Identifier to mark the start of each entry's content.
    ///
    /// This should _always_ be `"type": "structured-content"` in the file.
    /// If not, the dictionary is not valid.
    #[serde(rename = "type")]
    content_type: String,
    /// Contains the main content of the entry.
    /// _(see: [`ContentMatchType`] )_.
    ///
    /// Will _always_ be either an `Element (obj)` or a `Content (array)` _(ie: Never a String)_.
    content: ContentMatchType,
}

fn extract_dict_zip<P: AsRef<std::path::Path>>(
    zip_path: P,
) -> Result<std::path::PathBuf, ImportError> {
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

impl Yomichan {
    pub fn import_dictionaries<P: AsRef<Path> + Send + Sync>(
        &mut self,
        zip_paths: &[P],
    ) -> Result<(), DBError> {
        let settings = self.options.get_options_mut();
        let db_path = &self.db_path;
        let db = &self.db;

        let mut dictionary_options: Vec<DictionaryOptions> = zip_paths
            .par_iter()
            .map(|path| import_dictionary(path, settings, db))
            .collect::<Result<Vec<DictionaryOptions>, DBError>>()?;

        let current_profile = settings.get_current_profile_mut();
        current_profile
            .options
            .dictionaries
            .append(&mut dictionary_options);

        Ok(())
    }
}

pub fn import_dictionary<P: AsRef<Path>>(
    zip_path: P,
    settings: &Options,
    //db_path: &OsString,
    db: &Database,
) -> Result<DictionaryOptions, DBError> {
    let data: DatabaseDictData = prepare_dictionary(zip_path, settings)?;
    let rwtx = db.rw_transaction()?;
    db_rwriter(&rwtx, data.term_list)?;
    {
        let term_meta_list = data.term_meta_list;
        for item in term_meta_list {
            if let Some(freq) = item.frequency {
                rwtx.insert(freq)?;
            }
            if let Some(pitch) = item.pitch {
                rwtx.insert(pitch)?;
            }
            if let Some(ipa) = item.phonetic {
                rwtx.insert(ipa)?;
            }
        }
    }

    rwtx.commit()?;
    Ok(data.dictionary_options)
}

fn db_rwriter<L: ToInput>(rwtx: &RwTransaction, list: Vec<L>) -> Result<(), DBError> {
    for item in list {
        rwtx.insert(item)?;
    }
    Ok(())
}

pub fn prepare_dictionary<P: AsRef<Path>>(
    zip_path: P,
    settings: &Options,
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

    // let paths_len = tag_bank_paths.len() + term_bank_paths.len() + term_meta_bank_paths.len() + 1;
    let index: Index = convert_index_file(index_path)?;
    let dict_name = index.title.clone();
    let tag_list: Vec<Vec<DictDataTag>> = convert_tag_bank_files(tag_bank_paths)?;

    let term_banks: Result<Vec<Vec<DatabaseTermEntry>>, ImportError> = term_bank_paths
        .into_par_iter()
        .map(|path| convert_term_bank_file(path, &dict_name))
        .collect::<Result<Vec<Vec<DatabaseTermEntry>>, ImportError>>();

    let term_list: Vec<DatabaseTermEntry> = match term_banks {
        Ok(tl) => tl.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term banks | {}",
                e
            )));
        }
    };

    let kanji_meta_banks: Result<Vec<Vec<DatabaseMeta>>, ImportError> = kanji_meta_bank_paths
        .into_par_iter()
        .map(|path| DatabaseMeta::convert_kanji_meta_file(path, dict_name.clone()))
        .collect::<Result<Vec<Vec<DatabaseMeta>>, ImportError>>();

    let kanji_meta_list: Vec<DatabaseMeta> = match kanji_meta_banks {
        Ok(kml) => kml.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji_meta_banks | {}",
                e
            )))
        }
    };

    let term_meta_banks: Result<Vec<Vec<DatabaseMeta>>, ImportError> = term_meta_bank_paths
        .into_par_iter()
        .map(|path| DatabaseMeta::convert_term_meta_file(path, dict_name.clone()))
        .collect::<Result<Vec<Vec<DatabaseMeta>>, ImportError>>();

    let term_meta_list: Vec<DatabaseMeta> = match term_meta_banks {
        Ok(tml) => tml.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term_meta_banks | {}",
                e
            )))
        }
    };

    let kanji_banks: Result<Vec<Vec<DatabaseKanjiEntry>>, ImportError> = kanji_bank_paths
        .into_iter()
        .map(|path| convert_kanji_bank(path, &dict_name))
        .collect::<Result<Vec<Vec<DatabaseKanjiEntry>>, ImportError>>();

    let kanji_list: Vec<DatabaseKanjiEntry> = match kanji_banks {
        Ok(kl) => kl.into_iter().flatten().collect(),
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji banks | {}",
                e
            )))
        }
    };

    let term_meta_counts = MetaCounts::new(&term_meta_list);
    let kanji_meta_counts = MetaCounts::new(&kanji_meta_list);

    let counts = SummaryCounts::new(
        term_list.len(),
        term_meta_list.len(),
        tag_list.len(),
        kanji_meta_list.len(),
        kanji_list.len(),
        term_meta_counts,
        kanji_meta_counts,
    );

    let summary = Summary::new(
        index,
        settings.global.database.prefix_wildcards_supported,
        counts,
    );

    let dictionary_options = DictionaryOptions::new(settings, dict_name);

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
    let index_str = fs::read_to_string(outpath)
        .map_err(|e| ImportError::Custom(format!("Failed to convert index | Err: {e}")))?;
    let index: Index = serde_json::from_str(&index_str)?;
    Ok(index)
}

// this one should probabaly be refactored to:
// 1. include the file and err if it throws like the rest of the converts
// 2. only handle one file and have the iteration be handled in the caller function
fn convert_tag_bank_files(outpaths: Vec<PathBuf>) -> Result<Vec<Vec<DictDataTag>>, ImportError> {
    outpaths
        .into_iter()
        .map(|p| {
            let tag_str = fs::read_to_string(p)?;
            let tag: Vec<DictDataTag> = serde_json::from_str(&tag_str)?;
            Ok(tag)
        })
        .collect()
}

/****************** Kanji Bank Functions ******************/

fn convert_kanji_bank(
    outpath: PathBuf,
    dict_name: &str,
) -> Result<Vec<DatabaseKanjiEntry>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<KanjiBank>();
    let mut entries = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(e)) => {
            return Err(ImportError::InvalidJson {
                file: outpath,
                e: Some(e.to_string()),
            })
        }
        None => return Err(ImportError::Empty { file: outpath }),
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
) -> Result<Vec<DatabaseTermEntry>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("file: {:#?} | err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<TermBank>();
    let mut entries = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(e)) => {
            return Err(ImportError::InvalidJson {
                file: outpath,
                e: Some(e.to_string()),
            })
        }
        None => return Err(ImportError::Empty { file: outpath }),
    };

    // Beginning of each word/phrase/expression (entry)
    // ie: ["headword","reading","","",u128,[{/* main */}]]];
    let terms: Vec<DatabaseTermEntry> = entries
        .into_iter()
        .map(|mut entry| {
            let id = uuid::Uuid::new_v4().to_string();
            let expression = entry.expression;
            let reading = entry.reading;
            let expression_reverse = rev_str(&expression);
            let reading_reverse = rev_str(&reading);
            let mut db_term = DatabaseTermEntry {
                id,
                expression,
                expression_reverse,
                reading,
                reading_reverse,
                definition_tags: entry.def_tags,
                rules: entry.rules,
                score: entry.score,
                sequence: Some(entry.sequence),
                term_tags: Some(entry.term_tags),
                file_path: outpath.clone().into_os_string(),
                dictionary: dict_name.to_owned(),
                ..Default::default()
            };

            let structured_content = entry.structured_content.swap_remove(0);
            let defs = get_string_content(structured_content.content);
            let gloss_content = TermGlossaryContent::new(defs.concat(), None, None, None);
            let gloss = TermGlossary::Content(Box::new(gloss_content));
            db_term.glossary = gloss;

            db_term
        })
        .collect();

    Ok(terms)
}

fn rev_str(expression: &str) -> String {
    expression.chars().rev().collect()
}

fn get_string_content(c_match_type: ContentMatchType) -> Vec<String> {
    match c_match_type {
        ContentMatchType::String(string) => vec![string],
        ContentMatchType::Element(element) => handle_content_match_type(vec![*element]),
        ContentMatchType::Content(vec) => handle_content_match_type(vec),
    }
}

fn handle_content_match_type(content: Vec<Element>) -> Vec<String> {
    let mut content_strings: Vec<String> = Vec::new();

    for e in content {
        match e {
            Element::UnknownString(string) => content_strings.push(string),
            Element::Link(mut element) => {
                if let Some(content) = std::mem::take(&mut element.content) {
                    content_strings.extend(get_string_content(content));
                }
            }
            Element::Styled(mut element) => {
                if let Some(content) = std::mem::take(&mut element.content) {
                    content_strings.extend(get_string_content(content));
                }
            }
            Element::Unstyled(mut element) => {
                if let Some(content) = std::mem::take(&mut element.content) {
                    content_strings.extend(get_string_content(content));
                }
            }
            Element::Table(mut element) => {
                if let Some(content) = std::mem::take(&mut element.content) {
                    content_strings.extend(get_string_content(content));
                }
            }
            // img elements don't have children
            Element::Image(_) => {}
            // br elements don't have children
            Element::LineBreak(_) => {}
            _ => {
                panic!(
                    "handle_content_match_type err: matched nothing! | line: {}",
                    line!()
                )
            }
        }
    }

    content_strings
}

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
        settings::Options,
        yomichan_test_utils, Yomichan,
    };

    #[test]
    fn dict() {
        #[cfg(target_os = "linux")]
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap();

        let options = Options::default();
        let path = std::path::Path::new("./test_dicts/daijisen");
        prepare_dictionary(path, &options).unwrap();

        #[cfg(target_os = "linux")]
        if let Ok(report) = guard.report().build() {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}
