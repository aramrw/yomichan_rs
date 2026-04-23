#[cfg(test)]
mod settings {
    use crate::utils::test_utils::open_test_ycd;
    use pretty_assertions::assert_eq;

    #[test]
    fn persist_enable_dictionary() {
        let mut initial_enabled = false;
        // enable a dictionary
        {
            let ycd = open_test_ycd();
            ycd.with_profile_mut(|pf| {
                let dicts = pf.options_mut().dictionaries_mut();
                let first = dicts
                    .first_mut()
                    .expect("no dictionarys\n  [help] run init_db");
                let enabled = &mut first.1.enabled;
                initial_enabled = *enabled;
                *enabled = !&*enabled;
            })
            .unwrap();
            ycd.save_settings().unwrap();
        }

        // verify
        {
            let ycd = open_test_ycd();
            let is_enabled = ycd
                .with_profile(|pf| {
                    let dicts = pf.options().dictionaries();
                    dicts.first().map(|(_, d)| d.enabled).unwrap_or(false)
                })
                .unwrap();
            // is the opposite of what it was initially
            assert_eq!(!initial_enabled, is_enabled);
        }
    }
}
