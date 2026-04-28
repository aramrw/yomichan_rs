use yomichan_rs::settings::core::{YomichanOptions, DictionaryOptions};
use native_model::encode;

#[test]
fn test_encode_options_with_dict() {
    let mut options = YomichanOptions::new();
    let dict_opts = DictionaryOptions::new("test_dict".to_string());
    
    // Get the default profile and add the dict
    {
        let profile = options.get_current_profile().unwrap();
        let mut profile_guard = profile.write_arc();
        profile_guard.options_mut().dictionaries_mut().insert("test_dict".to_string(), dict_opts);
    }
    
    let res = encode(&options);
    assert!(res.is_ok(), "Failed to encode YomichanOptions with dict: {:?}", res.err());
}
