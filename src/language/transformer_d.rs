use std::collections::HashMap;

use regex::Regex;

use crate::dictionary::{InflectionRule, InflectionRuleChain};

use super::{
    transformer_internal_d::{InternalTransform, Trace, TraceFrame, TransformedText},
    transforms::suffix_inflection,
};

pub struct LanguageTransformer {
    next_flag_index: usize,
    transforms: Vec<InternalTransform>,
    condition_type_to_condition_flags_map: HashMap<String, usize>,
    part_of_speech_to_condition_flags_map: HashMap<String, usize>,
}

impl<'a> LanguageTransformer {
    fn new() -> Self {
        Self {
            next_flag_index: 0,
            transforms: Vec::new(),
            condition_type_to_condition_flags_map: HashMap::new(),
            part_of_speech_to_condition_flags_map: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.next_flag_index = 0;
        self.transforms.clear();
        self.condition_type_to_condition_flags_map.clear();
        self.part_of_speech_to_condition_flags_map.clear();
    }

    fn add_descriptor<T>(descriptor: LanguageTransformDescriptor) {
        let condition_entries: Vec<Condition> = descriptor.conditions.values().cloned().collect();
    }

    fn get_condition_flags_from_parts_of_speech(
        &self,
        parts_of_speech: &[&'a str],
    ) -> Option<usize> {
        self.get_condition_flags(&self.part_of_speech_to_condition_flags_map, parts_of_speech)
    }

    fn get_condition_flags_from_condition_types(
        &self,
        condition_types: &[&'a str],
    ) -> Option<usize> {
        self.get_condition_flags(&self.condition_type_to_condition_flags_map, condition_types)
    }

    fn get_condition_flag_from_condition_type<T: AsRef<str>>(
        &self,
        condition_type: T,
    ) -> Option<usize> {
        self.get_condition_flags(
            &self.condition_type_to_condition_flags_map,
            &[condition_type.as_ref()],
        )
    }

    fn get_user_facing_inflection_rules(
        &self,
        inflection_rules: &[&'a str],
    ) -> InflectionRuleChain {
        inflection_rules
            .iter()
            .map(|rule| {
                let full_rule = &self
                    .transforms
                    .iter()
                    .find(|transform| transform.id == *rule);
                if let Some(full_rule) = full_rule {
                    return InflectionRule {
                        name: full_rule.name.clone(),
                        description: full_rule.description.clone(),
                    };
                }
                InflectionRule {
                    name: rule.to_string(),
                    description: None,
                }
            })
            .collect()
    }

    fn create_transformed_text(text: &str, conditions: u32, trace: Trace) -> TransformedText {
        TransformedText {
            text: text.to_string(),
            conditions,
            trace,
        }
    }

    /// If `currentConditions` is `0`, then `nextConditions` is ignored and `true` is returned.
    /// Otherwise, there must be at least one shared condition between `currentConditions` and `nextConditions`.
    fn conditions_match(current_conditions: usize, next_conditions: usize) -> bool {
        current_conditions == 0 || (current_conditions & next_conditions) != 0
    }

    fn get_condition_flags_strict(
        &self,
        condition_flags_map: &HashMap<String, usize>,
        condition_types: &[&'a str],
    ) -> Option<usize> {
        let mut flags = 0;
        for condition_type in condition_types {
            let flags2 = condition_flags_map.get(*condition_type);
            if let Some(flags2) = flags2 {
                flags |= flags2;
                return Some(flags);
            }
        }
        None
    }

    fn get_condition_flags(
        &self,
        condition_flags_map: &HashMap<String, usize>,
        condition_types: &[&'a str],
    ) -> Option<usize> {
        let mut flags = 0;
        for condition_type in condition_types {
            let mut flags2 = 0;
            if let Some(val) = condition_flags_map.get(*condition_type) {
                flags2 = *val;
                return Some(flags);
            }
            flags |= flags2;
        }
        None
    }

    fn extend_trace(trace: Trace, new_frame: TraceFrame) -> Trace {
        let mut new_trace = vec![new_frame];
        for t in trace {
            new_trace.push(t);
        }
        new_trace
    }
}

pub struct LanguageTransformDescriptor<'a> {
    pub language: &'a str,
    pub conditions: &'a ConditionMap<'a>,
    pub transforms: &'a TransformMap<'a>,
}

// Named `ConditionMapObject` in yomitan.
pub type ConditionMap<'a> = HashMap<&'a str, Condition<'a>>;

#[derive(Clone)]
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
    pub description: Option<String>,
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

pub struct Rule<'a, F>
where
    F: Fn(&str, &str, &str) -> String,
{
    pub rule_type: RuleType,
    pub is_inflected: Regex,
    /// deinflect: (inflectedWord: string) => string;
    pub deinflect: F,
    pub conditions_in: Vec<&'a str>,
    pub conditions_out: Vec<&'a str>,
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

impl<F> Rule<'_, F>
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
