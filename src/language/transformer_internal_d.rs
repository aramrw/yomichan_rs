use std::collections::HashMap;

use regex::Regex;

use super::transformer_d::RuleType;

pub struct Transform {
    id: String,
    name: String,
    rules: Vec<Rule>,
    heuristic: Regex,
    description: Option<String>,
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
    text: String,
    conditions: u32,
    trace: Trace,
}

pub type Trace = Vec<TraceFrame>;

pub struct TraceFrame {
    text: String,
    transform: String,
    rule_index: u32,
}

pub type ConditionTypeToConditionFlagsMap = HashMap<String, u32>;

pub struct LanguageTransformDescriptorInternal {
    transforms: Vec<Transform>,
    condition_type_to_condition_flags_map: ConditionTypeToConditionFlagsMap,
    part_of_speech_to_condition_flags_map: ConditionTypeToConditionFlagsMap,
}
