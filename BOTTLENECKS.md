The flamegraph and database analysis reveal that the primary bottleneck during imports is the INSERT operation into
  the terms table, which is expected given the sheer volume of data (over 1.4 million entries, averaging ~345 bytes per
  data blob, totaling ~435MB just for the terms table content).

  The SQLite INSERT performance is limited by:
   1. Serialization of the data blob: The native_model encoding to postcard happens row-by-row inside the term_list
      iteration.
   2. Transaction overhead: While you are using a transaction (unchecked_transaction), the sheer number of execute calls
      and the associated data transfer to SQLite (via rusqlite::params!) for 1.4 million rows is the dominant factor.
   3. Indexing: The terms table has five indices (expression, reading, expression_reverse, reading_reverse, dictionary).
      Each INSERT must update all five B-trees. This is verified by the index sizes: sqlite_autoindex_terms_1 (67MB) and
      the four other indices totaling ~101MB.

  Recommendations for Optimization:
   - Batching: SQLite's INSERT speed is excellent, but 1.4 million individual INSERT commands will always be slower than
     bulk inserts. If rusqlite supports it, use multi-value insert statements (INSERT INTO terms (...) VALUES (...),
     (...), ...) to reduce the number of opcode executions.
   - Defer Indexing: A standard technique to speed up bulk loading is to DROP all non-primary key indices before
     importing, import all data, and CREATE the indices once finished. SQLite can build indices much faster from a table
     scan than by incrementally updating them per row.
   - Parallel Encoding: The import_dictionaries function currently uses par_iter() for the dictionary list level, but
     import_dictionary (the per-dictionary function) imports synchronously. You could potentially parallelize the
     conversion of external_data.term_list into DatabaseTermEntry objects, although the current rusqlite connection is
     Arc<Mutex<Connection>>, which serializes the writes anyway.
