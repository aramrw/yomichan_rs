//! # Settings and Profiles
//!
//! This module manages the configuration for the Yomichan engine.
//!
//! Settings are organized into **Profiles**. A profile contains all the options
//! that determine how text is scanned, which dictionaries are searched, and
//! how results are ranked.
//!
//! ## Key Concepts
//!
//! - **[`YomichanProfile`](crate::settings::core::YomichanProfile)**: A named collection of settings.
//! - **[`ProfileOptions`](crate::settings::core::ProfileOptions)**: The actual configuration values (scanning, deinflection, etc.).
//! - **[`DictionaryOptions`](crate::settings::core::DictionaryOptions)**: Configuration for which dictionaries are enabled.
//!
//! ## Usage Example
//!
//! You typically interact with settings via the [`Yomichan`](crate::Yomichan) struct's
//! profile methods.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("db.ycd").unwrap();
//! // Accessing the current profile to check if a dictionary is enabled
//! ycd.with_profile(|profile| {
//!     let is_enabled = profile.options().dictionaries.contains_key("JMdict");
//!     println!("JMdict enabled: {}", is_enabled);
//! }).unwrap();
//!
//! // Enabling a dictionary and changing the language
//! ycd.with_profile_mut(|profile| {
//!     profile.set_language("ja");
//!     // profile.enable_dictionary("JMdict", true); // Conceptually
//! }).unwrap();
//! ```

/// Core profile and settings logic.
pub mod core;
/// Dictionary-specific configuration and enabling/disabling.
pub mod dictionary_options;
/// Environment-specific information (platform, OS, etc.).
pub mod environment;
/// Detailed configuration options for scanning and translation.
pub mod options;
// Tests file for settings
pub mod test;
