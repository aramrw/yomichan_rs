#[cfg(feature = "anki")]
use crate::anki::core::DisplayAnkiError;
use crate::settings::core::ProfileError;
use yomichan_importer::dictionary_importer::DictionarySummaryError;
use native_db::db_type;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
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
    use super::InitError;
    #[cfg(feature = "anki")]
    mod anki {
        use super::super::InitError;
        use crate::anki::core::DisplayAnkiError;
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

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("failed to search")]
    Failed,
}

#[derive(Error, Debug)]
pub enum YomichanError {
    #[error("[<yc_error::import>]:\n  {0}")]
    Import(#[from] ImportError),
    #[error("[<yc_error::db>]:\n  {0}")]
    Database(#[from] DBError),
    #[error("(-)[yc_error::<profile>]:\n  {0}")]
    Profile(#[from] ProfileError),
    #[cfg(feature = "anki")]
    #[error("[yc_error::<anki>]:\n  {0}")]
    Anki(#[from] DisplayAnkiError),
    #[error("[yc_error::<init>]:\n  {0}")]
    Init(#[from] InitError),
    #[error("[yc_error::<search>]:\n  {0}")]
    Search(#[from] SearchError),
}
impl From<Box<InitError>> for YomichanError {
    fn from(value: Box<InitError>) -> Self {
        YomichanError::Init(*value)
    }
}

#[derive(Error, Debug)]
pub enum ImportZipError {
    #[error("the zip path: `{0}` does not exist")]
    DoesNotExist(PathBuf),
    #[error("zip crate error: {0}")]
    ZipCrate(#[from] zip::result::ZipError),
    #[error("filesystemIO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ImportZipError {
    pub fn check_zip_paths(paths: &[impl AsRef<Path>]) -> Result<(), Self> {
        for zp in paths {
            let zp = zp.as_ref();
            if !zp.exists() {
                return Err(Self::DoesNotExist(zp.to_path_buf()));
            }
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum DictionaryFileError {
    #[error("failed to deserialize file: `{outpath}`\nreason: {reason}")]
    File { outpath: PathBuf, reason: String },
    #[error(
        "no data in term_bank stream, is the file empty?
         file: {0}"
    )]
    Empty(PathBuf),
    #[error("failed to open file: {outpath}\nreason: {reason}")]
    FailedOpen { outpath: PathBuf, reason: String },
}

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("cannot import {0} as it is already installed\n[help]: if you are attempting to update it, first call `Yomichan::delete_dictionaries(&self, names: &[&{0}])`, and try importing again")]
    DictionaryAlreadyExists(String),
    #[error("dictionary file error: {0}")]
    DictionaryFile(#[from] DictionaryFileError),
    #[error("{0}")]
    Zip(#[from] ImportZipError),
    #[error("db err: {0}")]
    Database(#[from] Box<db_type::Error>),
    #[error("io err: {0}")]
    IO(#[from] std::io::Error),
    #[error("json err: {0}")]
    Json(#[from] serde_json::error::Error),
    #[error("thread err: {0}")]
    ThreadErr(#[from] std::thread::AccessError),
    #[error("error at line {0}: {1}")]
    LineErr(u32, Box<ImportError>),
    #[error("json err: {0}")]
    Custom(String),
    #[error(
        "failed to deserialize file: {file}
         reason: {e:#?}"
    )]
    InvalidJson { file: PathBuf, e: Option<String> },
    #[error("failed to create summary: {0}")]
    Summary(#[from] DictionarySummaryError),
    #[error("profile error: {0}")]
    Profile(#[from] ProfileError),
    #[error("external importer error: {0}")]
    ExternalImporter(String),
}

impl From<yomichan_importer::errors::ImportError> for ImportError {
    fn from(err: yomichan_importer::errors::ImportError) -> Self {
        ImportError::ExternalImporter(err.to_string())
    }
}

impl From<native_db::db_type::Error> for ImportError {
    fn from(err: native_db::db_type::Error) -> Self {
        ImportError::Database(Box::new(err))
    }
}

impl From<rusqlite::Error> for ImportError {
    fn from(err: rusqlite::Error) -> Self {
        ImportError::ExternalImporter(err.to_string())
    }
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error("db err: {0}")]
    Database(#[from] Box<db_type::Error>),
    #[error("rusqlite err: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("query err: {0}")]
    Query(String),
    #[error("none found err: {0}")]
    NoneFound(String),
    #[error("import err: {0}")]
    Import(#[from] ImportError),
    #[error("profile err: {0}")]
    Profile(#[from] ProfileError),
    #[error("dictionary database err: {0}")]
    DictionaryDatabase(Box<crate::database::dictionary_database::DictionaryDatabaseError>),
}

impl From<Box<crate::database::dictionary_database::DictionaryDatabaseError>> for DBError {
    fn from(e: Box<crate::database::dictionary_database::DictionaryDatabaseError>) -> Self {
        DBError::DictionaryDatabase(e)
    }
}

impl From<native_db::db_type::Error> for DBError {
    fn from(err: native_db::db_type::Error) -> Self {
        DBError::Database(Box::new(err))
    }
}

impl From<(u32, std::io::Error)> for ImportError {
    fn from(err: (u32, std::io::Error)) -> ImportError {
        ImportError::LineErr(err.0, Box::new(ImportError::from(err.1)))
    }
}

impl From<(u32, serde_json::error::Error)> for ImportError {
    fn from(err: (u32, serde_json::error::Error)) -> ImportError {
        ImportError::LineErr(err.0, Box::new(ImportError::from(err.1)))
    }
}

pub mod error_helpers {
    pub fn fmterr_module(modules: Vec<&str>) -> String {
        modules.join("::")
    }
}

#[macro_export]
macro_rules! fmt_mod_error {
    ($($mod:expr),*) => {
        $crate::utils::errors::error_helpers::fmterr_module(vec![$($mod),*])
    };
}
