#[cfg(test)]
mod settings {
    use crate::utils::test_utils::open_test_ycd;

    #[test]
    fn persist_enable_dictionary() {
        // 1. Setup: Enable a dictionary
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                let dicts = pf.options_mut().dictionaries_mut();
                // We need to make sure there is at least one dictionary
                if let Some(first) = dicts.first_mut() {
                    first.1.enabled = true;
                }
            }).unwrap();
            ycd.save_settings().unwrap();
        }

        // 2. Verify: Re-open and check setting
        {
            let ycd = open_test_ycd();
            let is_enabled = ycd.with_profile(|pf| {
                let dicts = pf.options().dictionaries();
                dicts.first().map(|(_, d)| d.enabled).unwrap_or(false)
            }).unwrap();
            assert!(is_enabled);
        }
    }
}
