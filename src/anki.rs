use anki_direct::{
    cache::model::ModelCache, error::AnkiResult, model::FullModelDetails, notes::Note, AnkiClient,
};
use indexmap::IndexMap;
use parking_lot::{
    ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{collections::HashMap, ops::Deref, sync::Arc};

use crate::{
    settings::{AnkiDuplicateScope, AnkiNoteOptions, AnkiOptions, NoteModelsMap, ProfileOptions},
    Yomichan,
};

#[derive(Clone)]
pub struct DisplayAnki {
    client: AnkiClient,
    /// a ptr to current [Profile] [AnkiOptions]
    options: Arc<RwLock<AnkiOptions>>,
}
impl Deref for DisplayAnki {
    type Target = AnkiClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// Setup Impl
impl DisplayAnki {
    pub async fn new_auto(port: &str, options: Arc<RwLock<AnkiOptions>>) -> AnkiResult<Self> {
        let client = AnkiClient::new_auto(port).await?;
        let res = Self { client, options };
        Ok(res)
    }
    pub fn new_sync(port: &str, version: u8, options: Arc<RwLock<AnkiOptions>>) -> Self {
        Self {
            client: AnkiClient::new_sync(port, version),
            options,
        }
    }
    pub fn default_latest(options: Arc<RwLock<AnkiOptions>>) -> Self {
        Self {
            client: AnkiClient::default(),
            options,
        }
    }
}

impl DisplayAnki {
    pub fn options_mut(&self) -> ArcRwLockWriteGuard<RawRwLock, AnkiOptions> {
        self.options.write_arc()
    }
    pub fn options(&self) -> ArcRwLockReadGuard<RawRwLock, AnkiOptions> {
        self.options.read_arc()
    }

    /// Updates [AnkiDirect::Cache] & [AnkiNoteOptions] to contain all found (note) models and
    /// decks
    ///
    /// This function is useful for updating a display list of models and decks,
    /// and letting the user select which model to create a note for, via [AnkiOptions]`.not_models_map.selected`
    pub async fn update_all_anki_maps(
        &mut self,
    ) -> AnkiResult<ArcRwLockWriteGuard<RawRwLock, AnkiOptions>> {
        // --- Phase 1: Fetch Data ---
        // We create a new scope `{ ... }`. All variables declared inside, including the
        // mutable borrow `cache`, will be dropped at the end of the scope.
        // This is the key to releasing the borrow on `self.client`.
        let (latest_models, latest_decks) = {
            // `cache` mutably borrows `self.client`. This borrow is now confined to this block.
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
        }; // <-- The mutable borrow on `self.client` (via `cache`) ends here.

        // --- Phase 2: Update Options ---
        // Now that the first borrow is gone, we can safely create a new one for `self.options`.
        let mut anki_options = self.options_mut(); // This gets the write lock guard

        // Update the maps using the data we fetched in phase 1
        anki_options.note_models_map_mut().set_map(latest_models);
        anki_options.deck_models_map_mut().set_map(latest_decks);

        // Return the write guard as required by the function signature
        Ok(anki_options)
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
    use crate::{anki::DisplayAnki, settings::AnkiOptions, test_utils::TEST_PATHS, Yomichan};
    use anki_direct::model::{FullModelDetails, TemplateInfo};
    use indexmap::IndexMap;
    use parking_lot::{Mutex, RwLock};
    use pretty_assertions::assert_eq;
    use std::{
        collections::HashMap,
        sync::{Arc, LazyLock},
    };

    static DISPLAYANKI: LazyLock<Mutex<DisplayAnki>> = LazyLock::new(|| {
        let options = Arc::new(RwLock::new(AnkiOptions::default()));
        Mutex::new(DisplayAnki::default_latest(options))
    });

    #[test]
    fn note_options_mut() {
        let mut da = DISPLAYANKI.lock();
        let mut options = da.options_mut();
        let mut note_models_map = options.note_models_map_mut();
        note_models_map.insert("JapaneseVocab".into(), FullModelDetails::mock());
        assert_eq!(note_models_map.len(), 1);
        let details = note_models_map.get("JapaneseVocab").unwrap();
        let templates = details
            .find_templates_by_names(&["Recognition"])
            .collect::<IndexMap<&str, &TemplateInfo>>();
        assert_eq!(templates.len(), 1, "'Recognition' template not found");

        // This unwrap is now safe
        let found = *templates.get("Recognition").unwrap();
        assert_eq!(found.front, "<div class='sentence'>{{Sentence}}</div>");
        let found = *templates.get("Recognition").unwrap();
        assert!(found.front == "<div class='sentence'>{{Sentence}}</div>");
    }

    #[test]
    fn anki_options_mut() {
        let default_opts = AnkiOptions::default();
        let mut da = DISPLAYANKI.lock();
        let mut opts = da.options_mut();
        opts.set_suspend_new_cards(true);
        assert!(default_opts != *opts);
    }
}
