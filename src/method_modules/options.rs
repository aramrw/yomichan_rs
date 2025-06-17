use super::create_method_module;
use crate::{settings, Yomichan};

create_method_module!(Yomichan, ModOptions, options);

// impl<'a> ModOptions<'a> {
//     fn get_current_profile_mut(&mut self) -> &mut settings::Profile {
//         self.ycd.backend.options.get_current_profile_mut()
//     }
// }
