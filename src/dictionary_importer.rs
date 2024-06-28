use crate::dictionary_data::TermGlossaryImage;
use crate::dictionary_database::DatabaseTermEntry;
use crate::dictionary_database::{db_stores, TermEntry};
use crate::errors;
use crate::Yomichan;
use serde_json::Deserializer;
use tempfile::tempdir;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;
use std::io::{self, BufReader};

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

        Ok(())
    }
}

pub type Entries = Vec<Vec<EntryItem>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryItem {
    Str(String),
    Int(i64),
    ContentBlock(Vec<serde_json::Value>),
}


pub fn prepare_dictionary<P: AsRef<std::path::Path>>(
    zip_path: P,
) -> Result<(), errors::ImportError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let dir = tempdir()?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if outpath.to_str().unwrap().ends_with('/')
            || !outpath.to_str().unwrap().starts_with("term")
        {
            continue;
        }

        let outpath = dir.path().join(outpath);

        let mut outfile = fs::File::create(&outpath)?;
        io::copy(&mut file, &mut outfile)?;

        let file = fs::File::open(&outpath)?;
        let reader = BufReader::new(file);

        let mut stream = Deserializer::from_reader(reader).into_iter::<Entries>();

        let entries = match stream.next() {
            Some(Ok(entries)) => entries,
            Some(Err(err)) => return Err(err.into()),
            None => {
                return Err(errors::ImportError::OtherJSON(
                    "no data in dictionary stream".to_string(),
                ))
            }
        };

        for entry in entries {
            let (headword, reading) = match (&entry[0], &entry[1]) {
                (EntryItem::Str(headword), EntryItem::Str(reading)) => (headword, reading),
                _ => continue,
            };
        }
    }

    dir.close()?;

    Ok(())
}
