# Design Spec: Interior Mutability for Yomichan-RS

**Topic:** Refactor core components to support thread-safe, immutable usage via `&self`.

## 1. Objective
Allow the `Yomichan` struct to be stored in a global (e.g., `LazyLock`) and used concurrently from multiple threads without requiring a top-level `RwLock` or `Arc<Mutex>`. This is achieved by moving shared mutable state (like caches) into thread-safe internal containers.

## 2. Structural Changes

### 2.1. Translator (`src/translator.rs`)
- **`tag_cache`**: Change from `IndexMap<String, TagCache>` to `parking_lot::RwLock<IndexMap<String, TagCache>>>`.
- **`find_terms` & helpers**: Update signature to `&self`.
- **`prepare()`**: Ensure it's called during `new()` so that `text_processors` and `reading_normalizers` are read-only after initialization.

### 2.2. TextScanner (`src/text_scanner.rs`)
- **Search methods**: Update `search_sentence`, `_search_internal`, and `find_term_dictionary_entries` to `&self`.
- **State Management**: Ensure any temporary state used during a search is stored in local variables rather than struct fields.

### 2.3. Backend & Yomichan (`src/backend.rs`, `src/lib.rs`)
- **Top-level API**: Update `search`, `set_language`, and `delete_dictionaries_*` to `&self`.
- **Settings**: Leverage existing `Ptr<YomichanOptions>` (which is already `Arc<RwLock>`) to allow `&self` updates.

## 3. API Impact
- **Non-Breaking Improvements**: Changing `&mut self` to `&self` is generally a non-breaking change for consumers, as `&self` is more permissive.
- **Concurrency**: Users can now call `.search()` from multiple threads simultaneously.

## 4. Implementation Plan

### Step 1: Translator Refactor
- Wrap `tag_cache` in `RwLock`.
- Update `find_terms` and `_expand_tag_groups` to use `read()`/`write()` locks.
- **Validation**: `cargo test search_dbg`.

### Step 2: TextScanner Refactor
- Update methods to `&self`.
- Ensure no shared mutable state is used during scanning.
- **Validation**: `cargo test search`.

### Step 3: Backend & Yomichan Refactor
- Update top-level methods to `&self`.
- Update examples and documentation to reflect immutable usage.
- **Validation**: All tests and examples.
