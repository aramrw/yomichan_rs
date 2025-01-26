use std::sync::{Arc, LazyLock};

use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json::error;
use snafu::{ensure, ensure_whatever, whatever, OptionExt, ResultExt, Whatever};

use crate::dictionary::{InflectionRule, InflectionRuleChain};

use super::{
    transformer_internal_d::{InternalRule, InternalTransform, Trace, TraceFrame, TransformedText},
    transforms::suffix_inflection,
};

/// Errors for [`LanguageTransformer`].
#[derive(snafu::Snafu, Debug)]
pub enum LanguageTransformerError {
    #[snafu(display("Invalid for transform: {transform_id}.rules[{index}]"))]
    InvalidConditions {
        source: ConditionError,
        transform_id: String,
        index: usize,
    },
    #[snafu(display("Failed to get conditions_flag_map: {e}"))]
    ConditionsFlagMap { e: String },
}

#[derive(thiserror::Error)]
pub enum ConditionError {
    #[error("Map does not contain condition: ({condition:?})")]
    Missing { index: usize, condition: String },
    #[error("`condition_types` is empty.")]
    EmptyTypes,
    #[error("Cycle detected in sub-rule declarations. The conditions [{conditions}] form a dependency cycle. Sub-rules cannot reference each other in a loop.")]
    SubRuleCycle { conditions: String },
    #[error("Maximum Number of Conditions was Exceeded.")]
    MaxConditions,
}

impl std::fmt::Debug for ConditionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self)
    }
}

#[derive(Debug, Clone)]
pub struct LanguageTransformer {
    next_flag_index: usize,
    transforms: Vec<InternalTransform>,
    condition_type_to_condition_flags_map: IndexMap<String, usize>,
    part_of_speech_to_condition_flags_map: IndexMap<String, usize>,
}

impl LanguageTransformer {
    pub fn new() -> Self {
        Self {
            next_flag_index: 0,
            transforms: Vec::new(),
            condition_type_to_condition_flags_map: IndexMap::new(),
            part_of_speech_to_condition_flags_map: IndexMap::new(),
        }
    }

    fn clear(&mut self) {
        self.next_flag_index = 0;
        self.transforms.clear();
        self.condition_type_to_condition_flags_map.clear();
        self.part_of_speech_to_condition_flags_map.clear();
    }

    //

    pub fn add_descriptor(
        &mut self,
        descriptor: &LanguageTransformDescriptor,
    ) -> Result<(), LanguageTransformerError> {
        let transforms: &TransformMapInner = &descriptor.transforms;
        let condition_entries = LanguageTransformDescriptor::_get_condition_entries(descriptor);
        let condition_flags_map = match self
            .get_condition_flags_map(condition_entries.clone(), self.next_flag_index)
        {
            Ok(cfm) => cfm,
            Err(e) => return Err(LanguageTransformerError::ConditionsFlagMap { e: e.to_string() }),
        };

        let mut transforms2: Vec<InternalTransform> = Vec::new();

        for entry in transforms.iter() {
            let (transform_id, transform) = entry;
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

                let condition_flags_in = LanguageTransformer::get_condition_flags_strict(
                    &condition_flags_map.map,
                    &conditions_in,
                )
                .context(InvalidConditionsSnafu {
                    index: j,
                    transform_id: transform_id.to_string(),
                })?;

                let condition_flags_out = LanguageTransformer::get_condition_flags_strict(
                    &condition_flags_map.map,
                    &conditions_out,
                )
                .context(InvalidConditionsSnafu {
                    index: j,
                    transform_id: transform_id.to_string(),
                })?;

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
        for ConditionMapEntry(condition_type, condition) in &condition_entries {
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
        inflection_rules: &[&str],
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

    pub fn get_condition_flags_map(
        &self,
        conditions: Vec<ConditionMapEntry>,
        next_flag_index: usize,
    ) -> Result<ConditionFlagsMap, ConditionError> {
        const MAX_FLAG_LIMIT: usize = 32;
        let mut next_flag_index = next_flag_index;
        let mut condition_flags_map = IndexMap::with_capacity(conditions.len());
        let mut targets = conditions;
        while !targets.is_empty() {
            let mut next_targets = Vec::with_capacity(targets.len());
            let targets_len = targets.len();
            for target in targets {
                let ConditionMapEntry(condition_type, condition) = target.clone();
                let sub_conditions = condition.sub_conditions;
                let mut flags = 0;
                match sub_conditions {
                    Some(sub_conditions) => {
                        let Ok(multi_flags) = LanguageTransformer::get_condition_flags_strict(
                            &condition_flags_map,
                            &sub_conditions,
                        ) else {
                            next_targets.push(target);
                            continue;
                        };
                        flags = multi_flags
                    }
                    None => {
                        if next_flag_index >= MAX_FLAG_LIMIT {
                            return Err(ConditionError::MaxConditions);
                        }
                        flags = 1 << next_flag_index;
                        next_flag_index += 1;
                    }
                }
                condition_flags_map.insert(condition_type, flags);
            }
            if next_targets.len() == targets_len {
                // Collect condition identifiers for error reporting
                let cycle_conditions: Vec<String> = next_targets
                    .iter()
                    .map(|entry| format!("{:?}", entry.0)) // Adjust based on your ConditionType's Display/Debug
                    .collect();
                return Err(ConditionError::SubRuleCycle {
                    conditions: cycle_conditions.join(" -> "),
                });
            }
            targets = std::mem::take(&mut next_targets);
        }
        Ok(ConditionFlagsMap {
            map: condition_flags_map,
            next_flag_index,
        })
    }

    pub fn get_condition_flags_strict<'a>(
        condition_flags_map: &IndexMap<String, usize>,
        condition_types: &'a impl IntoDeref<'a>,
    ) -> Result<usize, ConditionError> {
        let mut flags = 0;

        for (index, condition_type) in condition_types.into_deref().enumerate() {
            let Some(flags2) = condition_flags_map.get(condition_type) else {
                return Err(ConditionError::Missing {
                    index,
                    condition: condition_type.to_string(),
                });
            };
            flags |= flags2;
        }

        Ok(flags)
    }

    fn get_condition_flags(
        &self,
        condition_flags_map: &IndexMap<String, usize>,
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

/// Named [ConditionMapObject](https://github.com/yomidevs/yomitan/blob/37d13a8a1abc15f4e91cef5bfdc1623096855bb0/types/ext/language-transformer.d.ts#L24) in yomitan.
#[derive(Debug, Clone, Deserialize)]
pub struct ConditionMap(pub IndexMap<String, Condition>);

impl std::ops::Deref for ConditionMap {
    type Target = IndexMap<String, Condition>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ConditionMapEntry(String, Condition);

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageTransformDescriptor {
    pub language: String,
    pub conditions: ConditionMap,
    pub transforms: TransformMap,
}

impl LanguageTransformDescriptor {
    pub fn _get_condition_entries(&self) -> Vec<ConditionMapEntry> {
        self.conditions
            .iter()
            .map(|(str, cond)| ConditionMapEntry(str.to_string(), cond.to_owned()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionFlagsMap {
    pub map: IndexMap<String, usize>,
    pub next_flag_index: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub name: String,
    pub is_dictionary_form: bool,
    pub i18n: Option<Vec<RuleI18n>>,
    pub sub_conditions: Option<Vec<String>>,
}

#[derive(thiserror::Error, Clone, Debug, Deserialize)]
enum DeserializeTransformMapError {
    #[error("failed to deserialize transform map")]
    Failed,
}

type TransformMapInner = IndexMap<String, Transform>;
// Named `TransformMapObject` in yomitan.
#[derive(Debug, Clone)]
pub struct TransformMap(pub TransformMapInner);

/// [needless_lifetimes]
/// the following explicit lifetimes could be elided: 'a
impl<'de> Deserialize<'de> for TransformMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use the IndexMap's deserialization
        let inner = TransformMapInner::deserialize(deserializer)?;
        Ok(TransformMap(inner))
    }
}

impl std::ops::Deref for TransformMap {
    type Target = TransformMapInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Transform {
    pub name: String,
    pub description: Option<String>,
    pub i18n: Option<Vec<TransformI18n>>,
    pub rules: Vec<SuffixRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransformI18n {
    pub language: String,
    pub name: String,
    pub description: Option<String>,
}

pub trait DeinflectFnTrait: Fn(String) -> String + Send + Sync + 'static {}
impl<F: Fn(String) -> String + Send + Sync + 'static> DeinflectFnTrait for F {}
pub type DeinflectFunction = Arc<dyn DeinflectFnTrait>;

fn regex_default() -> Regex {
    Regex::new(r"\d").unwrap()
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuffixRule {
    #[serde(rename = "type")]
    pub rule_type: RuleType,
    // Use custom deserialization function for `Regex`
    #[serde(deserialize_with = "deserialize_regex")]
    pub is_inflected: Regex,
    pub deinflected: String,
    #[serde(skip_deserializing, default = "arc_default")]
    pub deinflect: Arc<dyn DeinflectFnTrait>,
    pub conditions_in: Vec<String>,
    pub conditions_out: Vec<String>,
}

fn arc_default() -> Arc<dyn DeinflectFnTrait> {
    std::sync::Arc::new(|_| "<unimplemented_deinflect_fn_trait>".into())
}

pub trait IntoDeref<'a> {
    fn into_deref(&'a self) -> Box<dyn Iterator<Item = &'a str> + 'a>;
}

impl<'a> IntoDeref<'a> for Vec<String> {
    fn into_deref(&'a self) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        Box::new(self.iter().map(|s| s.as_str()))
    }
}

impl<'a> IntoDeref<'a> for &'a Vec<String> {
    fn into_deref(&'a self) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        Box::new(self.iter().map(|s| s.as_str()))
    }
}

/// Custom deserialization function for javascript Regex.
///
/// Note: If `RegExp.prototype.toJSON()` isn't used to serialize the regex,
/// it will default to an object: `{}`.
///> {"rgx":"/qux$/gi","date":"2014-03-21T23:11:33.749Z"}"
fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let s: serde_json::Value = Deserialize::deserialize(deserializer)?;
    if let serde_json::Value::Object(obj) = s {
        if let Some(re_val) = obj.get("rgx") {
            if let serde_json::Value::String(re_str) = re_val {
                return Regex::new(re_str).map_err(serde::de::Error::custom);
            }
            unreachable!();
        }
        let def = Regex::new(r"").unwrap();
        return Ok(def);
    }
    panic!("'isInflected': was expected to be a regex object, found {s:?}");
}

impl std::fmt::Debug for SuffixRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuffixRule")
            .field("rule_type", &self.rule_type)
            .field("is_inflected", &self.is_inflected)
            .field("deinflected", &self.deinflected)
            .field("deinflect", &"<deinflect_function>")
            .field("conditions_in", &self.conditions_in)
            .field("conditions_out", &self.conditions_out)
            .finish()
    }
}

#[cfg(test)]
mod suffix_rule {
    use std::sync::Arc;

    use regex::Regex;

    use super::{RuleType, SuffixRule};

    #[test]
    fn debug_display() {
        let sr = SuffixRule {
            rule_type: RuleType::Suffix,
            is_inflected: Regex::new(r"\d").unwrap(),
            deinflected: "食べる".into(),
            deinflect: Arc::new(|a: String| a),
            conditions_in: vec!["".to_string()],
            conditions_out: vec!["".to_string()],
        };
        dbg!(sr);
    }
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

#[derive(Debug, Clone, Deserialize)]
pub struct RuleI18n {
    pub language: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleType {
    Suffix,
    Prefix,
    WholeWord,
    Other,
}

#[cfg(test)]
mod language_transformer_tests {
    use std::ops::Deref;

    use crate::language::{
        descriptors::LANGUAGE_DESCRIPTORS_MAP, ja::transforms::JAPANESE_TRANSFORMS,
    };

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn add_descriptor() {
        let mut language_transformer = LanguageTransformer::new();
        language_transformer
            .add_descriptor(&JAPANESE_TRANSFORMS)
            .unwrap();
    }
    #[test]
    fn get_condition_flags_map() {
        let assert_map = ConditionFlagsMap {
            map: IndexMap::from_iter([
                ("v1d".to_string(), 1),
                ("v1p".to_string(), 2),
                ("v5d".to_string(), 4),
                ("v5ss".to_string(), 8),
                ("v5sp".to_string(), 16),
                ("vk".to_string(), 32),
                ("vs".to_string(), 64),
                ("vz".to_string(), 128),
                ("adj-i".to_string(), 256),
                ("-ます".to_string(), 512),
                ("-ません".to_string(), 1024),
                ("-て".to_string(), 2048),
                ("-ば".to_string(), 4096),
                ("-く".to_string(), 8192),
                ("-た".to_string(), 16384),
                ("-ん".to_string(), 32768),
                ("-なさい".to_string(), 65536),
                ("-ゃ".to_string(), 131072),
                ("v1".to_string(), 3),
                ("v5s".to_string(), 24),
                ("v5".to_string(), 28),
                ("v".to_string(), 255),
            ]),
            next_flag_index: 18,
        };

        let mut lt = LanguageTransformer::new();
        let conditions = LanguageTransformDescriptor::_get_condition_entries(&JAPANESE_TRANSFORMS);
        let condition_flags_map =
            LanguageTransformer::get_condition_flags_map(&lt, conditions, lt.next_flag_index);
        assert_eq!(condition_flags_map.unwrap(), assert_map);
    }
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
// &self}
