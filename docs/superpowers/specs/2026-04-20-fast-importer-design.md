# Design Spec: High-Performance Dictionary Importer

Move dictionary import performance from ~17s to ~5s for large dictionaries (1.4M+ terms) by optimizing serialization and database insertion patterns.

## Problem Statement
The current importer performs serialization and database insertion sequentially within a locked SQLite transaction.
1. **Serialization Bottleneck:** `native_model` encoding happens row-by-row on a single thread.
2. **IO Bottleneck:** Individual `INSERT` statements for 1.4M rows incur massive overhead in the SQLite virtual machine.
3. **Lock Contention:** The database connection is held under a `Mutex` for the entire duration of the heavy serialization and insertion loop.

## Proposed Changes

### 1. Parallel Pre-Serialization
Move the construction of `DatabaseTermEntry` and its Postcard serialization out of the database transaction and onto multiple threads using `rayon`.

- Define a `SerializedTerm` intermediate structure.
- Use `external_data.term_list.into_par_iter()` to perform encoding in parallel.
- Collect into a `Vec<SerializedTerm>` before acquiring the database lock.

### 2. Multi-Value Batch Insertion
Implement a batching engine that uses multi-value `INSERT` statements.
- **Batch Size:** ~100 rows per statement (800 parameters). This stays safely under SQLite's default `SQLITE_LIMIT_VARIABLE_NUMBER` (999 on older versions, 32766 on newer).
- **SQL Template:** `INSERT OR REPLACE INTO terms (...) VALUES (?,?,?,?,?,?,?,?), (?,?,?,?,?,?,?,?), ...`
- **Parameter Flattening:** Use `rusqlite::params_from_iter` to pass all parameters for the batch in one call.

### 3. PRAGMA Optimization
Apply session-specific PRAGMAs before starting the import transaction:
- `PRAGMA temp_store = MEMORY;` (Reduces disk IO for temporary tables/indices during build).
- `PRAGMA cache_size = -200000;` (Increases page cache to ~200MB to keep indices in RAM).

### 4. Consolidated Insertion Logic
Refactor `insert_kanji_batched`, `insert_tags_batched`, and `insert_kanji_meta_batched` to use the same multi-value batching pattern for consistency and speed.

## Data Structures

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

## Performance Targets
- **Serialization:** Should drop from several seconds to <1s on multi-core systems.
- **Insertion:** Multi-value batches should reduce opcode execution overhead by 90%+.
- **Overall:** Aiming for 5s total import time for a 1.4M entry dictionary.

## Verification Plan
1. **Correctness:** Run existing integration tests to ensure data integrity is maintained.
2. **Performance:** Re-run the benchmarks in `BOTTLENECKS.md` to confirm the speedup.
3. **Memory:** Monitor RAM usage during import to ensure the ~500MB pre-serialization buffer is handled gracefully.
