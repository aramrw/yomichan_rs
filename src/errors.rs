use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitError {
    #[error("database connection failed")]
    DatabaseConnectionFailed(#[from] redb::DatabaseError),
}

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("database err")]
    Database(#[from] redb::DatabaseError),
    #[error("io err")]
    IO(#[from] std::io::Error),
    #[error("zip err")]
    Zip(#[from] zip::result::ZipError),
    #[error("json err: {0}")]
    Json(#[from] serde_json::error::Error),
    #[error("json err: {0}")]
    Custom(String),
    #[error("thread err: {0}")]
    ThreadErr(#[from] std::thread::AccessError),
    #[error("error at line {0}: {1}")]
    LineErr(u32, Box<ImportError>),
}

#[derive(Error, Debug)]
pub enum DBError {
    #[error("storage err: {0}")]
    Storage(#[from] redb::StorageError),
    #[error("tx err: {0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("table err: {0}")]
    Table(#[from] redb::TableError),
    #[error("commit err: {0}")]
    Commit(#[from] redb::CommitError),
    #[error("binary err: {0}")]
    Binary(#[from] bincode::Error),
    #[error("import err: {0}")]
    Import(#[from] ImportError),
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
        ImportError::LineErr(err.0, Box::new(ImportError::from(err.1)))
    }
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
