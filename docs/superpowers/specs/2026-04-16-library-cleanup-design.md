# Spec: Library Cleanup and "De-Gunking"

The goal of this task is to clean up the `yomichan_rs` codebase by removing unused crates, dependencies, commented-out code, and redundant types. This will reveal the "outline" of the project and make it easier to perform future architectural changes.

## 1. Workspace & Root Cleanup

### Remove Unused Crates
- Delete `crates/kanji_processor/`
- Delete `crates/module_macros/`

### Prune `Cargo.toml`
- Remove `rmpv` dependency.
- Remove `kanji_processor` and `module_macros` from `[dependencies]` and `[workspace]`.
- Remove `module_macros` from `[dependencies]` (currently commented out).

## 2. Codebase "De-Gunking"

### Strip Commented-Out Code
- **`src/lib.rs`**: Remove commented-out imports and example code that is no longer valid.
- **`src/translator.rs`**: Remove the large blocks of commented-out JavaScript-to-Rust conversion notes and old implementations.
- **`src/errors.rs`**: Remove unused macros and helper functions that are commented out or purely "just-in-case".

### Remove Unused Types & Functions
- **`src/errors.rs`**: Remove `YomichanResult`.
- **`src/lib.rs`**: Remove any unused `Ptr` helper methods if they are identified during the cleanup.

## 3. Warning Resolution

### Fix Compiler Warnings
- Resolve unused imports across all files in `src/`.
- Resolve unused variables (prefix with `_` or remove) in `src/`.
- Resolve unused macro definitions.

## 4. Verification Plan

### Automated Tests
- Run `cargo test` to ensure that removing "dead" code hasn't accidentally broken functional paths.
- Run `cargo check` to verify that all warnings have been resolved and no new ones were introduced.

### Manual Verification
- Verify that the project still builds in release mode: `cargo build --release`.
