//use yomichan_rs::database::dictionary_database::DatabaseTermEntry;
use yomichan_importer::structured_content::{TermGlossaryContentGroup, TermGlossaryGroupType};

#[test]
fn test_glossary_postcard() {
    let glossary = vec![TermGlossaryGroupType::Content(TermGlossaryContentGroup {
        plain_text: "test".into(),
        html: None,
    })];
    let bytes = postcard::to_allocvec(&glossary).unwrap();
    let decoded: Vec<TermGlossaryGroupType> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(glossary, decoded);
}
