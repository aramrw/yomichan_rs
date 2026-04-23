//! # Yomichan RS
//!
//! A high-performance Rust engine for processing and searching Yomitan-format dictionaries.
//!
//! `yomichan_rs` provides a unified, thread-safe API for dictionary lookups, text scanning,
//! and Anki integration. It is designed to be embedded in applications that need
//! fast and accurate Japanese (and other language) dictionary features.
//!
//! ## Quick Start
//!
//! To use the engine, follow these steps in order:
//!
//! ### 1. Initialize
//! The main entry point is the [`Yomichan`] struct. Initializing it requires a path to a
//! database file (`.ycd`).
//!
//! ```rust,no_run
//! use yomichan_rs::Yomichan;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let ycd = Yomichan::new("path/to/database.ycd")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 2. Import Dictionaries
//! You must import Yomitan-format dictionaries (usually `.zip` files) before you can search.
//! This automatically updates the persistent settings in the database.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let ycd = Yomichan::new("db.ycd")?;
//! let dictionary_paths = vec!["path/to/jmdict.zip", "path/to/kanjidic.zip"];
//!
//! // Import multiple dictionaries at once
//! ycd.import_dictionaries(&dictionary_paths)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 3. Configure
//! You can change settings like the active language or enable/disable specific dictionaries.
//! Remember to call [`Yomichan::save_settings`] to persist these changes.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let ycd = Yomichan::new("db.ycd")?;
//! // Set the primary language to Spanish
//! ycd.set_language("es")?;
//!
//! // Persist changes to the database
//! ycd.save_settings()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 4. Search
//! Perform lookups on text. The search result hierarchy is designed for building rich UIs:
//! - **[`SearchSegment`]**: A discrete chunk of the input sentence (a "word").
//! - **[`TermDictionaryEntry`]**: A collection of related meanings (definitions) and written forms (headwords) for a word.
//!
//! To make rendering easy, use the **[`linked_definitions`](TermDictionaryEntry::linked_definitions)** helper.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let ycd = Yomichan::new("db.ycd")?;
//! let result = ycd.search("日本語が好きです")?;
//!
//! for segment in result.segments {
//!     for entry in &segment.entries {
//!         // Ergonomic helper: automatically links meanings to their headwords
//!         for linked in entry.linked_definitions() {
//!             println!("Meaning from {}:", linked.definition.dictionary);
//!             
//!             for headword in linked.headwords {
//!                 println!("  Variant: {} ({})", headword.term, headword.reading);
//!             }
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `anki`: Enables high-level Anki integration via AnkiConnect.
//! - `audio`: (Optional) Features for audio playback and management.

/// Anki integration and note generation.
#[cfg(feature = "anki")]
pub mod anki;
/// Audio playback and management.
pub mod audio;
mod backend;
/// Database interaction, schema definition, and import logic.
pub mod database;
/// Data models representing dictionaries, terms, definitions, and frequencies.
pub mod models;
/// Text scanning and sentence parsing algorithms.
pub mod scanner;
/// Settings, profiles, and application configuration.
pub mod settings;
/// Core translation logic to interpret dictionary database entries.
pub mod translator;
/// Utility types, custom errors, and test helpers.
pub mod utils;

use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// public re-exports:
pub use crate::database::dictionary_importer;
pub use crate::database::DictionaryService;
pub use crate::models::dictionary::{
    TermDefinition, TermDictionaryEntry, TermFrequency, TermHeadword, TermPronunciation,
};
pub use crate::scanner::core::{
    SearchResult, SearchSegment, TermSearchResults, TermSearchResultsSegment, TextScanner,
};
#[cfg(feature = "anki")]
use crate::settings::core::AnkiOptions;
pub use crate::settings::core::{ProfileResult, YomichanProfile};
pub use crate::translator::Translator;
pub use crate::utils::errors::{DBError, InitError, YomichanError};
pub use crate::utils::{Ptr, PtrRGaurd, PtrWGaurd};

// External re-exports
pub use anki_direct;
pub use indexmap;
pub use parking_lot;

/// A Yomichan Dictionary instance, providing a comprehensive interface for dictionary lookups,
/// text processing, and anki integration.
pub struct Yomichan {
    db: Arc<DictionaryDatabase>,
    backend: Backend,
}

impl Yomichan {
    /// Creates a new `Yomichan` instance and initializes the backend database.
    ///
    /// If a directory is provided, it searches for a database file or creates one inside
    /// a `yomichan_rs` subdirectory.
    ///
    /// # Arguments
    /// * `path` - A path to a `.ycd` file or a directory where the database should be stored.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YomichanError> {
        let path = path.as_ref().to_path_buf();
        let db_path = resolve_db_path(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));
        #[cfg(not(feature = "anki"))]
        let backend = Backend::new(db.clone()).map_err(|err| {
            DBError::Import(crate::utils::errors::ImportError::ExternalImporter(
                err.to_string(),
            ))
        })?;
        #[cfg(feature = "anki")]
        let backend = Backend::default_sync(db.clone())?;
        Ok(Self { db, backend })
    }

    /// Returns a reference to the underlying dictionary database.
    pub fn db(&self) -> &Arc<DictionaryDatabase> {
        &self.db
    }

    /// Returns a reference to the internal backend.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    /// Searches for terms in the given text using the currently selected profile.
    ///
    /// This method is an alias for [`Yomichan::search_structured`].
    ///
    /// # Arguments
    /// * `text` - The text to search. Typically a single sentence or word.
    ///
    /// # Returns
    /// A [`SearchResult`] containing parsed segments and matched dictionary entries.
    pub fn search(&self, text: &str) -> Result<SearchResult, YomichanError> {
        self.search_structured(text)
    }

    /// Performs a structured search on the given text using the current profile.
    ///
    /// The input text is scanned for matching dictionary terms using a longest-match strategy.
    /// The result is segmented into chunks of matched terms and unmatched raw text,
    /// making it suitable for UI display (e.g., clickable words).
    ///
    /// # Arguments
    /// * `text` - The text to search. Typically a single sentence or word.
    ///
    /// # Returns
    /// A [`SearchResult`] containing parsed segments and matched dictionary entries.
    pub fn search_structured(&self, text: &str) -> Result<SearchResult, YomichanError> {
        tracing::info!("Structured search requested for text: '{}'", text);
        let profile = self.backend.get_current_profile().map_err(|e| {
            tracing::error!("Failed to get current profile: {:?}", e);
            e
        })?;
        let profile = profile.read();
        let opts = profile.options();
        let res = self
            .backend
            .scanner
            .search_sentence(text, opts)
            .ok_or_else(|| {
                tracing::warn!("search_sentence returned None for text: '{}'", text);
                YomichanError::Search(crate::utils::errors::SearchError::Failed)
            })?;

        tracing::info!("Search found {} total dictionary entries", res.dictionary_entries.len());
        let segments = crate::scanner::core::SentenceParser::parse_to_structured(res);
        tracing::info!("Parsed into {} segments", segments.len());
        
        Ok(SearchResult {
            segments,
            original_text: text.to_string(),
        })
    }

    /// Deletes all database files and directories associated with Yomichan in the given path.
    pub fn nuke_database(path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(());
        }
        if path.is_file() {
            if path.extension() == Some(std::ffi::OsStr::new("ycd")) {
                std::fs::remove_file(path)?;
            }
            return Ok(());
        }
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                if entry_path.is_file() && entry_path.extension() == Some(std::ffi::OsStr::new("ycd")) {
                    std::fs::remove_file(entry_path)?;
                }
            }
            // Try to remove the default subdirectory if it exists
            let subdir = path.join("yomichan_rs");
            if subdir.is_dir() {
                let _ = std::fs::remove_dir_all(subdir);
            }
        }
        Ok(())
    }
}

fn resolve_db_path(p: PathBuf) -> Result<PathBuf, Box<InitError>> {
    const DB_FILENAME: &str = "db.ycd";
    const DB_SUBDIR: &str = "yomichan_rs";
    if p.extension() == Some(std::ffi::OsStr::new("ycd")) {
        if let Some(parent) = p.parent() {
            if !parent.exists() {
                return Err(InitError::MissingParent { p }.into());
            }
        }
        return Ok(p);
    }
    if p.is_file() {
        return Err(InitError::InvalidPath { p }.into());
    }
    if p.is_dir() {
        let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&p)
            .map_err(|e| Box::new(InitError::Io(e)))?
            .filter_map(Result::ok)
            .collect();
        if let Some(entry) = entries.iter().find(|e| {
            e.path().is_file() && e.path().extension() == Some(std::ffi::OsStr::new("ycd"))
        }) {
            return Ok(entry.path());
        }
        let subdir_path = p.join(DB_SUBDIR);
        if subdir_path.is_dir() {
            return Ok(subdir_path.join(DB_FILENAME));
        }
        let db_path = if entries.is_empty() {
            p.join(DB_FILENAME)
        } else {
            // Only create subdirectory if there are other files present
            let _ = std::fs::create_dir(&subdir_path);
            subdir_path.join(DB_FILENAME)
        };
        return Ok(db_path);
    }
    Err(InitError::InvalidPath { p }.into())
}

#[cfg(test)]
mod yomichan_ergonomics_tests {
    use super::*;
    use crate::utils::test_utils::TEST_PATHS;
    #[test]
    fn test_with_profile_mut_ergonomics() {
        let ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        let lang = ycd
            .with_profile_mut(|profile| {
                profile.set_language("es");
                profile.options().general.language.clone()
            })
            .expect("Should access profile");
        assert_eq!(lang, "es");
        let read_lang = ycd
            .with_profile(|profile| profile.options().general.language.clone())
            .expect("Should access profile");
        assert_eq!(read_lang, "es");
    }
}
