use std::collections::HashMap;

use derive_more::Debug;
use regex::Regex;

use super::transformer_d::{DeinflectFnTrait, Rule, RuleType, SuffixRule};
#[derive(Debug, Clone)]
pub struct InternalTransform {
    pub id: String,
    pub name: String,
    pub rules: Vec<InternalRule>,
    pub heuristic: Regex,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InternalRule {
    pub rule_type: RuleType,
    pub is_inflected: Regex,
    pub deinflected: &'static str,
    pub conditions_in: u32,
    pub conditions_out: u32,
}

impl DeinflectFnTrait for InternalRule {
    fn inflected(&self) -> &str {
        self.is_inflected.as_str()
    }
    fn deinflected(&self) -> &str {
        self.deinflected
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformedText {
    pub text: String,
    pub conditions: u32,
    pub trace: Trace,
}

impl TransformedText {
    pub fn create_transformed_text(text: String, conditions: u32, trace: Trace) -> TransformedText {
        TransformedText {
            text,
            conditions,
            trace,
        }
    }
}

pub type Trace = Vec<TraceFrame>;

#[derive(Debug, Clone, PartialEq)]
pub struct TraceFrame {
    pub text: String,
    pub transform: String,
    pub rule_index: u32,
}

pub type ConditionTypeToConditionFlagsMap = HashMap<String, u32>;

pub struct LanguageTransformDescriptorInternal {
    transforms: Vec<InternalTransform>,
    condition_type_to_condition_flags_map: ConditionTypeToConditionFlagsMap,
    part_of_speech_to_condition_flags_map: ConditionTypeToConditionFlagsMap,
}
