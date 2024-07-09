use crate::dictionary_data::TermGlossaryImage;
use crate::dictionary_database::{
    db_stores, DatabaseTermEntry, MediaDataArrayBufferContent, TermEntry,
};
use crate::structured_content::ContentMatchType;

use crate::errors;
use crate::Yomichan;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::HashMap;
use std::time::Instant;
use tempfile::tempdir;

use rayon::prelude::*;
use std::fs;
use std::io::BufReader;
use std::sync::atomic::{AtomicUsize, Ordering};

//use chrono::{DateTime, Local};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSteps {
    Uninitialized,
    ValidateIndex,
    ValidateSchema,
    FormatDictionary,
    ImportMedia,
    ImportData,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    result: Option<Summary>,
    //errors: Vec<ImportError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDetails {
    prefix_wildcards_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrequencyMode {
    RankBased,
    OccuranceBased,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryCounts {
    terms: SummaryItemCount,
    term_meta: SummaryMetaCount,
    kanji: SummaryItemCount,
    kanji_meta: SummaryMetaCount,
    tag_meta: SummaryItemCount,
    media: SummaryItemCount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryItemCount {
    total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryMetaCount {
    total: u64,
    meta: HashMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageImportMatchType {
    Image,
    StructuredContentImage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageImportRequirement {
    /// This is of type [`ImageImportType::Image`]
    image_type: ImageImportMatchType,
    target: TermGlossaryImage,
    source: TermGlossaryImage,
    entry: DatabaseTermEntry,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredContentImageImportRequirement {
    /// This is of type [`ImageImportType::StructuredContentImage`]
    image_type: ImageImportMatchType,
    target: TermGlossaryImage,
    source: TermGlossaryImage,
    entry: DatabaseTermEntry,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRequirementContext {
    //file_map: ArchiveFileMap,
    media: HashMap<String, MediaDataArrayBufferContent>,
}

// #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
// pub struct ArchiveFileMap {
//     Hashmap<String, >
// }

impl Yomichan {
    /// Adds a term entry to the database
    pub fn add_term(&self, key: &str, term: TermEntry) -> Result<(), errors::DBError> {
        let tx = self.ycdatabase.db.begin_write()?;
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
        let tx = self.ycdatabase.db.begin_read()?;
        let table = tx.open_table(db_stores::TERMS_STORE)?;

        if let Some(value_guard) = table.get(key)? {
            let stored_term: TermEntry = bincode::deserialize(value_guard.value())?;
            Ok(Some(stored_term))
        } else {
            Ok(None)
        }
    }
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

pub type Entries = Vec<Vec<EntryItem>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryItem {
    Str(String),
    /// `i64` because `i128` & `u128` dont work with untagged enums.
    /// [serde_json/issues/1155](https://github.com/serde-rs/json/issues/1155)
    /// is an `integer-overflow` so it needs a fix
    Int(i64),
    ContentVec(Vec<StructuredContent>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    /// This should **always** have `"type": "structured-content"` inside the json.
    /// If not, the dictionary is not valid.
    #[serde(rename = "type")]
    content_type: String,
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

pub fn prepare_dictionary<P: AsRef<std::path::Path>>(
    zip_path: P,
) -> Result<(), errors::ImportError> {
    let instant = Instant::now();
    let files_read = AtomicUsize::new(0);
    //let temp_dir_path = extract_dict_zip(zip_path)?;

    fs::read_dir(&zip_path)?
        .par_bridge()
        .try_for_each(|entry| {
            let entry = entry?;
            let outpath = entry.path();

            if outpath.to_str().unwrap().contains("term_bank")
                && !outpath.to_str().unwrap().ends_with('/')
            {
                let file = fs::File::open(&outpath)?;
                let reader = BufReader::new(file);

                let mut stream = Deserializer::from_reader(reader).into_iter::<Entries>();
                let entries = match stream.next() {
                    Some(Ok(entries)) => entries,
                    Some(Err(err)) => {
                        return Err(errors::ImportError::OtherJSON(format!(
                            "File: {} | Err: {}",
                            &outpath.to_str().unwrap(),
                            err
                        )))
                    }
                    None => {
                        return Err(errors::ImportError::OtherJSON(
                            "no data in dictionary stream".to_string(),
                        ))
                    }
                };

                // Beginning of each word/phrase/expression (entry)
                // ie: ["headword","reading","","",u128,[{/* main */}]]];
                for entry in entries {
                    //println!("{:#?}", entry);
                    let (headword, reading) = match (&entry[0], &entry[1]) {
                        (EntryItem::Str(headword), EntryItem::Str(reading)) => (headword, reading),
                        _ => continue,
                    };

                    if let EntryItem::ContentVec(content) = &entry[5] {
                        let struct_cont = &content[0];
                        //println!("{:#?}", struct_cont);
                    }
                }

                files_read.fetch_add(1, Ordering::SeqCst);
                println!("{:?}", files_read);
            }
            Ok(())
        })?;

    println!(
        "{} files read in {}s",
        files_read.load(Ordering::SeqCst),
        instant.elapsed().as_secs_f32()
    );

    Ok(())
}

// fn process_content(content_obj: &Content) {
//     match &*content_obj.content {
//         ContentValue::Str(def) => println!("{}", def),
//         ContentValue::Obj(nest_cont) => {
//             for entry_section in nest_cont {
//                 if let Ok(nested_content_obj) =
//                     serde_json::from_value::<Content>(entry_section.clone())
//                 {
//                     process_content(&nested_content_obj);
//                 }
//             }
//         }
//     }
// }
