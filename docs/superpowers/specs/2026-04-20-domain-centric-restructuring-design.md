# Domain-Centric Restructuring Design

## Goal
Reorganize the `yomichan_rs` project structure to follow idiomatic Rust module conventions. Consolidate logic based on domain (e.g., translator, scanner, settings) rather than keeping everything as flat files in `src/`. This will improve findability, reduce cognitive load, and simplify `lib.rs`.

## Architecture
The project will be restructured into the following specialized module directories:

### 1. `src/translator/` (Text Analysis & Translation)
Consolidates all translation logic and internal types.
* `mod.rs`: Public exports.
* `core.rs`: Currently `translator.rs` & `translator.rs_top`.
* `types.rs`: Combines `translation.rs` and `translation_internal.rs`.
* `regex_util.rs`: Used heavily by the translator.

### 2. `src/settings/` (Configuration & Environment)
Consolidates everything related to user preferences and the environment.
* `mod.rs`: Public exports.
* `core.rs`: Currently `settings.rs`.
* `environment.rs`: Moved from `src/environment.rs`.
* `options.rs`: Moved from `src/method_modules/options.rs`.
* `dictionary_options.rs`: Moved from `src/method_modules/dictionary_options.rs`.

### 3. `src/models/` (Shared Data Structures)
A central place for core domain types to prevent circular dependencies and organize scattered structs.
* `mod.rs`: Public exports.
* `dictionary.rs`: Currently `dictionary.rs` (contains `TermDefinition`, `TermFrequency`, etc.).
* `freq.rs`: Moved from `src/freq.rs`.

### 4. `src/scanner/` (Text Processing)
* `mod.rs`: Public exports.
* `core.rs`: Currently `text_scanner.rs`.

### 5. `src/anki/`, `src/audio/`, `src/utils/`
Small, focused modules for specific integrations and helpers.
* `src/anki/mod.rs` & `src/anki/core.rs`: Currently `anki.rs`.
* `src/audio/mod.rs` & `src/audio/core.rs`: Currently `audio.rs`.
* `src/utils/mod.rs` & `src/utils/errors.rs`, `src/utils/test_utils.rs`: Currently `errors.rs` and `test_utils.rs`.

### 6. `src/backend.rs` & `src/lib.rs`
* `src/lib.rs`: Significantly simplified to only expose the public API of the new modules.
* `src/backend.rs`: Remains at the root as the core orchestrator.
* `src/database/`: Remains mostly as-is, focusing purely on data persistence.
* `src/method_modules/`: Will be deleted after its contents are migrated to `src/settings/`.

## Data Flow & Error Handling
Data flow remains conceptually identical. The primary change is how structs and functions are imported across the codebase.
* `use crate::translator::...` instead of `use crate::translation::...`.
* Errors will be centralized in `crate::utils::errors` instead of `crate::errors`.

## Testing
Tests in `tests/` and `src/test_utils.rs` (now `src/utils/test_utils.rs`) will need their import statements updated to reflect the new module paths. No new tests are required for this purely structural change, but all existing tests must pass.
