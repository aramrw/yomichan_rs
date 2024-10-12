use crate::language::languages::get_all_language_transform_descriptors;

use super::transformer_d::LanguageTransformer;
use std::collections::HashMap;

pub struct MultiLanguageTransformer<'a> {
    language_transformers: HashMap<&'a str, LanguageTransformer>,
}

impl<'a> MultiLanguageTransformer<'a> {
    fn new() -> Self {
        Self {
            language_transformers: HashMap::new(),
        }
    }

    // fn prepare() -> Result<()> {
    //     let languages_with_transformers = get_all_language_transform_descriptors();
    //     for lang_and_transforms in languages_with_transformers {
    //         let language_transformer = LanguageTransformer::new();
    //         language_transformer.add_descriptor(*lang_and_transforms.language_transforms)
    //     }
    // }
}
