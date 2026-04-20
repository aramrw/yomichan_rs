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

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create `src/models/mod.rs`**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 2: Setup `utils` module

**Files:**
- Create: `src/utils/mod.rs`
- Move: `src/errors.rs` -> `src/utils/errors.rs`
- Move: `src/test_utils.rs` -> `src/utils/test_utils.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create `src/utils/mod.rs`**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 3: Setup `audio` and `anki` modules

**Files:**
- Create: `src/audio/mod.rs`, `src/anki/mod.rs`
- Move: `src/audio.rs` -> `src/audio/core.rs`
- Move: `src/anki.rs` -> `src/anki/core.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create mod files**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 4: Setup `settings` module

**Files:**
- Create: `src/settings/mod.rs`
- Move: `src/settings.rs` -> `src/settings/core.rs`
- Move: `src/environment.rs` -> `src/settings/environment.rs`
- Move: `src/method_modules/options.rs` -> `src/settings/options.rs`
- Move: `src/method_modules/dictionary_options.rs` -> `src/settings/dictionary_options.rs`
- Delete: `src/method_modules/mod.rs` and the directory.

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create `src/settings/mod.rs`**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 5: Setup `scanner` module

**Files:**
- Create: `src/scanner/mod.rs`
- Move: `src/text_scanner.rs` -> `src/scanner/core.rs`

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create `src/scanner/mod.rs`**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 6: Setup `translator` module

**Files:**
- Create: `src/translator/mod.rs`
- Move: `src/translator.rs` -> `src/translator/core.rs`
- Move: `src/translator.rs_top` -> `src/translator/top.rs`
- Move: `src/translation.rs` -> `src/translator/types.rs`
- Move: `src/translation_internal.rs` -> `src/translator/internal_types.rs`
- Move: `src/regex_util.rs` -> `src/translator/regex_util.rs`

- [x] **Step 1: Create directories and move files**
- [x] **Step 2: Create `src/translator/mod.rs`**
- [x] **Step 3: Update `src/lib.rs`**
- [x] **Step 4: Fix imports across the crate to make `cargo check` pass**
- [x] **Step 5: Run tests and Commit**

---

### Task 7: Final Stabilization and Cleanup

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/utils/mod.rs`
- Modify: `src/utils/errors.rs`
- Modify: `src/backend.rs`

- [x] **Step 1: Move Ptr and related macros to `src/utils/mod.rs`**
- [x] **Step 2: Centralize all InitError logic in `src/utils/errors.rs`**
- [x] **Step 3: Fix Backend anki field and initialization in `src/backend.rs`**
- [x] **Step 4: Simplify src/lib.rs and clean up unused imports**
- [x] **Step 5: Verify with `cargo check` and `cargo test`**
