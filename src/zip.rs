use crate::errors;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use tempfile::tempdir;

use std::fs;
use std::io::{self, BufReader};

pub type Entries = Vec<Vec<EntryItem>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryItem {
    Str(String),
    Int(i64),
    ContentBlock(Vec<serde_json::Value>),
}

pub fn import_dictionary<P: AsRef<std::path::Path>>(
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
