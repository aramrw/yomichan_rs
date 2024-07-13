use crate::dictionary_data::{Index, TermGlossaryImage, TermV3, TermV4};
use crate::dictionary_database::{
    db_stores, DatabaseTermEntry, MediaDataArrayBufferContent, TermEntry,
};
use crate::structured_content::{ContentMatchType, Element, LinkElement};

use crate::errors::{DBError, ImportError};
use crate::Yomichan;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use serde_untagged::UntaggedEnumVisitor;

use rayon::prelude::*;

use tempfile::tempdir;

use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

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
    TermMetaBank,
    KanjiBank,
    KanjiMetaBank,
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
    RankBased,
    OccuranceBased,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    title: String,
    revision: String,
    sequenced: bool,
    version: u8,
    import_date: String,
    prefix_wildcards_supported: bool,
    counts: SummaryCounts,
    author: Option<String>,
    url: Option<String>,
    description: Option<String>,
    attribution: Option<String>,
    source_language: Option<String>,
    target_language: Option<String>,
    frequency_mode: Option<FrequencyMode>,
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

impl Yomichan {
    async fn import_dictionary(&self) -> Result<(), DBError> {
        use db_stores::*;
        let txn = self.ycdatabase.db.begin_write()?;
        {
            let mut table = txn.open_table(DICTIONARIES_STORE);
            // table.insert(/* not sure what I'm going to do here yet */);
        }
        txn.commit()?;

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct StructuredContent {
            /// This should **always** have `"type": "structured-content"` inside the json.
            /// If not, the dictionary is not valid.
            #[serde(rename = "type")]
            content_type: String,
            /// Will **always** be either an `Obj` or a `Vec` _(ie: Never a String)_.
            content: ContentMatchType,
        }
        Ok(())
    }
}

/// Deserializable type mapping a `term_bank_$i.json` file.
pub type TermBank = Vec<Vec<EntryItemMatchType>>;

/// An `untagged` match type to generically match
/// the `header`, `reading`, and `structured-content`
/// of a `term_bank_$i.json` entry item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EntryItemMatchType {
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

/// The object holding all html & information about an entry.
/// There is only 1 per entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    /// Identifier to mark the start of each entry.
    ///
    /// This should **always** be `"type": "structured-content"` in the file.
    /// If not, the dictionary is not valid.
    #[serde(rename = "type")]
    content_type: String,
    /// Contains the main content of the entry.
    /// _(see: [`ContentMatchType`] )_.
    ///
    /// Will **always** be either an `Obj` or a `Vec` _(ie: Never a String)_.
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

pub fn prepare_dictionary<P: AsRef<Path>>(zip_path: P) -> Result<(), ImportError> {
    let instant = Instant::now();
    //let temp_dir_path = extract_dict_zip(zip_path)?;

    let mut index_path = PathBuf::new();
    let mut tag_bank_paths: Vec<PathBuf> = Vec::new();
    let mut term_bank_paths: Vec<PathBuf> = Vec::new();

    fs::read_dir(&zip_path)?.try_for_each(|entry| -> Result<(), std::io::Error> {
        let entry = entry?;
        let outpath_buf = entry.path();
        let outpath = outpath_buf.to_str().unwrap();

        if !outpath.ends_with('/') {
            if outpath.contains("term_bank") {
                term_bank_paths.push(outpath_buf);
            } else if outpath.contains("index.json") {
                index_path = outpath_buf;
            } else if outpath.contains("tag_bank") {
                tag_bank_paths.push(outpath_buf);
            }
        }

        Ok(())
    })?;

    let index = convert_index_file(index_path)?;
    let paths_len = tag_bank_paths.len() + term_bank_paths.len();

    let term_banks: Result<Vec<TermV3>, ImportError> = term_bank_paths
        .into_par_iter()
        .map(convert_term_bank_file)
        .collect::<Result<Vec<Vec<TermV3>>, ImportError>>() // Collect nested results
        .map(|nested| nested.into_iter().flatten().collect()); // Flatten nested Vecs

    let files_len = paths_len;
    print_timer(instant, files_len);

    Ok(())
}

fn convert_index_file(outpath: PathBuf) -> Result<Index, ImportError> {
    let index_str = fs::read_to_string(outpath)
        .map_err(|e| ImportError::Custom(format!("Failed to convert index | Err: {e}")))?;
    let index: Index = serde_json::from_str(&index_str)?;
    Ok(index)
}

fn convert_term_bank_file(outpath: PathBuf) -> Result<Vec<TermV3>, ImportError> {
    let file = fs::File::open(&outpath).map_err(|e| {
        ImportError::Custom(format!("File: {:?} | Err: {e}", outpath.to_string_lossy()))
    })?;
    let reader = BufReader::new(file);

    let mut stream = JsonDeserializer::from_reader(reader).into_iter::<TermBank>();
    let entries = match stream.next() {
        Some(Ok(entries)) => entries,
        Some(Err(err)) => {
            return Err(ImportError::Custom(format!(
                "File: {} | Err: {}",
                &outpath.to_string_lossy(),
                err
            )))
        }
        None => {
            return Err(ImportError::Custom(
                "no data in dictionary stream".to_string(),
            ))
        }
    };

    // Beginning of each word/phrase/expression (entry)
    // ie: ["headword","reading","","",u128,[{/* main */}]]];

    //#[cfg(feature = "disabled")]
    let terms: Vec<TermV3> = entries
        .into_iter()
        .filter_map(|mut entry| {
            let (headword, reading) = match (entry[0].clone(), entry[1].clone()) {
                (EntryItemMatchType::String(headword), EntryItemMatchType::String(reading)) => {
                    (headword, reading)
                }
                _ => return None,
            };

            #[cfg(feature = "termv3")]
            let mut v3_term = TermV3 {
                expression: headword.to_string(),
                reading: reading.to_string(),
                ..Default::default()
            };

            let mut v4_term = TermV4 {
                expression: headword,
                reading,
                ..Default::default()
            };

            if let EntryItemMatchType::StructuredContentVec(mut content) = entry.swap_remove(5) {
                // Now we own content and can move it
                if let Some(structured_content) = content.get_mut(0).map(|c| {
                    std::mem::replace(&mut c.content, ContentMatchType::String(String::new()))
                }) {
                    match structured_content {
                        ContentMatchType::Element(html_element) => {
                            match *html_element {
                                Element::Link(mut link_element) => {
                                    // a link has 2 import parts
                                    // a reference word (relates to the
                                    // main word) & the href.
                                    // Should figure out what I should do to combine
                                    // these two properly.
                                    if let Some(content) = std::mem::take(&mut link_element.content)
                                    {
                                        match content {
                                            ContentMatchType::String(ref_word) => {
                                                v4_term.definitions.push(ref_word);
                                            }
                                            _ => { /* todo */ }
                                        }
                                    }
                                }
                                Element::Unstyled(_) => return None,
                                _ => return None,
                            }
                        }
                        ContentMatchType::Content(elem_vec) => {
                            // Handle ContentMatchType::Content case
                            // println!("{:#?}", elem_vec);
                            return None;
                        }
                        ContentMatchType::String(_) => unreachable!(),
                    }
                }
            }
            None
        })
        .collect();
    Ok(terms)
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
