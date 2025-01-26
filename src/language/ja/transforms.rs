use indexmap::IndexMap;
use serde_json::from_reader;

use crate::language::transformer_d::{
    Condition, ConditionMap, LanguageTransformDescriptor, RuleI18n, Transform, TransformI18n,
    TransformMap,
};
use crate::language::transforms::suffix_inflection;

use std::mem;
use std::sync::{Arc, LazyLock};

const SHIMAU_ENGLISH_DESCRIPTION: &str = "1. Shows a sense of regret/surprise when you did have volition in doing something, but it turned out to be bad to do.\n2. Shows perfective/punctual achievement. This shows that an action has been completed.\n 3. Shows unintentional action–“accidentally”.\n";

const PASSIVE_ENGLISH_DESCRIPTION: &str = "1. Indicates an action received from an action performer.\n2. Expresses respect for the subject of action performer.\n";

pub static CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
    ConditionMap(IndexMap::from([
        (
            "v".to_string(),
            Condition {
                name: "Verb".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "動詞".to_string(),
                }]),
                sub_conditions: Some(vec![
                    "v1".to_string(),
                    "v5".to_string(),
                    "vk".to_string(),
                    "vs".to_string(),
                    "vz".to_string(),
                ]),
            },
        ),
        (
            "v1d".to_string(),
            Condition {
                name: "Ichidan verb, dictionary form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "一段動詞、辞書形".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v1".to_string(),
            Condition {
                name: "Ichidan verb".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "一段動詞".to_string(),
                }]),
                sub_conditions: Some(vec!["v1d".to_string(), "v1p".to_string()]),
            },
        ),
        (
            "v1p".to_string(),
            Condition {
                name: "Ichidan verb, progressive or perfect form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "一段動詞、～てる・でる".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5".to_string(),
            Condition {
                name: "Godan verb".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞".to_string(),
                }]),
                sub_conditions: Some(vec!["v5d".to_string(), "v5s".to_string()]),
            },
        ),
        (
            "v5d".to_string(),
            Condition {
                name: "Godan verb, dictionary form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞、終止形".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5s".to_string(),
            Condition {
                name: "Godan verb, short causative form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞、～す・さす".to_string(),
                }]),
                sub_conditions: Some(vec!["v5ss".to_string(), "v5sp".to_string()]),
            },
        ),
        (
            "v5ss".to_string(),
            Condition {
                name: "Godan verb, short causative form having さす ending (cannot conjugate with passive form)".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞、～さす".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5sp".to_string(),
            Condition {
                name: "Godan verb, short causative form not having さす ending (can conjugate with passive form)".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞、～す".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "vk".to_string(),
            Condition {
                name: "Kuru verb".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "来る動詞".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "vs".to_string(),
            Condition {
                name: "Suru verb".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "する動詞".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "vz".to_string(),
            Condition {
                name: "Zuru verb".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "ずる動詞".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "adj-i".to_string(),
            Condition {
                name: "Adjective with i ending".to_string(),
                is_dictionary_form: true,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "形容詞".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "-ます".to_string(),
            Condition {
                name: "Polite -ます ending".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ません".to_string(),
            Condition {
                name: "Polite negative -ません ending".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-て".to_string(),
            Condition {
                name: "Intermediate -て endings for progressive or perfect tense".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ば".to_string(),
            Condition {
                name: "Intermediate -ば endings for conditional contraction".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-く".to_string(),
            Condition {
                name: "Intermediate -く endings for adverbs".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-た".to_string(),
            Condition {
                name: "-た form ending".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ん".to_string(),
            Condition {
                name: "-ん negative ending".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-なさい".to_string(),
            Condition {
                name: "Intermediate -なさい ending (polite imperative)".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ゃ".to_string(),
            Condition {
                name: "Intermediate -や ending (conditional contraction)".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
    ]))
});

pub static TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    TransformMap(IndexMap::from([
        (
            "-ば".to_string(),
            Transform {
                name: "-ば".to_string(),
                description: Some(
                    "(1) Conditional form; shows that the previous stated condition's establishment is the condition for the latter stated condition to occur. (2) Shows a trigger for a latter stated perception or judgment. Usage: Attach ば to the hypothetical/realis form (kateikei/izenkei) of verbs and i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ば".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("ければ", "い", vec!["-ば".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("えば", "う", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("けば", "く", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("げば", "ぐ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("せば", "す", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("てば", "つ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ねば", "ぬ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("べば", "ぶ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("めば", "む", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("れば", "る", vec!["-ば".to_string()], vec!["v1".to_string(), "v5".to_string(), "vk".to_string(), "vs".to_string(), "vz".to_string()]),
                    suffix_inflection("れば", "", vec!["-ば".to_string()], vec!["-ます".to_string()]),
                ],
            },
        ),
    ]))
});

pub static JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: "ja".to_string(),
        conditions: CONDITIONS.clone(),
        transforms: TRANSFORMS.clone(),
    });
