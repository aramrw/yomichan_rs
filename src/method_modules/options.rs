use module_macros::ref_variant;

use super::create_method_module_mut;
use crate::{
    settings::{self, Profile},
    Yomichan,
};

#[ref_variant]
impl<'a> ModOptionsMut<'a> {
    fn get_current_profile_mut(&mut self) -> &mut Profile {
        self.ycd.backend.options.get_current_profile_mut()
    }
    fn get_all_profiles_mut(&mut self) -> &mut Vec<Profile> {
        &mut self.ycd.backend.options.profiles
    }
}

#[cfg(test)]
mod mod_options {
    use crate::{test_utils::TEST_PATHS, Yomichan};
    use std::sync::{LazyLock, RwLock};
    static YCD: LazyLock<RwLock<Yomichan>> = LazyLock::new(|| {
        let mut ycd = Yomichan::new(&TEST_PATHS.tests_yomichan_db_path).unwrap();
        // no need to set language, you do it in the test functions
        RwLock::new(ycd)
    });

    #[test]
    fn mod_options() {
        let ycd = YCD.read().unwrap();
        ycd.mod_options().get_all_profiles();
        ycd.mod_options().get_all_profiles_owned();
        ycd.mod_options().get_current_profile();
        ycd.mod_options().get_current_profile_owned();
    }
}
