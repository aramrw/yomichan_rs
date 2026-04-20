use compact_str::CompactString;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestStruct {
    s1: CompactString,
    s2: Option<CompactString>,
}

#[test]
fn test_compact_string_postcard() {
    let t = TestStruct {
        s1: "test".into(),
        s2: None,
    };
    let bytes = postcard::to_allocvec(&t).unwrap();
    let decoded: TestStruct = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(t, decoded);
}
