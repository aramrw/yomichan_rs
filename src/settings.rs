use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

#[cfg(feature = "anki")]
use crate::anki::DisplayAnki;
use crate::backend::Backend;
use crate::Ptr;
use crate::PtrRGaurd;
use crate::PtrWGaurd;
use anki_direct::cache::model::ModelCache;
use anki_direct::decks::DeckConfig;
use anki_direct::model::FullModelDetails;
use anki_direct::notes::MediaSource;
use better_default::Default;
use derivative::Derivative;
use derive_more::derive::Deref;
use derive_more::derive::DerefMut;
use derive_more::derive::From;
use derive_where::derive_where;
use getset::Getters;
use getset::MutGetters;
use getset::Setters;
use indexmap::IndexMap;
use indexmap::IndexSet;
use native_db::native_db;
use native_db::ToKey;
use native_model::native_model;
use native_model::Model;
use parking_lot::ArcRwLockReadGuard;
use parking_lot::ArcRwLockUpgradableReadGuard;
use parking_lot::RawRwLock;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_with::SerializeDisplay;
use url::form_urlencoded::Target;

use crate::{
    database::dictionary_importer::DictionarySummary, translation::FindTermsSortOrder,
    translator::FindTermsMode,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalOptions {
    pub database: GlobalDatabaseOptions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalDatabaseOptions {
    pub prefix_wildcards_supported: bool,
}

/// Global Yomichan Settings.
#[native_model(id = 20, version = 1)]
#[native_db]
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, MutGetters, Setters,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct YomichanOptions {
    #[primary_key]
    id: String,
    pub version: String,
    pub profiles: Vec<Ptr<YomichanProfile>>,
    pub current_profile: usize,
    pub global: GlobalOptions,
    pub anki: Ptr<GlobalAnkiOptions>,
}

impl YomichanOptions {
    pub fn new() -> Self {
        let global_anki_options = Ptr::new(GlobalAnkiOptions::default());
        Self {
            id: "global_user_options".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            profiles: vec![YomichanProfile::new_default(&global_anki_options).into()],
            current_profile: 0,
            global: GlobalOptions::default(),
            anki: global_anki_options,
        }
    }
    /// Gets a [Ptr] to the currently selected profile.
    /// Returns None if the index is out of bounds.
    pub fn get_current_profile(&self) -> ProfileResult<Ptr<YomichanProfile>> {
        let Some(pf) = self.profiles.get(self.current_profile) else {
            return Err(ProfileError::SelectedOutofBounds {
                selected: *self.current_profile(),
                len: self.profiles.len(),
            });
        };
        Ok(pf.clone())
    }
}

impl Backend<'_> {
    pub fn get_current_profile(&self) -> ProfileResult<Ptr<YomichanProfile>> {
        let opts = self.options.read_arc();
        opts.get_current_profile()
    }
}

/// Returns `T` or  [ProfileError]
pub type ProfileResult<T> = Result<T, ProfileError>;
#[derive(thiserror::Error, Debug)]
pub enum ProfileError {
    #[error("tried to index selected_profile[selected], but profiles.len() = {len}")]
    SelectedOutofBounds { selected: usize, len: usize },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, MutGetters)]
pub struct YomichanProfile {
    pub name: String,
    pub condition_groups: Vec<ProfileConditionGroup>,
    #[getset(get = "pub", get_mut = "pub")]
    pub options: ProfileOptions,
}

impl YomichanProfile {
    pub fn new_default(global_anki_options: &Ptr<GlobalAnkiOptions>) -> Self {
        let mut default = Self::default();
        default.options = ProfileOptions::new_default(global_anki_options.clone());
        default
    }
    pub fn new(
        name: String,
        condition_groups: Vec<ProfileConditionGroup>,
        options: ProfileOptions,
    ) -> Self {
        Self {
            name,
            condition_groups,
            options,
        }
    }

    /// Sets the current [YomichanProfile]'s language to the iso
    // TODO: use an enum instead of `&'str`
    pub fn set_language(&mut self, iso: &str) {
        self.options.general.language = iso.to_string();
    }

    pub fn extend_dictionaries(
        &mut self,
        new: impl IntoIterator<Item = (String, DictionaryOptions)>,
    ) {
        self.options.dictionaries.extend(new);
    }

    pub fn get_dictionary_options_from_name(&self, find: &str) -> Option<&DictionaryOptions> {
        self.options.dictionaries.get(find)
    }

    pub fn anki_options(&self) -> &AnkiOptions {
        &self.options().anki
    }
    pub fn anki_options_mut(&mut self) -> &mut AnkiOptions {
        &mut self.options_mut().anki
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileConditionGroup {
    pub conditions: Vec<ProfileCondition>,
}

/// Profile usage conditions are used to automatically select certain profiles based on context.
/// For example, different profiles can be used,
/// depending on the nested level of the popup, or based on the website's URL.
/// Conditions are organized into groups corresponding to the order in which they are checked.
/// If all of the conditions in any group of a profile are met, then that profile will be used for that context.
/// If no conditions are specified, the profile will only be used if it is selected as the default profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum ProfileConditionType {
    #[default]
    PopupLevel,
    Url,
    ModifierKeys,
    Flags,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileCondition {
    pub condition_type: ProfileConditionType,
    pub operator: String,
    pub value: String,
}

fn compare_prog_opts(a: &Arc<RwLock<AnkiOptions>>, b: &Arc<RwLock<AnkiOptions>>) -> bool {
    // If the pointers are the same, they are equal.
    if Arc::ptr_eq(a, b) {
        return true;
    }
    // Otherwise, lock and compare the inner data.
    // We use try_read() to avoid deadlocks in more complex scenarios,
    // though unwrap() is fine if you're sure it won't deadlock.
    if let (Some(a_lock), Some(b_lock)) = (a.try_read(), b.try_read()) {
        *a_lock == *b_lock
    } else {
        // Handle the case where locking fails.
        // maybe means they are not equal, (or want to panic).
        false
    }
}

/// A struct used for managing Profiles
///
/// # Usage
/// Can be used for seperating profiles by language or user
#[derive(Clone, Debug, Serialize, Deserialize, Default, Derivative, Getters, MutGetters)]
#[derivative(PartialEq)]
pub struct ProfileOptions {
    #[getset(get = "pub", get_mut = "pub")]
    pub general: GeneralOptions,
    pub popup_window: PopupWindowOptions,
    /// should be moved to [crate::audio]
    pub audio: AudioOptions,
    pub scanning: ScanningOptions,
    pub translation: TranslationOptions,
    #[getset(get = "pub", get_mut = "pub")]
    pub dictionaries: IndexMap<String, DictionaryOptions>,
    pub parsing: ParsingOptions,
    /// a ptr for [DisplayAnki]
    //#[derivative(PartialEq(compare_with = "compare_prog_opts"))]
    pub anki: AnkiOptions,
    pub sentence_parsing: SentenceParsingOptions,
    pub inputs: InputsOptions,
    pub clipboard: ClipboardOptions,
    pub accessibility: AccessibilityOptions,
}
impl ProfileOptions {
    fn new_default(global_anki_options: Ptr<GlobalAnkiOptions>) -> Self {
        let mut default = Self::default();
        let mut anki_opts = &mut default.anki;
        anki_opts.global_anki_options = global_anki_options;
        default
    }
    /// Gets the main dictionary for this [YomichanProfile] from it's [ProfileOptions]
    pub fn main_dictionary(&self) -> &str {
        self.general.main_dictionary.as_str()
    }
    /// Updates the current dictionary and returns the previous one
    pub fn set_main_dictionary(&mut self, new: String) -> String {
        let old = self.general.main_dictionary.as_mut_string();
        std::mem::replace(old, new)
    }
}
impl YomichanProfile {
    /// Gets the main dictionary for this [YomichanProfile]
    pub fn get_main_dictionary(&self) -> &str {
        self.options.main_dictionary()
    }
    /// Updates the main dictionary and returns the previous one
    pub fn set_main_dictionary(&mut self, new: String) -> String {
        self.options.set_main_dictionary(new)
    }
    /// Gets a ref to this profiles dictionaries
    pub fn dictionaries(&self) -> &IndexMap<String, DictionaryOptions> {
        self.options.dictionaries()
    }
    pub fn dictionaries_mut(&mut self) -> &mut IndexMap<String, DictionaryOptions> {
        self.options.dictionaries_mut()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GeneralOptions {
    pub enable: bool,
    pub language: String,
    pub result_output_mode: FindTermsMode,
    pub prefix_wildcard_supported: bool,
    pub debug_info: bool,
    pub max_results: u8,
    pub show_advanced: bool,
    pub font_family: String,
    pub font_size: u8,
    pub line_height: String,
    pub popup_display_mode: PopupDisplayMode,
    pub popup_width: u8,
    pub popup_height: u8,
    pub popup_horizontal_offset: u8,
    pub popup_vertical_offset: u8,
    pub popup_horizontal_offset2: u8,
    pub popup_vertical_offset2: u8,
    pub popup_horizontal_text_position: PopupHorizontalTextPosition,
    pub popup_vertical_text_position: PopupVerticalTextPosition,
    pub popup_scaling_factor: u8,
    pub popup_scale_relative_to_page_zoom: bool,
    pub popup_scale_relative_to_visual_viewport: bool,
    pub show_guide: bool,
    pub enable_context_menu_scan_selected: bool,
    pub compact_tags: bool,
    pub compact_glossaries: bool,
    pub main_dictionary: String,
    pub popup_theme: PopupTheme,
    pub popup_outer_theme: PopupShadow,
    pub custom_popup_css: String,
    pub custom_popup_outer_css: String,
    pub enable_wanakana: bool,
    pub show_pitch_accent_downstep_notation: bool,
    pub show_pitch_accent_position_notation: bool,
    pub show_pitch_accent_graph: bool,
    pub show_iframe_popups_in_root_frame: bool,
    pub use_secure_popup_frame_url: bool,
    pub use_popup_shadow_dom: bool,
    pub use_popup_window: bool,
    pub popup_current_indicator_mode: PopupCurrentIndicatorMode,
    pub popup_action_bar_visibility: PopupActionBarVisibility,
    pub popup_action_bar_location: PopupActionBarLocation,
    pub frequency_display_mode: FrequencyDisplayStyle,
    pub term_display_mode: TermDisplayStyle,
    pub sort_frequency_dictionary: Option<String>,
    pub sort_frequency_dictionary_order: FindTermsSortOrder,
    pub sticky_search_header: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PopupWindowOptions {
    pub width: u8,
    pub height: u8,
    pub left: u8,
    pub top: u8,
    pub use_left: bool,
    pub use_top: bool,
    pub window_type: PopupWindowType,
    pub window_state: PopupWindowState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioOptions {
    pub enabled: bool,
    pub volume: u8,
    pub auto_play: bool,
    pub sources: Vec<AudioSourceOptions>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioSourceOptions {
    pub audio_source_type: AudioSourceType,
    pub url: String,
    pub voice: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningOptions {
    pub inputs: Vec<ScanningInput>,
    pub prevent_middle_mouse: ScanningPreventMiddleMouseOptions,
    pub touch_input_enabled: bool,
    pub pointer_events_enabled: bool,
    pub select_text: bool,
    pub alphanumeric: bool,
    pub auto_hide_results: bool,
    pub delay: u8,
    pub hide_delay: u8,
    pub length: u8,
    pub deep_dom_scan: bool,
    pub popup_nesting_max_depth: u8,
    pub enable_popup_search: bool,
    pub enable_on_popup_expressions: bool,
    pub enable_on_search_page: bool,
    pub enable_search_tags: bool,
    pub layout_aware_scan: bool,
    pub match_type_prefix: bool,
    pub hide_popup_on_cursor_exit: bool,
    pub hide_popup_on_cursor_exit_delay: u8,
    pub normalize_css_zoom: bool,
    pub scan_alt_text: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInput {
    pub include: String,
    pub exclude: String,
    pub types: ScanningInputTypes,
    pub options: ScanningInputOptions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInputTypes {
    pub mouse: bool,
    pub touch: bool,
    pub pen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInputOptions {
    pub show_advanced: bool,
    pub search_terms: bool,
    pub search_kanji: bool,
    pub scan_on_touch_move: bool,
    pub scan_on_touch_press: bool,
    pub scan_on_touch_release: bool,
    pub scan_on_touch_tap: bool,
    pub scan_on_pen_move: bool,
    pub scan_on_pen_hover: bool,
    pub scan_on_pen_release_hover: bool,
    pub scan_on_pen_press: bool,
    pub scan_on_pen_release: bool,
    pub prevent_touch_scrolling: bool,
    pub prevent_pen_scrolling: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningPreventMiddleMouseOptions {
    pub on_web_pages: bool,
    pub on_popup_pages: bool,
    pub on_search_pages: bool,
    pub on_search_query: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationOptions {
    pub convert_half_width_characters: TranslationConvertType,
    pub convert_numeric_characters: TranslationConvertType,
    pub alphabetic_to_hiragana: TranslationConvertType,
    pub convert_hiragana_to_katakana: TranslationConvertType,
    pub convert_katakana_to_hiragana: TranslationConvertType,
    pub collapse_emphatic_sequences: TranslationCollapseEmphaticSequences,
    pub text_replacements: TranslationTextReplacementOptions,
    pub search_resolution: SearchResolution,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
/// # Example
///
/// `Letter`: A dog → _"A dog"_ | _"A do"_ | _"A d"_ | _"A"_.
///
/// `Word`: A dog → _"A dog"_ | _"A"_.
pub enum SearchResolution {
    #[default]
    Letter,
    Word,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationTextReplacementOptions {
    pub search_original: bool,
    pub groups: Vec<Vec<TranslationTextReplacementGroup>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationTextReplacementGroup {
    pub pattern: String,
    pub ignore_case: bool,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DictionaryOptions {
    /// The title of the dictionary.
    pub name: String,
    pub alias: String,
    // /// What order the dictionary's results get returned.
    // pub priority: usize,
    /// Whether or not the dictionary will be used.
    pub enabled: bool,
    /// If you have two dictionaries, `Dict 1` and `Dict 2`:
    /// - Set the [`ResultOutputMode`] to `Group` results for the main dictionary entry.
    /// - Choose `Dict 1` as the main dictionary for merged mode.
    /// - Enable `allow_secondary_searches` on `Dict 2`.
    ///   _(Can be enabled for multiple dictionaries)_.
    ///
    /// Yomichan_rs will now first perform an _initial_ lookup in `Dict 1`, fetching the grouped definition.
    /// It will then use the headwords from `Dict 1`to perform a secondary lookup in `Dict 2`,
    /// merging the two dictionary's definitions.
    pub allow_secondary_searches: bool,
    /// Dictionary definitions can be collapsed if they exceed a certain line count,
    /// which may be useful for dictionaries with long definitions. There are five different modes:
    ///
    /// By default, the number of lines shown for a definition is 3.
    /// This can be configured by adjusting the Custom CSS `styles`;
    ///
    /// _(Value can be a unitless integer or decimal number)_.
    pub definitions_collapsible: DictionaryDefinitionsCollapsible,
    /// When deinflecting words, only dictionary entries whose POS
    /// matches that expected by the deinflector will be shown.
    pub parts_of_speech_filter: bool,
    /// Deinflections from this dictionary will be used.
    pub use_deinflections: bool,
    /// # Example
    ///
    /// ```
    /// /* Globally set the line count */
    /// :root {
    /// --collapsible-definition-line-count: 2;
    /// }
    ///
    /// /* Set the line count for a specific dictionary */
    /// .definition-item[data-dictionary='JMdict'] {
    /// --collapsible-definition-line-count: 2;
    /// }
    ///
    /// /* Spoiler-like functionality, use with Force collapsed mode */
    /// .definition-item[data-dictionary='JMdict'] .definition-item-inner.collapsible.collapsed {
    /// color: #000000;
    /// background-color: #000000;
    /// }
    /// ```
    pub styles: Option<String>,
}

impl DictionaryOptions {
    pub fn new(dict_name: String) -> Self {
        DictionaryOptions {
            name: dict_name.clone(),
            alias: dict_name,
            //priority: p_len,
            enabled: true,
            allow_secondary_searches: false,
            definitions_collapsible: DictionaryDefinitionsCollapsible::Expanded,
            parts_of_speech_filter: false,
            use_deinflections: true,
            styles: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ParsingOptions {
    pub enable_scanning_parser: bool,
    pub enable_mecab_parser: bool,
    pub selected_parser: Option<String>,
    pub term_spacing: bool,
    pub reading_mode: ParsingReadingMode,
}

/// `yomichan_rs`-unique struct for caching discovered models from Anki.
/// # Fields
/// * `selected`: an index in the map for the (note) model to use when creating notes.
#[derive(
    Clone, Serialize, Deserialize, Debug, PartialEq, Default, Setters, Getters, MutGetters,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct DecksMap {
    map: IndexMap<String, Option<DeckConfig>>,
    selected: usize,
}
impl From<IndexMap<String, Option<DeckConfig>>> for DecksMap {
    fn from(map: IndexMap<String, Option<DeckConfig>>) -> Self {
        Self { map, selected: 0 }
    }
}

impl DecksMap {
    /// Returns currently selected [DeckConfig]
    pub fn selected_deck(&self) -> Option<(usize, &str, &Option<DeckConfig>)> {
        let i = self.selected;
        let (name, config) = self.map.get_index(i)?;
        Some((i, name.as_str(), config))
    }
}
impl Deref for DecksMap {
    type Target = IndexMap<String, Option<DeckConfig>>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl DerefMut for DecksMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[derive(Clone, Copy)]
pub enum FieldIndex {
    Term(usize),
    Reading(usize),
    Sentence(usize),
    Definition(usize),
    TermAudio(usize),
    SentenceAudio(usize),
    Image(usize),
    Frequency(usize),
}

/// Defines Anki fields types that correspond to a [DictionaryTermEntry] for making notes
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnkiTermFieldType {
    Term(String),
    Reading(String),
    Sentence(String),
    Definition(String),
    TermAudio(String),
    SentenceAudio(String),
    Image(String),
    Frequency(String),
}
impl AnkiTermFieldType {
    /// Converts a slice of user-provided `FieldIndex` mappings into a persistent,
    /// name-based `Vec<AnkiTermFieldType>`.
    ///
    /// This function acts as the bridge between a simple, user-friendly input format
    /// (mapping by index) and a robust internal storage format (mapping by field name).
    /// It validates that each index provided by the user is valid for the given Anki model.
    ///
    /// # Arguments
    ///
    /// * `mappings`: A slice of `FieldIndex` enums provided by the user.
    /// * `model`: The specific `FullModelDetails` for the Anki note type being configured.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AnkiTermFieldType>)`: A vector of the successfully resolved field types.
    /// * `Err(usize)`: The first invalid index encountered that was out of bounds for the model.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let model = FullModelDetails {
    ///     name: "MyModel".into(),
    ///     fields: vec!["Expression".into(), "Meaning".into(), "Reading".into()],
    ///     // ... other details
    /// };
    ///
    /// let user_mappings = &[FieldIndex::Term(0), FieldIndex::Definition(1)];
    ///
    /// let persistent_fields = AnkiTermFieldType::from_field_indices(user_mappings, &model).unwrap();
    ///
    /// assert_eq!(persistent_fields, vec![
    ///     AnkiTermFieldType::Term("Expression".to_string()),
    //      AnkiTermFieldType::Definition("Meaning".to_string()),
    /// ]);
    /// ```
    pub fn from_field_indices(
        mappings: &[FieldIndex],
        model: &FullModelDetails,
    ) -> Result<Vec<AnkiTermFieldType>, AnkiFieldsError> {
        let anki_model_fields = &model.fields;
        let mut resolved_fields = Vec::with_capacity(mappings.len());

        for mapping in mappings {
            // 1. Deconstruct the user's mapping to get the index and the constructor function.
            let (index, constructor): (usize, fn(String) -> AnkiTermFieldType) = match *mapping {
                FieldIndex::Term(i) => (i, AnkiTermFieldType::Term),
                FieldIndex::Reading(i) => (i, AnkiTermFieldType::Reading),
                FieldIndex::Sentence(i) => (i, AnkiTermFieldType::Sentence),
                FieldIndex::Definition(i) => (i, AnkiTermFieldType::Definition),
                FieldIndex::TermAudio(i) => (i, AnkiTermFieldType::TermAudio),
                FieldIndex::SentenceAudio(i) => (i, AnkiTermFieldType::SentenceAudio),
                FieldIndex::Image(i) => (i, AnkiTermFieldType::Image),
                FieldIndex::Frequency(i) => (i, AnkiTermFieldType::Frequency),
            };

            // 2. Use the index to look up the field name from the model.
            //    If the index is out of bounds, return an error with the invalid index.
            let field_name = match anki_model_fields.get(index) {
                Some(name) => name.clone(),
                None => {
                    return Err(AnkiFieldsError::ModelFieldsOutOfBounds {
                        model: model.name.clone(),
                        index,
                        fields_len: anki_model_fields.len(),
                    })
                }
            };

            // 3. Call the constructor with the now-validated field name and add it to the result.
            resolved_fields.push(constructor(field_name));
        }

        Ok(resolved_fields)
    }
    /// Constructs a new [AnkiTermFieldType] from a `T`
    /// Helper function to construct a variant from a value and a constructor function.
    ///
    /// This generic function avoids repetitive code by taking a value of any type `T`
    /// and a `constructor` (like `TermFieldType::Term` or `TermFieldType::Image`)
    /// and applying the constructor to the value.
    ///
    /// # Type Parameters
    /// * `T`: The type of the value to be wrapped in the enum (e.g., `String`, `MediaSource`).
    /// * `F`: A function or closure that takes `T` and returns `Self`.
    fn from_value<T, F>(value: T, constructor: F) -> Self
    where
        F: FnOnce(T) -> Self,
    {
        constructor(value)
    }
}

/// `yomichan_rs`-unique struct for caching discovered models from Anki.
/// # Fields
/// * `selected`: an index in the map for the (note) model to use when creating notes.
#[derive(
    Clone,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Default,
    Setters,
    Getters,
    MutGetters,
    Deref,
    From,
    DerefMut,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct NoteModelsMap {
    #[deref]
    #[deref_mut]
    map: IndexMap<String, FullModelDetails>,
    selected: usize,
}
impl From<IndexMap<String, FullModelDetails>> for NoteModelsMap {
    fn from(map: IndexMap<String, FullModelDetails>) -> Self {
        Self { map, selected: 0 }
    }
}

impl<'n> From<(usize, &'n str, &'n FullModelDetails)> for NoteModelNode<'n> {
    fn from(value: (usize, &'n str, &'n FullModelDetails)) -> Self {
        NoteModelNode(value)
    }
}
#[derive(Clone, Debug, PartialEq, Getters, Setters, MutGetters, DerefMut, Deref)]
pub struct NoteModelNode<'n>((usize, &'n str, &'n FullModelDetails));
impl NoteModelsMap {
    pub fn selected_note(&self) -> Option<NoteModelNode<'_>> {
        let i = self.selected;
        let (name, details) = self.map.get_index(i)?;
        let node: NoteModelNode<'_> = (i, name.as_str(), details).into();
        Some(node)
    }
}

/// Struct for [Options] that caches note models and decks from Anki
#[derive(
    Clone, Debug, Getters, Setters, MutGetters, Serialize, Deserialize, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub")]
pub struct GlobalAnkiOptions {
    /// [IndexMap] of [Note](https://docs.ankiweb.net/getting-started.html#note-types)
    note_models_map: NoteModelsMap,
    /// [IndexMap] of [Deck](https://docs.ankiweb.net/getting-started.html#note-types)
    deck_models_map: DecksMap,
}
impl GlobalAnkiOptions {
    pub fn get_selected_model(
        &self,
        i: usize,
    ) -> Result<(&str, &FullModelDetails), AnkiFieldsError> {
        let map = self.note_models_map();
        map.get_index(i)
            .map(|(k, v)| (k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelOutOfBounds(i))
    }
    pub fn get_selected_deck(
        &self,
        i: usize,
    ) -> Result<(&str, &Option<DeckConfig>), AnkiFieldsError> {
        let map = self.deck_models_map();
        map.get_index(i)
            .map(|(k, v)| (k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelOutOfBounds(i))
    }
    pub fn find_model_by_name(
        &self,
        name: &str,
    ) -> Result<(usize, &str, &FullModelDetails), AnkiFieldsError> {
        let map = self.note_models_map();
        map.get_full(name)
            .map(|(i, k, v)| (i, k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelNotFound(name.to_string()))
    }
    pub fn find_deck_by_name(
        &self,
        name: &str,
    ) -> Result<(usize, &str, &Option<DeckConfig>), AnkiFieldsError> {
        let map = self.deck_models_map();
        map.get_full(name)
            .map(|(i, k, v)| (i, k.as_str(), v))
            .ok_or(AnkiFieldsError::DeckNotFound(name.to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AnkiFieldsError {
    #[error("unitialized yomichan anki fields")]
    Uninitialized,
    #[error("global anki models at index [{0}] is out of bounds")]
    ModelOutOfBounds(usize),
    #[error("global anki decks at index [{0}] is out of bounds")]
    DeckOutOfBounds(usize),
    #[error("GlobalAnkiOptions.note_models_map doesn't contain a note with name: {0}")]
    ModelNotFound(String),
    #[error("GlobalAnkiOptions.deck_models_map doesn't contain a deck with name: {0}")]
    DeckNotFound(String),
    #[error("[ankimodel::{model}] attempted to index field [{index}], but model only has {fields_len} fields")]
    ModelFieldsOutOfBounds {
        model: String,
        index: usize,
        fields_len: usize,
    },
}
/// Type to cache Anki note creation options
#[derive(
    Clone, Default, Debug, PartialEq, Serialize, Deserialize, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct AnkiFields {
    /// field that maps term result info to selected model fields
    fields: Vec<AnkiTermFieldType>,
    /// selected note model create notes with
    selected_model: usize,
    /// selected deck to add notes to
    selected_deck: usize,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", set = "pub")]
pub struct AnkiOptions {
    enable: bool,
    #[default("8765".into())]
    server: String,
    tags: Vec<String>,
    screenshot: AnkiScreenshotOptions,
    global_anki_options: Ptr<GlobalAnkiOptions>,
    // Pre-mapped fields that specify which Anki field to insert term data
    anki_fields: Option<AnkiFields>,
    duplicate_scope: AnkiDuplicateScope,
    duplicate_scope_check_all_models: bool,
    duplicate_behavior: AnkiDuplicateBehavior,
    #[default(true)]
    check_for_duplicates: bool,
    field_templates: Vec<String>,
    suspend_new_cards: bool,
    display_tags: AnkiDisplayTags,
    note_gui_mode: AnkiNoteGuiMode,
    api_key: String,
    /// The maximum time _(in milliseconds)_ before an idle download will be cancelled;
    /// 0 = no limit.
    ///
    /// Audio files can be downloaded from remote servers when creating Anki cards,
    /// and sometimes these downloads can stall due to server or internet connectivity issues.
    /// When this setting has a non-zero value, if a download has stalled for longer
    /// than the time specified, the download will be cancelled.
    #[default(2000)]
    download_timeout: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct AnkiNoteOptions {
    pub deck: String,
    pub model: String,
    pub fields: AnkiNoteFields,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AnkiScreenshotOptions {
    pub format: AnkiScreenshotFormat,
    pub quality: u8,
}

pub type AnkiNoteFields = IndexMap<String, String>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SentenceParsingOptions {
    /// Adjust how many characters are bidirectionally scanned to form a sentence.
    pub scan_extent: u16,
    pub termination_character_mode: SentenceTerminationCharacterMode,
    pub termination_characters: Vec<SentenceParsingTerminationCharacterOption>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SentenceParsingTerminationCharacterOption {
    pub enabled: bool,
    pub character1: String,
    pub character2: Option<String>,
    pub include_character_at_start: bool,
    pub include_character_at_end: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InputsOptions {
    pub hotkeys: Vec<InputsHotkeyOptions>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InputsHotkeyOptions {
    pub action: String,
    pub argument: String,
    pub key: Option<String>,
    pub modifiers: Vec<InputsHotkeyModifier>,
    pub scopes: Vec<InputsHotkeyScope>,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ClipboardOptions {
    pub enable_background_monitor: bool,
    pub enable_search_page_monitor: bool,
    pub auto_search_content: bool,
    /// Limit the number of characters used when searching clipboard text.
    pub maximum_search_length: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccessibilityOptions {
    pub force_google_docs_html_rendering: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreventMiddleMouseOptions {
    pub on_web_pages: bool,
    pub on_popup_pages: bool,
    pub on_search_pages: bool,
    pub on_search_query: bool,
}

/// Change how related results are grouped.
///
/// The Primary dictionary option should be assigned to a
/// dictionary which contains related term information,
/// and configuring the Secondary dictionaries will allow definitions for the
/// related terms to be included from other dictionaries.
///
/// _Not all dictionaries are able to be selected as the Primary dictionary_.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResultOutputMode {
    /// No grouping.
    ///
    /// Every definition will be listed as a separate entry.
    Split,
    /// Group term-reading pairs.
    ///
    /// Definitions for the same term with the same reading will be grouped together.
    #[default]
    Group,
    /// Group related terms.
    ///
    /// Related terms that share the same definitions will be grouped together.
    Merge,
}

/// Change the layout of the popup.
///
/// The `Default` mode will position the popup relative to the scanned text.
/// The `Full Width` mode will anchor the popup to the top or bottom of the screen and
/// take up the full width of the screen, which can be useful on devices with touch screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupDisplayMode {
    #[default]
    Default,
    FullWidth,
}
/// Change where the popup is positioned relative to horizontal text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupHorizontalTextPosition {
    #[default]
    Below,
    Above,
}

/// Change where the popup is positioned relative to vertical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupVerticalTextPosition {
    #[default]
    Before,
    After,
    Left,
    Right,
}

/// Adjust the main style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupTheme {
    Light,
    #[default]
    Dark,
    System,
}

/// Control when the action bar is visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupActionBarVisibility {
    #[default]
    Auto,
    Always,
}

/// Control where the action bar is visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupActionBarLocation {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}

/// Adjust the shadow style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupShadow {
    /// Casts no shadow.
    Light,
    #[default]
    /// Casts a white shadow.
    Dark,
    System,
}

/// Change how the selected definition entry is visually indicated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupCurrentIndicatorMode {
    None,
    Asterisk,
    Triangle,
    #[default]
    VerticalBarLeft,
    VerticalBarRight,
    DotLeft,
    DotRight,
}

/// Change how frequency information is presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FrequencyDisplayStyle {
    Tags,
    TagsGrouped,
    SplitTags,
    #[default]
    SplitTagsGrouped,
    InlineList,
    List,
}

/// Change how terms and their readings are displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TermDisplayStyle {
    #[default]
    Ruby,
    RubyAndReading,
    TermAndReading,
    TermOnly,
}

/// Frequency sorting mode
///
/// Dictionary frequency data can be represented in one of two ways:
///
/// `Occurrence`-based, where the frequency corresponds to a number of occurrences.
/// - Large values indicate a more common term.
///
/// `Rank`-based, where the frequency value corresponds to a ranking index.
/// - Smaller values indicate a more common term.
///
/// _`Occurrence`-based frequency dictionaries are highly discouraged, do not use them!!_
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortFrequencyDictionaryOrder {
    Occurance,
    #[default]
    Rank,
}

/// Change the appearance of the window type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupWindowType {
    Normal,
    #[default]
    Popup,
}

/// Change the state of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PopupWindowState {
    #[default]
    Normal,
    Maximized,
    Fullscreen,
}

/// When searching for audio, the sources are checked in order until the first valid source is found.
/// This allows for selecting a fallback source if the first choice is not available.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AudioSourceType {
    #[default]
    Jpod101,
    Jpod101Alternate,
    Jisho,
    LinguaLibre,
    Wiktionary,
    TextToSpeech,
    TextToSpeechReading,
    Custom,
    CustomJson,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TranslationConvertType {
    #[default]
    False,
    True,
    Variant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TranslationCollapseEmphaticSequences {
    #[default]
    False,
    True,
    Full,
}

/// Customize dictionary collapsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DictionaryDefinitionsCollapsible {
    /// Definitions will not be collapsed.
    NotCollapsible,
    /// Definitions will show a collapse button if their size exceeds the max height,
    /// and they will be collapsed by default.
    #[default]
    Expanded,
    /// Definitions will show a collapse button if their size exceeds the max height,
    /// and they will be expanded by default.
    Collapsed,
    /// Definitions will always show a collapse button,
    /// and they will be collapsed by default.
    ForceCollapsed,
    ///  Definitions will always show a collapse button,
    ///  and they will be expanded by default.
    ForceExpanded,
}

/// Change what type of furigana is displayed for parsed text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ParsingReadingMode {
    #[default]
    Hiragana,
    Katakana,
    Romaji,
    DictionaryReading,
    None,
}

/// Adjust the format and quality of screenshots created for cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AnkiScreenshotFormat {
    #[default]
    Png,
    Jpeg,
}

/// A card is considered a duplicate if the value of the first field matches that of any other card.
/// By default, this check will include cards across all decks in a collection,
/// but this constraint can be relaxed by using either the Deck or Deck root option.
///
/// The Deck option will only check for duplicates in the target deck.
/// The Deck root option will additionally check for duplicates in all child decks of the root deck.
/// This allows adding cards that are unique for decks including a subdeck structure.
/// For decks which don't have any parent-child hierarchy, both options function the same.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AnkiDuplicateScope {
    #[default]
    Collection,
    Deck,
    DeckRoot,
}

/// When a duplicate is detected.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AnkiDuplicateBehavior {
    #[default]
    Prevent,
    Overwrite,
    New,
}

/// Show card tags
///
/// When coming across a word that is already in an Anki deck,
/// a button will appear that shows the tags the card has.
///
/// If set to `Non-Standard`, all tags that are included in the Card tags option will be filtered out from the list.
/// If no tags remain after filtering, then the button will not be shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AnkiDisplayTags {
    #[default]
    Never,
    Always,
    NonStandard,
}

/// Note viewer window
///
/// Clicking the View added note button shows this window.
///
/// AnkiConnect releases after around `2022-05-29` support a new note editor window-
/// which can be shown when clicking the View added note button.
/// This can be tested using the buttons below.
///
/// _If an error occurs, [Anki](https://apps.ankiweb.net/) and/or [AnkiConnect](https://ankiweb.net/shared/info/2055492159) may need to be updated_.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AnkiNoteGuiMode {
    #[default]
    CardBrowser,
    NoteEditor,
}

/// Sentence termination characters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum SentenceTerminationCharacterMode {
    #[default]
    Custom,
    CustomNoNewlines,
    Newlines,
    None,
}

/// Hold a key while moving the cursor to scan text.
///
/// A keyboard modifier key can be used to activate text scanning when the cursor is moved.
/// Alternatively, the `No Key` option can be used
/// to scan text whenever the cursor is moved, without requiring any key to be held.
///
/// More advanced scanning input customization can be set up
/// by enabling the `Advanced` option and clicking `Configure Advanced Scanning Inputs`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum InputsHotkeyModifier {
    NoKey,
    Alt,
    Ctrl,
    #[default]
    Shift,
    Meta,
}

// I think this is maybe `Configure input action prevention` in the settings...?
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum InputsHotkeyScope {
    #[default]
    Popup,
    Search,
    Web,
}
