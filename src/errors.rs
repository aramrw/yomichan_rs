#[cfg(feature = "anki")]
use crate::anki::DisplayAnkiError;
use crate::{settings::ProfileError, InitError};
use importer::dictionary_importer::DictionarySummaryError;
use native_db::db_type;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Abstraction over results for
pub enum YomichanResult<T> {
    Result(T),
    Err(YomichanError),
}

/// All possible `yomichan_rs` [Error] paths
#[derive(Error, Debug)]
pub enum YomichanError {
    #[error(
        "(-)[<yc_error::import>] -> 
{0}"
    )]
    Import(#[from] ImportError),
    #[error("(-)[<yc_error::db>]")]
    Database(#[from] DBError),
    #[error("(-)[yc_error::<profile>]")]
    Profile(#[from] ProfileError),
    #[cfg(feature = "anki")]
    #[error("(-)[yc_error::<anki>]")]
    Anki(#[from] DisplayAnkiError),
    #[error("(-)[yc_error::<anki>]")]
    Init(#[from] InitError),
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
    #[error("{0:?} is an unknown file; you can only deserialize zip files or unzipped folders")]
    UnknownFileType(Option<String>),
    #[error("{0:?}'s files do not contain index.json (*required)")]
    IndexFileNotFound(PathBuf),
}

impl ImportZipError {
    // Prefer a concrete error type (adjust to your actual error enum)
    pub fn check_zip_paths(paths: &[impl AsRef<Path>]) -> Result<(), ImportZipError> {
        for zp in paths {
            let zp = zp.as_ref();
            if !zp.exists() {
                return Err(ImportZipError::DoesNotExist(zp.to_path_buf()));
            }

            // accept .zip files (do nothing)
            if zp.is_file() && matches!(zp.extension().and_then(|s| s.to_str()), Some("zip")) {
                continue;
            } else if zp.is_dir() {
                let mut has_index_file = false;
                for entry in std::fs::read_dir(zp)? {
                    let entry = entry?;
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let is_json = path.extension().and_then(|s| s.to_str()) == Some("json");
                    let is_index_stem = path.file_stem().and_then(|s| s.to_str()) == Some("index");

                    if is_json && is_index_stem {
                        has_index_file = true;
                        break;
                    }
                }

                if !has_index_file {
                    return Err(ImportZipError::IndexFileNotFound(zp.to_path_buf()));
                }
            } else {
                return Err(ImportZipError::UnknownFileType(
                    zp.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string()),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum DictionaryFileError {
    #[error("failed to deserialize file: `{outpath}`\nreason: {reason}")]
    File { outpath: PathBuf, reason: String },
    #[error("no data in term_bank stream, is the file empty?\n         file: {0}")]
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
    #[error("failed to deserialize file: {file}\n         reason: {e:#?}")]
    InvalidJson { file: PathBuf, e: Option<String> },
    #[error("failed to create summary: {0}")]
    Summary(#[from] DictionarySummaryError),
    #[error("profile error: {0}")]
    Profile(#[from] ProfileError),
    #[error("importer error: {0}")]
    Importer(#[from] importer::errors::ImportError),
}

impl From<native_db::db_type::Error> for ImportError {
    fn from(err: native_db::db_type::Error) -> Self {
        ImportError::Database(Box::new(err))
    }
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error("db err: {0}")]
    Database(#[from] Box<db_type::Error>),
    #[error("query err: {0}")]
    Query(String),
    #[error("none found err: {0}")]
    NoneFound(String),
    #[error("import err: {0}")]
    Import(#[from] ImportError),
    #[error("(-)[yc_error::profile]")]
    Profile(#[from] ProfileError),
}

impl From<native_db::db_type::Error> for DBError {
    fn from(err: native_db::db_type::Error) -> Self {
        DBError::Database(Box::new(err))
    }
}

#[macro_export]
macro_rules! try_with_line {
    () => {
        macro_rules! line_number {
            () => {
                line!()
            };
        }

        ($expr:expr) => {
            match $expr {
                Ok(val) => val,
                Err(err) => return Err(errors::ImportError::from((line_number!(), err))),
            }
        };
    };
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
    /// # Example
    ///
    /// ```
    /// #[error("[error::{}]", fmterr_module(vec!["main", "database"]))]
    /// // [error::main::database]
    /// ```
    pub fn fmterr_module(mods: Vec<&str>) -> String {
        mods.join("::")
    }

    /// A helper macro to create a standard module error message attribute.
    #[macro_export]
    macro_rules! fmt_mod_error {
    ( $($path_part:literal),* ) => {
        // This macro expands to the full #[error(...)] attribute
        #[error("[{}]", error_helpers::fmterr_module(&[ $($path_part),* ]))]
    };
}
}
