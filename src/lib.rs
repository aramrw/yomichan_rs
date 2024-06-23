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

    /// Adds a term entry to the database
    pub fn add_term(&self, key: &str, term: TermEntry) -> Result<(), Box<dyn Error>> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TERMS_TABLE)?;

            let term_bytes = bincode::serialize(&term)?;
            table.insert(key, &*term_bytes)?;
        }
        tx.commit()?;

        Ok(())
    }

}
