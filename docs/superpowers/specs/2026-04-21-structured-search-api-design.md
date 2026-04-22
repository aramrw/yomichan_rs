# Specification: Structured Search API Refactor

## 1. Objective
Refactor the Yomichan RS `search` API to provide a predictable, hierarchical data structure (`SearchResult` -> `SearchSegment` -> `entries`) instead of the current flat, ambiguous `Option<Vec<TermSearchResultsSegment>>` pattern.

## 2. API Design

### 2.1 Data Structures
```rust
pub struct SearchResult {
    pub original_text: String,
    pub segments: Vec<SearchSegment>,
}

pub struct SearchSegment {
    pub text: String,
    pub entries: Vec<TermDictionaryEntry>,
}
```

### 2.2 Function Signatures
- **`Yomichan::search_structured(text: &str) -> Result<SearchResult, YomichanError>`**
- The method must encapsulate the logic currently in `SentenceParser::parse` and bubble up database or profile access errors as `YomichanError`.

## 3. Implementation Requirements

### 3.1 Scanner Core (`src/scanner/core.rs`)
- **`SentenceParser`**:
    - Must be made `pub` if accessed from `lib.rs`.
    - `parse_to_structured(results: TermSearchResults) -> Vec<SearchSegment>`:
        - Implement robust segmentation using the "longest match" algorithm.
        - **Critical**: Use character-based indexing (`char_indices`) to prevent out-of-bounds byte slicing panics on UTF-8 text.
        - Handle segments without dictionary entries as `entries: vec![]`.
        - Deduplicate dictionary entries per segment.

### 3.2 Main API (`src/lib.rs`)
- **`Yomichan` Implementation**:
    - Implement `search_structured` as the primary entry point.
    - Ensure it handles profile fetching, scanning, and parsing in one cohesive flow.
    - (Optional) Retain a `search` alias for backward compatibility if needed, but ensure the core logic is consolidated.

### 3.3 Robustness & Safety
- **Error Handling**: Use `YomichanError` variants instead of returning `Option`.
- **Slicing**: Avoid byte-indexing into strings where possible. Use `char_indices` to iterate and segment correctly.
- **Logging**: Include `tracing` (info/warn/error) in the search flow to allow runtime debugging of segmentation or search failures.

## 4. Verification Plan
- **Unit Testing**: 
    - Create a suite in `tests/` that verifies `SearchResult` correctly populates segments and entries.
    - Ensure the logic correctly handles both matched terms and "passthrough" (unmatched) text.
- **Panic Prevention**: Ensure the segmentation logic cannot panic on edge cases (e.g., end-of-string slicing).
- **Environment**: Ensure the test dictionary environment (`init_db`) is correctly initialized before running the structured search tests.
