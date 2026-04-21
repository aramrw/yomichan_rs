#[cfg(test)]
mod tests {
    use yomichan_rs::utils::test_utils::YCD;

    #[test]
    fn test_search_structured_returns_expected_segments() {
        let ycd = &YCD;
        ycd.set_language("es").unwrap();
        
        // This is the new, structured API we are verifying
        let result = ycd.search_structured("bueno").expect("Search should succeed");
        
        // Assert: We expect at least one segment for 'bueno'
        assert!(!result.segments.is_empty(), "Result should contain segments");
        
        // Verify structure: Segment contains text and a non-empty list of entries
        for segment in &result.segments {
            assert!(!segment.text.is_empty(), "Segment text should not be empty");
            // Optionally check for entries
        }
        
        assert_eq!(result.original_text, "bueno");
    }
}
