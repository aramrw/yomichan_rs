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
use settings::Profile;

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

/// A Yomichan Dictionary instance.
pub struct Yomichan {
    db_path: OsString,
    options: Options,
}

impl Yomichan {
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary Database.
    ///
    /// # Arguments
    ///
    /// * `db_path` - The location where the `yomichan/data.db` will be created/opened.
    ///
    /// ```
    /// use yomichan_rs::Yomichan;
    ///
    /// // create the database at `C:\\Users\\1\\Desktop\\yomichan\\data.db`
    /// let mut ycd = Yomichan::new("C:\\Users\\1\\Desktop");
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, errors::InitError> {
        let db_path = if let Some(existing) = check_db_exists(&path)? {
            existing
        } else {
            init_db_path(&path)?
        };
        let db = native_db::Builder::new().create(&DB_MODELS, &db_path)?;

        let mut options = Options::default();
        options.profiles.push(Profile::default());

        Ok(Self {
            db_path,
            options,
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

    // Create the path for the database file with the .yc extension
    let db_path = yomichan_dir.join("data").with_extension("yc");

    Ok(db_path.into_os_string())
}
