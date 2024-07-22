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

fn check_db_exists<P: AsRef<Path>>(path: P) -> Result<Option<OsString>, InitError> {
    let path = path.as_ref();

    // Function to check if the path has a .yc extension
    fn check_ext(path: &Path) -> Option<OsString> {
        if let Some(ext) = path.extension() {
            if ext == "yc" {
                return Some(path.into());
            }
        }
        None
    }

    // Check if the direct path is a .yc file
    if let Some(db_path) = check_ext(path) {
        return Ok(Some(db_path));
    }

    // Iterate over the directory entries
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            // Check files in the directory
            if entry_path.is_file() {
                if let Some(db_path) = check_ext(&entry_path) {
                    return Ok(Some(db_path));
                }
            }

            // Recursively check the `yomichan` subdirectory
            if entry_path.is_dir() {
                if let Some(dir_name) = entry_path.file_name() {
                    if dir_name == "yomichan" {
                        if let Some(db_path) = check_db_exists(&entry_path)? {
                            return Ok(Some(db_path));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Formats the path to include `.yc` as the extension.
fn format_db_path<P: AsRef<Path>>(path: P) -> Result<String, InitError> {
fn init_db_path<P: AsRef<Path>>(path: P) -> Result<OsString, InitError> {
    let path_ref = path.as_ref();

    // Check if the parent directory exists
    if let Some(parent_path) = path_ref.parent() {
        if !parent_path.exists() {
            return Err(InitError::Path(format!(
                "parent dir does not exist for path: {}",
                path_ref.to_string_lossy()
            )));
        }
    } else {
        return Err(InitError::Path(format!(
            "invalid parent dir for path: {}",
            path_ref.to_string_lossy()
        )));
    }

    // Create the `yomichan` subdirectory
    let yomichan_dir = path_ref.join("yomichan");
    if !yomichan_dir.exists() {
        fs::create_dir_all(&yomichan_dir).map_err(|e| {
            InitError::Path(format!(
                "failed to create directory at: {} | err: {}",
                path_ref.to_string_lossy(),
                e
            ))
        })?;
    }
    Err(InitError::Path(format!(
        "path does not exist: {}",
        path_ref.to_string_lossy()
    )))
}
