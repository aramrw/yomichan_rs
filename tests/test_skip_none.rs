//use yomichan_rs::database::dictionary_database::DatabaseTermEntry;
//use native_model::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct BrokenStruct {
    pub a: Option<String>,
    pub b: String,
}

#[test]
fn test_skip_serializing_none_postcard() {
    let t = BrokenStruct {
        a: None,
        b: "hello".to_string(),
    };
    let bytes = postcard::to_allocvec(&t).unwrap();
    let decoded: BrokenStruct = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(t, decoded);
}
