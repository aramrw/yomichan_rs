use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

#[cfg(feature = "anki")]
use crate::anki::DisplayAnki;
use crate::backend::Backend;
use crate::Ptr;
use crate::PtrRGaurd;
use crate::PtrWGaurd;
use crate::Yomichan;
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
use parking_lot::{ArcRwLockReadGuard, ArcRwLockUpgradableReadGuard, RawRwLock, RwLock};
use serde::{Deserialize, Serialize};
use serde_with::SerializeDisplay;
use url::form_urlencoded::Target;

use importer::dictionary_importer::DictionarySummary;
use crate::{translation::FindTermsSortOrder, translator::FindTermsMode,};

/// Global application-level options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalOptions {
    /// Database-related global options.
    pub database: GlobalDatabaseOptions,
}

/// Global database-related options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalDatabaseOptions {
    /// Whether prefix wildcards are supported in database queries.
    pub prefix_wildcards_supported: bool,
}

/// Global Yomichan Settings.
///
/// This struct holds all global configuration options for the Yomichan application,
/// including user profiles, current profile selection, and global Anki settings.
#[native_model(id = 20, version = 1, with = native_model::rmp_serde_1_3::RmpSerdeNamed)]
#[native_db]
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, MutGetters, Setters,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct YomichanOptions {
    #[primary_key]
    id: String,
    /// The version of the Yomichan application.
    pub version: String,
    /// A map of user-defined profiles, where each profile contains specific settings.
    /// The key is the profile name, and the value is a pointer to the `YomichanProfile`.
    pub profiles: IndexMap<String, Ptr<YomichanProfile>>,
    /// The index of the currently selected profile in the `profiles` map.
    pub current_profile: usize,
    /// Global application-level options.
    pub global: GlobalOptions,
    /// Global Anki-related options, shared across all profiles.
    pub anki: Ptr<GlobalAnkiOptions>,
}

impl Yomichan<'_> {
    /// Returns a pointer to the global `YomichanOptions`.
    pub fn options(&self) -> Ptr<YomichanOptions> {
        self.backend.options.clone()
    }
}

impl YomichanOptions {
    /// Creates a new `YomichanOptions` instance with default settings.
    ///
    /// This includes a default profile named "Default" and default global Anki options.
    pub fn new() -> Self {
        let global_anki_options = Ptr::new(GlobalAnkiOptions::default());
        Self {
            id: "global_user_options".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            profiles: IndexMap::from_iter([(
                "Default".into(),
                Ptr::new(YomichanProfile::new(
                    "Default".to_string(),
                    &global_anki_options,
                )),
            )]),
            current_profile: 0,
            global: GlobalOptions::default(),
            anki: global_anki_options,
        }
    }

    /// Gets a [Ptr] to the currently selected profile by using [YomichanOptions::current_profile].
    ///
    /// # Returns
    /// - `Ok(Ptr<YomichanProfile>)` if the profile is found.
    /// - `Err(ProfileError::SelectedOutofBounds)` if the index is out of bounds.
    pub fn get_current_profile(&self) -> ProfileResult<Ptr<YomichanProfile>> {
        let Some((name, pf)) = self.profiles.get_index(self.current_profile) else {
            return Err(ProfileError::SelectedOutofBounds {
                selected: *self.current_profile(),
                len: self.profiles.len(),
            });
        };
        Ok(pf.clone())
    }

    /// Gets a [Ptr] to a [YomichanProfile] by its name.
    ///
    /// # Arguments
    /// * `name` - The name of the profile to find.
    ///
    /// # Returns
    /// - `Ok(Ptr<YomichanProfile>)` if the profile is found.
    /// - `Err(ProfileError::ProfileNotFound)` if the profile does not exist.
    pub fn find_profile_by_name(&self, name: &str) -> ProfileResult<Ptr<YomichanProfile>> {
        let Some(ptr) = self.profiles.get(name) else {
            return Err(ProfileError::ProfileNotFound {
                find: name.into(),
                available: self.profiles().keys().cloned().collect::<Vec<String>>(),
            });
        };
        Ok(ptr.clone())
    }

    /// Sets the currently selected [YomichanProfile] via its name and returns it.
    ///
    /// # Arguments
    /// * `name` - The name of the profile to set as current.
    ///
    /// # Returns
    /// - `Ok((usize, &str, Ptr<YomichanProfile>))` containing the index, name, and pointer to the profile.
    /// - `Err(ProfileError::ProfileNotFound)` if the profile does not exist.
    pub fn get_profile_by_name_full(
        &self,
        name: &str,
    ) -> ProfileResult<(usize, &str, Ptr<YomichanProfile>)> {
        let ((i, k, ptr)) = {
            let profiles = self.profiles();
            let Some((i, k, ptr)) = profiles.get_full(name) else {
                return Err(ProfileError::ProfileNotFound {
                    find: name.into(),
                    available: self.profiles().keys().cloned().collect::<Vec<String>>(),
                });
            };
            (i, k, ptr)
        };
        Ok((i, k.as_str(), ptr.clone()))
    }

    /// Creates a new profile with the given name, copying settings from the "Default" profile.
    ///
    /// # Arguments
    /// * `name` - The name for the new profile.
    ///
    /// # Returns
    /// - `Ok(())` if the profile was created successfully.
    /// - `Err(ProfileError::ProfileAlreadyExists)` if a profile with the given name already exists.
    pub fn create_new_profile(&mut self, name: &str) -> ProfileResult<()> {
        if self.profiles.contains_key(name) {
            return Err(ProfileError::ProfileAlreadyExists {
                name: name.to_string(),
            });
        }

        // Get a read lock on the default profile to copy its contents
        let default_profile_data = self
            .profiles
            .get("Default")
            .map(|ptr| ptr.read_arc().clone())
            .unwrap_or_else(|| YomichanProfile::new_default(&self.anki));

        let mut new_profile_data = default_profile_data;
        new_profile_data.name = name.to_string();

        // Create a new Ptr (Arc<RwLock<YomichanProfile>>) for the new profile
        let new_profile_ptr = Ptr::new(new_profile_data);

        self.profiles.insert(name.to_string(), new_profile_ptr);
        Ok(())
    }

    /// Creates a new profile with an auto generated name (uuid v7)
    /// Returns the name of the new profile
    pub fn create_new_profile_auto(&mut self) -> ProfileResult<String> {
        let name = uuid::Uuid::now_v7()
            .to_string()
            .chars()
            .take(7)
            .collect::<String>();
        self.create_new_profile(&name);
        Ok(name)
    }
}

impl Backend<'_> {
    /// Retrieves the currently selected `YomichanProfile`.
    ///
    /// # Returns
    /// - `Ok(Ptr<YomichanProfile>)` if the profile is successfully retrieved.
    /// - `Err(ProfileError)` if there is an issue accessing or finding the profile.
    pub fn get_current_profile(&self) -> ProfileResult<Ptr<YomichanProfile>> {
        let opts = self.options.read_arc();
        opts.get_current_profile()
    }
}

/// A type alias for results that may return a `ProfileError`.
pub type ProfileResult<T> = Result<T, ProfileError>;

/// Errors that can occur when managing or accessing `YomichanProfile`s.
#[derive(thiserror::Error, Debug)]
pub enum ProfileError {
    /// Indicates that the selected profile index is out of bounds.
    #[error("tried to index selected_profile[selected], but profiles.len() = {len}")]
    SelectedOutofBounds { selected: usize, len: usize },
    /// Indicates that a profile with the specified name was not found.
    #[error("[profile-not-found]: {find}; [available]: {available:?}")]
    ProfileNotFound {
        find: String,
        available: Vec<String>,
    },
    /// Indicates that a profile with the given name already exists.
    #[error("profile already exists: {name}")]
    ProfileAlreadyExists { name: String },
}

/// Represents a user-configurable profile within Yomichan.
///
/// Profiles allow users to customize various settings, such as dictionary preferences,
/// scanning behavior, and Anki integration, and switch between them easily.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, MutGetters)]
pub struct YomichanProfile {
    /// The name of the profile.
    pub name: String,
    /// A list of conditions that determine when this profile should be automatically activated.
    pub condition_groups: Vec<ProfileConditionGroup>,
    /// The specific options configured for this profile.
    #[getset(get = "pub", get_mut = "pub")]
    pub options: ProfileOptions,
}

impl YomichanProfile {
    /// Creates a new `YomichanProfile` with default settings, linked to global Anki options.
    ///
    /// The profile name will be a randomly generated UUID prefix.
    pub fn new_default(global_anki_options: &Ptr<GlobalAnkiOptions>) -> Self {
        let mut default = Self::default();
        default.name = uuid::Uuid::new_v4().to_string().chars().take(5).collect();
        default.options = ProfileOptions::new_default(global_anki_options.clone());
        default
    }

    /// Creates a new `YomichanProfile` with a specified name and default options.
    ///
    /// # Arguments
    /// * `name` - The name for the new profile.
    /// * `global_anki_options` - A pointer to the global Anki options to associate with this profile.
    pub fn new(name: String, global_anki_options: &Ptr<GlobalAnkiOptions>) -> Self {
        Self {
            name,
            options: ProfileOptions::new_default(global_anki_options.clone()),
            ..Default::default()
        }
    }

    /// Sets the language for this profile.
    ///
    /// # Arguments
    /// * `iso` - The ISO 639-1 code for the language (e.g., "ja" for Japanese, "en" for English).
    pub fn set_language(&mut self, iso: &str) {
        self.options.general.language = iso.to_string();
    }

    /// Extends the profile's dictionary options with new entries.
    ///
    /// # Arguments
    /// * `new` - An iterator of `(String, DictionaryOptions)` pairs to add or update.
    pub fn extend_dictionaries(
        &mut self,
        new: impl IntoIterator<Item = (String, DictionaryOptions)>,
    ) {
        self.options.dictionaries.extend(new);
    }

    /// Retrieves the `DictionaryOptions` for a specific dictionary by its name.
    ///
    /// # Arguments
    /// * `find` - The name of the dictionary to find.
    ///
    /// # Returns
    /// An `Option` containing a reference to `DictionaryOptions` if found, otherwise `None`.
    pub fn get_dictionary_options_from_name(&self, find: &str) -> Option<&DictionaryOptions> {
        self.options.dictionaries.get(find)
    }

    /// Returns a reference to the Anki options for this profile.
    pub fn anki_options(&self) -> &AnkiOptions {
        &self.options().anki
    }

    /// Returns a mutable reference to the Anki options for this profile.
    pub fn anki_options_mut(&mut self) -> &mut AnkiOptions {
        &mut self.options_mut().anki
    }
}

/// A group of conditions that must all be met for a profile to be activated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileConditionGroup {
    /// The list of individual conditions within this group.
    pub conditions: Vec<ProfileCondition>,
}

/// Defines the type of condition used for profile activation.
///
/// Profile usage conditions are used to automatically select certain profiles based on context.
/// For example, different profiles can be used, depending on the nested level of the popup,
/// or based on the website's URL. Conditions are organized into groups corresponding to the
/// order in which they are checked. If all of the conditions in any group of a profile are met,
/// then that profile will be used for that context. If no conditions are specified, the profile
/// will only be used if it is selected as the default profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum ProfileConditionType {
    #[default]
    PopupLevel,
    Url,
    ModifierKeys,
    Flags,
}

/// A single condition used to determine if a profile should be activated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileCondition {
    /// The type of condition to evaluate.
    pub condition_type: ProfileConditionType,
    /// The operator to use for evaluation (e.g., "equals", "contains").
    pub operator: String,
    /// The value to compare against.
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
#[derive(Clone, Debug, Serialize, PartialEq, Deserialize, Default, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
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

/// General application settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct GeneralOptions {
    /// Whether the extension is enabled.
    pub enable: bool,
    /// The primary language for text processing (e.g., "ja" for Japanese).
    pub language: String,
    /// How search results are grouped and displayed.
    pub result_output_mode: FindTermsMode,
    /// Whether prefix wildcards are supported in search queries.
    pub prefix_wildcard_supported: bool,
    /// Whether to show debug information.
    pub debug_info: bool,
    /// The maximum number of results to display.
    pub max_results: u8,
    /// Whether to show advanced settings.
    pub show_advanced: bool,
    /// The font family to use for displaying text.
    pub font_family: String,
    /// The font size for displaying text.
    pub font_size: u8,
    /// The line height for displaying text.
    pub line_height: String,
    /// The display mode for the popup window.
    pub popup_display_mode: PopupDisplayMode,
    /// The width of the popup window.
    pub popup_width: u8,
    /// The height of the popup window.
    pub popup_height: u8,
    /// The horizontal offset of the popup window.
    pub popup_horizontal_offset: u8,
    /// The vertical offset of the popup window.
    pub popup_vertical_offset: u8,
    /// The second horizontal offset of the popup window.
    pub popup_horizontal_offset2: u8,
    /// The second vertical offset of the popup window.
    pub popup_vertical_offset2: u8,
    /// The horizontal text position within the popup.
    pub popup_horizontal_text_position: PopupHorizontalTextPosition,
    /// The vertical text position within the popup.
    pub popup_vertical_text_position: PopupVerticalTextPosition,
    /// The scaling factor for the popup window.
    pub popup_scaling_factor: u8,
    /// Whether the popup scales relative to page zoom.
    pub popup_scale_relative_to_page_zoom: bool,
    /// Whether the popup scales relative to the visual viewport.
    pub popup_scale_relative_to_visual_viewport: bool,
    /// Whether to show a guide.
    pub show_guide: bool,
    /// Whether to enable context menu scan selected.
    pub enable_context_menu_scan_selected: bool,
    /// Whether to compact tags.
    pub compact_tags: bool,
    /// Whether to compact glossaries.
    pub compact_glossaries: bool,
    /// The name of the main dictionary to use.
    pub main_dictionary: String,
    /// The theme of the popup.
    pub popup_theme: PopupTheme,
    /// The outer theme of the popup.
    pub popup_outer_theme: PopupShadow,
    /// Custom CSS for the popup.
    pub custom_popup_css: String,
    /// Custom outer CSS for the popup.
    pub custom_popup_outer_css: String,
    /// Whether to enable Wanakana (Japanese text conversion).
    pub enable_wanakana: bool,
    /// Whether to show pitch accent downstep notation.
    pub show_pitch_accent_downstep_notation: bool,
    /// Whether to show pitch accent position notation.
    pub show_pitch_accent_position_notation: bool,
    /// Whether to show the pitch accent graph.
    pub show_pitch_accent_graph: bool,
    /// Whether to show iframe popups in the root frame.
    pub show_iframe_popups_in_root_frame: bool,
    /// Whether to use a secure popup frame URL.
    pub use_secure_popup_frame_url: bool,
    /// Whether to use popup shadow DOM.
    pub use_popup_shadow_dom: bool,
    /// Whether to use a popup window.
    pub use_popup_window: bool,
    /// The current indicator mode for the popup.
    pub popup_current_indicator_mode: PopupCurrentIndicatorMode,
    /// The visibility of the popup action bar.
    pub popup_action_bar_visibility: PopupActionBarVisibility,
    /// The location of the popup action bar.
    pub popup_action_bar_location: PopupActionBarLocation,
    /// The display style for frequency information.
    pub frequency_display_mode: FrequencyDisplayStyle,
    /// The display style for terms.
    pub term_display_mode: TermDisplayStyle,
    /// The dictionary to sort frequency by.
    pub sort_frequency_dictionary: Option<String>,
    /// The order to sort frequency by.
    pub sort_frequency_dictionary_order: FindTermsSortOrder,
    /// Whether the search header is sticky.
    pub sticky_search_header: bool,
}

/// Options for the popup window behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PopupWindowOptions {
    /// The width of the popup window.
    pub width: u8,
    /// The height of the popup window.
    pub height: u8,
    /// The left position of the popup window.
    pub left: u8,
    /// The top position of the popup window.
    pub top: u8,
    /// Whether to use the left position.
    pub use_left: bool,
    /// Whether to use the top position.
    pub use_top: bool,
    /// The type of popup window.
    pub window_type: PopupWindowType,
    /// The state of the popup window.
    pub window_state: PopupWindowState,
}

/// Options for audio playback.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioOptions {
    /// Whether audio playback is enabled.
    pub enabled: bool,
    /// The volume for audio playback (0-100).
    pub volume: u8,
    /// Whether audio should play automatically.
    pub auto_play: bool,
    /// A list of audio sources to use, in order of preference.
    pub sources: Vec<AudioSourceOptions>,
}

/// Options for a single audio source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioSourceOptions {
    /// The type of audio source (e.g., Jpod101, TextToSpeech).
    pub audio_source_type: AudioSourceType,
    /// The URL for custom audio sources.
    pub url: String,
    /// The voice to use for text-to-speech sources.
    pub voice: String,
}

/// Options for text scanning behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningOptions {
    /// A list of scanning input configurations.
    pub inputs: Vec<ScanningInput>,
    /// Options for preventing middle mouse button actions.
    pub prevent_middle_mouse: ScanningPreventMiddleMouseOptions,
    /// Whether touch input scanning is enabled.
    pub touch_input_enabled: bool,
    /// Whether pointer events scanning is enabled.
    pub pointer_events_enabled: bool,
    /// Whether to select text during scanning.
    pub select_text: bool,
    /// Whether to scan alphanumeric characters.
    pub alphanumeric: bool,
    /// Whether to automatically hide results after a delay.
    pub auto_hide_results: bool,
    /// The delay before scanning (in milliseconds).
    pub delay: u8,
    /// The delay before hiding results (in milliseconds).
    pub hide_delay: u8,
    /// The maximum length of text to scan.
    pub length: u8,
    /// Whether to perform a deep DOM scan.
    pub deep_dom_scan: bool,
    /// The maximum nesting depth for popup windows.
    pub popup_nesting_max_depth: u8,
    /// Whether to enable popup search.
    pub enable_popup_search: bool,
    /// Whether to enable scanning on popup expressions.
    pub enable_on_popup_expressions: bool,
    /// Whether to enable scanning on search pages.
    pub enable_on_search_page: bool,
    /// Whether to enable search tags.
    pub enable_search_tags: bool,
    /// Whether to use layout-aware scanning.
    pub layout_aware_scan: bool,
    /// Whether to use prefix matching for terms.
    pub match_type_prefix: bool,
    /// Whether to hide the popup when the cursor exits.
    pub hide_popup_on_cursor_exit: bool,
    /// The delay before hiding the popup on cursor exit (in milliseconds).
    pub hide_popup_on_cursor_exit_delay: u8,
    /// Whether to normalize CSS zoom.
    pub normalize_css_zoom: bool,
    /// Whether to scan alt text.
    pub scan_alt_text: bool,
}

/// Configuration for a single scanning input.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInput {
    /// Include pattern for scanning.
    pub include: String,
    /// Exclude pattern for scanning.
    pub exclude: String,
    /// Types of input to consider for scanning.
    pub types: ScanningInputTypes,
    /// Additional options for this scanning input.
    pub options: ScanningInputOptions,
}

/// Specifies which input types trigger scanning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInputTypes {
    /// Whether mouse input triggers scanning.
    pub mouse: bool,
    /// Whether touch input triggers scanning.
    pub touch: bool,
    /// Whether pen input triggers scanning.
    pub pen: bool,
}

/// Additional options for scanning inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningInputOptions {
    /// Whether to show advanced options for this input.
    pub show_advanced: bool,
    /// Whether to search for terms.
    pub search_terms: bool,
    /// Whether to search for kanji.
    pub search_kanji: bool,
    /// Whether to scan on touch move events.
    pub scan_on_touch_move: bool,
    /// Whether to scan on touch press events.
    pub scan_on_touch_press: bool,
    /// Whether to scan on touch release events.
    pub scan_on_touch_release: bool,
    /// Whether to scan on touch tap events.
    pub scan_on_touch_tap: bool,
    /// Whether to scan on pen move events.
    pub scan_on_pen_move: bool,
    /// Whether to scan on pen hover events.
    pub scan_on_pen_hover: bool,
    /// Whether to scan on pen release hover events.
    pub scan_on_pen_release_hover: bool,
    /// Whether to scan on pen press events.
    pub scan_on_pen_press: bool,
    /// Whether to scan on pen release events.
    pub scan_on_pen_release: bool,
    /// Whether to prevent touch scrolling.
    pub prevent_touch_scrolling: bool,
    /// Whether to prevent pen scrolling.
    pub prevent_pen_scrolling: bool,
}

/// Options for preventing middle mouse button actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScanningPreventMiddleMouseOptions {
    /// Prevent on web pages.
    pub on_web_pages: bool,
    /// Prevent on popup pages.
    pub on_popup_pages: bool,
    /// Prevent on search pages.
    pub on_search_pages: bool,
    /// Prevent on search query.
    pub on_search_query: bool,
}

/// Options for text translation and transformation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationOptions {
    /// Whether to convert half-width characters.
    pub convert_half_width_characters: TranslationConvertType,
    /// Whether to convert numeric characters.
    pub convert_numeric_characters: TranslationConvertType,
    /// Whether to convert alphabetic characters to hiragana.
    pub alphabetic_to_hiragana: TranslationConvertType,
    /// Whether to convert hiragana to katakana.
    pub convert_hiragana_to_katakana: TranslationConvertType,
    /// Whether to convert katakana to hiragana.
    pub convert_katakana_to_hiragana: TranslationConvertType,
    /// Whether to collapse emphatic sequences.
    pub collapse_emphatic_sequences: TranslationCollapseEmphaticSequences,
    /// Options for text replacements.
    pub text_replacements: TranslationTextReplacementOptions,
    /// The resolution for search (Letter or Word).
    pub search_resolution: SearchResolution,
}

/// Defines the resolution for text search.
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

/// Options for text replacements during translation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationTextReplacementOptions {
    /// Whether to search the original text.
    pub search_original: bool,
    /// Groups of text replacement rules.
    pub groups: Vec<Vec<TranslationTextReplacementGroup>>,
}

/// A single text replacement rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TranslationTextReplacementGroup {
    /// The pattern to search for.
    pub pattern: String,
    /// Whether the search should ignore case.
    pub ignore_case: bool,
    /// The replacement string.
    pub replacement: String,
}

/// Options specific to a single dictionary.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DictionaryOptions {
    /// The title of the dictionary.
    pub name: String,
    /// An alias for the dictionary, used for display purposes.
    pub alias: String,
    /// Whether or not the dictionary is enabled for lookups.
    pub enabled: bool,
    /// If `true`, definitions from this dictionary will be included in secondary searches
    /// when merging results from multiple dictionaries.
    ///
    /// # Merged Mode Example
    /// If you have two dictionaries, `Dict 1` and `Dict 2`:
    /// - Set the [`ResultOutputMode`] to `Group` results for the main dictionary entry.
    /// - Choose `Dict 1` as the main dictionary for merged mode.
    /// - Enable `allow_secondary_searches` on `Dict 2`.
    ///   (Can be enabled for multiple dictionaries).
    ///
    /// Yomichan_rs will now first perform an _initial_ lookup in `Dict 1`, fetching the grouped definition.
    /// It will then use the headwords from `Dict 1` to perform a secondary lookup in `Dict 2`,
    /// merging the two dictionary's definitions.
    pub allow_secondary_searches: bool,
    /// Controls how definitions from this dictionary are collapsed or expanded.
    ///
    /// Dictionary definitions can be collapsed if they exceed a certain line count,
    /// which may be useful for dictionaries with long definitions. There are five different modes:
    ///
    /// By default, the number of lines shown for a definition is 3.
    /// This can be configured by adjusting the Custom CSS `styles`;
    ///
    /// (Value can be a unitless integer or decimal number).
    pub definitions_collapsible: DictionaryDefinitionsCollapsible,
    /// When deinflecting words, only dictionary entries whose part-of-speech
    /// matches that expected by the deinflector will be shown.
    pub parts_of_speech_filter: bool,
    /// Whether deinflections from this dictionary will be used.
    pub use_deinflections: bool,
    /// Custom CSS styles to apply to the dictionary's definitions in the popup.
    ///
    /// # Example
    /// ```css
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
    /// Creates a new `DictionaryOptions` instance with default values for a given dictionary name.
    pub fn new(dict_name: String) -> Self {
        DictionaryOptions {
            name: dict_name.clone(),
            alias: dict_name,
            enabled: true,
            allow_secondary_searches: false,
            definitions_collapsible: DictionaryDefinitionsCollapsible::Expanded,
            parts_of_speech_filter: false,
            use_deinflections: true,
            styles: None,
        }
    }
}

/// Options for text parsing and tokenization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ParsingOptions {
    /// Whether to enable the scanning parser.
    pub enable_scanning_parser: bool,
    /// Whether to enable the MeCab parser.
    pub enable_mecab_parser: bool,
    /// The currently selected parser (e.g., "MeCab", "Scanning").
    pub selected_parser: Option<String>,
    /// Whether to add spacing between terms.
    pub term_spacing: bool,
    /// The reading mode for parsed text (e.g., Hiragana, Katakana).
    pub reading_mode: ParsingReadingMode,
}

/// A map for caching discovered Anki decks.
///
/// This struct stores a collection of `DeckConfig` objects, indexed by their names,
/// and keeps track of the currently selected deck.
#[derive(
    Clone, Serialize, Deserialize, Debug, PartialEq, Default, Setters, Getters, MutGetters,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct DecksMap {
    /// The underlying map storing deck configurations.
    map: IndexMap<String, Option<DeckConfig>>,
    /// The index of the currently selected deck within the map.
    selected: usize,
}

impl From<IndexMap<String, Option<DeckConfig>>> for DecksMap {
    fn from(map: IndexMap<String, Option<DeckConfig>>) -> Self {
        Self { map, selected: 0 }
    }
}

impl DecksMap {
    /// Returns the currently selected `DeckConfig`.
    ///
    /// # Returns
    /// An `Option` containing a tuple of `(index, name, &DeckConfig)` if a deck is selected,
    /// otherwise `None`.
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

/// Represents a specific field type within an Anki note.
#[derive(Clone, Copy, Debug)]
pub enum AnkiField {
    Term,
    Reading,
    Sentence,
    Definition,
    TermAudio,
    SentenceAudio,
    Image,
    Frequency,
}

/// Represents an index mapping for an Anki field.
#[derive(Clone, Copy, Debug)]
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

/// Defines Anki field types that correspond to a [DictionaryTermEntry] for making notes.
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
    /// * `Err(AnkiFieldsError)`: An error indicating an invalid index or other issue.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use anki_direct::model::FullModelDetails;
    /// use yomichan_rs::settings::{AnkiTermFieldType, FieldIndex};
    ///
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
    ///     AnkiTermFieldType::Definition("Meaning".to_string()),
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

/// A map for caching discovered Anki note models.
///
/// This struct stores a collection of `FullModelDetails` objects, indexed by their names,
/// and keeps track of the currently selected model.
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
    /// The underlying map storing note model details.
    #[deref]
    #[deref_mut]
    map: IndexMap<String, FullModelDetails>,
    /// The index of the currently selected note model within the map.
    selected: usize,
}

impl NoteModelsMap {
    /// Moves the inner `IndexMap` out of itself and returns it.
    pub fn take_map(self) -> IndexMap<String, FullModelDetails> {
        self.map
    }

    #[deprecated(
        note = "use each YomichanProfile's AnkiOptions to select from the GlobalAnkiOptions maps"
    )]
    /// Returns the currently selected `NoteModelNode`.
    pub fn selected_note(&self) -> Option<NoteModelNode<'_>> {
        let i = self.selected;
        let (name, details) = self.map.get_index(i)?;
        let node: NoteModelNode<'_> = (i, name.as_str(), details).into();
        Some(node)
    }
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

/// A node representing a selected Anki note model.
#[derive(Clone, Debug, PartialEq, Getters, Setters, MutGetters, DerefMut, Deref)]
pub struct NoteModelNode<'n>((usize, &'n str, &'n FullModelDetails));

/// Struct for global Anki options that caches note models and decks from AnkiConnect.
#[derive(
    Clone, Debug, Getters, Setters, MutGetters, Serialize, Deserialize, PartialEq, Default,
)]
#[getset(get = "pub", get_mut = "pub")]
pub struct GlobalAnkiOptions {
    /// A map of available Anki note models.
    note_models_map: NoteModelsMap,
    /// A map of available Anki decks.
    decks_map: DecksMap,
}

impl GlobalAnkiOptions {
    /// Retrieves the selected note model by its index.
    ///
    /// # Arguments
    /// * `i` - The index of the model to retrieve.
    ///
    /// # Returns
    /// - `Ok((&str, &FullModelDetails))` containing the model's name and details.
    /// - `Err(AnkiFieldsError::ModelOutOfBounds)` if the index is out of bounds.
    pub fn get_selected_model(
        &self,
        i: usize,
    ) -> Result<(&str, &FullModelDetails), AnkiFieldsError> {
        let map = self.note_models_map();
        map.get_index(i)
            .map(|(k, v)| (k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelOutOfBounds(i))
    }

    /// Retrieves the selected deck by its index.
    ///
    /// # Arguments
    /// * `i` - The index of the deck to retrieve.
    ///
    /// # Returns
    /// - `Ok((&str, &Option<DeckConfig>))` containing the deck's name and configuration.
    /// - `Err(AnkiFieldsError::DeckOutOfBounds)` if the index is out of bounds.
    pub fn get_selected_deck(
        &self,
        i: usize,
    ) -> Result<(&str, &Option<DeckConfig>), AnkiFieldsError> {
        let map = self.decks_map();
        map.get_index(i)
            .map(|(k, v)| (k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelOutOfBounds(i))
    }

    /// Finds a note model by its name.
    ///
    /// # Arguments
    /// * `name` - The name of the model to find.
    ///
    /// # Returns
    /// - `Ok((usize, &str, &FullModelDetails))` containing the model's index, name, and details.
    /// - `Err(AnkiFieldsError::ModelNotFound)` if the model is not found.
    pub fn find_model_by_name(
        &self,
        name: &str,
    ) -> Result<(usize, &str, &FullModelDetails), AnkiFieldsError> {
        let map = self.note_models_map();
        map.get_full(name)
            .map(|(i, k, v)| (i, k.as_str(), v))
            .ok_or(AnkiFieldsError::ModelNotFound(name.to_string()))
    }

    /// Finds a deck by its name.
    ///
    /// # Arguments
    /// * `name` - The name of the deck to find.
    ///
    /// # Returns
    /// - `Ok((usize, &str, &Option<DeckConfig>))` containing the deck's index, name, and configuration.
    /// - `Err(AnkiFieldsError::DeckNotFound)` if the deck is not found.
    pub fn find_deck_by_name(
        &self,
        name: &str,
    ) -> Result<(usize, &str, &Option<DeckConfig>), AnkiFieldsError> {
        let map = self.decks_map();
        map.get_full(name)
            .map(|(i, k, v)| (i, k.as_str(), v))
            .ok_or(AnkiFieldsError::DeckNotFound(name.to_string()))
    }
}

/// Errors that can occur when interacting with Anki fields or models.
#[derive(thiserror::Error, Debug)]
pub enum AnkiFieldsError {
    /// Indicates that Anki fields are uninitialized.
    #[error("unitialized yomichan anki fields")]
    Uninitialized,
    /// Indicates that a global Anki model index is out of bounds.
    #[error("global anki models at index [{0}] is out of bounds")]
    ModelOutOfBounds(usize),
    /// Indicates that a global Anki deck index is out of bounds.
    #[error("global anki decks at index [{0}] is out of bounds")]
    DeckOutOfBounds(usize),
    /// Indicates that a note model with the specified name was not found.
    #[error("GlobalAnkiOptions.note_models_map doesn't contain a note with name: {0}")]
    ModelNotFound(String),
    /// Indicates that a deck with the specified name was not found.
    #[error("GlobalAnkiOptions.decks_map doesn't contain a deck with name: {0}")]
    DeckNotFound(String),
    /// Indicates that an attempted field index is out of bounds for a given Anki model.
    #[error("[ankimodel::{model}] attempted to index field [{index}], but model only has {fields_len} fields")]
    ModelFieldsOutOfBounds {
        model: String,
        index: usize,
        fields_len: usize,
    },
}

/// Type to cache Anki note creation options.
#[derive(
    Clone, Default, Debug, PartialEq, Serialize, Deserialize, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct AnkiFields {
    /// A vector of `AnkiTermFieldType` that maps term result information to selected model fields.
    fields: Vec<AnkiTermFieldType>,
    /// The index of the selected note model to use when creating notes.
    selected_model: usize,
    /// The index of the selected deck to add notes to.
    selected_deck: usize,
}

/// Options for Anki integration.
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", set = "pub")]
pub struct AnkiOptions {
    /// Whether Anki integration is enabled.
    enable: bool,
    /// The AnkiConnect server address.
    #[default("8765".into())]
    server: String,
    /// Tags to apply to new Anki cards.
    tags: Vec<String>,
    /// Options for taking screenshots for Anki cards.
    screenshot: AnkiScreenshotOptions,
    /// A pointer to global Anki options, shared across profiles.
    global_anki_options: Ptr<GlobalAnkiOptions>,
    /// Pre-mapped fields that specify which Anki field to insert term data.
    #[getset(get_mut = "pub")]
    anki_fields: Option<AnkiFields>,
    /// The scope for duplicate checking.
    duplicate_scope: AnkiDuplicateScope,
    /// Whether to check all models for duplicates.
    duplicate_scope_check_all_models: bool,
    /// The behavior when a duplicate is detected.
    duplicate_behavior: AnkiDuplicateBehavior,
    /// Whether to check for duplicates before adding new cards.
    #[default(true)]
    check_for_duplicates: bool,
    /// Templates for Anki fields.
    field_templates: Vec<String>,
    /// Whether to suspend new cards.
    suspend_new_cards: bool,
    /// Options for displaying Anki tags.
    display_tags: AnkiDisplayTags,
    /// The GUI mode for the Anki note viewer window.
    note_gui_mode: AnkiNoteGuiMode,
    /// The API key for AnkiConnect.
    api_key: String,
    /// The maximum time (in milliseconds) before an idle download will be cancelled; 0 = no limit.
    ///
    /// Audio files can be downloaded from remote servers when creating Anki cards,
    /// and sometimes these downloads can stall due to server or internet connectivity issues.
    /// When this setting has a non-zero value, if a download has stalled for longer
    /// than the time specified, the download will be cancelled.
    #[default(2000)]
    download_timeout: u32,
}

/// Options for creating an Anki note.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct AnkiNoteOptions {
    /// The name of the Anki deck to add the note to.
    pub deck: String,
    /// The name of the Anki note model to use.
    pub model: String,
    /// A map of field names to their corresponding values for the Anki note.
    pub fields: AnkiNoteFields,
}

/// Options for taking screenshots for Anki cards.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AnkiScreenshotOptions {
    /// The format of the screenshot (PNG or JPEG).
    pub format: AnkiScreenshotFormat,
    /// The quality of the screenshot (0-100).
    pub quality: u8,
}

/// A type alias for an `IndexMap` representing Anki note fields.
pub type AnkiNoteFields = IndexMap<String, String>;

/// Options for sentence parsing behavior.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SentenceParsingOptions {
    /// Adjust how many characters are bidirectionally scanned to form a sentence.
    pub scan_extent: u16,
    /// The mode for determining sentence termination characters.
    pub termination_character_mode: SentenceTerminationCharacterMode,
    /// A list of custom sentence termination characters.
    pub termination_characters: Vec<SentenceParsingTerminationCharacterOption>,
}

/// Options for a single sentence termination character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SentenceParsingTerminationCharacterOption {
    /// Whether this termination character is enabled.
    pub enabled: bool,
    /// The primary termination character.
    pub character1: String,
    /// An optional secondary termination character.
    pub character2: Option<String>,
    /// Whether to include the character at the start of the sentence.
    pub include_character_at_start: bool,
    /// Whether to include the character at the end of the sentence.
    pub include_character_at_end: bool,
}

/// Options for input hotkeys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InputsOptions {
    /// A list of configured hotkeys.
    pub hotkeys: Vec<InputsHotkeyOptions>,
}

/// Options for a single input hotkey.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct InputsHotkeyOptions {
    /// The action performed by the hotkey.
    pub action: String,
    /// An argument associated with the action.
    pub argument: String,
    /// The key pressed for the hotkey.
    pub key: Option<String>,
    /// Modifier keys (e.g., Alt, Ctrl, Shift) that must be held.
    pub modifiers: Vec<InputsHotkeyModifier>,
    /// The scopes in which the hotkey is active.
    pub scopes: Vec<InputsHotkeyScope>,
    /// Whether the hotkey is enabled.
    pub enabled: bool,
}

/// Options for clipboard monitoring and searching.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ClipboardOptions {
    /// Whether background clipboard monitoring is enabled.
    pub enable_background_monitor: bool,
    /// Whether clipboard monitoring on the search page is enabled.
    pub enable_search_page_monitor: bool,
    /// Whether to automatically search content from the clipboard.
    pub auto_search_content: bool,
    /// Limit the number of characters used when searching clipboard text.
    pub maximum_search_length: u16,
}

/// Options for accessibility features.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccessibilityOptions {
    /// Whether to force Google Docs HTML rendering.
    pub force_google_docs_html_rendering: bool,
}

/// Options for preventing middle mouse button actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreventMiddleMouseOptions {
    /// Prevent on web pages.
    pub on_web_pages: bool,
    /// Prevent on popup pages.
    pub on_popup_pages: bool,
    /// Prevent on search pages.
    pub on_search_pages: bool,
    /// Prevent on search query.
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

/// Frequency sorting mode.
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

/// Defines how characters are converted during translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TranslationConvertType {
    #[default]
    False,
    True,
    Variant,
}

/// Defines how emphatic sequences are collapsed during translation.
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

/// Show card tags.
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

/// Note viewer window.
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

/// Defines the scope in which an input hotkey is active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum InputsHotkeyScope {
    #[default]
    Popup,
    Search,
    Web,
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    #[ignore]
    fn create_new_profile_shares_dictionaries() {
        let mut options = YomichanOptions::new();
        let dict_name = "test_dict".to_string();
        let dict_opts = DictionaryOptions::new(dict_name.clone());

        // Add a dictionary to the default profile
        {
            let default_profile = options.get_current_profile().unwrap();
            let mut default_profile_write = default_profile.write_arc();
            default_profile_write
                .options
                .dictionaries
                .insert(dict_name.clone(), dict_opts);
        }

        let new_profile_name = "Test Profile";
        assert!(options.create_new_profile(new_profile_name).is_ok());

        // Check that the new profile exists
        let new_profile_ptr = options.find_profile_by_name(new_profile_name).unwrap();
        let new_profile = new_profile_ptr.read_arc();
        assert_eq!(new_profile.name, new_profile_name);

        // Check that the dictionary was copied
        assert!(new_profile.options.dictionaries.contains_key(&dict_name));
        assert_eq!(new_profile.options.dictionaries.len(), 1);

        // Try to create a profile with the same name, should fail
        let result = options.create_new_profile(new_profile_name);
        assert!(matches!(
            result,
            Err(ProfileError::ProfileAlreadyExists { .. })
        ));
    }
}
