#![allow(unused)]
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
use indexmap::IndexMap;
use settings::Options;
use settings::Profile;

use native_db::*;
use native_model::{native_model, Model};
use text_scanner::TermSearchResults;
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

// public exports:
pub use crate::database::dictionary_importer;
pub use crate::dictionary::{TermDefinition, TermFrequency, TermPronunciation};

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
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary Database.
    ///
    /// # Arguments
    /// * `db_path` - The location where the `yomichan/data.db` will be created/opened.
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
    pub fn new(path: impl AsRef<Path>) -> Result<Self, InitError> {
        let path = path.as_ref().to_path_buf();
        let db_path = fmt_dbpath(path)?;
        let db = Arc::new(DictionaryDatabase::new(db_path));
        let backend = Backend::new(db.clone())?;

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
fn fmt_dbpath(p: PathBuf) -> Result<PathBuf, InitError> {
    let fname = p.display().to_string();
    if p.is_file() && fname.ends_with(".ycd") {
        if p.exists() {
            return Ok(p);
        }
        if p.parent().map(|p| p.exists()).unwrap_or(false) {
            return Err(InitError::MissingParent { p });
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
        std::fs::create_dir_all(&p)?;
        return Ok(p.join("db.ycd"));
    }
    Err(InitError::InvalidPath { p })
}

#[derive(thiserror::Error)]
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
}

impl From<native_db::db_type::Error> for InitError {
    fn from(e: native_db::db_type::Error) -> Self {
        InitError::DatabaseConnectionFailed(Box::new(e))
    }
}

impl std::fmt::Debug for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
