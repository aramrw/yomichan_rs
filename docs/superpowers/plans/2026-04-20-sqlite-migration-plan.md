# SQLite Migration for DictionaryDatabase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the internal `native_db` (redb) storage engine with `rusqlite` to resolve performance bottlenecks while keeping the public API stable.

**Architecture:** Hybrid storage model using SQLite for indexing and Postcard (via `native_model`) for data blobs. Use SQL `IN` clauses for bulk query optimization.

**Tech Stack:** Rust, `rusqlite` (bundled), `native_model` (postcard).

---

### Task 1: Add Dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add rusqlite to Cargo.toml**

```toml
[dependencies]
# ... other dependencies
rusqlite = { version = "0.33.0", features = ["bundled"] }
```

- [ ] **Step 2: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add rusqlite with bundled feature"
```

---

### Task 2: Update DictionaryDatabase Struct and Initialization

**Files:**
- Modify: `src/database/dictionary_database.rs`

- [ ] **Step 1: Update imports and DictionaryDatabase struct**
Remove `native_db` fields and add `rusqlite::Connection`.

```rust
use rusqlite::{params, Connection, Transaction};
// ... other imports

pub struct DictionaryDatabase<'a> {
    conn: Connection,
    _marker: std::marker::PhantomData<&'a ()>,
}
```

- [ ] **Step 2: Update `new` to handle SQLite initialization and migration**
If the file exists but isn't SQLite, delete it.

```rust
impl<'a> DictionaryDatabase<'a> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if path.exists() {
            // Check if it's a valid SQLite file by trying to open it
            if Connection::open(path).and_then(|c| c.execute("PRAGMA user_version", [])).is_err() {
                let _ = std::fs::remove_file(path);
            }
        }
        let conn = Connection::open(path).expect("Failed to open SQLite database");
        
        // Initial setup (PRAGMAs for speed)
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
        ").expect("Failed to set PRAGMAs");

        let mut db = Self { conn, _marker: std::marker::PhantomData };
        db.setup_tables().expect("Failed to setup tables");
        db
    }

    fn setup_tables(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS summaries (
                title TEXT PRIMARY KEY,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS terms (
                id TEXT PRIMARY KEY,
                expression TEXT,
                reading TEXT,
                expression_reverse TEXT,
                reading_reverse TEXT,
                sequence INTEGER,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_terms_expression ON terms(expression);
            CREATE INDEX IF NOT EXISTS idx_terms_reading ON terms(reading);
            CREATE INDEX IF NOT EXISTS idx_terms_expression_reverse ON terms(expression_reverse);
            CREATE INDEX IF NOT EXISTS idx_terms_reading_reverse ON terms(reading_reverse);
            CREATE INDEX IF NOT EXISTS idx_terms_dictionary ON terms(dictionary);

            CREATE TABLE IF NOT EXISTS term_meta (
                id TEXT PRIMARY KEY,
                term TEXT,
                mode TEXT,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_term_meta_term ON term_meta(term);
            CREATE INDEX IF NOT EXISTS idx_term_meta_dictionary ON term_meta(dictionary);

            CREATE TABLE IF NOT EXISTS kanji (
                character TEXT PRIMARY KEY,
                dictionary TEXT,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS kanji_meta (
                character TEXT PRIMARY KEY,
                dictionary TEXT,
                data BLOB
            );
            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT,
                dictionary TEXT,
                data BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
        ")
    }
}
```

- [ ] **Step 3: Fix Error types**
Update `DictionaryDatabaseError` to wrap `rusqlite::Error`.

- [ ] **Step 4: Commit**

---

### Task 3: Implement Bulk Query Methods

**Files:**
- Modify: `src/database/dictionary_database.rs`

- [ ] **Step 1: Implement `find_terms_bulk` using SQL IN clause**
Construct a query dynamically based on the term list size.

- [ ] **Step 2: Implement `find_term_meta_bulk`**
One or more queries to fetch meta for all terms in the list.

- [ ] **Step 3: Implement `find_terms_exact_bulk`**

- [ ] **Step 4: Implement `get_dictionary_summaries` and `remove_dictionary_by_name`**

- [ ] **Step 5: Commit**

---

### Task 4: Update DictionaryImporter to use SQLite

**Files:**
- Modify: `src/database/dictionary_importer.rs`

- [ ] **Step 1: Update `import_dictionary` to use SQLite Transactions**
Replace `native_db` transaction calls with `rusqlite` transaction calls.

- [ ] **Step 2: Update insertion logic to use Prepared Statements**
Instead of `rwtx.insert(item)`, use `INSERT INTO ...` statements with the serialized `native_model::encode(item)`.

- [ ] **Step 3: Commit**

---

### Task 5: Verification

- [ ] **Step 1: Run `init_db` or related tests**
- [ ] **Step 2: Verify performance improvement (if possible via timing)**
- [ ] **Step 3: Final Commit**
