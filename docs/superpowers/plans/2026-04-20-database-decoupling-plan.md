# DictionaryService Decoupling and Migration Plan

**Goal:** Decouple the `Translator` from `native_db` by introducing a `DictionaryService` trait, allowing us to swap the underlying storage to `rusqlite` for high-performance dictionary ingestion.

**Architecture:**
1.  **Define `DictionaryService` Trait:** Create an interface in `src/database/mod.rs` containing the existing database query methods.
2.  **Existing Implementation:** Update `DictionaryDatabase` to implement this trait (it currently wraps `native_db`).
3.  **Translator Injection:** Update `Translator` to use `Arc<dyn DictionaryService>` instead of a concrete `DictionaryDatabase`.
4.  **Sqlite Implementation (Follow-up):** Once decoupled, implement the service using `rusqlite` for high-speed dictionary ingestion.

**Tech Stack:** Rust, `rusqlite` (upcoming), `native_db` (current).

---

### Task 1: Define `DictionaryService` Trait

**Files:**
- Modify: `src/database/mod.rs` (Define the trait)
- Modify: `src/database/dictionary_database.rs` (Implement the trait)

- [ ] **Step 1: Define the trait in `src/database/mod.rs`**

```rust
pub trait DictionaryService: Send + Sync {
    fn find_tag_meta_bulk(&self, queries: &[crate::database::dictionary_database::GenericQueryRequest]) -> Result<Vec<crate::importer::dictionary_database::DatabaseTagMeta>, crate::errors::DBError>;
    fn find_term_meta_bulk(&self, keys: &[String], enabled_dictionaries: &crate::database::dictionary_database::TermEnabledDictionaryMap) -> Result<Vec<crate::database::dictionary_database::DatabaseTermMeta>, crate::errors::DBError>;
    fn find_terms_exact_bulk(&self, terms: &[crate::database::dictionary_database::TermExactQueryRequest], enabled_dictionaries: &crate::database::dictionary_database::TermEnabledDictionaryMap) -> Result<Vec<crate::importer::dictionary_database::TermEntry>, crate::errors::DBError>;
    fn find_terms_by_sequence_bulk(&self, queries: Vec<crate::database::dictionary_database::GenericQueryRequest>) -> Result<Vec<crate::importer::dictionary_database::TermEntry>, crate::errors::DBError>;
}
```

- [ ] **Step 2: Implement the trait for `DictionaryDatabase`**

In `src/database/dictionary_database.rs`, add:
```rust
impl DictionaryService for DictionaryDatabase<'_> {
    fn find_tag_meta_bulk(...) -> ... { self.find_tag_meta_bulk(...) }
    // ... forward the other methods ...
}
```

- [ ] **Step 3: Commit**

### Task 2: Update `Translator` to use the trait

**Files:**
- Modify: `src/translator.rs`

- [ ] **Step 1: Change `Translator` struct field**

Change `pub db: Arc<DictionaryDatabase<'a>>` to `pub db: Arc<dyn DictionaryService>`.

- [ ] **Step 2: Fix compilation errors**

Since we are moving from a concrete type to a trait object, you may need to adjust the `Translator::new` and `init` functions to accept the trait object.

- [ ] **Step 3: Commit**

---

Plan complete. Once this is done, the `Translator` is completely agnostic of `native_db`, and we can implement the `SqliteDictionaryService` without touching any of the business logic.

**Two execution options:**

1. **Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration.

2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
