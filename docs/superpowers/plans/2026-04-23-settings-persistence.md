# Database Settings Persistence Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement settings persistence in the SQLite database and refactor tests to use isolated `Yomichan` instances.

**Architecture:**
1.  **Backend Persistence:** Implement `_update_options_internal` in `Backend` to serialize `YomichanOptions` into the SQLite database.
2.  **Test Utils:** Add `open_test_ycd()` to `src/utils/test_utils.rs` which returns a fresh `Yomichan` instance backed by the test DB file.
3.  **Test Refactor:** Update tests to use `open_test_ycd()` and verify settings persistence.

**Tech Stack:** Rust, SQLite (via `DictionaryService`/`DictionaryDatabase`), `serde`/`native-model` (for serialization).

---

### Task 1: Implement `Backend::_update_options_internal`

**Files:**
- Modify: `src/backend.rs`

- [ ] **Step 1: Implement `_update_options_internal`**

```rust
// In src/backend.rs
fn _update_options_internal(&self) -> Result<(), Box<DictionaryDatabaseError>> {
    let opts = self.options.read();
    let blob = native_model::encode(&*opts)
        .map_err(|e| DictionaryDatabaseError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    self.db.set_settings(&blob).map_err(|e| DictionaryDatabaseError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src/backend.rs
git commit -m "feat: implement settings serialization to sqlite"
```

### Task 2: Add `open_test_ycd` utility

**Files:**
- Modify: `src/utils/test_utils.rs`

- [ ] **Step 1: Add helper function**

```rust
// Add to src/utils/test_utils.rs
pub fn open_test_ycd() -> crate::Yomichan {
    crate::Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap()
}
```

- [ ] **Step 2: Commit**

```bash
git add src/utils/test_utils.rs
git commit -m "test: add open_test_ycd helper"
```

### Task 3: Refactor and implement settings persistence test

**Files:**
- Modify: `src/settings/test.rs`

- [ ] **Step 1: Update test in `src/settings/test.rs`**

```rust
#[cfg(test)]
mod settings {
    use crate::utils::test_utils::open_test_ycd;

    #[test]
    fn persist_enable_dictionary() {
        let db_path = crate::utils::test_utils::TEST_PATHS.tests_yomichan_db_path.clone();
        
        // 1. Setup: Enable a dictionary
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                let dicts = pf.options_mut().dictionaries_mut();
                let first = dicts.first_mut().unwrap();
                first.1.enabled = true;
            }).unwrap();
            ycd.save_settings().unwrap();
        }

        // 2. Verify: Re-open and check setting
        {
            let ycd = open_test_ycd();
            let is_enabled = ycd.with_profile(|pf| {
                let dicts = pf.options().dictionaries();
                let first = dicts.first().unwrap();
                first.1.enabled
            }).unwrap();
            assert!(is_enabled);
        }
    }
}
```

- [ ] **Step 2: Run test**

```bash
cargo test persist_enable_dictionary
```

- [ ] **Step 3: Commit**

```bash
git add src/settings/test.rs
git commit -m "test: add settings persistence verification"
```
