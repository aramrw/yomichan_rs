//! # Yomichan RS
//!
//! A Rust-based engine for processing and searching Yomitan-format dictionaries.
//!
//! This crate provides a comprehensive interface for dictionary lookups, text processing,
//! and Anki integration, designed for performance and ease of use. It encapsulates
//! the database connection, text scanning logic, and user settings into a unified,
//! thread-safe API.

pub mod anki;
pub mod audio;
mod backend;
pub mod database;
pub mod models;
pub mod scanner;
pub mod settings;
pub mod translator;
pub mod utils;

use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// public re-exports:
pub use crate::database::dictionary_importer;
pub use crate::database::DictionaryService;
pub use crate::models::dictionary::{
    TermDefinition, TermDictionaryEntry, TermFrequency, TermPronunciation,
};
pub use crate::scanner::core::{TermSearchResults, TermSearchResultsSegment, TextScanner};
pub use crate::translator::Translator;
pub use crate::utils::{Ptr, PtrRGaurd, PtrWGaurd};
pub use crate::utils::errors::{DBError, InitError, YomichanError};

// External re-exports
pub use anki_direct;
pub use indexmap;
pub use parking_lot;

/// A Yomichan Dictionary instance, providing a comprehensive interface for dictionary lookups,
/// text processing, and anki integration.
pub struct Yomichan<'a> {
    db: Arc<DictionaryDatabase>,
    backend: Backend<'a>,
}

impl<'a> Yomichan<'a> {
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
                if entry_path.is_file() {
                    if entry_path.extension() == Some(std::ffi::OsStr::new("ycd")) {
                        std::fs::remove_file(entry_path)?;
                    }
                } else if entry_path.is_dir() {
                    if entry_path.file_name() == Some(std::ffi::OsStr::new("yomichan_rs")) {
                        std::fs::remove_dir_all(entry_path)?;
                    }
                }
            }
        }
        Ok(())
    }
}

use crate::settings::core::{AnkiOptions, ProfileResult, YomichanProfile};

impl<'a> Yomichan<'a> {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YomichanError> {
        let path = path.as_ref().to_path_buf();
        let db_path = resolve_db_path(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));
        #[cfg(not(feature = "anki"))]
        let backend = Backend::new(db.clone()).map_err(|err| DBError::Import(crate::utils::errors::ImportError::ExternalImporter(err.to_string())))?;
        #[cfg(feature = "anki")]
        let backend = Backend::default_sync(db.clone())?;
        Ok(Self { db, backend })
    }

    pub fn with_profile<F, R>(&self, f: F) -> ProfileResult<R>
    where
        F: FnOnce(&YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let profile = profile_ptr.read();
        Ok(f(&profile))
    }

    pub fn with_profile_mut<F, R>(&self, f: F) -> ProfileResult<R>
    where
        F: FnOnce(&mut YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let mut profile = profile_ptr.write();
        Ok(f(&mut profile))
    }
}

#[cfg(feature = "anki")]
impl<'a> Yomichan<'a> {
    pub fn with_anki_options<F, R>(&self, f: F) -> ProfileResult<R>
    where
        F: FnOnce(&AnkiOptions) -> R,
    {
        self.with_profile(|p| f(p.anki_options()))
    }

    pub fn with_anki_options_mut<F, R>(&self, f: F) -> ProfileResult<R>
    where
        F: FnOnce(&mut AnkiOptions) -> R,
    {
        self.with_profile_mut(|p| f(p.anki_options_mut()))
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
        if let Some(entry) = entries
            .iter()
            .find(|e| e.path().is_file() && e.path().extension() == Some(std::ffi::OsStr::new("ycd")))
        {
            return Ok(entry.path());
        }
        let subdir_path = p.join(DB_SUBDIR);
        if subdir_path.is_dir() {
            return Ok(subdir_path.join(DB_FILENAME));
        }
        let db_path = if entries.is_empty() {
            p.join(DB_FILENAME)
        } else {
            std::fs::create_dir(&subdir_path).map_err(|e| Box::new(InitError::Io(e)))?;
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
