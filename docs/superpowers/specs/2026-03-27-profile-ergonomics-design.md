# Design Spec: Profile Ergonomics and Project Cleanup

This document outlines the design for streamlining user profile access and performing minor cleanup tasks in the `yomichan_rs` library.

## 1. Objectives
- **Reduce Boilerplate**: Simplify the nested `Ptr` locking logic required to read or mutate the active profile.
- **Functional Flexibility**: Allow users to return values from within configuration closures.
- **Maintain Control**: Keep I/O (database saves) explicit while making in-memory mutations fluent.
- **Project Hygiene**: Fix typos and remove obsolete code.

## 2. Architectural Changes

### 2.1. Scoped Profile Accessors
We will add closure-based helpers to the `Yomichan` struct. These methods handle the global options read-lock, profile lookup, and specific profile lock (read or write) in a single call.

**Added to `Yomichan` in `src/lib.rs`:**
- `with_profile<F, R>(&self, f: F) -> ProfileResult<R>`: Immutable access.
- `with_profile_mut<F, R>(&self, f: F) -> ProfileResult<R>`: Mutable access.
- `with_anki_options<F, R>(&self, f: F) -> ProfileResult<R>`: Immutable sub-access to `AnkiOptions`.
- `with_anki_options_mut<F, R>(&self, f: F) -> ProfileResult<R>`: Mutable sub-access to `AnkiOptions`.

**Rationale:** This pattern follows Rust's idiom for "borrowing" access through a guard-wrapped object, significantly reducing the "handshake" ceremony of:
`ycd.options().read().get_current_profile()?.write()...`

### 2.2. Environment Typo Fix
The `EnvironmentInfo` struct in `src/environment.rs` contains a public field `paltform`. This will be renamed to `platform`.

### 2.3. Method Module Cleanup
The files `src/method_modules/options.rs` and `src/method_modules/dictionary_options.rs` contain large blocks of commented-out code representing an older design strategy. These will be cleaned up to reduce project noise.

## 3. Usage Example

```rust
// Updating multiple settings fluently
ycd.with_profile_mut(|profile| {
    profile.set_language("ja");
    profile.anki_options_mut().set_enable(true);
})?;

// Reading a value while return it
let is_enabled = ycd.with_anki_options(|anki| anki.enable())?;
```

## 4. Verification Plan

### 4.1. Static Analysis
- Run `cargo check --all-features` to ensure no regressions in existing Anki or non-Anki builds.

### 4.2. Functional Testing
- Add a new unit test in `src/lib.rs` (within the `tests` module) that:
    1.  Uses `with_profile_mut` to change a setting.
    2.  Returns a value from the closure.
    3.  Asserts that the setting was changed and the value was returned correctly.
