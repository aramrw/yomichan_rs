use snafu::{OptionExt, Whatever};

use crate::{
    dictionary::{InflectionRule, InflectionRuleChain, InflectionRuleChainCandidate},
    language::languages::get_all_language_transform_descriptors,
};

use super::{transformer_d::LanguageTransformer, transformer_internal_d::TransformedText};
use std::collections::HashMap;

pub struct MultiLanguageTransformer {
    pub language_transformers: HashMap<String, LanguageTransformer>,
}

impl MultiLanguageTransformer {
    fn new() -> Self {
        Self {
            language_transformers: HashMap::new(),
        }
    }

    fn prepare(&mut self) -> Result<(), Whatever> {
        let languages_with_transformers = get_all_language_transform_descriptors();
        for descriptor in languages_with_transformers {
            let mut language_transformer = LanguageTransformer::new();
            language_transformer.add_descriptor(&descriptor.language_transforms);
            self.language_transformers.insert(
                descriptor.language_transforms.language.clone(),
                language_transformer,
            ).with_whatever_context(|| {
                format!("Could not insert descriptor.language_transforms.language ({}) into `MultiLanguageTransformer's `language_transformers",descriptor.language_transforms.language)
            })?;
        }

        Ok(())
    }

    pub fn get_condition_flags_from_parts_of_speech(
        &self,
        language: impl AsRef<str>,
        parts_of_speech: &[impl AsRef<str>],
    ) -> usize {
        let language = language.as_ref();
        if let Some(language_transformer) = self.language_transformers.get(language) {
            let condition_flags =
                language_transformer.get_condition_flags_from_parts_of_speech(parts_of_speech);
            return condition_flags;
        }
        0
    }

    pub fn get_condition_flags_from_condition_types(
        &self,
        language: impl AsRef<str>,
        condition_types: &[impl AsRef<str>],
    ) -> usize {
        let language = language.as_ref();
        if let Some(language_transformer) = self.language_transformers.get(language) {
            let condition_flags =
                language_transformer.get_condition_flags_from_condition_types(condition_types);
            {
                return condition_flags;
            }
        }
        0
    }

    pub fn get_condition_flags_from_single_condition_type(
        &self,
        language: impl AsRef<str>,
        condition_type: impl AsRef<str>,
    ) -> usize {
        let language = language.as_ref();
        if let Some(language_transformer) = self.language_transformers.get(language) {
            let condition_flags =
                language_transformer.get_condition_flags_from_single_condition_type(condition_type);
            return condition_flags;
        }
        0
    }

    pub fn transform(
        &self,
        language: impl AsRef<str>,
        source_text: impl AsRef<str>,
    ) -> Vec<TransformedText> {
        let language = language.as_ref();
        if let Some(language_transformer) = self.language_transformers.get(language) {
            let transformed_text = language_transformer.transform(source_text).unwrap();
            return transformed_text;
        }
        vec![LanguageTransformer::create_transformed_text(
            source_text,
            0,
            Vec::new(),
        )]
    }

    pub fn get_user_facing_inflection_rules(
        &self,
        language: impl AsRef<str>,
        inflection_rules: &[&str],
    ) -> InflectionRuleChain {
        let language = language.as_ref();
        if let Some(language_transformer) = self.language_transformers.get(language) {
            return language_transformer.get_user_facing_inflection_rules(inflection_rules);
        }
        inflection_rules
            .iter()
            .map(|rule| InflectionRule {
                name: rule.to_string(),
                description: None,
            })
            .collect::<InflectionRuleChain>()
    }
}
