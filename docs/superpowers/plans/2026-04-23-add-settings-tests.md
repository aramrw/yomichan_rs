# Add more settings tests Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add comprehensive tests to `src/settings/test.rs` to verify various settings persistence scenarios.

**Architecture:**
1.  **Test Suite Expansion:** Add tests to `src/settings/test.rs` covering:
    -   Changing dictionary enabled state.
    -   Changing popup settings.
    -   Changing scan delay settings.
2.  **Infrastructure:** Continue to use `open_test_ycd()` for test isolation.

**Tech Stack:** Rust.

---

### Task 1: Add Dictionary Enabled State Persistence Test

**Files:**
- Modify: `src/settings/test.rs`

- [ ] **Step 1: Add `persist_disable_dictionary` test**

```rust
    #[test]
    fn persist_disable_dictionary() {
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                let dicts = pf.options_mut().dictionaries_mut();
                if let Some(first) = dicts.first_mut() {
                    first.1.enabled = false;
                }
            }).unwrap();
            ycd.save_settings().unwrap();
        }

        {
            let ycd = open_test_ycd();
            let is_enabled = ycd.with_profile(|pf| {
                let dicts = pf.options().dictionaries();
                dicts.first().map(|(_, d)| d.enabled).unwrap_or(true)
            }).unwrap();
            assert!(!is_enabled);
        }
    }
```

- [ ] **Step 2: Commit**

```bash
git add src/settings/test.rs
git commit -m "test: add persist_disable_dictionary"
```

### Task 2: Add Popup Settings Persistence Test

**Files:**
- Modify: `src/settings/test.rs`

- [ ] **Step 1: Add `persist_popup_width` test**

```rust
    #[test]
    fn persist_popup_width() {
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                pf.options_mut().general.popup_width = 123;
            }).unwrap();
            ycd.save_settings().unwrap();
        }

        {
            let ycd = open_test_ycd();
            let width = ycd.with_profile(|pf| {
                pf.options().general.popup_width
            }).unwrap();
            assert_eq!(width, 123);
        }
    }
```

- [ ] **Step 2: Commit**

```bash
git add src/settings/test.rs
git commit -m "test: add persist_popup_width"
```

### Task 3: Add Scan Delay Settings Persistence Test

**Files:**
- Modify: `src/settings/test.rs`

- [ ] **Step 1: Add `persist_scan_delay` test**

```rust
    #[test]
    fn persist_scan_delay() {
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                pf.options_mut().scanning.delay = 45;
            }).unwrap();
            ycd.save_settings().unwrap();
        }

        {
            let ycd = open_test_ycd();
            let delay = ycd.with_profile(|pf| {
                pf.options().scanning.delay
            }).unwrap();
            assert_eq!(delay, 45);
        }
    }
```

- [ ] **Step 2: Commit**

```bash
git add src/settings/test.rs
git commit -m "test: add persist_scan_delay"
```
