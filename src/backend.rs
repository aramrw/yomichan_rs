//! # Backend Engine
//!
//! The backend handles the coordination between the database, the text scanner,
//! and user profiles. It provides the internal logic used by the [`Yomichan`](crate::Yomichan)
//! facade.
//!
//! Most users should interact with [`Yomichan`](crate::Yomichan) directly rather than
//! using the backend.

use std::sync::Arc;

use crate::translator::types::FindTermsMatchType;
use crate::{
    database::{DictionaryDatabaseError, DictionaryService, DictionarySummary},
    scanner::core::TextScanner,
    settings::core::{ProfileResult, YomichanOptions},
    settings::environment::EnvironmentInfo,
    utils::errors::DBError,
    Ptr, Yomichan,
};
//use native_model::decode;

#[cfg(feature = "anki")]
use crate::anki::core::{DisplayAnki, DisplayAnkiError};

pub struct Backend {
    pub _environment: EnvironmentInfo,
    #[cfg(feature = "anki")]
    pub anki: Ptr<DisplayAnki>,
    pub scanner: TextScanner,
    pub db: Arc<dyn DictionaryService>,
    pub options: Ptr<YomichanOptions>,
}

impl Backend {
    #[cfg(not(feature = "anki"))]
    pub fn new(db: Arc<dyn DictionaryService>) -> Result<Self, Box<DictionaryDatabaseError>> {
        let opts_blob = db.get_settings()?;
        let options = match opts_blob {
            Some(blob) => native_model::decode::<YomichanOptions>(blob)
                .map(|(t, _)| t)
                .expect("Failed to decode options"),
            None => YomichanOptions::new(),
        };
        let backend = Self {
            _environment: EnvironmentInfo::default(),
            scanner: TextScanner::new(db.clone()),
            db: db.clone(),
            options: Ptr::new(options),
        };
        Ok(backend)
    }

    #[cfg(feature = "anki")]
    pub fn default_sync(db: Arc<dyn DictionaryService>) -> Result<Self, DisplayAnkiError> {
        // TODO: r_transaction was part of native_db.
        // Need to implement settings retrieval via DictionaryService (sqlite).
        let opts_blob = db
            .get_settings()
            .map_err(|e| DisplayAnkiError::Custom(e.to_string()))?;
        let options: YomichanOptions = match opts_blob {
            Some(blob) => native_model::decode::<YomichanOptions>(blob)
                .map(|(t, _)| t)
                .expect("Failed to decode options"),
            None => YomichanOptions::new(),
        };
        let options: Ptr<YomichanOptions> = options.into();
        let anki = Ptr::new(DisplayAnki::default_latest(options.clone()));
        let backend = Self {
            _environment: EnvironmentInfo::default(),
            scanner: TextScanner::new(db.clone()),
            anki,
            db: db.clone(),
            options: options.clone(),
        };
        Ok(backend)
    }

    /// The internal impl to write global options to the database.
    /// Takes an [Option<RwTransaction>] so rwtx's can be reused if necessary.
    ///
    /// # Returns
    ///
    /// * Option<YomichanOptions>: The previous [YomichanOptions] found in the db.
    ///   [None] should only ever happen after [Yomichan] creates a brand new `ycd.db`.
    fn _update_settings_internal(&self) -> Result<(), Box<DictionaryDatabaseError>> {
        let opts = self.options.read();
        let blob = native_model::encode(&*opts)
            .map_err(|e| DictionaryDatabaseError::Database(Box::new(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))))?;
        self.db.set_settings(&blob)?;
        Ok(())
    }
}

impl Yomichan {
    /// Saves global options for all profiles to the database;
    /// Meant to be called after you mutate a profile
    /// (ie. via [Self::mod_options_mut().get_current_profile_mut])
    pub fn update_options(&self) -> Result<(), Box<DictionaryDatabaseError>> {
        self.backend._update_settings_internal()?;
        Ok(())
    }

    /// Alias for [Self::update_options]
    pub fn save_settings(&self) -> Result<(), Box<DictionaryDatabaseError>> {
        self.update_options()
    }

    #[cfg(feature = "anki")]
    /// Returns a direct handle to the Anki integration for the current profile.
    pub fn anki(&self) -> crate::utils::PtrRGaurd<DisplayAnki> {
        self.backend.anki.read_arc()
    }

    /// Sets the current profile's main language.
    ///
    /// Only updates the language in memory;
    /// To save the set language to the db, call [Self::update_options] after.
    ///
    /// # Example
    ///
    /// Returns [ProfileError::SelectedOutOfBounds] if the language len is out of bounds.
    ///
    /// ```ignore
    /// fn persist_language() -> Option<()> {
    ///    set_language("es").ok()
    /// }
    ///
    /// ```
    pub fn set_language(&self, language_iso: &str) -> ProfileResult<()> {
        self.with_profile_mut(|profile| {
            profile.set_language(language_iso);
        })
    }

    /// Deletes dictionaries from the database and options by name, in memory only.
    pub fn delete_dictionaries_by_names_in_memory(
        &self,
        names: &[impl AsRef<str>],
    ) -> Result<(), DBError> {
        let opts = self.options();
        let opts_guard = opts.read();
        let current_profile_ptr = opts_guard
            .get_current_profile()
            .map_err(|e| DBError::from(e))?;
        current_profile_ptr.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for name in names {
                let name = name.as_ref();
                dictionaries.swap_remove(name);
            }
        });
        Ok(())
    }

    pub fn delete_dictionaries_by_indexes(&self, indexes: &[usize]) -> Result<(), DBError> {
        let opts = self.options();
        let opts_guard = opts.read();
        let current_profile_ptr = opts_guard
            .get_current_profile()
            .map_err(|e| DBError::from(e))?;
        current_profile_ptr.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for i in indexes {
                dictionaries.swap_remove_index(*i);
            }
        });
        self.update_options().map_err(|e| {
            DBError::Import(crate::utils::errors::ImportError::ExternalImporter(
                e.to_string(),
            ))
        })?;

        Ok(())
    }

    pub fn dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>> {
        self.db.get_dictionary_summaries()
    }

    // 1. Remove from databasel + all profiles in memory
    // 2. Persist updated options to database via
    pub fn remove_dictionary(&self, name: &str) -> Result<(), DBError> {
        self.db.get_dictionary_summaries().map_err(|e| {
            DBError::Import(crate::utils::errors::ImportError::ExternalImporter(
                e.to_string(),
            ))
        })?;

        {
            let opts_ptr = self.options();
            let mut opts = opts_ptr.write();
            for profile in opts.profiles.values_mut() {
                profile.with_ptr_mut(|p| {
                    p.dictionaries_mut().swap_remove(name);
                });
            }
        }

        self.update_options().map_err(|e| {
            DBError::Import(crate::utils::errors::ImportError::ExternalImporter(
                e.to_string(),
            ))
        })?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindTermsDetails {
    pub match_type: Option<FindTermsMatchType>,
    pub deinflect: Option<bool>,
    pub primary_reading: Option<String>,
}
impl Default for FindTermsDetails {
    fn default() -> Self {
        Self {
            match_type: Some(FindTermsMatchType::Exact),
            deinflect: Some(true),
            primary_reading: None,
        }
    }
}
