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

}
