# Search API Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Yomichan RS `search` API to return a structured, hierarchical `SearchResult` instead of the current ambiguous flat structure.

**Architecture:** Introduce `SearchResult` and `SearchSegment` data structures. Implement `Yomichan::search_structured` that consolidates scanning and parsing into a single error-bubbling API.

**Tech Stack:** Rust, `yomichan_rs` core.

---

### Task 1: Define Data Structures

**Files:**
- Modify: `src/lib.rs` (or create a new `models/search.rs` if preferred - checking existing structure, `models/mod.rs` exists, I will put it in `models/search.rs`)
- Test: `tests/test_structured_search.rs`

- [ ] **Step 1: Create `src/models/search.rs`**

```rust
use crate::models::dictionary::TermDictionaryEntry;

pub struct SearchResult {
    pub original_text: String,
    pub segments: Vec<SearchSegment>,
}

pub struct SearchSegment {
    pub text: String,
    pub entries: Vec<TermDictionaryEntry>,
}
```

- [ ] **Step 2: Expose new models in `src/models/mod.rs`**

- [ ] **Step 3: Add unit test to `tests/test_structured_search.rs` for structure**

Verify that we can construct a `SearchResult`.

- [ ] **Step 4: Commit**

### Task 2: Implement Structured Scanner Core

**Files:**
- Modify: `src/scanner/core.rs`

- [ ] **Step 1: Implement `parse_to_structured` in `SentenceParser`**

Ensure it uses `char_indices` and handles unmatched segments as empty entry lists.

- [ ] **Step 2: Add unit tests to `src/scanner/core.rs`**

Verify longest-match segmentation.

- [ ] **Step 3: Commit**

### Task 3: Implement Main API `search_structured`

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add `search_structured` to `Yomichan`**

Integrate backend scanner and parser. Handle errors properly.

- [ ] **Step 2: Add integration test**

Use `tests/test_structured_search.rs` with the `test_dicts` to verify end-to-end flow.

- [ ] **Step 3: Commit**

### Task 4: Cleanup & Verification

**Files:**
- Modify: `tests/test_structured_search.rs`

- [ ] **Step 1: Ensure all tests pass**

- [ ] **Step 2: Final Verification**

Check against spec requirement (panic prevention, logging).

- [ ] **Step 3: Commit**
