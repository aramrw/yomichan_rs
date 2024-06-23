use std::collections::HashMap;
mod database;
mod dictionary;
mod dictionary_data;
mod errors;
mod freq;
mod structured_content;
mod zip;
mod tests;

// dictionary.rs
use crate::dictionary_data::TermEntry;
use redb::{Database, ReadableTable, TableDefinition};
use std::error::Error;

const TERMS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("terms");

pub struct Yomichan {
    pub db: Database,
}

    /// Initializes a new Yomichan Dictionary
    pub fn new(db_path: &str) -> Result<Self, errors::InitError> {
        let db = Database::create(format!("{}.redb", db_path))?;
        Ok(Self { db })
    }

}
