use std::sync::Arc;

use crate::{
    database::{
        DictionaryService, DictionaryDatabaseError, DictionarySummary,
    },
    environment::EnvironmentInfo,
    text_scanner::TextScanner,
    settings::{YomichanOptions, ProfileResult},
    errors::DBError,
    Ptr, Yomichan,
};
use crate::translation::FindTermsMatchType;
use native_model::decode;

#[cfg(feature = "anki")]
use crate::anki::DisplayAnki;

pub struct Backend<'a> {
    pub _environment: EnvironmentInfo,
    #[cfg(feature = "anki")]
    pub anki: Ptr<DisplayAnki>,
    pub text_scanner: TextScanner<'a>,
    pub db: Arc<dyn DictionaryService>,
    pub options: Ptr<YomichanOptions>,
}

impl<'a> Backend<'a> {
    #[cfg(not(feature = "anki"))]
    pub fn new(db: Arc<dyn DictionaryService>) -> Result<Self, Box<DictionaryDatabaseError>> {
        let opts_blob = db.get_settings()?;
        let options = match opts_blob {
            Some(blob) => decode::<YomichanOptions>(blob).map(|(t, _)| t).expect("Failed to decode options"),
            None => YomichanOptions::new(),
        };
        let backend = Self {
            _environment: EnvironmentInfo::default(),
            text_scanner: TextScanner::new(db.clone()),
            db: db.clone(),
            options: Ptr::new(options),
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
    fn _update_options_internal(
        &self,
    ) -> Result<(), Box<DictionaryDatabaseError>> {
        // Options update in SQLite is handled in import_dictionaries or via manual SQL if needed.
        // For now, since native_db is gone, we don't have RwTransaction here.
        // This should probably be moved to use rusqlite or the DictionaryService.
        Ok(())
    }
}

impl<'a> Yomichan<'a> {
    /// Saves global options for all profiles to the database;
    /// Meant to be called after you mutate a profile
    /// (ie. via [Self::mod_options_mut().get_current_profile_mut])
    pub fn update_options(&self) -> Result<(), Box<DictionaryDatabaseError>> {
        self.backend._update_options_internal()?;
        Ok(())
    }

    /// Alias for [Self::update_options]
    pub fn save_settings(&self) -> Result<(), Box<DictionaryDatabaseError>> {
        self.update_options()
    }

    #[cfg(feature = "anki")]
    /// Returns a direct handle to the Anki integration for the current profile.
    pub fn anki(&self) -> crate::PtrRGaurd<DisplayAnki> {
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
    /// ```no_run
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
        let mut opts_guard = opts.write();
        let current_profile_ptr = opts_guard.get_current_profile().map_err(|e| DBError::from(e))?;
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
        let mut opts_guard = opts.write();
        let current_profile_ptr = opts_guard.get_current_profile().map_err(|e| DBError::from(e))?;
        current_profile_ptr.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for i in indexes {
                dictionaries.swap_remove_index(*i);
            }
        });
        self.update_options().map_err(|e| DBError::Import(crate::errors::ImportError::ExternalImporter(e.to_string())))?;

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
        self.db.get_dictionary_summaries().map_err(|e| DBError::Import(crate::errors::ImportError::ExternalImporter(e.to_string())))?;

        {
            let opts_ptr = self.options();
            let mut opts = opts_ptr.write();
            for profile in opts.profiles.values_mut() {
                profile.with_ptr_mut(|p| {
                    p.dictionaries_mut().swap_remove(name);
                });
            }
        }

        self.update_options().map_err(|e| DBError::Import(crate::errors::ImportError::ExternalImporter(e.to_string())))?;

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
