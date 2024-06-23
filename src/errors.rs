use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitError {
    #[error("database connection failed")]
    DatabaseConnectionFailed(#[from] redb::DatabaseError),
}

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("database error")]
    Database(#[from] redb::DatabaseError),
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("zip error")]
    Zip(#[from] zip::result::ZipError),
    #[error("json error: {0}")]
    JSON(#[from] serde_json::error::Error),
    #[error("json error: {0}")]
    OtherJSON(String),
    #[error("error at line {0}: {1}")]
    LineError(u32, Box<ImportError>),
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

impl From<(u32, redb::DatabaseError)> for ImportError {
    fn from(err: (u32, redb::DatabaseError)) -> ImportError {
        ImportError::LineError(err.0, Box::new(ImportError::from(err.1)))
    }
}

impl From<(u32, std::io::Error)> for ImportError {
    fn from(err: (u32, std::io::Error)) -> ImportError {
        ImportError::LineError(err.0, Box::new(ImportError::from(err.1)))
    }
}

impl From<(u32, zip::result::ZipError)> for ImportError {
    fn from(err: (u32, zip::result::ZipError)) -> ImportError {
        ImportError::LineError(err.0, Box::new(ImportError::from(err.1)))
    }
}

