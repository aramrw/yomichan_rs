//! # Text Scanning and Sentence Parsing
//!
//! This module handles the logic for breaking down sentences into searchable terms.
//!
//! It provides the [`TextScanner`](crate::scanner::core::TextScanner) which uses a "longest match"
//! algorithm to find all possible dictionary entries within a block of text.
//!
//! ## How it works
//!
//! 1. **Scan**: The scanner traverses the input text and looks up substrings in the dictionary.
//! 2. **Segment**: The results are grouped into [`SearchSegment`](crate::scanner::core::SearchSegment)s.
//! 3. **Display**: These segments distinguish between matched dictionary terms and raw, unmatched text.
//!
//! ## Example
//!
//! ```rust,no_run
//! # use yomichan_rs::Yomichan;
//! # let ycd = Yomichan::new("db.ycd").unwrap();
//! let text = "これはテストです。";
//! let search_result = ycd.search_structured(text).unwrap();
//!
//! for segment in search_result.segments {
//!     if !segment.entries.is_empty() {
//!         println!("'{}' is a word!", segment.text);
//!     } else {
//!         println!("'{}' is raw text.", segment.text);
//!     }
//! }
//! ```

/// Core components for text scanning and sentence segmentation.
pub mod core;
