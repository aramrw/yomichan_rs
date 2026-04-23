//! # Database and Dictionary Management
//!
//! This module provides the infrastructure for storing and retrieving dictionary data.
//!
//! It handles the low-level SQLite or in-memory storage, as well as the logic for
//! importing Yomitan-format dictionaries.
//!
//! ## Key Components
//!
//! - **[`DictionaryDatabase`](crate::database::dictionary_database::DictionaryDatabase)**: The primary database engine.
//! - **[`DictionaryService`]**: A trait that abstracts dictionary lookup operations.
//! - **[`dictionary_importer`]**: Utilities for importing dictionary ZIP files.
//!
//! ## Importing Dictionaries
//!
//! To use `yomichan_rs`, you must first import one or more dictionaries.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let ycd = Yomichan::new("db.ycd")?;
//!
//! // Import dictionaries from ZIP files
//! ycd.import_dictionaries(&["jmdict.zip", "kanjidic.zip"])?;
//! # Ok(())
//! # }
//! ```

/// Low-level database interface for dictionary storage and retrieval.
pub mod dictionary_database;
/// Logic for importing Yomitan-format dictionaries into the local database.
pub mod dictionary_importer;

pub use dictionary_database::{
    DatabaseKanjiEntry, DatabaseKanjiMeta, DatabaseMetaFrequency, DatabaseMetaMatchType,
    DatabaseMetaPhonetic, DatabaseMetaPitch, DatabaseTag, DatabaseTermEntry, DatabaseTermMeta,
    DictionaryDatabase, DictionaryDatabaseError, GenericQueryRequest, QueryRequestError,
    QueryRequestMatchType, QueryType, TermExactQueryRequest,
};
pub use dictionary_importer::DictionarySummary;

/// A trait for services that provide dictionary lookup capabilities.
///
/// This trait abstracts over different database backends (e.g., in-memory or SQLite)
/// to provide a consistent interface for the translator and scanner.
pub trait DictionaryService: Send + Sync {
    /// Retrieves the raw settings data stored in the database.
    fn get_settings(&self) -> Result<Option<Vec<u8>>, Box<DictionaryDatabaseError>>;

    /// Saves the raw settings data to the database.
    fn set_settings(&self, blob: &[u8]) -> Result<(), Box<DictionaryDatabaseError>>;

    /// Returns a summary of all dictionaries currently installed in the database.
    fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>>;

    /// Performs a bulk lookup of tag metadata for the given queries.
    fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>>;

    /// Performs a bulk lookup of term metadata (like frequency) for the given keys.
    ///
    /// # Arguments
    /// * `keys` - A set of terms or readings to look up.
    /// * `enabled_dictionaries` - The set of dictionaries to search within.
    fn find_term_meta_bulk(
        &self,
        keys: &indexmap::IndexSet<String>,
        enabled_dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
    ) -> Result<Vec<DatabaseTermMeta>, Box<DictionaryDatabaseError>>;

    /// Finds dictionary entries that exactly match the given terms.
    fn find_terms_exact_bulk(
        &self,
        terms: &[TermExactQueryRequest],
        enabled_dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;

    /// Finds terms based on a sequence of queries, often used for deinflection.
    fn find_terms_by_sequence_bulk(
        &self,
        queries: Vec<GenericQueryRequest>,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;

    /// Generic bulk term lookup with configurable match types (exact, prefix, etc.).
    fn find_terms_bulk(
        &self,
        term_list: &[String],
        dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
        match_type: yomichan_importer::dictionary_database::TermSourceMatchType,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;
}
