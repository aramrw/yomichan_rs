use crate::dictionary_data::TermGlossaryImage;
use crate::dictionary_database::{
    db_stores, DatabaseTermEntry, MediaDataArrayBufferContent, TermEntry,
};
use crate::structured_content::{ContentMatchType, Element, LinkElement};

use crate::errors;
use crate::Yomichan;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Deserializer as JsonDeserializer;
use serde_untagged::UntaggedEnumVisitor;

use rayon::prelude::*;

use tempfile::tempdir;

use std::collections::HashMap;
use std::time::Instant;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::BufReader;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;

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
    async fn import_dictionary(&self) -> Result<(), errors::DBError> {
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
) -> Result<std::path::PathBuf, errors::ImportError> {
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

pub fn prepare_dictionary<P: AsRef<Path>>(
    zip_path: P,
) -> Result<(), errors::ImportError> {
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
            } else if outpath == "index.json" {
                index_path = outpath_buf;
            } else if outpath.contains("tag_bank") {
                tag_bank_paths.push(outpath_buf);
            }
        }

        Ok(())
    })?;

    // println!(
    //     "{} files read in {}s",
    //     instant.elapsed().as_secs_f32()
    // );

    Ok(())
}

// fn convert_term_bank_file() {
//         let file = fs::File::open(&outpath)?;
//                 let reader = BufReader::new(file);
//
//                 let mut stream = JsonDeserializer::from_reader(reader).into_iter::<TermBank>();
//                 let entries = match stream.next() {
//                     Some(Ok(entries)) => entries,
//                     Some(Err(err)) => {
//                         return Err(errors::ImportError::OtherJSON(format!(
//                             "File: {} | Err: {}",
//                             &outpath.to_str().unwrap(),
//                             err
//                         )))
//                     }
//                     None => {
//                         return Err(errors::ImportError::OtherJSON(
//                             "no data in dictionary stream".to_string(),
//                         ))
//                     }
//                 };
//
//                 let
//
//                 // Beginning of each word/phrase/expression (entry)
//                 // ie: ["headword","reading","","",u128,[{/* main */}]]];
//                 //#[cfg(feature = "disabled")]
//                 for entry in entries {
//                     let (headword, reading) = match (&entry[0], &entry[1]) {
//                         (
//                             EntryItemMatchType::String(headword),
//                             EntryItemMatchType::String(reading),
//                         ) => (headword, reading),
//                         _ => continue,
//                     };
//
//                     if let EntryItemMatchType::StructuredContentVec(content) = &entry[5] {
//                         let structured_content = &content[0].content;
//                         match structured_content {
//                             ContentMatchType::Element(html_element) => {
//                                 //println!("{:#?}", html_elem);
//                                 match html_element.as_ref() {
//                                     Element::Link(link_element) => {
//
//                                     }
//                                     Element::Unstyled(unstyled_element) => {
//
//                                     }
//                                     _ => {}
//                                 }
//                             }
//                             ContentMatchType::Content(elem_vec) => {
//                                 //println!("{:#?}", elem_vec);
//                             }
//                             ContentMatchType::String(_) => unreachable!(),
//                         }
//                     }
//                 }
//
//                 files_read.fetch_add(1, Ordering::SeqCst);
//                 println!("{:?}", files_read);
//
// }
fn convert_index_file(outpath: PathBuf) -> Result<Index, ImportError> {
    let index_str = fs::read_to_string(outpath)
        .map_err(|e| ImportError::Custom(format!("Failed to convert index | Err: {e}")))?;
    let index: Index = serde_json::from_str(&index_str)?;
    Ok(index)
}

