//! # Yomichan RS
//!
//! A Rust-based engine for processing and searching Yomitan-format dictionaries.
//!
//! This crate provides a comprehensive interface for dictionary lookups, text processing,
//! and Anki integration, designed for performance and ease of use. It encapsulates
//! the database connection, text scanning logic, and user settings into a unified,
//! thread-safe API.
//!
//! ## Examples
//!
//! Here is a basic example of how to initialize the engine, import dictionaries, and perform a search.
//!
//! ```no_run
//! use yomichan_rs::Yomichan;
//! use std::sync::{LazyLock};
//!
//! // Best practice: Initialize Yomichan once as a static variable
//! // to avoid repeatedly opening the database.
//! static YCD: LazyLock<Yomichan> = LazyLock::new(|| {
//!     // Create a new Yomichan instance.
//!     // This will create a `db.ycd` file in the specified directory.
//!     let mut ycd = Yomichan::new("path/to/your/db_directory").unwrap();
//!
//!     // Import dictionaries (e.g., from a folder containing Yomitan-format dictionaries).
//!     // This only needs to be done once.
//!     if ycd.dictionary_summaries().unwrap().is_empty() {
//!         // ycd.import_dictionaries(&["path/to/your/dictionaries"]).unwrap();
//!     }
//!
//!     // Set the language for text processing.
//!     ycd.set_language("ja").unwrap();
//!
//!     ycd
//! });
//!
//! fn main() {
//!     // Perform a search using a shared reference (&Yomichan).
//!     if let Some(results) = YCD.search("日本語を勉強している") {
//!         for segment in results {
//!             if let Some(search_results) = segment.results {
//!                 println!("Found term: {}", segment.text);
//!                 for entry in search_results.dictionary_entries {
//!                     // Process each dictionary entry.
//!                     println!("  - Headword: {}", entry.get_headword_text_joined());
//!                 }
//!             }
//!         }
//!     }
//! }
//! ```

#[cfg(feature = "anki")]
pub mod anki;
mod audio;
mod backend;
pub mod database;
pub mod models;
mod environment;
mod errors;
mod method_modules;
mod regex_util;
pub mod settings;
pub mod test_utils;
pub mod text_scanner;
mod translation;
mod translation_internal;
mod translator;

pub use anki_direct;
use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use derive_more::derive::DerefMut;
pub use indexmap;
pub use parking_lot::{
    ArcRwLockReadGuard, ArcRwLockUpgradableReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock,
};
use serde::Deserialize;
use serde::Serialize;
use settings::YomichanProfile;

use native_db::*;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;

use std::fs::DirEntry;
use std::sync::Arc;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "anki")]
use crate::anki::DisplayAnkiError;
// public exports:
pub use crate::database::dictionary_importer;
pub use crate::models::dictionary::{
    TermDefinition, TermDictionaryEntry, TermFrequency, TermPronunciation,
};
pub use crate::errors::DBError;
use crate::errors::YomichanError;
pub use crate::text_scanner::{TermSearchResults, TermSearchResultsSegment};
pub use crate::translator::Translator;
pub use crate::database::DictionaryService;
pub use crate::environment::EnvironmentInfo;
pub use crate::text_scanner::TextScanner;

// re-export parking lot cuz its too good
use derive_more::Deref;
pub use parking_lot;

#[macro_export]
macro_rules! iter_type_to_iter_variant {
    ($v:expr, $variant:path) => {
        $v.into_iter().map(|item| $variant(item))
    };
}

#[macro_export]
macro_rules! iter_variant_to_iter_type {
    ($v:expr, $variant:path) => {
        $v.into_iter()
            .filter_map(|item| {
                if let $variant(inner) = item {
                    Some(inner)
                } else {
                    None
                }
            })
            .collect()
    };
}

/// type alias for a [ArcRwLockReadGuard];
pub type PtrRGaurd<T> = ArcRwLockReadGuard<RawRwLock, T>;
pub type PtrWGaurd<T> = ArcRwLockWriteGuard<RawRwLock, T>;
/// Simple abstraction over [parking_lot::RwLock]
#[derive(Deref, DerefMut)]
pub struct Ptr<T>(Arc<RwLock<T>>);

impl<T: ToKey> ToKey for Ptr<T> {
    fn to_key(&self) -> Key {
        // get exclusive read & write access before writing to the database
        let ptr = &*self.clone().write_arc();
        ptr.to_key()
    }
    fn key_names() -> Vec<String> {
        vec!["Ptr".into(), "YomichanPtr".into()]
    }
}

impl<T> Ptr<T> {
    pub fn new(val: T) -> Self {
        Ptr(Arc::new(RwLock::new(val)))
    }
    /// Executes a closure with an immutable reference to the inner data.
    /// Used for quick reads to the inner `&T`
    ///
    /// # Example
    /// ```
    /// let name = my_ptr.with(|data| data.name.clone());
    /// ```
    pub fn with_ptr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read();
        f(&*guard)
    }

    /// Acquires a write lock, runs the closure, and releases the lock.
    /// Used for quick writes to the inner `&mut T`
    ///
    /// # Example
    ///
    /// ```
    /// my_ptr.with_ptr_mut(|data| {
    ///     data.counter += 1;
    /// });
    /// ```
    pub fn with_ptr_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write();
        f(&mut *guard)
    }
}

impl<T> From<T> for Ptr<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(parking_lot::RwLock::new(value)))
    }
}
impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T: PartialEq> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        // Lock both for reading and then compare the values.
        // The locks are short-lived and dropped at the end of the statement.
        let self_guard = self.0.read();
        let other_guard = other.0.read();
        *self_guard == *other_guard
    }
}
impl<T: Eq> Eq for Ptr<T> {}
impl<T: PartialOrd> PartialOrd for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.read().partial_cmp(&*other.0.read())
    }
}
impl<T: Ord> Ord for Ptr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.read().cmp(&*other.0.read())
    }
}
impl<T: Hash> Hash for Ptr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.read().hash(state);
    }
}
impl<T: fmt::Debug> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ptr").field(&*self.0.read()).finish()
    }
}
impl<T: Default> Default for Ptr<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(T::default())))
    }
}
impl<'de, T> Deserialize<'de> for Ptr<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: T = T::deserialize(deserializer)?;
        Ok(Ptr::from(value))
    }
}
impl<T> Serialize for Ptr<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let guard = self.0.read();
        T::serialize(&*guard, serializer)
    }
}

/// A Yomichan Dictionary instance, providing a comprehensive interface for dictionary lookups,
/// text processing, and anki integration.
///
/// This struct is the primary entry point for interacting with the Yomichan library. It encapsulates
/// the database connection, text scanning logic, and user settings, offering a unified and
/// thread-safe API.
///
/// # Examples
///
/// ```no_run
/// use yomichan_rs::Yomichan;
/// use std::sync::{LazyLock, RwLock};
///
/// // Best practice: Initialize Yomichan once as a static variable
/// // to avoid repeatedly opening the database.
/// static YCD: LazyLock<RwLock<Yomichan>> = LazyLock::new(|| {
///     // Create a new Yomichan instance.
///     // This will create a `db.ycd` file in the specified directory.
///     let mut ycd = Yomichan::new("path/to/your/db_directory").unwrap();
///
///     // Import dictionaries (e.g., from a folder containing Yomitan-format dictionaries).
///     // This only needs to be done once.
///     if ycd.get_dictionaries().unwrap().is_empty() {
///         ///         ycd.import_dictionaries(&["path/to/your/dictionaries"]).unwrap();
///     }
///
///     // Set the language for text processing.
///     ycd.set_language("ja").unwrap();
///
///     RwLock::new(ycd)
/// });
///
/// fn main() {
///     // Lock the Yomichan instance for writing to perform a search.
///     let mut ycd = YCD.write().unwrap();
///
///     // Perform a search.
///     if let Some(results) = ycd.search("日本語を勉強している") {
///         for segment in results {
///             if let Some(search_results) = segment.results {
///                 println!("Found term: {}", segment.text);
///                 for entry in search_results.dictionary_entries {
///                     // Process each dictionary entry.
///                     println!("  - Headword: {}", entry.get_headword_text_joined());
///                 }
///             } else {
///                 // This segment of text did not match any dictionary entries.
///                 println!("Unrecognized text: {}", segment.text);
///             }
///         }
///     }
/// }
/// ```
///
/// For more details on search results, see [`TermSearchResults`].
pub struct Yomichan<'a> {
    db: Arc<DictionaryDatabase>,
    backend: Backend<'a>,
}

impl<'a> Yomichan<'a> {
    /// Deletes all database files and directories associated with Yomichan in the given path.
    ///
    /// This is a "nuke" option that will permanently delete all dictionary data and settings.
    /// This should only be called when the Yomichan instance is NOT active.
    pub fn nuke_database(path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(());
        }

        // 1. If it's a file, delete it if it ends in .ycd
        if path.is_file() {
            if path.extension() == Some(std::ffi::OsStr::new("ycd")) {
                std::fs::remove_file(path)?;
            }
            return Ok(());
        }

        // 2. If it's a directory, look for *.ycd files and the yomichan_rs subdirectory
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

    /// Opens or creates a Yomichan database at the specified path.
    ///
    /// This function provides a flexible and robust way to initialize the database,
    /// automatically handling path resolution for different scenarios.
    ///
    /// - **If `path` ends in `.ycd`**: It directly opens or creates the specified database file.
    ///   The parent directory must exist.
    /// - **If `path` is a directory**:
    ///   - If the directory is empty, it creates `db.ycd` inside it.
    ///   - If the directory is not empty, it creates a `yomichan_rs` subdirectory and
    ///     places `db.ycd` inside to avoid cluttering the existing directory.
    ///   - If a `.ycd` file or a `yomichan_rs` subdirectory already exists, it will use it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use yomichan_rs::Yomichan;
    ///
    /// // Example 1: Explicitly name the database file.
    /// // Creates or opens `/path/to/my_database.ycd`.
    /// let ycd1 = Yomichan::new("/path/to/my_database.ycd ").unwrap();
    ///
    /// // Example 2: Point to an empty directory.
    /// // Creates `/path/to/empty_dir/db.ycd`.
    /// let ycd2 = Yomichan::new("/path/to/empty_dir ").unwrap();
    ///
    /// // Example 3: Point to a non-empty directory (e.g., your Desktop).
    /// // Creates a subdirectory: `/path/to/desktop/yomichan_rs/db.ycd`.
    /// let ycd3 = Yomichan::new("/path/to/desktop ").unwrap();
    ///
    /// // Example 4: Open an existing database created by a previous run.
    /// // This will successfully open the database created in Example 3.
    /// let ycd4 = Yomichan::new("/path/to/desktop ").unwrap();
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YomichanError> {
        let path = path.as_ref().to_path_buf();
        // Use the new, consolidated path resolution logic.
        let db_path = resolve_db_path(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));

        #[cfg(not(feature = "anki"))]
        let backend = Backend::new(db.clone()).map_err(|err| DBError::Import(crate::errors::ImportError::ExternalImporter(err.to_string())))?;
        #[cfg(feature = "anki")]
        let backend = Backend::default_sync(db.clone())?;

        Ok(Self { db, backend })
    }

    /// Executes a closure with immutable access to the current profile.
    pub fn with_profile<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let profile = profile_ptr.read();
        Ok(f(&profile))
    }

    /// Executes a closure with mutable access to the current profile.
    pub fn with_profile_mut<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&mut YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let mut profile = profile_ptr.write();
        Ok(f(&mut profile))
    }

    #[cfg(feature = "anki")]
    /// Executes a closure with immutable access to the current profile's AnkiOptions.
    pub fn with_anki_options<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&settings::AnkiOptions) -> R,
    {
        self.with_profile(|p| f(p.anki_options()))
    }

    #[cfg(feature = "anki")]
    /// Executes a closure with mutable access to the current profile's AnkiOptions.
    pub fn with_anki_options_mut<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&mut settings::AnkiOptions) -> R,
    {
        self.with_profile_mut(|p| f(p.anki_options_mut()))
    }
}

/// Determines the correct database file path based on the user's input.
///
/// This function mimics the robust path handling of libraries like `redb`:
///
/// 1.  **Explicit `.ycd` File Path**: If the path ends with `.ycd`, it's used directly.
///     - The parent directory must exist.
///     - The database file itself doesn't need to exist; it will be created.
///     - Example: `Yomichan::new("path/to/my_db.ycd")`
///
/// 2.  **Directory Path**: If the path is an existing directory, the function will search for or
///     determine the location of the database file inside it.
///     - It first looks for an existing `*.ycd` file within the directory.
///     - If not found, it looks for a `yomichan_rs` subdirectory to use.
///     - If neither is found, it decides where to create the database:
///         - If the directory is **empty**, it creates `db.ycd` directly inside it.
///           (e.g., `/path/to/empty_dir/` -> `/path/to/empty_dir/db.ycd`)
///         - If the directory is **not empty**, it creates a `yomichan_rs` subdirectory
///           and places the database inside.
///           (e.g., `/path/to/non_empty_dir/` -> `/path/to/non_empty_dir/yomichan_rs/db.ycd`)
///
/// 3.  **Invalid Paths**: Any other case, such as pointing to an existing file that is not
///     a `.ycd` file, or a path that doesn't exist and isn't a valid `.ycd` target,
///     will result in an error.
fn resolve_db_path(p: PathBuf) -> Result<PathBuf, Box<InitError>> {
    const DB_FILENAME: &str = "db.ycd";
    const DB_SUBDIR: &str = "yomichan_rs";

    // Rule 1: Path is explicitly a .ycd file.
    if p.extension() == Some(OsStr::new("ycd")) {
        // Ensure the parent directory exists, so the DB file can be created.
        if let Some(parent) = p.parent() {
            if !parent.exists() {
                // If parent is a file or doesn't exist, it's an error.
                return Err(InitError::MissingParent { p }.into());
            }
        }
        return Ok(p);
    }

    // If the path points to an existing file that is NOT a .ycd file, it's invalid.
    if p.is_file() {
        return Err(InitError::InvalidPath { p }.into());
    }

    // Rule 2: Path is a directory.
    if p.is_dir() {
        let entries: Vec<DirEntry> = fs::read_dir(&p)
            .map_err(|e| Box::new(InitError::Io(e)))?
            .filter_map(Result::ok)
            .collect();

        // Search for an existing .ycd file in the directory.
        if let Some(entry) = entries
            .iter()
            .find(|e| e.path().is_file() && e.path().extension() == Some(OsStr::new("ycd")))
        {
            return Ok(entry.path());
        }

        // Check for an existing `yomichan_rs` subdirectory.
        let subdir_path = p.join(DB_SUBDIR);
        if subdir_path.is_dir() {
            return Ok(subdir_path.join(DB_FILENAME));
        }

        // Decide where to create the new database.
        let db_path = if entries.is_empty() {
            // Directory is empty, create DB file directly inside.
            p.join(DB_FILENAME)
        } else {
            // Directory is not empty, create a subdirectory for the DB.
            fs::create_dir(&subdir_path).map_err(|e| Box::new(InitError::Io(e)))?;
            subdir_path.join(DB_FILENAME)
        };
        return Ok(db_path);
    }

    // Rule 3: Path doesn't exist and is not a .ycd file path. It's invalid.
    Err(InitError::InvalidPath { p }.into())
}

#[derive(thiserror::Error, Debug)]
#[error("could not create yomichan_rs dictionary database:")]
pub enum InitError {
    #[error(
        r#"
invalid path: {p} | help:
  1. "~/.home/db.ycd" - opens a ycd instance
  2. "~/.home/test"   - creates a new (blank) .ycd file"#
    )]
    InvalidPath { p: PathBuf },
    #[error("path does not have a parent: {p}")]
    MissingParent { p: PathBuf },
    #[error("db conn err: {0}")]
    DatabaseConnectionFailed(#[from] Box<db_type::Error>),
    #[error("io err: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "anki")]
    #[error("display anki: {0}")]
    DisplayAnki(#[from] DisplayAnkiError),
}
mod init_err_impls {
    use crate::InitError;

    #[cfg(feature = "anki")]
    mod anki {
        use crate::{anki::DisplayAnkiError, InitError};

        impl From<Box<DisplayAnkiError>> for InitError {
            fn from(e: Box<DisplayAnkiError>) -> Self {
                InitError::DisplayAnki(*e)
            }
        }
        impl From<Box<DisplayAnkiError>> for Box<InitError> {
            fn from(e: Box<DisplayAnkiError>) -> Self {
                Box::new(InitError::DisplayAnki(*e))
            }
        }
    }
    impl From<native_db::db_type::Error> for InitError {
        fn from(e: native_db::db_type::Error) -> Self {
            InitError::DatabaseConnectionFailed(Box::new(e))
        }
    }
    impl From<native_db::db_type::Error> for Box<InitError> {
        fn from(e: native_db::db_type::Error) -> Self {
            Box::new(InitError::DatabaseConnectionFailed(Box::new(e)))
        }
    }
    impl From<Box<native_db::db_type::Error>> for Box<InitError> {
        fn from(e: Box<native_db::db_type::Error>) -> Self {
            Box::new(InitError::DatabaseConnectionFailed(e))
        }
    }
}

#[cfg(test)]
mod yomichan_ergonomics_tests {
    use super::*;
    use crate::test_utils::TEST_PATHS;

    #[test]
    fn test_with_profile_mut_ergonomics() {
        let ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();

        // Mutate and return a value
        let lang = ycd
            .with_profile_mut(|profile| {
                profile.set_language("es");
                profile.options().general.language.clone()
            })
            .expect("Should access profile");

        assert_eq!(lang, "es");

        // Verify via read accessor
        let read_lang = ycd
            .with_profile(|profile| profile.options().general.language.clone())
            .expect("Should access profile");

        assert_eq!(read_lang, "es");
    }
}
