use crate::language::transformer::{
    Condition, ConditionMap, LanguageTransformDescriptor, RuleI18n, Transform, TransformI18n,
    TransformMap,
};
use crate::language::transforms::suffix_inflection;

use std::collections::HashMap;
use std::sync::LazyLock;

const SHIMAU_ENGLISH_DESCRIPTION: &str = "1. Shows a sense of regret/surprise when you did have volition in doing something, but it turned out to be bad to do.\n2. Shows perfective/punctual achievement. This shows that an action has been completed.\n 3. Shows unintentional action–“accidentally”.\n";

const PASSIVE_ENGLISH_DESCRIPTION: &str = "1. Indicates an action received from an action performer.\n2. Expresses respect for the subject of action performer.\n";

pub static CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
    HashMap::from([
        (
            "v",
            Condition {
                name: "Verb",
                is_dictionary_form: false,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "動詞",
                }]),
                sub_conditions: Some(&["v1", "v5", "vk", "vs", "vz"]),
            },
        ),
        (
            "v1",
            Condition {
                name: "Ichidan verb",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "一段動詞",
                }]),
                sub_conditions: Some(&["v1d", "v1p"]),
            },
        ),
        (
            "v1d",
            Condition {
                name: "Ichidan verb, dictionary form",
                is_dictionary_form: false,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "一段動詞、辞書形",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v1p",
            Condition {
                name: "Ichidan verb, progressive or perfect form",
                is_dictionary_form: false,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "一段動詞、進行形または完了形",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5",
            Condition {
                name: "Godan verb",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "五段動詞",
                }]),
                sub_conditions: Some(&["v5d", "v5m"]),
            },
        ),
        (
            "v5d",
            Condition {
                name: "Godan verb, dictionary form",
                is_dictionary_form: false,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "五段動詞、辞書形",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5m",
            Condition {
                name: "Godan verb, polite (masu) form",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "vk",
            Condition {
                name: "Kuru verb",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "来る動詞",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "vs",
            Condition {
                name: "Suru verb",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "する動詞",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "vz",
            Condition {
                name: "Zuru verb",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "ずる動詞",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "adj-i",
            Condition {
                name: "Adjective with i ending",
                is_dictionary_form: true,
                i18n: Some(&[RuleI18n {
                    language: "ja",
                    name: "形容詞",
                }]),
                sub_conditions: None,
            },
        ),
        (
            "-te",
            Condition {
                name: "Intermediate -te endings for progressive or perfect tense",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ba",
            Condition {
                name: "Intermediate -ba endings for conditional contraction",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "adv",
            Condition {
                name: "Intermediate -ku endings for adverbs",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "past",
            Condition {
                name: "-ta past form ending",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
    ])
});

pub static TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    HashMap::from([(
        "-ba",
        Transform {
            name: "-ba",
            description: Some(
                "1. Conditional form; shows that the previous stated condition's establishment is the 
                    condition for the latter stated condition to occur.
                \n2. Shows a trigger for a latter stated perception or judgment.
                \nUsage: Attach ば to the hypothetical/realis form (kateikei/izenkei) of verbs and i-adjectives.",
            ),
            i18n: Some(vec![TransformI18n {
                language: "ja",
                name: "～ば",
                description: Some("仮定形"),
            }]),
            rules: vec![suffix_inflection(
                "ければ",
                "い",
                vec!["-ba"],
                vec!["adj-i"],
            ), suffix_inflection("えば", "う", vec!["-ba"], vec!["v5"]),
                suffix_inflection("けば", "く", vec!["-ba"], vec!["v5"]),
                suffix_inflection("げば", "ぐ", vec!["-ba"], vec!["v5"]),
                suffix_inflection("せば", "す", vec!["-ba"], vec!["v5"]),
                suffix_inflection("てば", "つ", vec!["-ba"], vec!["v5"]),
                suffix_inflection("ねば", "ぬ", vec!["-ba"], vec!["v5"]),
                suffix_inflection("べば", "ぶ", vec!["-ba"], vec!["v5"]),
                suffix_inflection("めば", "む", vec!["-ba"], vec!["v5"]),
                suffix_inflection("れば", "る", vec!["-ba"], vec!["v1", "v5", "vk", "vs", "vz"]),
            ],
        },
    ),
    (
        "-ya",
        Transform {
            name: "-ya",
            description: Some("Contraction of -ba."),
             i18n: Some(vec![TransformI18n
                {
                    language: "ja",
                    name: "～ゃ",
                    description: Some("仮定形の縮約系"),
                },
            ]),
            rules: vec![
                suffix_inflection("けりゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("きゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("や",   "えば",   vec!["-ya"], vec!["-ba"]),
                suffix_inflection("きゃ", "けば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("ぎゃ", "げば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("しゃ", "せば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("ちゃ", "てば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("にゃ", "ねば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("びゃ", "べば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("みゃ", "めば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("りゃ", "れば", vec!["-ya"], vec!["-ba"]),
            ],

        }
    ),
 (
        "-cha",
        Transform {
            name: "-cha",
            description: Some("Contraction of ～ては.\n 
            1. Explains how something always happens under the condition that it marks.\n
            2. Expresses the repetition (of a series of) actions.\n 
            3. Indicates a hypothetical situation in which the speaker gives a (negative) evaluation about the other party\"s intentions.\n
            4. Used in `Must Not` patterns like ～てはいけない.\n 
            Usage: Attach は after the te-form of verbs, contract ては into ちゃ."),             
                    i18n: Some(vec![TransformI18n
               {
                    language: "ja",
                    name: "～ちゃ",
                    description: Some("「～ては」の縮約系"),
                },
                    ]),
            rules: vec![
                suffix_inflection("ちゃ", "る", vec!["v5"], vec!["v1"]),
                suffix_inflection("いじゃ", "ぐ", vec!["v5"], vec!["v5"]),
                suffix_inflection("いちゃ", "く", vec!["v5"], vec!["v5"]),
                suffix_inflection("しちゃ", "す", vec!["v5"], vec!["v5"]),
                suffix_inflection("っちゃ", "う", vec!["v5"], vec!["v5"]),
                suffix_inflection("っちゃ", "く", vec!["v5"], vec!["v5"]),
                suffix_inflection("っちゃ", "つ", vec!["v5"], vec!["v5"]),
                suffix_inflection("っちゃ", "る", vec!["v5"], vec!["v5"]),
                suffix_inflection("んじゃ", "ぬ", vec!["v5"], vec!["v5"]),
                suffix_inflection("んじゃ", "ぶ", vec!["v5"], vec!["v5"]),
                suffix_inflection("んじゃ", "む", vec!["v5"], vec!["v5"]),
                suffix_inflection("じちゃ", "ずる", vec!["v5"], vec!["vz"]),
                suffix_inflection("しちゃ", "する", vec!["v5"], vec!["vs"]),
                suffix_inflection("為ちゃ", "為る", vec!["v5"], vec!["vs"]),
                suffix_inflection("きちゃ", "くる", vec!["v5"], vec!["vk"]),
                suffix_inflection("来ちゃ", "来る", vec!["v5"], vec!["vk"]),
                suffix_inflection("來ちゃ", "來る", vec!["v5"], vec!["vk"]),

            ],

        }
    )

    ])
});

pub static JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: "ja",
        conditions: &CONDITIONS,
        transforms: &TRANSFORMS,
    });
