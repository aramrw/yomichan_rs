use crate::dictionary::KanjiDictionaryEntry;
use crate::dictionary_data::{
    GenericFrequencyData, Index, Tag as DictDataTag, TermGlossary, TermGlossaryContent,
    TermGlossaryImage, TermMeta, TermMetaDataMatchType, TermMetaFrequency,
    TermMetaFrequencyDataType, TermMetaModeType, TermMetaPitchData, TermV3, TermV4,
};
use crate::dictionary_database::{
    db_stores, DatabaseDictData, DatabaseKanjiEntry, DatabaseKanjiMetaFrequency, DatabaseTermEntry,
    DatabaseTermMeta, DatabaseTermMetaFrequency, DatabaseTermMetaPhonetic, DatabaseTermMetaPitch,
    KanjiEntry, MediaDataArrayBufferContent, TermEntry,
};
use crate::structured_content::{ContentMatchType, Element, LinkElement};

use crate::errors::{DBError, ImportError};
use crate::Yomichan;

use unicode_segmentation::UnicodeSegmentation;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use serde_untagged::UntaggedEnumVisitor;

use rayon::prelude::*;

use tempfile::tempdir;

use std::collections::{HashMap, VecDeque};
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Name of the dictionary.
    pub title: String,
    /// Revision of the dictionary. This value is only used for displaying information.
    pub revision: String,
    /// Whether or not this dictionary contains sequencing information for related terms.
    pub sequenced: bool,
    /// Format of data found in the JSON data files.
    pub version: u8,
    /// Date the dictionary was added to the db.
    pub import_date: String,
    /// Whether or not wildcards can be used for the search query.
    ///
    /// Rather than searching for the source text exactly,
    /// the text will only be required to be a prefix of an existing term.
    /// For example, scanning `読み` will effectively search for `読み*`
    /// which may bring up additional results such as *読み方*.
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryCounts {
    terms: SummaryItemCount,
    term_meta: SummaryMetaCount,
    kanji: SummaryItemCount,
    kanji_meta: SummaryMetaCount,
    tag_meta: SummaryItemCount,
    media: SummaryItemCount,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryItemCount {
    total: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryMetaCount {
    total: u64,
    meta: HashMap<String, u64>,
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

//  terms: {total: termList.length}, // FINISHED
//  termMeta: this._getMetaCounts(termMetaList), // FINISHED
//  kanji: {total: kanjiList.length}, // FINISHED
//  kanjiMeta: this._getMetaCounts(kanjiMetaList), // FINISHED
//  tagMeta: {total: tagList.length}, // FINISHED
//  media: {total: media.length}, // TODO FOR LATER
impl Yomichan {
    async fn import_dictionary<P: AsRef<Path>>(&self, zip_path: P) -> Result<(), DBError> {
        use db_stores::*;

        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(DICTIONARIES_STORE);
            // table.insert(/* not sure what I'm going to do here yet */);
        }
        txn.commit()?;

        Ok(())
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
    /// Will _always_ be either an `Obj` or a `Vec` _(ie: Never a String)_.
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

pub fn prepare_dictionary<P: AsRef<Path>>(zip_path: P) -> Result<DatabaseDictData, ImportError> {
    let instant = Instant::now();
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

    let paths_len = tag_bank_paths.len() + term_bank_paths.len() + term_meta_bank_paths.len() + 1;
    let index: Index = convert_index_file(index_path)?;
    let dict_name = index.title;
    let tag_list: Vec<Vec<DictDataTag>> = convert_tag_bank_files(tag_bank_paths)?;

    let kanji_meta_banks: Result<Vec<DatabaseKanjiMetaFrequency>, ImportError> =
        kanji_meta_bank_paths
            .into_par_iter()
            .map(|path| convert_kanji_meta_file(path, dict_name.clone()))
            .collect::<Result<Vec<Vec<DatabaseKanjiMetaFrequency>>, ImportError>>()
            .map(|v| v.into_iter().flatten().collect());

    let kanji_meta_list = match kanji_meta_banks {
        Ok(kml) => kml,
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji_meta_banks | {}",
                e
            )))
        }
    };

    let term_meta_banks: Result<Vec<DatabaseTermMeta>, ImportError> = term_meta_bank_paths
        .into_par_iter()
        .map(|path| convert_term_meta_file(path, dict_name.clone()))
        .collect::<Result<Vec<Vec<DatabaseTermMeta>>, ImportError>>()
        .map(|v| v.into_iter().flatten().collect());

    let term_meta_list = match term_meta_banks {
        Ok(tml) => tml,
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term_meta_banks | {}",
                e
            )))
        }
    };

    let kanji_banks: Result<Vec<DatabaseKanjiEntry>, ImportError> = kanji_bank_paths
        .into_iter()
        .map(|path| convert_kanji_bank(path, dict_name.clone()))
        .collect::<Result<Vec<Vec<DatabaseKanjiEntry>>, ImportError>>()
        .map(|nested| nested.into_iter().flatten().collect());

    let kanji_list = match kanji_banks {
        Ok(kb) => kb,
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert kanji banks | {}",
                e
            )))
        }
    };

    let term_banks: Result<Vec<DatabaseTermEntry>, ImportError> = term_bank_paths
        .into_par_iter()
        .map(convert_term_bank_file)
        .collect::<Result<Vec<Vec<DatabaseTermEntry>>, ImportError>>() // Collect into Result<Vec<Vec<DatabaseTermEntry>>, ImportError>
        .map(|nested| nested.into_iter().flatten().collect());

    let term_list = match term_banks {
        Ok(tb) => tb,
        Err(e) => {
            return Err(ImportError::Custom(format!(
                "Failed to convert term banks | {}",
                e
            )))
        }
    };

    for t in &term_list {
        println!("{:#?}", t);
    }
    let counts = SummaryCounts {
        terms: SummaryItemCount {
            total: term_list.len() as u16,
        },
        term_meta: SummaryMetaCount {
            total: term_meta_list.len() as u16,
            meta: term_meta_counts,
        },
        tag_meta: SummaryItemCount {
            total: tag_list.len() as u16,
        },
        kanji_meta: SummaryMetaCount {
            total: kanji_meta_list.len() as u16,
            meta: kanji_meta_counts,
        },
        kanji: SummaryItemCount {
            total: kanji_list.len() as u16,
        },
        // Can't deserialize media (yet).
        media: SummaryItemCount { total: 0 },
    };

    let counts = (
        tag_list.len(),
        kanji_meta_list.len(),
        kanji_list.len(),
        term_meta_list.len(),
        term_list.len(),
    );

    print_timer(instant, paths_len);

    Ok(DatabaseDictData {
        tag_list,
        kanji_meta_list,
        kanji_list,
        term_meta_list,
        term_list,
    })
}

fn get_meta_counts(metas: &Vec<DatabaseMeta>) -> MetaCounts {
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

fn create_summary(
    index: Index,
    prefix_wildcards_supported: bool,
    counts: SummaryCounts,
) -> Summary {
    let local: DateTime<Local> = Local::now();
    let formatted = local
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        .to_string()
        .rsplit_once('T')
        .unwrap()
        .0
        .to_string();

    Summary {
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

/****************** Meta Functions ******************/

fn convert_kanji_meta_file(
    outpath: PathBuf,
    mut dict_name: String,
) -> Result<Vec<DatabaseMeta>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<Vec<TermMetaFrequency>>();
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

    let kanji_metas: Vec<DatabaseKanjiMetaFrequency> = entries
        .into_iter()
        .map(|entry| DatabaseKanjiMetaFrequency {
            character: entry.expression,
            mode: TermMetaModeType::Freq,
            data: entry.data,
            dictionary: mem::take(&mut dict_name),
        })
        .collect();
    Ok(kanji_metas)
}

fn convert_term_meta_file(
    outpath: PathBuf,
    mut dict_name: String,
) -> Result<Vec<DatabaseTermMeta>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
    })?;
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

    let term_metas: Vec<DatabaseTermMeta> = entries
        .into_iter()
        .map(|entry| {
            let mut meta = DatabaseTermMeta {
                frequency: None,
                pitch: None,
                phonetic: None,
            };

            match entry.mode {
                TermMetaModeType::Freq => {
                    if let TermMetaDataMatchType::Frequency(data) = entry.data {
                        meta.frequency = Some(DatabaseTermMetaFrequency {
                            expression: entry.expression,
                            mode: TermMetaModeType::Freq,
                            data,
                            dictionary: mem::take(&mut dict_name),
                        });
                    }
                }
                TermMetaModeType::Pitch => {
                    if let TermMetaDataMatchType::Pitch(data) = entry.data {
                        meta.pitch = Some(DatabaseTermMetaPitch {
                            expression: entry.expression,
                            mode: TermMetaModeType::Pitch,
                            data,
                            dictionary: mem::take(&mut dict_name),
                        });
                    }
                }
                TermMetaModeType::Ipa => {
                    if let TermMetaDataMatchType::Phonetic(data) = entry.data {
                        meta.phonetic = Some(DatabaseTermMetaPhonetic {
                            expression: entry.expression,
                            mode: TermMetaModeType::Freq,
                            data,
                            dictionary: mem::take(&mut dict_name),
                        });
                    }
                }
            }

            meta
        })
        .collect();
    Ok(term_metas)
}

/****************** Kanji Bank Functions ******************/

fn convert_kanji_bank(
    outpath: PathBuf,
    mut dict_name: String,
) -> Result<Vec<DatabaseKanjiEntry>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<KanjiBank>();
    let mut entries = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(e)) => {
            return Err(ImportError::Custom(format!(
                "File: {} | Err: {e}",
                &outpath.to_string_lossy(),
            )))
        }
        None => {
            return Err(ImportError::Custom(String::from(
                "no data in term_bank stream",
            )))
        }
    };

    for item in &mut entries {
        item.dictionary = Some(mem::take(&mut dict_name))
    }

    Ok(entries)
}

/****************** Term Bank Functions ******************/

fn convert_term_bank_file(outpath: PathBuf) -> Result<Vec<DatabaseTermEntry>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:#?} | Err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<TermBank>();
    let entries: Vec<TermEntryItem> = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(e)) => {
            return Err(ImportError::Custom(format!(
                "File: {} | Err: {e}",
                &outpath.to_string_lossy(),
            )))
        }
        None => {
            return Err(ImportError::Custom(String::from(
                "no data in term_bank stream",
            )))
        }
    };

    // Beginning of each word/phrase/expression (entry)
    // ie: ["headword","reading","","",u128,[{/* main */}]]];
    let terms: Vec<DatabaseTermEntry> = entries
        .into_iter()
        .map(|mut entry| {
            let expression = entry.expression;
            let reading = entry.reading;
            let expression_reverse = rev_jp_str(&expression);
            let reading_reverse = rev_jp_str(&reading);
            let mut db_term = DatabaseTermEntry {
                expression,
                expression_reverse,
                reading,
                reading_reverse,
                definition_tags: entry.def_tags,
                rules: entry.rules,
                score: entry.score,
                sequence: Some(entry.sequence),
                term_tags: Some(entry.term_tags),
                ..Default::default()
            };

            let structured_content = entry.structured_content.swap_remove(0);
            let defs = get_string_content(structured_content.content);
            db_term.glossary = create_glossary(defs.concat());

            db_term
        })
        .collect();

    Ok(terms)
}

fn create_glossary(def_str: String) -> TermGlossaryContent {
    TermGlossaryContent {
        term_glossary_string: def_str,
        term_glossary_text: None,
        term_glossary_structured_content: None,
        term_glossary_image: None,
    }
}

fn rev_jp_str(expression: &str) -> String {
    UnicodeSegmentation::graphemes(expression, true)
        .rev()
        .collect::<String>()
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

fn print_timer<T>(inst: Instant, print: T)
where
    T: std::fmt::Debug,
{
    let duration = inst.elapsed();
    #[allow(unused_assignments)]
    let mut time = String::new();
    {
        let dur_sec = duration.as_secs();
        let dur_mill = duration.as_millis();
        let dur_nan = duration.as_nanos();

        if dur_sec == 0 {
            if dur_mill == 0 {
                time = format!("{}ns", dur_mill);
            } else {
                time = format!("{}ms", dur_nan);
            }
        } else if dur_sec > 60 {
            let min = dur_sec / 60;
            let sec = dur_sec % 60;
            time = format!("{}m{}s", min, sec);
        } else {
            time = format!("{}s", dur_sec);
        }
    }

    println!("{:?} files", print);
    println!("in {}", time);
}
