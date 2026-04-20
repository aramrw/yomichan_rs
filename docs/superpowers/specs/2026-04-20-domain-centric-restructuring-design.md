# Domain-Centric Restructuring Design

## Goal
Reorganize the `yomichan_rs` project structure to follow idiomatic Rust module conventions. Consolidate logic based on domain (e.g., translator, scanner, settings) rather than keeping everything as flat files in `src/`. This improves findability, reduces cognitive load, and simplifies `lib.rs` to a clean entry point.

## Architecture
The project is restructured into specialized module directories. Each module follows a `mod.rs` (public exports) and internal file pattern.

### 1. `src/translator/` (Text Analysis & Translation)
Consolidates all translation logic and internal types.
* `mod.rs`: Public exports.
* `core.rs`: Main `Translator` implementation.
* `top.rs`: Legacy/Compatibility layer for top-level translation.
* `types.rs`: Publicly exposed translation types.
* `internal_types.rs`: Logic-internal deinflection and candidate types.
* `regex_util.rs`: Regex helpers for text replacement.

### 2. `src/settings/` (Configuration & Environment)
Consolidates user preferences, profiles, and environment metadata.
* `mod.rs`: Public exports.
* `core.rs`: `YomichanProfile` and `YomichanOptions`.
* `environment.rs`: `EnvironmentInfo` (OS, paths).
* `options.rs`: General application settings.
* `dictionary_options.rs`: Per-dictionary settings.

### 3. `src/models/` (Shared Data Structures)
Central place for core domain types to prevent circular dependencies.
* `mod.rs`: Public exports.
* `dictionary.rs`: `TermDefinition`, `TermDictionaryEntry`.
* `freq.rs`: Frequency data structures.

### 4. `src/scanner/` (Text Processing)
* `mod.rs`: Public exports.
* `core.rs`: `TextScanner` and search result types.

### 5. `src/utils/` (Helpers & Errors)
Logic-agnostic utilities and centralized error handling.
* `mod.rs`: Contains the `Ptr<T>` abstraction (Arc+RwLock wrapper) and shared macros (`iter_type_to_iter_variant!`).
* `errors.rs`: Centralized `YomichanError`, `DBError`, and `InitError`.
* `test_utils.rs`: Shared testing helpers.

### 6. `src/anki/` & `src/audio/`
Focused integration modules.
* `src/anki/`: AnkiConnect integration logic.
* `src/audio/`: Audio source and playback abstractions.

### 7. `src/backend.rs` & `src/lib.rs`
* `src/lib.rs`: Simplified to a high-level API entry point. It handles database resolution, instance management, and public re-exports.
* `src/backend.rs`: Core orchestrator that ties the database, scanner, and options together.

## Data Flow & Error Handling
* **Imports:** Use domain-centric paths (e.g., `use crate::translator::Translator;`).
* **Errors:** All significant errors are unified in `crate::utils::errors::YomichanError`.
* **State Management:** Shared state is managed via `Ptr<T>` (defined in `utils`), which provides thread-safe interior mutability with ergonomic `with_ptr` and `with_ptr_mut` accessors.

## Testing
Existing integration tests in `tests/` and unit tests in `src/utils/test_utils.rs` verify that functionality remains intact after restructuring. The `yomichan_ergonomics_tests` in `lib.rs` confirm the API usability.
