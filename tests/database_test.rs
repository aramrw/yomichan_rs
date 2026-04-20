#[cfg(test)]
mod db_tests {
    use yomichan_rs::database::dictionary_database::{DictionarySet, TermExactQueryRequest};
    use yomichan_rs::test_utils;
    use indexmap::IndexSet;
    use yomichan_rs::Yomichan;

    #[test]
    fn test_search_es() {
        let db_path = &*test_utils::TEST_PATHS.tests_yomichan_db_path;
        let mut ycd = Yomichan::new(db_path.parent().unwrap()).unwrap();
        ycd.set_language("en").unwrap();
        ycd.with_profile_mut(|p| p.options_mut().scanning_mut().alphanumeric = true).unwrap();
        
        let req = TermExactQueryRequest {
            term: "bueno".to_string(),
            reading: "".to_string(),
        };
        let mut dicts = IndexSet::new();
        dicts.insert("wty-es-en".to_string());
        
        let db = &test_utils::SHARED_DB_INSTANCE;
        let entries = db.find_terms_exact_bulk(&[req], &dicts).unwrap();
        assert!(!entries.is_empty(), "Should have found bueno");
    }
}
