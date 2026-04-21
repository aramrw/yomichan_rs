# High-Performance Dictionary Importer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce dictionary import time from ~17s to ~5s by parallelizing serialization and using multi-value SQLite batch inserts.

**Architecture:** 
1. Pre-serialize data blobs in parallel using `rayon` before hitting the database lock.
2. Replace single-row `INSERT` statements with chunked multi-value `INSERT` statements (100 rows per batch).
3. Apply performance-tuning PRAGMAs specifically for the import session.

**Tech Stack:** Rust, rusqlite, rayon, native_model (postcard).

---

### Task 1: PRAGMA Tuning Helpers

**Files:**
- Modify: `src/database/dictionary_database.rs`

- [ ] **Step 1: Add PRAGMA optimization methods**

```rust
impl DictionaryDatabase {
    pub fn begin_import_session(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock();
        conn.execute_batch("
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -200000;
        ")
    }

    pub fn end_import_session(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock();
        conn.execute_batch("
            PRAGMA temp_store = DEFAULT;
            PRAGMA cache_size = -2000;
        ")
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/database/dictionary_database.rs
git commit -m "db: add import session pragma helpers"
```

---

### Task 2: Parallel Term Serialization

**Files:**
- Modify: `src/database/dictionary_importer.rs`

- [ ] **Step 1: Define SerializedTerm struct**

```rust
struct SerializedTerm {
    id: String,
    expression: String,
    reading: String,
    expression_reverse: String,
    reading_reverse: String,
    sequence: Option<i64>,
    dictionary: String,
    data: Vec<u8>,
}
```

- [ ] **Step 2: Implement parallel serialization in `import_dictionary`**

Replace the sequential term insertion loop with a `par_iter` collection.

```rust
    let serialized_terms: Vec<SerializedTerm> = external_data.term_list.into_par_iter().map(|t| {
        let entry = DatabaseTermEntry {
            id: t.0.clone(),
            expression: t.1.clone(),
            reading: t.2.clone(),
            expression_reverse: t.3.clone(),
            reading_reverse: t.4.clone(),
            definition_tags: t.5.map(|s| s.to_string()),
            tags: t.6.map(|s| s.to_string()),
            rules: t.7.to_string(),
            score: t.8,
            glossary: t.9.into_iter().map(|g| match g {
                yomichan_importer::structured_content::TermGlossaryGroupType::Content(c) => {
                    TermGlossaryGroupType::Content(TermGlossaryContentGroup { plain_text: c.plain_text, html: c.html })
                }
                yomichan_importer::structured_content::TermGlossaryGroupType::Deinflection(d) => {
                    TermGlossaryGroupType::Deinflection(TermGlossaryDeinflection { form_of: d.form_of, rules: d.rules.iter().map(|s| s.to_owned()).collect() })
                }
            }).collect(),
            sequence: t.10,
            term_tags: t.11.as_ref().map(|s| s.to_string()),
            dictionary: t.12.clone(),
            file_path: t.13.clone(),
        };
        let data_blob = encode(&entry).expect("Failed to encode");
        SerializedTerm {
            id: entry.id,
            expression: entry.expression,
            reading: entry.reading,
            expression_reverse: entry.expression_reverse,
            reading_reverse: entry.reading_reverse,
            sequence: entry.sequence.map(|s| s as i64),
            dictionary: entry.dictionary,
            data: data_blob,
        }
    }).collect();
```

- [ ] **Step 3: Commit**

```bash
git add src/database/dictionary_importer.rs
git commit -m "importer: implement parallel term serialization"
```

---

### Task 3: Multi-Value Batch Insertion for Terms

**Files:**
- Modify: `src/database/dictionary_importer.rs`

- [ ] **Step 1: Implement chunked batch insertion**

Replace the existing transaction loop with a chunked multi-value `INSERT`.

```rust
    {
        db.begin_import_session().expect("Failed to start import session");
        let mut conn_lock = db.conn.lock();
        let conn = conn_lock.unchecked_transaction().expect("Failed to start transaction");
        
        for chunk in serialized_terms.chunks(100) {
            let mut sql = String::from("INSERT OR REPLACE INTO terms (id, expression, reading, expression_reverse, reading_reverse, sequence, dictionary, data) VALUES ");
            let placeholders: Vec<String> = (0..chunk.len())
                .map(|_| "(?, ?, ?, ?, ?, ?, ?, ?)")
                .collect();
            sql.push_str(&placeholders.join(", "));

            let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(chunk.len() * 8);
            for term in chunk {
                params.push(&term.id);
                params.push(&term.expression);
                params.push(&term.reading);
                params.push(&term.expression_reverse);
                params.push(&term.reading_reverse);
                params.push(&term.sequence);
                params.push(&term.dictionary);
                params.push(&term.data);
            }

            conn.execute(&sql, rusqlite::params_from_iter(params)).expect("Batch insert failed");
        }
        
        conn.commit().expect("Failed to commit");
        drop(conn_lock);
        db.end_import_session().expect("Failed to end import session");
    }
```

- [ ] **Step 2: Commit**

```bash
git add src/database/dictionary_importer.rs
git commit -m "importer: implement multi-value batch insertion for terms"
```

---

### Task 4: Optimized Batching for Kanji, Tags, and Meta

**Files:**
- Modify: `src/database/dictionary_importer.rs`

- [ ] **Step 1: Refactor `insert_kanji_batched` to use multi-value INSERT**

```rust
fn insert_kanji_batched(db: Arc<DictionaryDatabase>, list: Vec<DatabaseKanjiEntry>) -> Result<(), rusqlite::Error> {
    let mut conn_lock = db.conn.lock();
    let conn = conn_lock.unchecked_transaction()?;
    for chunk in list.chunks(100) {
        let mut sql = String::from("INSERT OR REPLACE INTO kanji (character, dictionary, data) VALUES ");
        let placeholders: Vec<String> = (0..chunk.len()).map(|_| "(?, ?, ?)").collect();
        sql.push_str(&placeholders.join(", "));
        
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        let mut encoded_blobs = Vec::new();
        for item in chunk {
            encoded_blobs.push(encode(item).unwrap());
        }
        for (i, item) in chunk.iter().enumerate() {
            params.push(&item.character);
            params.push(&item.dictionary);
            params.push(&encoded_blobs[i]);
        }
        conn.execute(&sql, rusqlite::params_from_iter(params))?;
    }
    conn.commit()?;
    Ok(())
}
```

- [ ] **Step 2: Repeat pattern for `insert_tags_batched` and `insert_kanji_meta_batched`**
- [ ] **Step 3: Update `term_meta_list` insertion loop in `import_dictionary` to use batching**

- [ ] **Step 4: Commit**

```bash
git add src/database/dictionary_importer.rs
git commit -m "importer: optimize kanji, tags, and meta insertion with batching"
```

---

### Task 5: Final Verification

- [ ] **Step 1: Run tests to ensure no regressions**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 2: Verify performance improvement**

Run: `./test.sh` (or your benchmark command)
Expected: Import time reduced significantly (target < 6s)
