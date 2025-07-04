#![allow(unused)]
#[cfg(feature = "anki")]
pub mod anki;
mod audio;
mod backend;
mod database;
mod dictionary;
mod dictionary_data;
mod environment;
mod errors;
mod freq;
mod method_modules;
mod regex_util;
pub mod settings;
mod structured_content;
mod test_utils;
pub mod text_scanner;
mod translation;
mod translation_internal;
mod translator;

pub use anki_direct;
use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use database::dictionary_database::DB_MODELS;
use derive_more::derive::DerefMut;
use icu::time::scaffold::IntoOption;
pub use indexmap;
use indexmap::IndexMap;
pub use parking_lot::{
    ArcRwLockReadGuard, ArcRwLockUpgradableReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock,
};
use serde::Deserialize;
use serde::Serialize;
use settings::{YomichanOptions, YomichanProfile};

use native_db::*;
use native_model::{native_model, Model};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use text_scanner::TextScanner;
use transaction::RTransaction;
use translation::FindTermsOptions;
use translator::FindTermsMode;
use translator::Translator;

use std::collections::HashSet;
use std::fs::DirEntry;
use std::sync::Arc;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "anki")]
use crate::anki::DisplayAnkiError;
// public exports:
pub use crate::database::dictionary_importer;
pub use crate::dictionary::{
    TermDefinition, TermDictionaryEntry, TermFrequency, TermPronunciation,
};
#[cfg(not(feature = "anki"))]
use crate::errors::DBError;
use crate::errors::YomichanError;
pub use crate::text_scanner::{TermSearchResults, TermSearchResultsSegment};
// re-export parking lot cuz its too good
use derive_more::Deref;
pub use parking_lot;

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

/// A Yomichan Dictionary instance.
///
/// # Examples
/// ```
/// use yomichan_rs::Yomichan;
///
/// // must initialize as mut
/// let mut ycd = Yomichan::new("~/desktop/db.ycd");
/// // import dictionaries
/// ycd.import_dictionaries(&["~/desktop/dicts/daijirin"])?;
/// // set a language via it's iso
/// ycd.set_language("ja");
/// // optionally save the language to the db
/// ycd.update_options()?;
/// let res: Option<TermSearchResults> = ycd.search("まだ分かってない")
/// ```
/// For more info on results, reference the [TermSearchResults] docs.
///
/// # Best Practices
///
/// Unless you have a specific use case, it's best to initialize
/// the Yomichan struct once as an _interior mutable_ static variable:
/// ```
/// // Yomichan impls Send + Sync, so you can useRwLock over a Mutex.
/// static YCD: LazyLock<RwLock<Yomichan>> = LazyLock::new(|| {
///     let mut ycd = Yomichan::new("~/desktop/db.ycd").unwrap();
///     ycd.set_language("es");
///     ycd.update_options()?;
///     RwLock::new(ycd)
/// });
///
// Example should be updated once we can use search without writing
/// fn main() {
///     let mut ycd = YCD.write().unwrap();
///     let res = ycd.search("espanol es bueno");
/// }
/// ```
pub struct Yomichan<'a> {
    db: Arc<DictionaryDatabase<'a>>,
    backend: Backend<'a>,
}

impl<'a> Yomichan<'a> {
    /// Opens or creates a Yomichan database at the specified path.
    ///
    /// - **If `path` ends in `.ycd`**: Opens a ycd database.
    ///     - The file will be created if it doesn't exist, but its parent directory must exist.
    /// - **If `path` is a directory**: See [resolve_db_path] for detailed behavior.
    ///
    /// # Examples
    /// ```no_run
    /// use yomichan_rs::Yomichan;
    ///
    /// // Example 1: Explicitly name the database file.
    /// // Creates or opens `/path/to/my_database.ycd`.
    /// let ycd1 = Yomichan::new("/path/to/my_database.ycd").unwrap();
    ///
    /// // Example 2: Point to an empty directory.
    /// // Creates `/path/to/empty_dir/db.ycd`.
    /// let ycd2 = Yomichan::new("/path/to/empty_dir").unwrap();
    ///
    /// // Example 3: Point to a non-empty directory (e.g., your Desktop).
    /// // Creates a subdirectory: `/path/to/desktop/yomichan_rs/db.ycd`.
    /// let ycd3 = Yomichan::new("/path/to/desktop").unwrap();
    ///
    /// // Example 4: Open an existing database created by a previous run.
    /// // This will successfully open the database created in Example 3.
    /// let ycd4 = Yomichan::new("/path/to/desktop").unwrap();
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YomichanError> {
        let path = path.as_ref().to_path_buf();
        // Use the new, consolidated path resolution logic.
        let db_path = resolve_db_path(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));

        #[cfg(not(feature = "anki"))]
        let backend = Backend::new(db.clone()).map_err(|err| DBError::Database(err))?;
        #[cfg(feature = "anki")]
        let backend = Backend::default_sync(db.clone())?;

        Ok(Self { db, backend })
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
            if !parent.is_dir() {
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
        "\ninvalid path: {p} | help: 
  1. \"~/.home/db.ycd\" - opens a ycd instance
  2. \"~/.home/test\"   - creates a new (blank) .ycd file"
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
