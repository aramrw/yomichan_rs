use yomichan_rs::database::dictionary_database::DatabaseTermEntry;
use native_model::decode;
use rusqlite::Connection;

fn main() {
    let conn = Connection::open("tests/yomichan_rs/db.ycd").unwrap();
    let mut stmt = conn.prepare("SELECT data FROM terms LIMIT 1").unwrap();
    let mut rows = stmt.query([]).unwrap();
    if let Some(row) = rows.next().unwrap() {
        let data: Vec<u8> = row.get(0).unwrap();
        println!("Blob size: {}", data.len());
        match decode::<DatabaseTermEntry>(data) {
            Ok((entry, _)) => println!("Decoded entry expression: {}", entry.expression),
            Err(e) => println!("Failed to decode: {:?}", e),
        }
    }
}
