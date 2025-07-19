use anki_direct::{
    cache::model::ModelCache, error::AnkiResult, model::FullModelDetails, notes::Note, AnkiClient,
};
use derive_more::derive::From;
use getset::Getters;
use indexmap::IndexMap;
use parking_lot::{
    ArcRwLockReadGuard, ArcRwLockWriteGuard, MappedRwLockReadGuard, RawRwLock, RwLock,
    RwLockReadGuard, RwLockWriteGuard,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "anki")]
use crate::database::dictionary_database::DictionaryDatabase;
use crate::{
    errors::{error_helpers, DBError},
    settings::{
        AnkiDuplicateScope, AnkiFields, AnkiFieldsError, AnkiNoteOptions, AnkiOptions,
        AnkiTermFieldType, DecksMap, FieldIndex, GlobalAnkiOptions, NoteModelsMap, ProfileError,
        ProfileOptions, ProfileResult, YomichanOptions, YomichanProfile,
    },
    text_scanner::BuildNoteError,
    Ptr, TermDictionaryEntry, Yomichan,
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
    #[error("AnkiConnect error: {0}")]
    AnkiConnect(#[from] anki_direct::error::AnkiError),
    #[error("Model '{0}' not found")]
    ModelNotFound(String),
    #[error("Deck '{0}' not found")]
    DeckNotFound(String),
    #[error("Field index {0} is out of bounds for model '{1}' which has {2} fields.")]
    FieldIndexOutOfBounds(usize, String, usize),
    #[error("Anki fields are uninitialized.")]
    AnkiFieldsUninitialized,
    #[error("No Anki Models found. Please create at least one Note Type in Anki.")]
    NoModelsFound,
    #[error("No Anki Decks found. Please create at least one Deck in Anki.")]
    NoDecksFound,
    #[error("[anki-fields]")]
    AnkiFields(#[from] AnkiFieldsError),
}

#[derive(Clone, Getters, Debug)]
#[getset(get = "pub")]
pub struct DisplayAnki {
    client: Ptr<AnkiClient>,
    /// a [Ptr] to the global options
    options: Ptr<YomichanOptions>,
}

/// Setup Impl
impl DisplayAnki {
    pub fn new_auto(port: &str, options: Ptr<YomichanOptions>) -> AnkiResult<Self> {
        let client = AnkiClient::new_port(port)?.into();
        let res = Self { client, options };
        Ok(res)
    }
    pub fn new_sync(port: &str, version: u8, options: Ptr<YomichanOptions>) -> Self {
        Self {
            client: AnkiClient::new_port_version(port, version).into(),
            options,
        }
    }
    pub fn default_latest(options: Ptr<YomichanOptions>) -> Self {
        Self {
            client: AnkiClient::default().into(),
            options,
        }
    }
}

#[cfg(feature = "anki")]
impl<'a> crate::Backend<'a> {
    pub fn default_sync(db: Arc<DictionaryDatabase<'a>>) -> Result<Self, DisplayAnkiError> {
        use crate::{environment::EnvironmentInfo, text_scanner::TextScanner};

        let rtx = db.r_transaction()?;
        let opts: Option<YomichanOptions> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => YomichanOptions::new(),
        };
        let options: Ptr<YomichanOptions> = options.into();
        let profile: Ptr<YomichanProfile> = options.read_arc().get_current_profile()?;
        let anki = Ptr::new(DisplayAnki::default_latest(options.clone()));
        let backend = Self {
            environment: EnvironmentInfo::default(),
            text_scanner: TextScanner::new(&db),
            anki,
            db: db.clone(),
            options: options.clone(),
        };
        Ok(backend)
    }
}

impl Yomichan<'_> {
    /// Returns a clone of [Ptr<DisplayAnki>]
    /// Can be used across threads or async points w/ [RwLock::read_arc]
    pub fn display_anki(&self) -> Ptr<DisplayAnki> {
        self.backend.anki.clone()
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

    /// Updates in memory [anki_direct::cache::Cache] & [YomichanOptions] to contain up to date note & deck models
    /// To persist to the database, call [Yomichan::update_options]
    pub fn update_all_anki_maps(&self) -> AnkiResult<()> {
        // --- Phase 1: Fetch Data ---
        let (latest_models, latest_decks) = {
            let mut cache = self.client.write_arc();
            let cache = cache.cache_mut();
            // Fetch (note) models data
            let models_data = cache.models_mut().hydrate()?.get_cache().clone();
            // Fetch deck data
            let decks_data = cache.decks_mut().hydrate_names()?.clone().take_cache();

            (models_data, decks_data)
        };
        let latest_models: NoteModelsMap = latest_models.into();
        let latest_decks: DecksMap = latest_decks.into();

        self.options().with_ptr_mut(|opts: &mut YomichanOptions| {
            // Borrow, use, and implicitly drop all in one line.
            let mut global_anki_opts = opts.anki_mut().write();
            *global_anki_opts.note_models_map_mut() = latest_models;
            *global_anki_opts.decks_map_mut() = latest_decks;
        });
        Ok(())
    }

    pub fn build_note_from_entry(
        &self,
        entry: &TermDictionaryEntry,
        sentence: Option<&str>,
    ) -> Result<Note, DisplayAnkiError> {
        // --- Phase 1: Configuration Extraction (Locking Scope) ---
        // This is the ONLY part we are changing.
        // We will extract all needed values into owned variables here.

        // CHANGE 1: Define variables to hold the owned data we will extract.
        let model_name: String;
        let deck_name: String;
        let tags: Vec<String>;
        let field_mappings: Vec<AnkiTermFieldType>;

        {
            let profile = self.options.read().get_current_profile()?;
            let profile_guard = profile.read();
            let anki_opts = profile_guard.anki_options();
            let anki_fields = anki_opts
                .anki_fields()
                .as_ref()
                .ok_or(DisplayAnkiError::AnkiFieldsUninitialized)?;

            tags = anki_opts.tags().clone();
            field_mappings = anki_fields.fields().clone();

            let (model_name_owned, deck_name_owned) = {
                let global_opts = self.options.read();
                let global_anki_opts = global_opts.anki().read();

                let selected_model_idx = *anki_fields.selected_model();
                let selected_deck_idx = *anki_fields.selected_deck();

                let (model_ref, _) = global_anki_opts.get_selected_model(selected_model_idx)?;
                let (deck_ref, _) = global_anki_opts.get_selected_deck(selected_deck_idx)?;

                (model_ref.to_string(), deck_ref.to_string())
            };

            model_name = model_name_owned;
            deck_name = deck_name_owned
        }

        let mut note_builder = anki_direct::notes::NoteBuilder::default();

        note_builder.model_name(model_name).deck_name(deck_name);

        for mapping in &field_mappings {
            let headwords_iter = entry.headwords.iter();
            match mapping {
                AnkiTermFieldType::Term(field_name) => {
                    let value: String = headwords_iter
                        .map(|hw| hw.term.clone())
                        .collect::<Vec<_>>()
                        .join(" ");
                    note_builder.field(field_name, &value);
                }
                AnkiTermFieldType::Reading(field_name) => {
                    let readings: String = headwords_iter
                        .map(|hw| hw.reading.clone())
                        .collect::<Vec<_>>()
                        .join(" ");
                    note_builder.field(field_name, &readings);
                }
                AnkiTermFieldType::Definition(field_name) => {
                    let value: String = entry
                        .definitions
                        .iter()
                        .flat_map(|def| &def.entries)
                        .map(|g| g.plain_text.clone())
                        .collect::<Vec<_>>()
                        .join("<br>");
                    note_builder.field(field_name, &value);
                }
                AnkiTermFieldType::Sentence(field_name) => {
                    if let Some(s) = sentence {
                        note_builder.field(field_name, s);
                    }
                }
                _ => {}
            }
        }

        note_builder.tags(tags);

        let note = note_builder.build(Some(self.client.read_arc().reqwest_client()))?;
        Ok(note)
    }

    /// Configures note creation options for the current profile in one go.
    ///
    /// This helper function simplifies the process of setting up Anki note generation.
    /// It handles:
    /// 1. Ensuring the latest model and deck information is fetched from Anki.
    /// 2. Resolving human-readable model and deck names into the internal indices Anki uses.
    /// 3. Validating and converting user-defined field mappings (`FieldIndex`) into the
    ///    persistent `AnkiTermFieldType` format.
    /// 4. Saving this configuration to the currently active profile.
    ///
    /// # Arguments
    /// * `model_name`: The name of the Anki Note Type (e.g., "Basic", "Japanese").
    /// * `deck_name`: The name of the Anki Deck (e.g., "Default", "Japanese::Vocabulary").
    /// * `field_mappings`: A slice defining how dictionary data (Term, Reading, etc.)
    ///   maps to the fields of the chosen model, specified by index.
    ///
    /// # Errors
    /// Returns an error if the model or deck names are not found, if a field index
    /// is out of bounds for the given model, or if there's an issue accessing the profile.
    pub fn configure_note_creation(
        &mut self,
        model_name: &str,
        deck_name: &str,
        field_mappings: &[FieldIndex],
    ) -> Result<(), DisplayAnkiError> {
        // Step 1: Ensure all Anki data is up-to-date.
        //self.update_all_anki_maps().await?;

        // Step 2: Resolve names to indices and get model details.
        // This is done in a scoped block to release the read lock quickly.
        let (model_idx, model_details, deck_idx) = {
            let global_opts_guard = self.options.read();
            let global_anki_opts = global_opts_guard.anki().read();

            // Find the model by name to get its index and details.
            let (idx, _, details) = global_anki_opts
                .find_model_by_name(model_name)
                .map_err(|_| DisplayAnkiError::ModelNotFound(model_name.to_string()))?;
            let model_idx = idx;
            let model_details = details.clone(); // Clone to use outside the lock.

            // Find the deck by name to get its index.
            let (idx, _, _) = global_anki_opts
                .find_deck_by_name(deck_name)
                .map_err(|_| DisplayAnkiError::DeckNotFound(deck_name.to_string()))?;
            let deck_idx = idx;

            (model_idx, model_details, deck_idx)
        };

        // Step 3: Create the persistent, name-based field mappings.
        let persistent_fields =
            AnkiTermFieldType::from_field_indices(field_mappings, &model_details)?;
        // Step 4: Create the final AnkiFields configuration object.
        let mut new_anki_fields_config = AnkiFields::default();
        new_anki_fields_config
            .set_fields(persistent_fields)
            .set_selected_model(model_idx)
            .set_selected_deck(deck_idx);

        // Step 5: Save the configuration to the current profile. This requires a write lock.
        let profile_ptr = self.options.read().get_current_profile()?;
        let mut profile_guard = profile_ptr.write();
        profile_guard
            .anki_options_mut()
            .set_anki_fields(Some(new_anki_fields_config));

        Ok(())
    }

    /// Configures note creation using the first available model and deck found in Anki.
    ///
    /// helper for quickly setting up tests or new profiles without needing
    /// to know the exact names  the models Anki models or decks. It will:
    /// 1. Fetch the latest Anki data.
    /// 2. Select the very first model and deck returned by Anki (index 0).
    /// 3. Apply the given `field_mappings` to that model.
    /// 4. Save the configuration.
    ///
    /// # Arguments
    /// * `field_mappings`: A slice defining how dictionary data maps to the fields
    ///   of the *first available model*, specified by index.
    ///
    /// # Errors
    /// Returns an error if no models or decks are found in Anki, or if a field index
    /// is invalid for the first model.
    pub fn configure_note_creation_with_first_available(
        &self,
        field_mappings: &[FieldIndex],
    ) -> Result<(), DisplayAnkiError> {
        // Step 1: Ensure all Anki data is up-to-date.
        self.update_all_anki_maps()?;

        // Step 2: Get details for the *first* model and check for the *first* deck.
        // This is done in a scoped block to release the read lock quickly.
        let (model_details, model_idx, deck_idx) = {
            let global_opts_guard = self.options.read();
            let global_anki_opts = global_opts_guard.anki().read();

            // Get the model at index 0.
            let (_model_name, details) = global_anki_opts
                .get_selected_model(0)
                .map_err(|_| DisplayAnkiError::NoModelsFound)?;

            // Ensure a deck exists at index 0.
            global_anki_opts
                .get_selected_deck(0)
                .map_err(|_| DisplayAnkiError::NoDecksFound)?;

            (details.clone(), 0, 0)
        };

        // Step 3: Create the persistent, name-based field mappings.
        let persistent_fields =
            AnkiTermFieldType::from_field_indices(field_mappings, &model_details)?;

        // Step 4: Create the final AnkiFields configuration object.
        let mut new_anki_fields_config = AnkiFields::default();
        new_anki_fields_config
            .set_fields(persistent_fields)
            .set_selected_model(model_idx)
            .set_selected_deck(deck_idx);

        // Step 5: Save the configuration to the current profile. This requires a write lock.
        let profile_ptr = self.options.read().get_current_profile()?;
        let mut profile_guard = profile_ptr.write();
        profile_guard
            .anki_options_mut()
            .set_anki_fields(Some(new_anki_fields_config));

        Ok(())
    }

    /// A convenience helper that configures note creation with a common default layout
    /// for the first available model.
    ///
    /// This function assumes a standard vocabulary note layout:
    /// - Field 0: Term
    /// - Field 1: Reading
    /// - Field 2: Definition
    ///   It is the easiest way to get started if the user has a basic note type.
    pub fn configure_note_creation_with_first_available_defaults(
        &mut self,
    ) -> Result<(), DisplayAnkiError> {
        // A common, sensible default mapping.
        let default_mappings = [
            FieldIndex::Term(0),
            FieldIndex::Reading(1),
            FieldIndex::Definition(2),
        ];
        // Re-use the main helper function.
        self.configure_note_creation_with_first_available(&default_mappings)
    }

    /// Automatically configures note creation with zero user input.
    ///
    /// This is the most abstract helper, designed to "just work" for the most common use cases.
    /// It performs the following actions:
    /// 1. Fetches the latest model and deck information from Anki.
    /// 2. Selects the **first available model** and the **first available deck**.
    /// 3. Intelligently maps standard data fields to the model's fields in a priority order:
    ///    - `Term` -> Field 0
    ///    - `Reading` -> Field 1
    ///    - `Definition` -> Field 2
    ///    - `Sentence` -> Field 3
    ///    - (and so on...)
    /// 4. The mapping automatically stops if the model runs out of fields. For example, a model
    ///    with only 2 fields will be configured for `Term` and `Reading`.
    ///
    /// # Errors
    /// Returns an error if no models or decks are found in the user's Anki collection.
    pub fn configure_note_creation_auto(&self) -> Result<(), DisplayAnkiError> {
        // Step 1: Ensure all Anki data is up-to-date.
        self.update_all_anki_maps()?;

        // Step 2: Get details for the first model and check for the first deck.
        let (model_details, model_idx, deck_idx) = {
            let global_opts_guard = self.options.read();
            let global_anki_opts = global_opts_guard.anki().read();

            // Get the model at index 0.
            let (_model_name, details) = global_anki_opts
                .get_selected_model(0)
                .map_err(|_| DisplayAnkiError::NoModelsFound)?;

            // Ensure a deck exists at index 0.
            global_anki_opts
                .get_selected_deck(0)
                .map_err(|_| DisplayAnkiError::NoDecksFound)?;

            (details.clone(), 0, 0)
        };

        // Step 3: Automatically generate field mappings based on model's available fields.
        let field_mappings = {
            // Define the priority order of how fields should be mapped.
            let desired_field_constructors: &[fn(usize) -> FieldIndex] = &[
                FieldIndex::Term, // Highest priority
                FieldIndex::Reading,
                FieldIndex::Definition,
                FieldIndex::Sentence,
                FieldIndex::Frequency,
                FieldIndex::TermAudio,
                FieldIndex::SentenceAudio,
                FieldIndex::Image, // Lowest priority
            ];

            let num_fields_in_model = model_details.fields.len();
            let mut mappings = Vec::new();

            // Iterate through our desired order and assign to available field indices.
            for (i, constructor) in desired_field_constructors.iter().enumerate() {
                if i < num_fields_in_model {
                    // The model has a field at this index, so create the mapping.
                    mappings.push(constructor(i));
                } else {
                    // We've run out of fields on the model, so stop mapping.
                    break;
                }
            }
            mappings
        };

        // Step 4: Convert to persistent format and save the configuration.
        // This logic is now familiar from the other helpers.
        let persistent_fields =
            AnkiTermFieldType::from_field_indices(&field_mappings, &model_details)?;

        let mut new_anki_fields_config = AnkiFields::default();
        new_anki_fields_config
            .set_fields(persistent_fields)
            .set_selected_model(model_idx)
            .set_selected_deck(deck_idx);

        let profile_ptr = self.options.read().get_current_profile()?;
        let mut profile_guard = profile_ptr.write();
        profile_guard
            .anki_options_mut()
            .set_anki_fields(Some(new_anki_fields_config));

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "anki")]
mod displayanki {
    use crate::{
        anki::DisplayAnki,
        settings::{AnkiFields, AnkiOptions, AnkiTermFieldType, FieldIndex, YomichanOptions},
        test_utils::{TEST_PATHS, YCD},
        Ptr, Yomichan,
    };
    use anki_direct::model::{FullModelDetails, TemplateInfo};
    use indexmap::IndexMap;
    use parking_lot::{Mutex, RwLock};
    use pretty_assertions::assert_eq;
    use scopeguard;
    use std::sync::{Arc, LazyLock};

    static DISPLAYANKI: LazyLock<Mutex<DisplayAnki>> = LazyLock::new(|| {
        let options = Ptr::new(YomichanOptions::new());
        Mutex::new(DisplayAnki::default_latest(options))
    });

    #[ignore]
    #[test]
    fn test_cache_persisting() {
        {
            let ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
            let displayanki = ycd.display_anki();
            {
                let guard = displayanki.read();
                guard.update_all_anki_maps().unwrap();
                let opts = ycd.options().read_arc();
                let global_anki = opts.anki().read();
                let map = global_anki.note_models_map();
                // if you dbg map here it works
            }
            ycd.update_options().unwrap();
        }
        // this dbg is empty
        {
            let ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
            let opts = ycd.options().read_arc();
            let global_anki = opts.anki().read();
            let map = global_anki.note_models_map();
            dbg!(map);
        }
    }

    #[ignore]
    #[test]
    fn update_all_anki_maps() {
        let mut anki = DISPLAYANKI.lock();
        anki.update_all_anki_maps().unwrap();
        let opts = anki.options().read();
        let anki = opts.anki().read();
        let map = anki.note_models_map();
        dbg!(map);
    }

    #[ignore]
    #[test]
    fn build_note() {
        let mut ycd = YCD.write();
        ycd.set_language("ja");
        {
            let mut anki = ycd.display_anki();
            anki.read().update_all_anki_maps().unwrap();
            let anki = anki.read_arc();
            let global_opts = anki.options().read_arc();

            // check current models to update if some changed for this test
            {
                dbg!(global_opts.anki().read().decks_map());
            };

            let global_anki_opts = global_opts.anki().read();
            // should be 'aramrw'
            let current_model = global_anki_opts.get_selected_model(0).unwrap();
            let current_profile = global_opts.get_current_profile().unwrap();
            let mut current_profile = current_profile.write();
            let mut anki_opts: &mut AnkiOptions = current_profile.anki_options_mut();
            let fields = anki_opts.anki_fields();
            let fields: AnkiFields = match fields {
                Some(f) => f.clone(),
                // need to tell yomichan where to put what
                // entry information in the note type fields
                None => {
                    let intents = [
                        FieldIndex::Term(0),
                        FieldIndex::Reading(2),
                        FieldIndex::Definition(3),
                    ];
                    let fields =
                        AnkiTermFieldType::from_field_indices(&intents, current_model.1).unwrap();
                    let mut new = AnkiFields::default();
                    new.set_fields(fields)
                        .set_selected_model(0)
                        .set_selected_deck(0);
                    new
                }
            };
            anki_opts.set_anki_fields(Some(fields));
        }
        let sentence = "日本語が好きです";
        let res = ycd.search(sentence).unwrap();
        let mut notes = vec![];
        let display_anki = ycd.display_anki();
        for item in res {
            let entry: Arc<crate::TermSearchResults> = item.results.unwrap();
            let first = &entry.dictionary_entries[0];
            let note = display_anki
                .read()
                .build_note_from_entry(first, Some(sentence))
                .unwrap();
            notes.push(note);
        }
        dbg!(&notes);
        let id = display_anki
            .read()
            .client()
            .read()
            .notes()
            .add_notes(&[notes[0].clone()])
            .unwrap();
        display_anki
            .read()
            .client()
            .read()
            .notes()
            .gui_edit(id[0])
            .unwrap();
    }

    #[ignore]
    #[test]
    fn auto_note() {
        let mut ycd = YCD.write();
        ycd.set_language("ja");

        ycd.display_anki()
            .read_arc()
            .configure_note_creation_with_first_available(&[
                FieldIndex::Term(0),
                FieldIndex::Sentence(1),
                FieldIndex::Definition(3),
                FieldIndex::Reading(2),
            ])
            .unwrap();

        // The rest of the test logic remains the same.
        let sentence = "日本語が好きです";
        let res = ycd.search(sentence).unwrap();
        let mut notes = vec![];
        let display_anki = ycd.display_anki();
        for item in res {
            let entry: Arc<crate::TermSearchResults> = item.results.unwrap();
            let first = &entry.dictionary_entries[0];
            let note = display_anki
                .read_arc()
                .build_note_from_entry(first, Some(sentence))
                .unwrap();
            notes.push(note);
        }
        dbg!(&notes);
        let id = display_anki
            .read_arc()
            .client()
            .read_arc()
            .notes()
            .add_notes(&[notes[0].clone()])
            .unwrap();
        display_anki
            .read_arc()
            .client()
            .read_arc()
            .notes()
            .gui_edit(id[0])
            .unwrap();
    }

    #[ignore]
    #[test]
    fn build_note_auto_config() {
        let mut ycd = YCD.write();
        ycd.set_language("ja");

        let display_anki = ycd.display_anki().read_arc();
        display_anki.configure_note_creation_auto();

        // The rest of the test logic is identical.
        let sentence = format!(
            "日本語が好きです - {}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let res = ycd.search(sentence.as_str()).unwrap();
        let display_anki = ycd.display_anki();
        let entry: Arc<crate::TermSearchResults> = res.into_iter().next().unwrap().results.unwrap();
        let first = &entry.dictionary_entries[0];
        let note = display_anki
            .read_arc()
            .build_note_from_entry(first, Some(sentence.as_str()))
            .unwrap();
        dbg!(&note);
        let id = display_anki
            .read_arc()
            .client()
            .read_arc()
            .notes()
            .add_notes(&[note])
            .unwrap();

        display_anki
            .read_arc()
            .client()
            .read_arc()
            .notes()
            .gui_edit(id[0])
            .unwrap();

        // Use scopeguard to ensure the note is deleted after the test, even if it panics.
        let client = display_anki.read_arc().client().clone();
        scopeguard::defer! {
            client.read_arc().notes().delete_notes_by_ids(&id).unwrap();
        }
    }
}
