#![allow(unused)]
mod dictionary;
mod dictionary_data;
mod dictionary_database;
mod dictionary_importer;
mod errors;
mod freq;
mod structured_content;
mod tests;
mod dictionary_worker_handler;

// dictionary.rs
//use crate::dictionary_data::TermEntry;
use redb::{Database as ReDatabase /* ReadableTable, TableDefinition */};

pub struct YCDatabase {
    pub db: ReDatabase,
}

pub struct Yomichan {
    ycdatabase: YCDatabase,
}

impl YCDatabase {
    /// Initializes a new Yomichan Dictionary DB Connection
    pub fn new(db_path: &str) -> Result<Self, errors::InitError> {
        let db = ReDatabase::create(format!("{}.redb", db_path))?;
        Ok(Self { db })
    }
}
