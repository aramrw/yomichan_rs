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
mod text_scanner;
mod translation;
mod translation_internal;
mod translator;

use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use database::dictionary_database::DB_MODELS;
use derive_more::derive::DerefMut;
use icu::time::scaffold::IntoOption;
use indexmap::IndexMap;
use parking_lot::ArcRwLockReadGuard;
use parking_lot::ArcRwLockWriteGuard;
use parking_lot::RawRwLock;
use parking_lot::RwLock;
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
    // this new function should match redb exactly below later
    /// Opens the specified file as a yomichan database.
    /// * if the file does not exist, or is an empty file, a new database will be initialized in it
    /// * if the file is a valid yomichan database, it will be opened
    /// * otherwise this function will return an error
    ///
    /// # Examples
    /// ```
    /// use yomichan_rs::Yomichan;
    ///
    /// // creates a database at `~/desktop/yomichan_rs/data.db`
    /// // desktop is not an empty folder, so it creates a sub folder
    /// let mut ycd = Yomichan::new("c:/users/one/desktop");
    /// ```
    /// ```
    /// // `~/dev/empty_dir/data.db`
    /// // "empty_dir" is an empty folder, so it doesn't create a sub folder
    /// let mut ycd = Yomichan::new("c:/users/one/dev/empty_dir");
    /// ```
    /// ```
    /// // `~/desktop/yomichan_rs/data.db` was created above, so it makes a connection
    /// let mut ycd = Yomichan::new("c:/users/one/desktop/yomichan/data.db");
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Box<InitError>> {
        let path = path.as_ref().to_path_buf();
        let db_path = fmt_dbpath(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));
        #[cfg(not(feature = "anki"))]
        let backend = Backend::new(db.clone())?;
        #[cfg(feature = "anki")]
        let backend = Backend::default_sync(db.clone())?;

        Ok(Self { db, backend })
    }
}

/// # Returns
/// Terms for valid PathBuf ending in `.ycd`:
/// - dir is empty (assumes you wants db here)
/// - contains yomichan_rs folder (appends `.ycd` to the path)
/// - already contains a .ycd file
fn find_ydict_file(p: &Path) -> Option<PathBuf> {
    let mut valid_path: Option<PathBuf> = None;
    let rdir: HashSet<PathBuf> = std::fs::read_dir(p)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    // if empty db.ycd will b created directly
    if rdir.is_empty() {
        return Some(p.join("db.ycd"));
    }
    if let Some(p) = rdir.get(Path::new("yomichan_rs")) {
        return Some(p.join("db.ycd"));
    }
    rdir.into_iter()
        .find(|p| p.display().to_string().ends_with(".ycd"))
}

/// # Returns
/// A valid PathBuf ending in `.ycd`
/// ...can be opened or created with [`native_db::Builder::open`]
fn fmt_dbpath(p: PathBuf) -> Result<PathBuf, Box<InitError>> {
    let fname = p.display().to_string();
    if p.is_file() && fname.ends_with(".ycd") {
        if p.exists() {
            return Ok(p);
        }
        if p.parent().map(|p| p.exists()).unwrap_or(false) {
            return Err(InitError::MissingParent { p }.into());
        }
        return Ok(p);
    };
    if p.is_dir() {
        if let Some(p) = find_ydict_file(&p) {
            return Ok(p);
        }
        // ok bcz find_ydict_file garuntees:
        // path exists && is dir &&
        // yomichan_rs cannot exist
        let p = p.join("yomichan_rs");
        std::fs::create_dir_all(&p).map_err(|e| Box::new(InitError::Io(e)))?;
        return Ok(p.join("db.ycd"));
    }
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
