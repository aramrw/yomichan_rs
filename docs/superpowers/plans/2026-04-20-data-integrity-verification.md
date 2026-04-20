# Data Integrity Verification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify that the data in the SQLite database can be correctly decoded into `DatabaseTermEntry` and investigate search failures for specific terms.

**Architecture:** Add a diagnostic test to the database module to verify decoding. Use existing search tests to trace "bueno" search failure.

**Tech Stack:** Rust, rusqlite, native_model, postcard.

---

### Task 1: Add Data Integrity Test

**Files:**
- Modify: `src/database/dictionary_database.rs`

- [ ] **Step 1: Add the `verify_data_integrity` test case**

```rust
    #[test]
    fn verify_data_integrity() {
        let ycd = &test_utils::SHARED_DB_INSTANCE;
        let conn = ycd.conn.lock();
        let data: Vec<u8> = conn.query_row("SELECT data FROM terms LIMIT 1", [], |row| row.get(0)).expect("Failed to get data from terms");
        let decoded = decode::<DatabaseTermEntry>(data);
        match decoded {
            Ok((entry, _)) => {
                println!("Successfully decoded entry: {:?}", entry.id);
            }
            Err(e) => {
                panic!("Failed to decode DatabaseTermEntry: {:?}", e);
            }
        }
    }
```

- [ ] **Step 2: Run the test to verify data integrity**

Run: `cargo test database::dictionary_database::ycd::verify_data_integrity -- --nocapture`
Expected: PASS and print "Successfully decoded entry: ..."

### Task 2: Investigate "bueno" Search Failure

**Files:**
- Modify: `src/text_scanner.rs` (if needed for tracing)

- [ ] **Step 1: Run existing search test for "bueno"**

Run: `cargo test text_scanner::textscanner::search_dbg -- --nocapture`
Expected: Inspect output to see if "bueno" is found.

- [ ] **Step 2: Trace `find_terms_bulk` for "bueno"**

If "bueno" is missing, add logging to `find_terms_bulk` in `src/database/dictionary_database.rs` to see what's being queried and what's returned.

```rust
// In find_terms_bulk
println!("Querying for terms: {:?}", processed_term_list);
// ...
println!("SQL Query: {}", query);
// ...
println!("Rows found: {}", rows_count); // need to add a counter
```

### Task 3: investigate encoding mismatch (if Task 1 fails)

- [ ] **Step 1: Compare `native_model` configuration**
Check all structs used in `DatabaseTermEntry` for consistent `native_model` attributes and IDs.
Specifically check `TermGlossaryGroupType` and its sub-structs.
