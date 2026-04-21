//! # Anki Integration
//!
//! This module provides a high-level API for interacting with Anki via [AnkiConnect](https://ankiweb.net/shared/info/2055492159).
//!
//! The primary entry point is the [`DisplayAnki`] struct, which can be accessed via [`Yomichan::anki`](crate::Yomichan::anki).
//!
//! ## Requirements
//!
//! - The `anki` feature must be enabled in your `Cargo.toml`.
//! - Anki must be running with the [AnkiConnect](https://ankiweb.net/shared/info/2055492159) add-on installed.
//!
//! ## Example 1: Quick Start (Fully Automatic)
//!
//! This is the easiest way to get started. It automatically selects the first available deck 
//! and model it finds in Anki and sets up a default field mapping.
//!
//! ```rust,no_run
//! use yomichan_rs::Yomichan;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let ycd = Yomichan::new("path/to/db.ycd")?;
//! let anki = ycd.anki();
//!
//! // Auto-configure: picks first deck, first model, and maps fields automatically.
//! anki.configure_note_creation_auto()?;
//!
//! // Search and add a term to Anki
//! let results = ycd.search("日本語").ok_or("Search failed")?;
//! let first_entry = &results[0].results.as_ref().unwrap().dictionary_entries[0];
//!
//! anki.add_entry(first_entry, Some("日本語を勉強しています。"))?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Full Manual Workflow (GUI/Custom Setup)
//!
//! For full control, especially when building a GUI, follow these steps in order:
//!
//! ### 1. Hydrate
//! Fetch the latest deck and model information from your running Anki instance.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! let anki = ycd.anki();
//! anki.update_all_anki_maps()?;
//! ```
//!
//! ### 2. Discover and Select
//! List the available options and select which deck and note type (model) to use.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! # let anki = ycd.anki();
//! let decks = anki.deck_names();
//! let models = anki.model_names();
//!
//! // Select by index (e.g., from a user's selection in a dropdown)
//! anki.select_deck(0)?;
//! anki.select_model(0)?;
//! ```
//!
//! ### 3. Map the Fields
//! You must tell `yomichan_rs` which Anki fields in your selected 
//! model should receive specific pieces of dictionary data.
//!
//! Use [`FieldIndex`] to map a data type to a specific field index in your Anki model.
//!
//! ```rust,no_run
//! use yomichan_rs::settings::core::FieldIndex;
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! # let anki = ycd.anki();
//!
//! anki.set_field_mappings(&[
//!     FieldIndex::Term(0),       // Put the Term in Anki field 0
//!     FieldIndex::Reading(1),    // Put the Reading in Anki field 1
//!     FieldIndex::Definition(2), // Put the Definition in Anki field 2
//!     FieldIndex::Sentence(3),   // Put the Context Sentence in Anki field 3
//! ])?;
//! ```
//! Or alternatively: 
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # use yomichan_rs::settings::core::FieldIndex;
//! # let mut ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! let mappings = [FieldIndex::Term(0), FieldIndex::Definition(1)];
//!
//! ycd.anki().configure_note_creation(
//!     "Japanese Vocabulary", // Deck Name
//!     "Basic-Model",         // Model Name
//!     &mappings
//! )?;
//! ```
//!
//! ### 4. Action
//! Once configured, you can build and add notes directly.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! # let anki = ycd.anki();
//! # let sentence = "日本語が好きで勉強したくなってきた"
//! # let segments = ycd.search("日本語").unwrap();
//! // gets the entry for 日本語
//! # let first_entry = &segments[0].results.as_ref().unwrap().dictionary_entries[0];
//! let note_ids = anki.add_entry(first_entry, Some(sentence))?;
//! ```
//! ## Other Usecases
//!
//! You can generate `Note`s without adding them to Anki immediately.
//! This is useful for building a "pending" queue of notes that you can review 
//! or bulk-import later.
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("path/to/db.ycd").unwrap();
//! # let anki = ycd.anki();
//! let sentence = "日本語が好きで勉強したくなってきた";
//! let segments = ycd.search(sentence).ok_or("Search failed")?;
//!
//! let mut pending_notes = Vec::new();
//!
//! for segment in segments {
//!     if let Some(res) = segment.results {
//!         // Get the first entry for each recognized word
//!         let entry = &res.dictionary_entries[0];
//!
//!         // .build_note_from_entry creates an anki_direct::Note object 
//!         // but does not communicate with Anki yet.
//!         let note = anki.build_note_from_entry(entry, Some(sentence))?;
//!         pending_notes.push(note);
//!     }
//! }
//!
//! for note in pending_notes {
//!     anki.add_entry(note, sentence.into())?;
//! }
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod core;

pub use core::{DisplayAnki, DisplayAnkiError};
