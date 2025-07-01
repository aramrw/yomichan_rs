use anki_direct::{
    cache::model::ModelCache, error::AnkiResult, model::FullModelDetails, notes::Note, AnkiClient,
};
use getset::Getters;
use indexmap::IndexMap;
use parking_lot::{
    ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{collections::HashMap, ops::Deref, sync::Arc};

#[cfg(feature = "anki")]
use crate::database::dictionary_database::DictionaryDatabase;
use crate::{
    errors::{error_helpers, DBError},
    settings::{
        AnkiDuplicateScope, AnkiNoteOptions, AnkiOptions, DecksMap, NoteModelsMap, ProfileError,
        ProfileOptions, ProfileResult, YomichanOptions, YomichanProfile,
    },
    Ptr, Yomichan,
};

#[derive(thiserror::Error, Debug)]
pub enum DisplayAnkiError {
    #[error("[{}]", 
        error_helpers::fmterr_module(vec!["error", "display-anki", "profile"]))
    ]
    Profile(#[from] ProfileError),
    #[error("[{}]", 
        error_helpers::fmterr_module(vec!["error", "display-anki", "database"]))
    ]
    Database(#[from] native_db::db_type::Error),
}
impl From<native_db::db_type::Error> for Box<DisplayAnkiError> {
    fn from(err: native_db::db_type::Error) -> Self {
        Box::new(DisplayAnkiError::Database(err))
    }
}
impl From<ProfileError> for Box<DisplayAnkiError> {
    fn from(err: ProfileError) -> Self {
        Box::new(DisplayAnkiError::Profile(err))
    }
}

#[derive(Clone, Getters)]
#[getset(get = "pub")]
pub struct DisplayAnki {
    client: AnkiClient,
    /// a ptr to the global options
    options: Ptr<YomichanOptions>,
}
impl Deref for DisplayAnki {
    type Target = AnkiClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// Setup Impl
impl DisplayAnki {
    pub async fn new_auto(port: &str, options: Ptr<YomichanOptions>) -> AnkiResult<Self> {
        let client = AnkiClient::new_auto(port).await?;
        let res = Self { client, options };
        Ok(res)
    }
    pub fn new_sync(port: &str, version: u8, options: Ptr<YomichanOptions>) -> Self {
        Self {
            client: AnkiClient::new_sync(port, version),
            options,
        }
    }
    pub fn default_latest(options: Ptr<YomichanOptions>) -> Self {
        Self {
            client: AnkiClient::default(),
            options,
        }
    }
}

#[cfg(feature = "anki")]
impl<'a> crate::Backend<'a> {
    pub fn default_sync(db: Arc<DictionaryDatabase<'a>>) -> Result<Self, Box<DisplayAnkiError>> {
        use crate::{environment::EnvironmentInfo, text_scanner::TextScanner};

        let rtx = db.r_transaction()?;
        let opts: Option<YomichanOptions> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => YomichanOptions::new(),
        };
        let options: Ptr<YomichanOptions> = options.into();
        let profile: Ptr<YomichanProfile> = options.read_arc().get_current_profile()?;
        let backend = Self {
            environment: EnvironmentInfo::default(),
            text_scanner: TextScanner::new(&db),
            anki: DisplayAnki::default_latest(options.clone()),
            db: db.clone(),
            options: options.clone(),
        };
        Ok(backend)
    }
}

impl DisplayAnki {
    /// Executes a closure with immutable access to the current profile's AnkiOptions.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes an `&AnkiOptions` and returns a value of type `R`.
    ///
    /// # Returns
    ///
    /// * `Some(R)` if a profile was found and the closure was executed.
    /// * `None` if the current profile could not be determined.
    pub fn with_anki_options<F, R>(&self, f: F) -> ProfileResult<R>
    where
        F: FnOnce(&AnkiOptions) -> R,
    {
        let opts = self.options.read();
        let pf = opts.get_current_profile()?;
        let profile_guard = pf.read();
        let result = f(&profile_guard.options.anki);
        Ok(result)
    }

    /// Updates [AnkiDirect::Cache] & [AnkiNoteOptions] to contain all found (note) models and
    /// decks
    pub async fn update_all_anki_maps(&mut self) -> AnkiResult<()> {
        // --- Phase 1: Fetch Data ---
        let (latest_models, latest_decks) = {
            let mut cache = self.client.cache_mut();
            // Fetch (note) models data
            let models_data = cache.models_mut().hydrate().await?.get_cache().clone();
            // Fetch deck data
            let decks_data = cache
                .decks_mut()
                .hydrate_names()
                .await?
                .clone()
                .take_cache();

            (models_data, decks_data)
        };
        let latest_models: NoteModelsMap = latest_models.into();
        let latest_decks: DecksMap = latest_decks.into();

        self.options().with_ptr_mut(|opts| {
            // Borrow, use, and implicitly drop all in one line.
            let mut global_anki_opts = opts.anki_mut();
            *global_anki_opts.note_models_map_mut() = latest_models;
            *global_anki_opts.deck_models_map_mut() = latest_decks;
        });
        Ok(())
    }

    // /// Updates [AnkiDirect::Cache] as well as [AnkiNoteOptions] to use specified (note) models.
    // async fn auto_add_note_models_by_name(
    //     &mut self,
    //     model_names: &[&str],
    // ) -> AnkiResult<ArcRwLockWriteGuard<RawRwLock, AnkiOptions>> {
    //     let mut cache = self.client.cache_mut();
    //     let mut model_cache = cache.models_mut();
    //     let models = model_cache.hydrate().await?;
    //     let found: IndexMap<String, FullModelDetails> =
    //         models.find_many_from_key_owned(model_names).collect();
    //     Ok(self.extend_note_models(found))
    // }
}

impl<'a> Yomichan<'a> {
    async fn test_find_notes_new(&self) -> AnkiResult<()> {
        let res = self
            .backend
            .anki
            .notes()
            .find_notes(anki_direct::anki::AnkiQuery::Custom("is:new".into()))
            .await?;
        assert!(res.is_empty());
        Ok(())
    }
}

#[cfg(test)]
mod displayanki {
    use crate::{
        anki::DisplayAnki,
        settings::{AnkiOptions, YomichanOptions},
        test_utils::TEST_PATHS,
        Ptr, Yomichan,
    };
    use anki_direct::model::{FullModelDetails, TemplateInfo};
    use indexmap::IndexMap;
    use parking_lot::{Mutex, RwLock};
    use pretty_assertions::assert_eq;
    use std::{
        collections::HashMap,
        sync::{Arc, LazyLock},
    };

    static DISPLAYANKI: LazyLock<Mutex<DisplayAnki>> = LazyLock::new(|| {
        let options = Ptr::new(YomichanOptions::default());
        Mutex::new(DisplayAnki::default_latest(options))
    });

    // #[test]
    // fn note_options_mut() {
    //     let mut da = DISPLAYANKI.lock();
    //     let mut options = da.options_mut();
    //     let mut note_models_map = options.note_models_map_mut();
    //     note_models_map.insert("JapaneseVocab".into(), FullModelDetails::mock());
    //     assert_eq!(note_models_map.len(), 1);
    //     let details = note_models_map.get("JapaneseVocab").unwrap();
    //     let templates = details
    //         .find_templates_by_names(&["Recognition"])
    //         .collect::<IndexMap<&str, &TemplateInfo>>();
    //     assert_eq!(templates.len(), 1, "'Recognition' template not found");
    //
    //     // This unwrap is now safe
    //     let found = *templates.get("Recognition").unwrap();
    //     assert_eq!(found.front, "<div class='sentence'>{{Sentence}}</div>");
    //     let found = *templates.get("Recognition").unwrap();
    //     assert!(found.front == "<div class='sentence'>{{Sentence}}</div>");
    // }
    //
    // #[test]
    // fn anki_options_mut() {
    //     let default_opts = AnkiOptions::default();
    //     let mut da = DISPLAYANKI.lock();
    //     let mut opts = da.options_mut();
    //     opts.set_suspend_new_cards(true);
    //     assert!(default_opts != *opts);
    // }
}
