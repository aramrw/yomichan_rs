use std::sync::Arc;

use native_db::transaction::RwTransaction;

#[cfg(feature = "anki")]
use crate::anki::DisplayAnki;
use crate::{
    database::{
        dictionary_database::{DictionaryDatabase, DictionaryDatabaseError},
        dictionary_importer::DictionarySummary,
    },
    environment::EnvironmentInfo,
    errors::DBError,
    settings::{ProfileResult, YomichanOptions},
    text_scanner::TextScanner,
    translation::FindTermsMatchType,
    Ptr, PtrRGaurd, Yomichan,
};

/// `yomichan_rs` private engine
pub struct Backend<'a> {
    pub _environment: EnvironmentInfo,
    #[cfg(feature = "anki")]
    pub anki: Ptr<DisplayAnki>,
    pub text_scanner: TextScanner<'a>,
    pub db: Arc<DictionaryDatabase<'a>>,
    pub options: Ptr<YomichanOptions>,
}

impl<'a> Backend<'a> {
    #[cfg(not(feature = "anki"))]
    pub fn new(db: Arc<DictionaryDatabase<'a>>) -> Result<Self, Box<native_db::db_type::Error>> {
        let rtx = db.r_transaction()?;
        let opts: Option<YomichanOptions> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => YomichanOptions::new(),
        };
        let backend = Self {
            _environment: EnvironmentInfo::default(),
            text_scanner: TextScanner::new(&db),
            db,
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
        rwtx: Option<RwTransaction>,
    ) -> Result<Option<YomichanOptions>, Box<native_db::db_type::Error>> {
        let rwtx = rwtx.unwrap_or(self.db.rw_transaction()?);
        let clone = self.options.write().clone();
        let stale: Option<YomichanOptions> = rwtx.upsert(clone)?;
        rwtx.commit()?;
        Ok(stale)
    }
}

impl<'a> Yomichan<'a> {
    /// Saves global options for all profiles to the database;
    /// Meant to be called after you mutate a profile
    /// (ie. via [Self::mod_options_mut().get_current_profile_mut])
    pub fn update_options(&self) -> Result<(), Box<native_db::db_type::Error>> {
        self.backend._update_options_internal(None)?;
        Ok(())
    }

    /// Alias for [Self::update_options]
    pub fn save_settings(&self) -> Result<(), Box<native_db::db_type::Error>> {
        self.update_options()
    }

    #[cfg(feature = "anki")]
    /// Returns a direct handle to the Anki integration for the current profile.
    pub fn anki(&self) -> PtrRGaurd<DisplayAnki> {
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
        let current_profile = self.backend.get_current_profile()?;
        current_profile.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for name in names {
                let name = name.as_ref();
                dictionaries.swap_remove(name);
            }
        });
        //self.update_options()?;
        Ok(())
    }

    pub fn delete_dictionaries_by_indexes(&self, indexes: &[usize]) -> Result<(), DBError> {
        let current_profile = self.backend.get_current_profile()?;
        current_profile.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for i in indexes {
                dictionaries.swap_remove_index(*i);
            }
        });
        self.update_options()?;

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
        self.db.remove_dictionary_by_name(name)?;

        {
            let opts_ptr = self.options();
            let mut opts = opts_ptr.write();
            for profile in opts.profiles.values_mut() {
                profile.with_ptr_mut(|p| {
                    p.dictionaries_mut().swap_remove(name);
                });
            }
        }

        self.update_options()?;

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
