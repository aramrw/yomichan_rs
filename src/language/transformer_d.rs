use std::collections::HashMap;

use regex::Regex;

use super::transforms::suffix_inflection;

// pub struct LanguageTransformer<'a, F>
// where
//     F: Fn(&str, &str, &str) -> String,
// {
//     next_flag_index: u16,
//     transforms: Vec<Transform<'a>>,
//     condition_type_to_condition_flags_map: HashMap<String, u16>,
//     part_of_speech_to_condition_flags_map: HashMap<String, u16>,
// }
//
// impl<'a, F> LanguageTransformer<'a, F>
// where
//     F: Fn(&str, &str, &str) -> String,
// {
//     fn new() -> Self {
//         Self {
//             next_flag_index: 0,
//             transforms: Vec::new(),
//             condition_type_to_condition_flags_map: HashMap::new(),
//             part_of_speech_to_condition_flags_map: HashMap::new(),
//         }
//     }
//
//     fn clear(&mut self) {
//         self.next_flag_index = 0;
//         self.transforms.clear();
//         self.condition_type_to_condition_flags_map.clear();
//         self.part_of_speech_to_condition_flags_map.clear();
//     }
//
//     // fn add_descriptor<T>(descriptor: LanguageTransformDescriptor<T>) {
//     //     let (conditions, transforms) = descriptor;
//     // }
// }

pub struct LanguageTransformDescriptor<'a>
// where
//     F: Fn(&str, &str, &str) -> String,
{
    pub language: &'a str,
    pub conditions: &'a ConditionMap<'a>,
    pub transforms: &'a TransformMap<'a>,
}

// Named `ConditionMapObject` in yomitan.
pub type ConditionMap<'a> = HashMap<&'a str, Condition<'a>>;

pub struct Condition<'a> {
    pub name: &'a str,
    pub is_dictionary_form: bool,
    pub i18n: Option<&'a [RuleI18n<'a>]>,
    pub sub_conditions: Option<&'a [&'a str]>,
}

// Named `TransformMapObject` in yomitan.
pub type TransformMap<'a> = HashMap<&'a str, Transform<'a>>;

pub struct Transform<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub i18n: Option<Vec<TransformI18n<'a>>>,
    pub rules: Vec<SuffixRule<'a>>,
}

pub struct TransformI18n<'a> {
    pub language: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

pub struct SuffixRule<'a> {
    /// Is of type [`RuleType::Suffix`]
    pub rule_type: RuleType,
    pub is_inflected: Regex,
    pub deinflected: &'a str,
    /// deinflect: (inflectedWord: string) => string;
    // pub deinflect: fn(text: &str, inflected_suffix: &str, deinflected_suffix: &str) -> String,
    pub conditions_in: Vec<&'a str>,
    pub conditions_out: Vec<&'a str>,
}

pub struct Rule<F>
where
    F: Fn(&str, &str, &str) -> String,
{
    pub rule_type: RuleType,
    pub is_inflected: Regex,
    /// deinflect: (inflectedWord: string) => string;
    pub deinflect: F,
    pub conditions_in: Vec<String>,
    pub conditions_out: Vec<String>,
}

pub struct RuleI18n<'a> {
    pub language: &'a str,
    pub name: &'a str,
}

pub enum RuleType {
    Suffix,
    Prefix,
    WholeWord,
    Other,
}

impl SuffixRule<'_> {
    fn deinflect(&self, text: &str, inflected_suffix: &str, deinflected_suffix: &str) -> String {
        let base_length = text.len().saturating_sub(inflected_suffix.len());
        let base = &text[..base_length];
        format!("{}{}", base, deinflected_suffix)
    }
}

impl<F> Rule<F>
where
    F: Fn(&str, &str, &str) -> String,
{
    fn deinflect_prefix(
        &self,
        text: &str,
        inflected_prefix: &str,
        deinflected_prefix: &str,
    ) -> String {
        format!("{}{}", deinflected_prefix, &text[inflected_prefix.len()..])
    }
}
