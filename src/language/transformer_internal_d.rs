use std::collections::HashMap;

use regex::Regex;

use super::transformer_d::RuleType;

pub struct InternalTransform {
    pub id: String,
    pub name: String,
    pub rules: Vec<Rule>,
    pub heuristic: Regex,
    pub description: Option<String>,
}

pub struct Rule {
    rule_type: RuleType,
    is_inflected: Regex,
    /// deinflect: (inflectedWord: string) => string;
    deinflect: fn(&str) -> String,
    conditions_in: u32,
    conditions_out: u32,
}

pub struct TransformedText {
    pub text: String,
    pub conditions: u32,
    pub trace: Trace,
}

pub type Trace = Vec<TraceFrame>;

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
