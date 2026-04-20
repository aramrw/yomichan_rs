# Domain-Centric Restructuring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize the `yomichan_rs` project structure to follow idiomatic Rust module conventions by domain (models, utils, settings, scanner, translator, etc).

**Architecture:** We will create specific directory modules and move the flat source files into them, updating `lib.rs` and imports throughout the crate iteratively to ensure it compiles at each step.

**Tech Stack:** Rust, Cargo

---

### Task 1: Setup `models` module

**Files:**
- Create: `src/models/mod.rs`
- Move: `src/dictionary.rs` -> `src/models/dictionary.rs`
- Move: `src/freq.rs` -> `src/models/freq.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/models
git mv src/dictionary.rs src/models/dictionary.rs
git mv src/freq.rs src/models/freq.rs
```

- [ ] **Step 2: Create `src/models/mod.rs`**

```rust
pub mod dictionary;
pub mod freq;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `mod dictionary;` and `mod freq;`.
Add `pub mod models;`.
Update public exports:
```rust
// Replace:
// pub use crate::dictionary::{TermDefinition, TermDictionaryEntry, TermFrequency, TermPronunciation};
// With:
pub use crate::models::dictionary::{TermDefinition, TermDictionaryEntry, TermFrequency, TermPronunciation};
```

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check`. Find any files using `crate::dictionary::` or `crate::freq::` and replace them with `crate::models::dictionary::` and `crate::models::freq::`. Keep running `cargo check` until it passes.

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test
git add src/ models/ lib.rs Cargo.toml
git commit -m "refactor: extract models module"
```

---

### Task 2: Setup `utils` module

**Files:**
- Create: `src/utils/mod.rs`
- Move: `src/errors.rs` -> `src/utils/errors.rs`
- Move: `src/test_utils.rs` -> `src/utils/test_utils.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/utils
git mv src/errors.rs src/utils/errors.rs
git mv src/test_utils.rs src/utils/test_utils.rs
```

- [ ] **Step 2: Create `src/utils/mod.rs`**

```rust
pub mod errors;
pub mod test_utils;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `mod errors;` and `pub mod test_utils;`.
Add `pub mod utils;`.
Update public exports for `DBError` and `YomichanError` to point to `crate::utils::errors::...`

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check`. Replace `crate::errors::` with `crate::utils::errors::` and `crate::test_utils::` with `crate::utils::test_utils::`.

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test
git add src/
git commit -m "refactor: extract utils module"
```

---

### Task 3: Setup `audio` and `anki` modules

**Files:**
- Create: `src/audio/mod.rs`, `src/anki/mod.rs`
- Move: `src/audio.rs` -> `src/audio/core.rs`
- Move: `src/anki.rs` -> `src/anki/core.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/audio src/anki
git mv src/audio.rs src/audio/core.rs
git mv src/anki.rs src/anki/core.rs
```

- [ ] **Step 2: Create mod files**

`src/audio/mod.rs`:
```rust
pub mod core;
```

`src/anki/mod.rs`:
```rust
pub mod core;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `mod audio;` and `pub mod anki;`.
Add `pub mod audio;` and `#[cfg(feature = "anki")] pub mod anki;`.
Update exports like `use crate::anki::DisplayAnkiError` to `use crate::anki::core::DisplayAnkiError`.

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check` and `cargo check --features anki`. Replace `crate::audio::` with `crate::audio::core::` and `crate::anki::` with `crate::anki::core::`.

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test --all-features
git add src/
git commit -m "refactor: extract audio and anki modules"
```

---

### Task 4: Setup `settings` module

**Files:**
- Create: `src/settings/mod.rs`
- Move: `src/settings.rs` -> `src/settings/core.rs`
- Move: `src/environment.rs` -> `src/settings/environment.rs`
- Move: `src/method_modules/options.rs` -> `src/settings/options.rs`
- Move: `src/method_modules/dictionary_options.rs` -> `src/settings/dictionary_options.rs`
- Delete: `src/method_modules/mod.rs` and the directory.

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/settings
git mv src/settings.rs src/settings/core.rs
git mv src/environment.rs src/settings/environment.rs
git mv src/method_modules/options.rs src/settings/options.rs
git mv src/method_modules/dictionary_options.rs src/settings/dictionary_options.rs
git rm src/method_modules/mod.rs
rmdir src/method_modules
```

- [ ] **Step 2: Create `src/settings/mod.rs`**

```rust
pub mod core;
pub mod dictionary_options;
pub mod environment;
pub mod options;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `pub mod settings;`, `mod environment;`, `mod method_modules;`.
Add `pub mod settings;`.
Update `use settings::YomichanProfile` to `use settings::core::YomichanProfile`.
Update `pub use crate::environment::EnvironmentInfo` to `pub use crate::settings::environment::EnvironmentInfo`.

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check`. Replace `crate::settings::` with `crate::settings::core::`, `crate::environment::` with `crate::settings::environment::`, and `crate::method_modules::*` with `crate::settings::*`.

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test
git add src/
git commit -m "refactor: extract settings module"
```

---

### Task 5: Setup `scanner` module

**Files:**
- Create: `src/scanner/mod.rs`
- Move: `src/text_scanner.rs` -> `src/scanner/core.rs`

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/scanner
git mv src/text_scanner.rs src/scanner/core.rs
```

- [ ] **Step 2: Create `src/scanner/mod.rs`**

```rust
pub mod core;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `pub mod text_scanner;`.
Add `pub mod scanner;`.
Update exports for `TermSearchResults`, `TermSearchResultsSegment`, `TextScanner` to point to `crate::scanner::core::...`.

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check`. Replace `crate::text_scanner::` with `crate::scanner::core::`.

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test
git add src/
git commit -m "refactor: extract scanner module"
```

---

### Task 6: Setup `translator` module

**Files:**
- Create: `src/translator/mod.rs`
- Move: `src/translator.rs` -> `src/translator/core.rs`
- Move: `src/translator.rs_top` -> `src/translator/top.rs`
- Move: `src/translation.rs` -> `src/translator/types.rs`
- Move: `src/translation_internal.rs` -> `src/translator/internal_types.rs`
- Move: `src/regex_util.rs` -> `src/translator/regex_util.rs`

- [ ] **Step 1: Create directories and move files**

```bash
mkdir -p src/translator
git mv src/translator.rs src/translator/core.rs
git mv src/translator.rs_top src/translator/top.rs
git mv src/translation.rs src/translator/types.rs
git mv src/translation_internal.rs src/translator/internal_types.rs
git mv src/regex_util.rs src/translator/regex_util.rs
```

- [ ] **Step 2: Create `src/translator/mod.rs`**

```rust
pub mod core;
pub mod top;
pub mod types;
pub mod internal_types;
pub mod regex_util;
```

- [ ] **Step 3: Update `src/lib.rs`**

Remove `mod translator;`, `mod translation;`, `mod translation_internal;`, `mod regex_util;`.
Add `pub mod translator;`.
Update export `pub use crate::translator::Translator;` to `pub use crate::translator::core::Translator;`.

- [ ] **Step 4: Fix imports across the crate to make `cargo check` pass**

Run `cargo check`. Replace `crate::translation::` with `crate::translator::types::`, `crate::translation_internal::` with `crate::translator::internal_types::`, `crate::regex_util::` with `crate::translator::regex_util::`, and `crate::translator::` with `crate::translator::core::` (where applicable).

- [ ] **Step 5: Run tests and Commit**

```bash
cargo test
git add src/
git commit -m "refactor: extract translator module"
```
