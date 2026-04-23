# Public API Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Systematically document all public API surface area in `yomichan_rs` by resolving `#![warn(missing_docs)]` compiler warnings.

**Architecture:** We will enable the `missing_docs` lint at the crate level and incrementally document modules, ensuring examples and context are provided for all user-facing interfaces.

**Tech Stack:** Rust, `rustdoc`, `cargo doc`.

---

### Task 1: Enable missing_docs and Document Root API

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add the lint to lib.rs**

```rust
// Add to the very top of src/lib.rs:
#![warn(missing_docs)]

//! # Yomichan RS
// ... existing docs ...
```

- [ ] **Step 2: Run cargo doc to see warnings**

Run: `cargo doc --no-deps --document-private-items=false`
Expected: Output showing warnings for missing documentation on `Yomichan` and other public items in `lib.rs`.

- [ ] **Step 3: Document the `Yomichan` struct and its methods**

```rust
// In src/lib.rs, add documentation to the struct and its methods.
// For example:
/// The primary interface for the Yomichan RS library.
///
/// `Yomichan` manages the dictionary database, search operations, and user profiles.
pub struct Yomichan {
    // ...
}

impl Yomichan {
    /// Creates a new `Yomichan` instance from the given database path.
    ///
    /// If the path is a directory, it will initialize the database within a `yomichan_rs` subdirectory.
    ///
    /// # Arguments
    /// * `path` - The filesystem path to the database or directory.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YomichanError> {
        // ...
    }
    
    // Continue adding `///` comments to `search`, `search_structured`, `nuke_database`,
    // `with_profile`, `with_profile_mut`, etc.
}
```

- [ ] **Step 4: Verify warnings are reduced**

Run: `cargo doc --no-deps`
Expected: Warnings for `src/lib.rs` specifically (like the `Yomichan` struct and its methods) should disappear.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs
git commit -m "docs: Enable missing_docs lint and document root API"
```

### Task 2: Document the Models Module

**Files:**
- Modify: `src/models/dictionary.rs`, `src/models/freq.rs`, `src/models/mod.rs` (depending on what's public)

- [ ] **Step 1: Identify missing docs in models**

Run: `cargo doc --no-deps` and look for warnings under `src/models`.

- [ ] **Step 2: Add missing docs to models**

Open the files in `src/models/` and add `///` documentation to all exported structs, enums, and their public fields/methods that the compiler complains about. Add module-level `//!` docs if needed.

```rust
// Example for src/models/dictionary.rs:
/// Represents a parsed dictionary entry for a specific term.
pub struct TermDictionaryEntry {
    /// The term itself.
    pub term: String,
    // ...
}
```

- [ ] **Step 3: Verify the models module is clean**

Run: `cargo doc --no-deps`
Expected: No warnings from `src/models/*`.

- [ ] **Step 4: Commit**

```bash
git add src/models/
git commit -m "docs: Add missing documentation to models module"
```

### Task 3: Document the Translator and Scanner Modules

**Files:**
- Modify: `src/translator/*.rs`, `src/scanner/*.rs`

- [ ] **Step 1: Identify missing docs in translator/scanner**

Run: `cargo doc --no-deps` and look for warnings under `src/translator` and `src/scanner`.

- [ ] **Step 2: Add missing docs to translator/scanner**

Document the public interfaces like `Translator`, `TextScanner`, `SearchResult`, `SearchSegment`, etc. Provide examples where it clarifies usage.

- [ ] **Step 3: Verify cleanliness**

Run: `cargo doc --no-deps`
Expected: No warnings from `src/translator/*` or `src/scanner/*`.

- [ ] **Step 4: Commit**

```bash
git add src/translator/ src/scanner/
git commit -m "docs: Add missing documentation to translator and scanner modules"
```

### Task 4: Document Database Module

**Files:**
- Modify: `src/database/*.rs`

- [ ] **Step 1: Identify missing docs in database**

Run: `cargo doc --no-deps` and look for warnings under `src/database`.

- [ ] **Step 2: Add missing docs to database**

Document `DictionaryDatabase`, `DictionaryService`, `dictionary_importer`, and any public traits or structures.

- [ ] **Step 3: Verify cleanliness**

Run: `cargo doc --no-deps`
Expected: No warnings from `src/database/*`.

- [ ] **Step 4: Commit**

```bash
git add src/database/
git commit -m "docs: Add missing documentation to database module"
```

### Task 5: Document Settings, Audio, Anki, and Utils

**Files:**
- Modify: `src/settings/*.rs`, `src/audio/*.rs`, `src/anki/*.rs`, `src/utils/*.rs`

- [ ] **Step 1: Identify remaining missing docs**

Run: `cargo doc --no-deps` (and optionally `cargo doc --no-deps --features anki` if applicable) to find all remaining warnings.

- [ ] **Step 2: Add missing docs to remaining modules**

Document options structures, error types (`YomichanError`, etc.), and utility functions. Make sure all public errors and enum variants are documented.

- [ ] **Step 3: Final Verification**

Run: `cargo doc --no-deps`
Expected: **Zero** `missing_docs` warnings across the entire crate.

- [ ] **Step 4: Commit**

```bash
git add src/settings/ src/audio/ src/anki/ src/utils/
git commit -m "docs: Document remaining public modules and ensure zero warnings"
```