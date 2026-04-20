# Library Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clean up the `yomichan_rs` codebase by removing unused crates, dependencies, commented-out code, and redundant types to reveal the core project structure.

**Architecture:** Surgical removal of dead code and workspace pruning. No changes to core logic.

**Tech Stack:** Rust, Cargo

---

### Task 1: Workspace & Root Cleanup

**Files:**
- Modify: `Cargo.toml`
- Delete: `crates/kanji_processor/`
- Delete: `crates/module_macros/`

- [ ] **Step 1: Remove unused crates from filesystem**
Run: `rm -rf crates/kanji_processor crates/module_macros`

- [ ] **Step 2: Prune Cargo.toml**
Modify `Cargo.toml`:
- Remove `rmpv` from `[dependencies]`.
- Remove `kanji_processor` and `module_macros` path dependencies.
- Remove `[workspace]` members if empty.

- [ ] **Step 3: Run cargo check to verify workspace integrity**
Run: `cargo check`
Expected: PASS (with existing warnings)

- [ ] **Step 4: Commit**
```bash
git add Cargo.toml
git commit -m "chore: remove unused crates and dependencies"
```

### Task 2: Strip Commented-Out Code and Unused Types

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/translator.rs`
- Modify: `src/errors.rs`
- Modify: `src/backend.rs`

- [ ] **Step 1: Clean src/lib.rs**
- Remove `#![allow(unused)]` if present (to see actual warnings).
- Remove commented-out modules and imports.
- Remove outdated example code comments.

- [ ] **Step 2: Clean src/translator.rs**
- Remove all large blocks of commented-out code (JS notes, old logic).
- Remove unused helper functions identified during research.

- [ ] **Step 3: Clean src/errors.rs**
- Remove `YomichanResult` enum.
- Remove unused macros and helper modules (`error_helpers`).

- [ ] **Step 4: Clean src/backend.rs**
- Remove commented-out tests and debug prints in `ycd_tests`.

- [ ] **Step 5: Run cargo test**
Run: `cargo test`
Expected: PASS

- [ ] **Step 6: Commit**
```bash
git add src/lib.rs src/translator.rs src/errors.rs src/backend.rs
git commit -m "chore: strip commented-out code and unused types"
```

### Task 3: Resolve Compiler Warnings

**Files:**
- Modify: Multiple files in `src/`

- [ ] **Step 1: Fix unused imports**
Run: `cargo check` and remove any `unused import` warnings in `src/`.

- [ ] **Step 2: Fix unused variables**
Run: `cargo check` and prefix unused variables with `_` or remove them.

- [ ] **Step 3: Fix unused macros**
Remove any `unused macro definition` warnings.

- [ ] **Step 4: Final verification**
Run: `cargo check`
Expected: 0 warnings in `yomichan_rs` crate.

- [ ] **Step 5: Commit**
```bash
git add src/
git commit -m "chore: resolve compiler warnings"
```
