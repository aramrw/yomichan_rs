use native_db::db_type;
use snafu::Snafu;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("db err: {0}")]
    Database(#[from] Box<db_type::Error>),
    #[error("io err: {0}")]
    IO(#[from] std::io::Error),
    #[error("zip err: {0}")]
    Zip(#[from] zip::result::ZipError),
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
    #[error(
        "no data in term_bank stream, is the file empty?
         file: {file}"
    )]
    Empty { file: PathBuf },
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
        DBError::Database(Box::new(err)) // <-- Creates this variant
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

impl From<(u32, zip::result::ZipError)> for ImportError {
    fn from(err: (u32, zip::result::ZipError)) -> ImportError {
        ImportError::LineErr(err.0, Box::new(ImportError::from(err.1)))
    }
}

impl From<(u32, serde_json::error::Error)> for ImportError {
    fn from(err: (u32, serde_json::error::Error)) -> ImportError {
        ImportError::LineErr(err.0, Box::new(ImportError::from(err.1)))
    }
}
