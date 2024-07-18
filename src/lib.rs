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
