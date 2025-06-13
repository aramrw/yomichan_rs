#![allow(unused)]
mod backend;
mod database;
mod dictionary;
mod dictionary_data;
mod environment;
mod errors;
mod freq;
mod regex_util;
mod settings;
mod structured_content;
mod test_utils;
mod translation;
mod translation_internal;
mod translator;

use backend::Backend;
use database::dictionary_database::DictionaryDatabase;
use database::dictionary_database::DB_MODELS;
use indexmap::IndexMap;
use settings::Options;
use settings::Profile;

use native_db::*;
use native_model::{native_model, Model};
use transaction::RTransaction;
use translation::FindTermsOptions;
use translator::FindTermsMode;
use translator::Translator;

use std::collections::HashSet;
use std::fs::DirEntry;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

// public exports:
pub use database::dictionary_importer::DictionarySummary;

/// A Yomichan Dictionary instance.
pub struct Yomichan {
    pub backend: Backend,
    pub options: Options,
}

impl Yomichan {
    // pub fn search_terms(
    //     &self,
    //     text_to_search: &str,
    //     language_code: &str,
    //     mode: FindTermsMode,
    // ) -> YourResultType {
    //     let mut opts = &FindTermsOptions::default_for_language(language_code);
    //
    //     // Language-specific override for remove_non_japanese_characters
    //     if language_code.eq_ignore_ascii_case("ja") || language_code.eq_ignore_ascii_case("jpn") {
    //         opts.remove_non_japanese_characters = true;
    //     } else {
    //         opts.remove_non_japanese_characters = false; // Default from default_for_language is false
    //     }
    //
    //     let mut enabled_map = IndexMap::new();
    //     let mut main_dict_name: Option<String> = None;
    //     let mut lowest_priority_val = usize::MAX;
    //     let current_profile = self.options.get_current_profile();
    //
    //     for dict_profile_opt in current_profile.options.dictionaries {
    //         if dict_profile_opt.enabled {
    //             let search_config = translation::FindTermDictionary
    //                 { index, alias, allow_secondary_searches,
    //                 parts_of_speech_filter, use_deinflections
    //             } {
    //                 use_deinflections: dict_profile_opt.use_deinflections,
    //                 parts_of_speech_filter: dict_profile_opt.parts_of_speech_filter,
    //                 allow_secondary_searches: dict_profile_opt.allow_secondary_searches,
    //             };
    //             enabled_map.insert(dict_profile_opt.name.clone(), search_config);
    //
    //             if dict_profile_opt.priority < lowest_priority_val {
    //                 lowest_priority_val = dict_profile_opt.priority;
    //                 main_dict_name = Some(dict_profile_opt.name.clone());
    //             }
    //         }
    //     }
    //
    //     opts.enabled_dictionary_map = enabled_map;
    //     if let Some(name) = main_dict_name {
    //         opts.main_dictionary = name;
    //     }
    //     self.translator.find_terms(mode, text_to_search, opts)
    // }
    //
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
    pub fn new(path: impl AsRef<Path>) -> Result<Self, InitError> {
        let path = path.as_ref().to_path_buf();
        let db_path = fmt_dbpath(path)?;
        let backend = Backend::new(db_path);
        let rtx = backend.translator.db.r_transaction()?;
        let opts: Option<Options> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => {
                println!("no options found in db");
                Options::new()
            }
        };

        Ok(Self { backend, options })
    }
}

/// # Returns
/// A valid PathBuf ending in `.ycd` if:
/// - current dir is empty (assumes user wants db here)
/// - contains yomichan_rs folder (joins path)
/// - contains a .ycd file
///
/// idea: look at the assembly generated for:
/// for Ok(item) in read_dir(p)?
/// vs
/// for item in read_dir(p).into_iter().flatten().collect()
/// vs
/// read_dir(p).into_iter().flatten().map(|e|)
fn find_ydict_file(p: &Path) -> Option<PathBuf> {
    let mut valid_path: Option<PathBuf> = None;
    let rdir: HashSet<PathBuf> = std::fs::read_dir(p)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    // if empty db.ycd will b created directly
    if rdir.is_empty() {
        return Some(p.join("db.ycd"));
    }
    if let Some(p) = rdir.get(Path::new("yomichan_rs")) {
        return Some(p.join("db.ycd"));
    }
    rdir.into_iter()
        .find(|p| p.display().to_string().ends_with(".ycd"))
}

/// # Returns
/// A valid PathBuf ending in `.ycd`
/// ...can be opened or created with [`native_db::Builder::open`]
fn fmt_dbpath(p: PathBuf) -> Result<PathBuf, InitError> {
    let fname = p.display().to_string();
    if p.is_file() && fname.ends_with(".ycd") {
        if p.exists() {
            return Ok(p);
        }
        if p.parent().map(|p| p.exists()).unwrap_or(false) {
            return Err(InitError::MissingParent { p });
        }
        return Ok(p);
    };
    if p.is_dir() {
        if let Some(p) = find_ydict_file(&p) {
            return Ok(p);
        }
        // ok bcz find_ydict_file garuntees:
        // path exists && is dir &&
        // yomichan_rs cannot exist
        let p = p.join("yomichan_rs");
        std::fs::create_dir_all(&p)?;
        return Ok(p.join("db.ycd"));
    }
    Err(InitError::InvalidPath { p })
}

#[derive(thiserror::Error)]
#[error("could not create yomichan_rs dictionary database:")]
pub enum InitError {
    #[error(
        "\ninvalid path: {p} .. help: 
  1. \"~/.home/db.ycd\" - opens a ycd instance
  2. \"~/.home/test\"   - creates a new (blank) .ycd file"
    )]
    InvalidPath { p: PathBuf },
    #[error("path does not have a parent: {p}")]
    MissingParent { p: PathBuf },
    #[error("db conn err: {0}")]
    DatabaseConnectionFailed(#[from] Box<db_type::Error>),
    #[error("io err: {0}")]
    Io(#[from] std::io::Error),
}

impl From<native_db::db_type::Error> for InitError {
    fn from(e: native_db::db_type::Error) -> Self {
        InitError::DatabaseConnectionFailed(Box::new(e))
    }
}

impl std::fmt::Debug for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
