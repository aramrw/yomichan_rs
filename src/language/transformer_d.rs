use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use regex::Regex;
use serde_json::error;
use snafu::{ensure, ensure_whatever, whatever, OptionExt, ResultExt, Whatever};

use crate::{
    dictionary::{InflectionRule, InflectionRuleChain},
    errors::LanguageError,
};

use super::{
    transformer_internal_d::{InternalRule, InternalTransform, Trace, TraceFrame, TransformedText},
    transforms::suffix_inflection,
};

#[derive(thiserror::Error, Debug)]
pub enum LanguageTransformerError {
    #[error("Invalid `conditions_in` for transform: {transform_id}.rules[{index}]")]
    InvalidConditionsIn { transform_id: String, index: usize },
    #[error("Invalid `conditions_out` for transform: {transform_id}.rules[{index}]")]
    InvalidConditionsOut { transform_id: String, index: usize },
    #[error("Failed to get conditions_flag_map: {0}")]
    ConditionsFlagMap(String),
}

#[cfg(test)]
mod language_transformer_tests {
    use crate::language::ja::transforms::JAPANESE_TRANSFORMS;

    use super::*;

    #[test]
    fn add_descriptor_test() -> Result<(), LanguageTransformerError> {
        let mut language_transformer = LanguageTransformer::new();
        if let Err(e) = language_transformer.add_descriptor(&JAPANESE_TRANSFORMS) {
            panic!("{e}")
        }
        Ok(())
    }
}

pub struct LanguageTransformer {
    next_flag_index: usize,
    transforms: Vec<InternalTransform>,
    condition_type_to_condition_flags_map: HashMap<String, usize>,
    part_of_speech_to_condition_flags_map: HashMap<String, usize>,
}

impl<'a> LanguageTransformer {
    pub fn new() -> Self {
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

    pub fn add_descriptor(
        &mut self,
        descriptor: &LanguageTransformDescriptor,
    ) -> Result<(), LanguageTransformerError> {
        let transforms = &descriptor.transforms;
        let condition_entries: Vec<(&String, &Condition)> = descriptor.conditions.iter().collect();
        let condition_flags_map =
            match self.get_condition_flags_map(condition_entries.clone(), self.next_flag_index) {
                Ok(cfm) => cfm,
                Err(e) => return Err(LanguageTransformerError::ConditionsFlagMap(e.to_string())),
            };

        let mut transforms2: Vec<InternalTransform> = Vec::new();

        for entry in transforms.iter() {
            let transform_id = entry.0;
            let transform = entry.1;
            let Transform {
                name,
                description,
                i18n,
                rules,
            } = transform;
            let mut rules2: Vec<InternalRule> = Vec::new();
            for (j, rule) in rules.iter().enumerate() {
                let SuffixRule {
                    rule_type,
                    is_inflected,
                    deinflected,
                    deinflect,
                    conditions_in,
                    conditions_out,
                } = rule.clone();
                let condition_flags_in = match self
                    .get_condition_flags_strict(&condition_flags_map.map, &conditions_in)
                {
                    Some(cfi) => cfi,
                    None => {
                        return Err(LanguageTransformerError::InvalidConditionsIn {
                            transform_id: transform_id.to_string(),
                            index: j,
                        });
                    }
                };
                let condition_flags_out = match self
                    .get_condition_flags_strict(&condition_flags_map.map, &conditions_out)
                {
                    Some(cfo) => cfo,
                    None => {
                        return Err(LanguageTransformerError::InvalidConditionsIn {
                            transform_id: transform_id.to_string(),
                            index: j,
                        });
                    }
                };
                rules2.push(InternalRule {
                    rule_type: rule_type.clone(),
                    is_inflected: is_inflected.clone(),
                    deinflect,
                    conditions_in: condition_flags_in as u32,
                    conditions_out: condition_flags_out as u32,
                });
            }
            let is_inflected_regex_tests = rules
                .iter()
                .map(|rule| rule.is_inflected.clone())
                .collect::<Vec<Regex>>();
            // constructing a single heuristic regex by joining all patterns with a '|'
            let combined_pattern = is_inflected_regex_tests
                .iter()
                .map(|reg_exp| reg_exp.as_str()) // get pattern (similar to .source in JS)
                .collect::<Vec<&str>>()
                .join("|");
            // compile the combined pattern into a new Regex
            let heuristic = Regex::new(&combined_pattern).unwrap();
            transforms2.push(InternalTransform {
                id: transform_id.to_string(),
                name: name.to_string(),
                description: description.clone(),
                rules: rules2,
                heuristic,
            });
        }
        self.next_flag_index = condition_flags_map.next_flag_index;
        self.transforms.extend(transforms2);
        for (condition_type, condition) in &condition_entries {
            if let Some(flags) = condition_flags_map.map.get(condition_type.as_str()) {
                self.condition_type_to_condition_flags_map
                    .insert(condition_type.to_string(), *flags);
                if condition.is_dictionary_form {
                    self.part_of_speech_to_condition_flags_map
                        .insert(condition_type.to_string(), *flags);
                }
            }
        }
        Ok(())
    }

    pub fn get_condition_flags_from_parts_of_speech(
        &self,
        parts_of_speech: &[impl AsRef<str>],
    ) -> Option<usize> {
        self.get_condition_flags(&self.part_of_speech_to_condition_flags_map, parts_of_speech)
    }

    pub fn get_condition_flags_from_condition_types(
        &self,
        condition_types: &[impl AsRef<str>],
    ) -> Option<usize> {
        self.get_condition_flags(&self.condition_type_to_condition_flags_map, condition_types)
    }

    pub fn get_condition_flags_from_single_condition_type<T: AsRef<str>>(
        &self,
        condition_type: T,
    ) -> Option<usize> {
        self.get_condition_flags(
            &self.condition_type_to_condition_flags_map,
            &[condition_type.as_ref()],
        )
    }

    pub fn transform(
        &self,
        source_text: impl AsRef<str>,
    ) -> Result<Vec<TransformedText>, Whatever> {
        let source_text = source_text.as_ref();
        let mut results = vec![LanguageTransformer::create_transformed_text(
            source_text,
            0,
            Vec::new(),
        )];

        for i in 0..results.len() {
            let entry = &results[i];
            let text = entry.text.clone();
            let conditions = entry.conditions;
            let trace = entry.trace.clone();

            for transform in &self.transforms {
                if !transform.heuristic.is_match(&text) {
                    continue;
                }

                let id = &transform.id;
                for (j, rule) in transform.rules.iter().enumerate() {
                    if !LanguageTransformer::conditions_match(conditions, rule.conditions_in) {
                        continue;
                    }
                    if !rule.is_inflected.is_match(&text) {
                        continue;
                    }

                    let is_cycle = trace.iter().any(|frame| {
                        &frame.transform == id && frame.rule_index == j as u32 && frame.text == text
                    });
                    if is_cycle {
                        return whatever!("Cycle detected in transform[{}] rule[{j}] for text: {text}\nTrace: {:?}", transform.name, trace);
                    }

                    let new_text = (rule.deinflect)(text.clone());
                    let new_trace = self.extend_trace(
                        trace.clone(),
                        TraceFrame {
                            transform: id.clone(),
                            rule_index: j as u32,
                            text: text.clone(),
                        },
                    );

                    results.push(LanguageTransformer::create_transformed_text(
                        new_text,
                        rule.conditions_out,
                        new_trace,
                    ));
                }
            }
        }

        Ok(results)
    }

    pub fn extend_trace(&self, trace: Trace, new_frame: TraceFrame) -> Trace {
        let mut new_trace = vec![new_frame];
        for t in trace {
            new_trace.push(t);
        }
        new_trace
    }

    pub fn get_user_facing_inflection_rules(
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

    pub fn create_transformed_text(
        text: impl AsRef<str>,
        conditions: u32,
        trace: Trace,
    ) -> TransformedText {
        TransformedText {
            text: text.as_ref().to_string(),
            conditions,
            trace,
        }
    }

    /// If `currentConditions` is `0`, then `nextConditions` is ignored and `true` is returned.
    /// Otherwise, there must be at least one shared condition between `currentConditions` and `nextConditions`.
    pub fn conditions_match(current_conditions: u32, next_conditions: u32) -> bool {
        current_conditions == 0 || (current_conditions & next_conditions) != 0
    }

    /**
     * @param {import('language-transformer').ConditionMapEntries} conditions
     * @param {number} nextFlagIndex
     * @returns {{conditionFlagsMap: Map<string, number>, nextFlagIndex: number}}
     * @throws {Error}
     */
    pub fn get_condition_flags_map(
        &self,
        conditions: Vec<ConditionMapEntry>,
        next_flag_index: usize,
    ) -> Result<ConditionFlagsMap, Whatever> {
        let mut next_flag_index = next_flag_index;
        let mut condition_flags_map = HashMap::new();
        let mut targets = conditions;
        while !targets.is_empty() {
            let mut next_targets = Vec::new();
            for target in &targets {
                let condition_type = target.0.clone();
                let condition = target.1.clone();
                let sub_conditions: Option<Condition> = Some(condition);
                let mut flags = 0;
                if let Some(sub_conditions) = sub_conditions {
                    if let Some(sub_conditions) = sub_conditions.sub_conditions {
                        let multi_flags =
                            self.get_condition_flags_strict(&condition_flags_map, sub_conditions);
                        if let Some(multi_flags) = multi_flags {
                            flags = multi_flags
                        } else {
                            next_targets.push(*target);
                            continue;
                        }
                    }
                } else {
                    ensure_whatever!(
                        next_flag_index < 32,
                        "Maximum Number of Conditions was Exceeded."
                    );
                    flags = 1 << next_flag_index;
                    next_flag_index += 1;
                }
                condition_flags_map.insert(condition_type, flags);
            }
            ensure_whatever!(
                !next_targets.len() != targets.len(),
                "Maximum number of conditions was exceeded"
            );
            targets = next_targets;
        }
        Ok(ConditionFlagsMap {
            map: condition_flags_map,
            next_flag_index,
        })
    }

    pub fn get_condition_flags_strict(
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
        condition_types: &[impl AsRef<str>],
    ) -> Option<usize> {
        let mut flags = 0;
        for condition_type in condition_types {
            let mut flags2 = 0;
            if let Some(val) = condition_flags_map.get(condition_type.as_ref()) {
                flags2 = *val;
                return Some(flags);
            }
            flags |= flags2;
        }
        None
    }
}

#[derive(Clone)]
pub struct LanguageTransformDescriptor<'a> {
    pub language: String,
    pub conditions: ConditionMap<'a>,
    pub transforms: TransformMap<'a>,
}

// Named `ConditionMapObject` in yomitan.
pub type ConditionMap<'a> = HashMap<String, Condition<'a>>;
pub type ConditionMapEntry<'a> = (&'a String, &'a Condition<'a>);

#[derive(Clone)]
pub struct ConditionFlagsMap {
    pub map: HashMap<String, usize>,
    pub next_flag_index: usize,
}

#[derive(Clone)]
pub struct Condition<'a> {
    pub name: &'a str,
    pub is_dictionary_form: bool,
    pub i18n: Option<&'a [RuleI18n<'a>]>,
    pub sub_conditions: Option<&'a [&'a str]>,
}

// Named `TransformMapObject` in yomitan.
pub type TransformMap<'a> = HashMap<&'a str, Transform<'a>>;

#[derive(Clone)]
pub struct Transform<'a> {
    pub name: &'a str,
    pub description: Option<String>,
    pub i18n: Option<Vec<TransformI18n<'a>>>,
    pub rules: Vec<SuffixRule<'a>>,
}

#[derive(Clone)]
pub struct TransformI18n<'a> {
    pub language: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

pub trait DeinflectFnTrait: Fn(String) -> String + Send + Sync + 'static {}
impl<F: Fn(String) -> String + Send + Sync + 'static> DeinflectFnTrait for F {}
pub type DeinflectFunction = Arc<dyn DeinflectFnTrait>;

#[derive(Clone)]
pub struct SuffixRule<'a> {
    /// Is of type [`RuleType::Suffix`]
    pub rule_type: RuleType,
    pub is_inflected: Regex,
    pub deinflected: String,
    pub deinflect: DeinflectFunction,
    pub conditions_in: Vec<&'a str>,
    pub conditions_out: Vec<&'a str>,
}

// impl SuffixRule<'_> {
//     fn deinflect(&self, text: &str, inflected_suffix: &str, deinflected_suffix: &str) -> String {
//         let base_length = text.len().saturating_sub(inflected_suffix.len());
//         let base = &text[..base_length];
//         format!("{}{}", base, deinflected_suffix)
//     }
// }

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

#[derive(Clone)]
pub enum RuleType {
    Suffix,
    Prefix,
    WholeWord,
    Other,
}

// impl InternalRule
// // where
// //     F: Fn(&str, &str, &str) -> String,
// {
//     fn deinflect_prefix(
//         &self,
//         text: &str,
//         inflected_prefix: &str,
//         deinflected_prefix: &str,
//     ) -> String {
//         format!("{}{}", deinflected_prefix, &text[inflected_prefix.len()..])
//     }
// }
