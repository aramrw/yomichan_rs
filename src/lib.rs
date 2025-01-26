#![allow(unused)]
mod database;
mod dictionary;
mod dictionary_data;
mod errors;
mod freq;
mod language;
mod settings;
mod structured_content;

use database::dictionary_database::DB_MODELS;
use errors::InitError;
use settings::Options;
use settings::Profile;

use native_db::*;
use native_model::{native_model, Model};
use transaction::RTransaction;

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

/// A Yomichan Dictionary instance.
pub struct Yomichan {
    db: Database<'static>,
    db_path: OsString,
    options: Options,
}

impl Yomichan {
    /// Initializes _(or if one already exists, opens)_ a Yomichan Dictionary Database.
    ///
    /// # Arguments
    /// * `db_path` - The location where the `yomichan/data.db` will be created/opened.
    ///
    /// # Examples
    /// ```
    /// use yomichan_rs::Yomichan;
    ///
    /// // creates a database at `C:/Users/1/Desktop/yomichan/data.db`
    /// let mut ycd = Yomichan::new("c:/users/one/desktop");
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
            db,
            db_path,
            options,
        })
    }
}

fn check_db_exists<P: AsRef<Path>>(path: P) -> Result<Option<OsString>, InitError> {
    let path = path.as_ref();

    // check if the path has a .yc extension
    fn check_ext(path: &Path) -> Option<OsString> {
        if let Some(ext) = path.extension() {
            if ext == "yc" {
                return Some(path.into());
            }
        }
        None
    }

    if let Some(db_path) = check_ext(path) {
        return Ok(Some(db_path));
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                if let Some(db_path) = check_ext(&entry_path) {
                    return Ok(Some(db_path));
                }
            }

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
