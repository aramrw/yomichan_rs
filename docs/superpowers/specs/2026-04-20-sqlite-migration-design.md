# Spec: SQLite Migration for DictionaryDatabase

**Goal:** Replace the internal `native_db` (redb) storage engine with `rusqlite` to resolve performance bottlenecks caused by multiple secondary keys and write amplification.

## Architecture

We will implement a **Hybrid Storage Model** within the existing `DictionaryDatabase` struct:
- **Relational Indexing:** Fields used for searching (expression, reading, sequence, etc.) will be stored in standard SQLite columns with B-Tree indices.
- **Blob Storage:** The full Rust structs will be serialized using `postcard` (via the existing `native_model` implementation) and stored in a `data` BLOB column.

## Schema Design

### Tables

| Table | Index Columns | Data Column |
| :--- | :--- | :--- |
| `summaries` | `title` (PK) | `data` (DictionarySummary) |
| `terms` | `id` (PK), `expression`, `reading`, `expression_reverse`, `reading_reverse`, `sequence`, `dictionary` | `data` (DatabaseTermEntry) |
| `term_meta` | `id` (PK), `term`, `mode`, `dictionary` | `data` (DatabaseMetaFrequency/Pitch/Phonetic) |
| `kanji` | `character` (PK), `dictionary` | `data` (DatabaseKanjiEntry) |
| `kanji_meta` | `character` (PK), `dictionary` | `data` (DatabaseKanjiMeta) |
| `tags` | `id` (PK), `name`, `dictionary` | `data` (DatabaseTag) |

### Indices
- `terms`: `expression`, `reading`, `expression_reverse`, `reading_reverse`, `sequence`, `dictionary`
- `term_meta`: `term`, `dictionary`
- `kanji`: `character`, `dictionary`
- `tags`: `name`, `dictionary`

## Implementation Details

### Serialization
We will use `native_model::encode` and `native_model::decode` to handle the `data` BLOBs. This ensures we leverage the existing `postcard` logic and `#[native_model]` IDs.

### Query Strategy
- **Bulk Queries:** Methods like `find_terms_bulk` will be optimized by using SQL `IN` clauses to fetch multiple records in a single database round-trip.
- **Suffix Search:** Suffix matching will be handled by querying the `_reverse` columns with a prefix match, maintaining current logic but moving the execution into SQLite.

### Migration Strategy
`DictionaryDatabase::new` will:
1. Attempt to open the file as a SQLite database.
2. If it fails (e.g., old `redb` format), it will delete the file and create a new SQLite database.
3. This is acceptable as the user confirmed breaking changes are fine and re-importing is the expected workflow.

## Success Criteria
- **Performance:** Significant reduction in query time for bulk operations as shown in flamegraphs.
- **Stability:** Passing existing test suite (with re-importing).
- **Isolation:** No changes required to `Translator` or other high-level components.
