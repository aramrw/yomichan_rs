use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitError {
    #[error("database connection failed")]
    DatabaseConnectionFailed(#[from] redb::DatabaseError),
}

