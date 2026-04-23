//! # Data Models
//!
//! This module defines the common data structures used throughout the library to
//! represent dictionary entries, frequencies, and translation results.
//!
//! These models are the final output that users of the library typically consume.
//!
//! ## Main Models
//!
//! - **[`TermDictionaryEntry`](crate::models::dictionary::TermDictionaryEntry)**: Represents a single matched term from a dictionary.
//! - **[`TermDefinition`](crate::models::dictionary::TermDefinition)**: A specific meaning or definition for a term.
//! - **[`TermFrequency`](crate::models::dictionary::TermFrequency)**: Frequency data for a term (e.g., "common" or ranking).

/// Dictionary data models for terms, readings, and definitions.
pub mod dictionary;
/// Frequency data models for ranking terms.
pub mod freq;
