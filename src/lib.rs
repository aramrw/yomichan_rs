#![allow(unused)]
mod dictionary;
mod dictionary_data;
mod dictionary_database;
mod dictionary_importer;
mod dictionary_worker_handler;
mod errors;
mod freq;
mod settings;
mod structured_content;
mod tests;

use crate::settings::Options;

use redb::{Database as ReDatabase /* ReadableTable, TableDefinition */};

use std::path::Path;

pub struct Yomichan {
    db: ReDatabase,
    options: Options,
}

impl Yomichan {
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary DB Connection.
    ///
    /// # Arguments
    /// 
    /// * `db_path` - The location where the database will be created.
    ///
    /// ```
    /// use yomichan_rs::Yomichan;
    /// let ycd = Yomichan::new("C:\\Users\\1\\Desktop");
    /// ```
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, errors::InitError> {
        let db = ReDatabase::create(db_path)?;
        Ok(Self {
            db,
            options: Options::default(),
        })
    }
}

/// Formats the path to include `.yc` as the extension.
fn format_db_path<P: AsRef<Path>>(path: P) -> Result<String, InitError> {
    let path_ref = path.as_ref();
    if let Some(parent) = path_ref.parent() {
        return match parent.exists() {
            true => Ok(path_ref.with_extension("yc").to_string_lossy().to_string()),
            false => Err(InitError::Path(format!(
                "path does not exist: {}",
                path_ref.to_string_lossy()
            ))),
        };
    }
    Err(InitError::Path(format!(
        "path does not exist: {}",
        path_ref.to_string_lossy()
    )))
}
