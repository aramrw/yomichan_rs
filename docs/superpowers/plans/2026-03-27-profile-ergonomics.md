# Profile Ergonomics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Streamline access to the active profile's settings using scoped closures and perform minor project cleanup.

**Architecture:** Add `with_profile` and `with_profile_mut` (and Anki variants) to the `Yomichan` struct to abstract `Ptr` locking and `ProfileResult` handling.

**Tech Stack:** Rust, `parking_lot` (RwLock), `thiserror`.

---

### Task 1: Scoped Profile Accessors in `Yomichan`

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add the new methods to the `Yomichan` impl block**

```rust
impl<'a> Yomichan<'a> {
    /// Executes a closure with immutable access to the current profile.
    pub fn with_profile<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let profile = profile_ptr.read();
        Ok(f(&profile))
    }

    /// Executes a closure with mutable access to the current profile.
    pub fn with_profile_mut<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&mut YomichanProfile) -> R,
    {
        let opts = self.backend.options.read();
        let profile_ptr = opts.get_current_profile()?;
        let mut profile = profile_ptr.write();
        Ok(f(&mut profile))
    }

    #[cfg(feature = "anki")]
    /// Executes a closure with immutable access to the current profile's AnkiOptions.
    pub fn with_anki_options<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&settings::AnkiOptions) -> R,
    {
        self.with_profile(|p| f(p.anki_options()))
    }

    #[cfg(feature = "anki")]
    /// Executes a closure with mutable access to the current profile's AnkiOptions.
    pub fn with_anki_options_mut<F, R>(&self, f: F) -> settings::ProfileResult<R>
    where
        F: FnOnce(&mut settings::AnkiOptions) -> R,
    {
        self.with_profile_mut(|p| f(p.anki_options_mut()))
    }
}
```

- [ ] **Step 2: Verify with `cargo check --all-features`**

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: add scoped profile accessors to Yomichan"
```

---

### Task 2: Environment Typo Fix

**Files:**
- Modify: `src/environment.rs`

- [ ] **Step 1: Rename `paltform` to `platform`**

```rust
pub struct EnvironmentInfo {
    pub platform: &'static str, // Changed from paltform
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self { platform: OS }
    }
}

pub static CACHED_ENVIRONMENT_INFO: LazyLock<EnvironmentInfo> =
    LazyLock::new(|| EnvironmentInfo { platform: OS });
```

- [ ] **Step 2: Verify with `cargo check --all-features`**

- [ ] **Step 3: Commit**

```bash
git add src/environment.rs
git commit -m "fix: rename paltform to platform in EnvironmentInfo"
```

---

### Task 3: Refactor Backend `set_language`

**Files:**
- Modify: `src/backend.rs`

- [ ] **Step 1: Update `set_language` to use `with_profile_mut`**

```rust
    pub fn set_language(&self, language_iso: &str) -> ProfileResult<()> {
        self.with_profile_mut(|profile| {
            profile.set_language(language_iso);
        })
    }
```

- [ ] **Step 2: Verify with `cargo check --all-features`**

- [ ] **Step 3: Commit**

```bash
git add src/backend.rs
git commit -m "refactor: use with_profile_mut in set_language"
```

---

### Task 4: Clean up `method_modules`

**Files:**
- Modify: `src/method_modules/options.rs`
- Modify: `src/method_modules/dictionary_options.rs`

- [ ] **Step 1: Remove all commented-out code in `options.rs`**

- [ ] **Step 2: Remove all commented-out code in `dictionary_options.rs`**

- [ ] **Step 3: Verify with `cargo check --all-features`**

- [ ] **Step 4: Commit**

```bash
git add src/method_modules/options.rs src/method_modules/dictionary_options.rs
git commit -m "cleanup: remove obsolete commented-out code in method_modules"
```

---

### Task 5: Verification Unit Test

**Files:**
- Modify: `src/lib.rs` (tests module)

- [ ] **Step 1: Add a test for `with_profile_mut` and return value**

```rust
#[cfg(test)]
mod yomichan_ergonomics_tests {
    use super::*;
    use crate::test_utils::TEST_PATHS;

    #[test]
    fn test_with_profile_mut_ergonomics() {
        let ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        
        // Mutate and return a value
        let lang = ycd.with_profile_mut(|profile| {
            profile.set_language("es");
            profile.options().general.language.clone()
        }).expect("Should access profile");

        assert_eq!(lang, "es");
        
        // Verify via read accessor
        let read_lang = ycd.with_profile(|profile| {
            profile.options().general.language.clone()
        }).expect("Should access profile");
        
        assert_eq!(read_lang, "es");
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test yomichan_ergonomics_tests --all-features`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "test: add ergonomic profile accessor verification"
```
