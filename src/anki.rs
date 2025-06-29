use anki_direct::{
    cache::ModelCache, error::AnkiResult, model::FullModelDetails, notes::Note, AnkiClient,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

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
    pub fn options_mut(&self) -> Option<RwLockWriteGuard<AnkiOptions>> {
        self.options.write().ok()
    }
    pub fn options(&self) -> Option<RwLockReadGuard<AnkiOptions>> {
        self.options.read().ok()
    }
    // pub fn note_models_mut(&mut self) -> Option<&mut NoteModelsMap> {
    //     let options: Option<RwLockReadGuard<'_, AnkiOptions>> = self.options();
    //     if let Some(options) = options.and_then(|opt| opt.note_models_map_mut()).ok()) {
    //         self.options.note_models_map_mut()
    //     }
    // }

    /// [Vec::extend]s new [AnkiNoteOptions] to `note_options` of [AnkiOptions]
    pub fn add_note_models<K, V, I>(&mut self, new: I) -> RwLockWriteGuard<'_, AnkiOptions>
    where
        K: Into<String>,
        V: Into<FullModelDetails>,
        I: IntoIterator<Item = (K, V)>,
    {
        let options = self.options_mut();
        let mut wlock = options.unwrap();
        let note_models = wlock.note_models_map_mut();
        note_models.extend(new.into_iter().map(|(k, v)| (k.into(), v.into())));
        wlock
    }

    /// Updates [AnkiDirect::Cache] & [AnkiNoteOptions] to contain all found (note) models.
    ///
    /// This function is useful for displaying the list of models, and letting the user
    /// select which model to create a note for, via [AnkiOptions]`.not_models_map.selected`
    pub async fn auto_add_all_note_models(
        &mut self,
    ) -> AnkiResult<RwLockWriteGuard<'_, AnkiOptions>> {
        let mut cache = self.client.cache_mut();
        let mut model_cache = cache.models_mut();
        let latest = model_cache.hydrate().await?.clone();
        let map = latest.get_cache().clone();
        Ok(self.add_note_models(map))
    }

    /// Updates [AnkiDirect::Cache] as well as [AnkiNoteOptions] to use specified (note) models.
    async fn auto_add_note_models_by_name(
        &mut self,
        model_names: &[&str],
    ) -> AnkiResult<RwLockWriteGuard<'_, AnkiOptions>> {
        let mut cache = self.client.cache_mut();
        let mut model_cache = cache.models_mut();
        let models = model_cache.hydrate().await?;
        let found: IndexMap<String, FullModelDetails> =
            models.find_many_from_key_owned(model_names).collect();
        Ok(self.add_note_models(found))
    }
}

impl<'a> Yomichan<'a> {
    // fn build_note(&self) {
    //     let anki = &self.backend.anki;
    //     let model_cache = &self.mod_options();
    // }
}

// #[cfg(test)]
// mod displayanki {
//     use crate::{anki::DisplayAnki, settings::AnkiOptions};
//     use anki_direct::model::{FullModelDetails, TemplateInfo};
//     use indexmap::IndexMap;
//     use pretty_assertions::assert_eq;
//     use std::{
//         collections::HashMap,
//         sync::{LazyLock, Mutex, RwLock},
//     };
//
//     static DISPLAYANKI: LazyLock<Mutex<DisplayAnki>> =
//         LazyLock::new(|| Mutex::new(DisplayAnki::default_latest()));
//
//     #[test]
//     fn note_options_mut() {
//         let mut da = DISPLAYANKI.lock().unwrap();
//         let mut note_models_map = da.note_models_mut();
//         note_models_map.insert("JapaneseVocab".into(), FullModelDetails::mock());
//         assert_eq!(note_models_map.len(), 1);
//         let details = note_models_map.get("JapaneseVocab").unwrap();
//         let templates = details
//             .find_templates_by_names(&["Recognition"])
//             .collect::<IndexMap<&str, &TemplateInfo>>();
//         assert_eq!(templates.len(), 1, "'Recognition' template not found");
//
//         // This unwrap is now safe
//         let found = *templates.get("Recognition").unwrap();
//         assert_eq!(found.front, "<div class='sentence'>{{Sentence}}</div>");
//         let found = *templates.get("Recognition").unwrap();
//         assert!(found.front == "<div class='sentence'>{{Sentence}}</div>");
//     }
//
//     #[test]
//     fn anki_options_mut() {
//         let default_opts = AnkiOptions::default();
//         let mut da = DISPLAYANKI.lock().unwrap();
//         let mut opts = da.options_mut();
//         opts.set_suspend_new_cards(true);
//         assert!(&default_opts != opts);
//     }
// }
