use rusqlite::Connection;

fn main() {
    let path = "test_pragma.db";
    let _ = std::fs::remove_file(path);
    let conn = Connection::open(path).unwrap();
    
    // Test if execute returns an error for PRAGMA user_version
    let result = conn.execute("PRAGMA user_version", []);
    println!("PRAGMA user_version result: {:?}", result);
    
    // Test query_row
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    println!("PRAGMA user_version via query_row: {}", version);
}
