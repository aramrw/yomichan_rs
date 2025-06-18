use native_db::db_type;
use snafu::Snafu;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::database::dictionary_importer::DictionarySummaryError;

#[derive(Error, Debug)]
pub enum ImportZipError {
    #[error("the zip path: `{0}` does not exist")]
    DoesNotExist(PathBuf),
    #[error("`zip` crate error: {0}")]
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
    //#[error("token err: {0}")]
    //Token(#[from] lindera::LinderaError),
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
