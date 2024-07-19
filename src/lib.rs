#![allow(unused)]
mod dictionary;
mod dictionary_data;
mod dictionary_database;
mod dictionary_importer;
mod dictionary_worker_handler;
mod errors;
mod freq;
mod settings;
mod structured_content;
mod tests;

use crate::settings::Options;

use errors::InitError;
use native_db::*;
use native_model::{native_model, Model};

use dictionary_database::DB_MODELS;

use std::path::{Path, PathBuf};

/// A Yomichan Dictionary instance.
pub struct Yomichan {
    db_path: String,
    options: Options,
}

impl Yomichan {
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary Database.
    ///
    /// # Arguments
    ///
    /// * `db_path` - The location where the database will be created.
    ///
    /// ```
    /// use yomichan_rs::Yomichan;
    /// let mut ycd = Yomichan::new("C:\\Users\\1\\Desktop");
    /// ```
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, errors::InitError> {

        let new_path = format_db_path(db_path)?;
        let db = native_db::Builder::new().create(&DB_MODELS, &new_path)?;
        Ok(Self {
            db_path: new_path,
            options: Options::default(),
        })
    }
}

/// Formats the path to include `.yc` as the extension.
fn format_db_path<P: AsRef<Path>>(path: P) -> Result<String, InitError> {
    let path_ref = path.as_ref();
    if let Some(parent) = path_ref.parent() {
        return match parent.exists() {
            true => Ok(path_ref.with_extension("yc").to_string_lossy().to_string()),
            false => Err(InitError::Path(format!(
                "path does not exist: {}",
                path_ref.to_string_lossy()
            ))),
        };
    }
    Err(InitError::Path(format!(
        "path does not exist: {}",
        path_ref.to_string_lossy()
    )))
}
