#![allow(unused)]
mod database;
mod dictionary;
mod dictionary_data;
mod errors;
mod freq;
mod settings;
mod structured_content;

use database::dictionary_database::DB_MODELS;
use settings::Options;
use settings::Profile;

use native_db::*;
use native_model::{native_model, Model};
use transaction::RTransaction;

use std::collections::HashSet;
use std::fs::DirEntry;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

mod yomichan_test_utils {
    use std::{path::PathBuf, sync::LazyLock};
    use tempfile::{tempdir_in, TempDir};

    pub(crate) struct TestPaths {
        pub tests_dir: PathBuf,
        pub tests_yomichan_db_path: PathBuf,
        pub test_dicts_dir: PathBuf,
    }

    pub(crate) static TEST_PATHS: LazyLock<TestPaths> = LazyLock::new(|| TestPaths {
        tests_dir: PathBuf::from("./tests"),
        tests_yomichan_db_path: PathBuf::from("./tests").join("yomichan_rs").join("db.ycd"),
        test_dicts_dir: PathBuf::from("tests").join("test_dicts"),
    });

    pub(crate) enum BacktraceKind {
        One,
        Full,
    }

    impl std::fmt::Display for BacktraceKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let x = match self {
                BacktraceKind::One => "1",
                BacktraceKind::Full => "full",
            };
            write!(f, "{x}")
        }
    }

    pub(crate) fn set_backtrace(kind: BacktraceKind) {
        std::env::set_var("RUST_BACKTRACE", kind.to_string());
    }

    /// Copies the test database to a temporary directory.
    /// Necessary because native_db cannot have two test threads open the same database at the same time.
    pub(crate) fn copy_test_db() -> (PathBuf, TempDir) {
        let dir = tempdir_in(&*TEST_PATHS.tests_dir).unwrap();
        let tydbp = &*TEST_PATHS.tests_yomichan_db_path;
        let f_path = dir.path().join("data.ycd");
        if !tydbp.exists() {
            panic!("tests/yomichan_db_path doesn't exist! : {tydbp:?}");
        }
        std::fs::copy(tydbp, &f_path).unwrap();
        if !f_path.exists() {
            panic!("path doesn't exist! : {f_path:?}");
        }
        (f_path, dir)
    }

    pub(crate) fn print_timer<T>(inst: std::time::Instant, print: T)
    where
        T: std::fmt::Debug,
    {
        let duration = inst.elapsed();
        #[allow(unused_assignments)]
        let mut time = String::new();
        {
            let dur_sec = duration.as_secs();
            let dur_mill = duration.as_millis();
            let dur_nan = duration.as_nanos();
            if dur_sec == 0 {
                if dur_mill == 0 {
                    time = format!("{dur_mill}ns");
                } else {
                    time = format!("{dur_nan}ms");
                }
            } else if dur_sec > 60 {
                let min = dur_sec / 60;
                let sec = dur_sec % 60;
                time = format!("{min}m{sec}s");
            } else {
                time = format!("{dur_sec}s");
            }
        }
        println!("{print:?} files");
        println!("in {time}");
    }
}

/// A Yomichan Dictionary instance.
pub struct Yomichan {
    db: Database<'static>,
    db_path: PathBuf,
    options: Options,
}

#[derive(thiserror::Error)]
#[error("could not create yomichan_rs dictionary database:")]
pub enum InitError {
    #[error(
        "\ninvalid path: {p} .. help: 
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

impl Yomichan {
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary Database.
    ///
    /// # Arguments
    /// * `db_path` - The location where the `yomichan/data.db` will be created/opened.
    ///
    /// # Examples
    /// ```
    /// use yomichan_rs::Yomichan;
    ///
    /// // creates a database at `C:/Users/1/Desktop/yomichan/data.db`
    /// let mut ycd = Yomichan::new("c:/users/one/desktop");
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, InitError> {
        let path = path.as_ref().to_path_buf();
        let db_path = fmt_dbpath(path)?;
        let db = native_db::Builder::new().create(&DB_MODELS, &db_path)?;

        let mut options = Options::default();
        options.profiles.push(Profile::default());

        Ok(Self {
            db,
            db_path,
            options,
        })
    }
}

/// # Returns
/// A valid PathBuf ending in `.ycd` if:
/// - current dir is empty (assumes user wants db here)
/// - contains yomichan_rs folder (joins path)
/// - contains a .ycd file
///
/// idea: look at the assembly generated for:
/// for Ok(item) in read_dir(p)?
/// vs
/// for item in read_dir(p).into_iter().flatten().collect()
/// vs
/// read_dir(p).into_iter().flatten().map(|e|)
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
