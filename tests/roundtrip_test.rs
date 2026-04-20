#[cfg(test)]
mod db_tests {
    use importer::structured_content::{TermGlossaryContentGroup, TermGlossaryGroupType};
    use native_model::{decode, encode};
    use yomichan_rs::database::dictionary_database::DatabaseTermEntry;

    #[test]
    fn test_serialization_roundtrip() {
        let entry = DatabaseTermEntry {
            id: "test-id".to_string(),
            expression: "español".to_string(),
            reading: "español".to_string(),
            expression_reverse: "loñapse".to_string(),
            reading_reverse: "loñapse".to_string(),
            definition_tags: Some("test-tag".to_string()),
            tags: None,
            rules: "v".to_string(),
            score: 0,
            glossary: vec![TermGlossaryGroupType::Content(TermGlossaryContentGroup {
                plain_text: "test".into(),
                html: None,
            })],
            sequence: Some(0),
            term_tags: Some("".to_string()),
            dictionary: "wty-es-en".to_string(),
            file_path: "test.json".to_string(),
        };

        println!("Original entry: {:?}", entry);

        let encoded = encode(&entry).expect("Failed to encode");
        println!("Encoded length: {}", encoded.len());
        println!("Encoded bytes: {:?}", encoded);

        let decoded: Result<(DatabaseTermEntry, u32), _> = decode(encoded.clone());

        match decoded {
            Ok((decoded_entry, _)) => {
                println!("Decoded entry: {:?}", decoded_entry);
                assert_eq!(
                    entry, decoded_entry,
                    "Decoded entry does not match original"
                );
            }
            Err(e) => {
                panic!("Failed to decode: {:?}. Data bytes: {:?}", e, encoded);
            }
        }
    }
}
