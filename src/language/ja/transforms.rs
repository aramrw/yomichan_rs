use crate::language::transformer_d::{
    Condition, ConditionMap, LanguageTransformDescriptor, RuleI18n, Transform, TransformI18n,
    TransformMap,
};
use crate::language::transforms::suffix_inflection;

use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, LazyLock};

const SHIMAU_ENGLISH_DESCRIPTION: &str = "1. Shows a sense of regret/surprise when you did have volition in doing something, but it turned out to be bad to do.\n2. Shows perfective/punctual achievement. This shows that an action has been completed.\n 3. Shows unintentional action–“accidentally”.\n";

const PASSIVE_ENGLISH_DESCRIPTION: &str = "1. Indicates an action received from an action performer.\n2. Expresses respect for the subject of action performer.\n";

/// a smaller version of japanese conditions for testing.
pub static TEST_CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
    HashMap::from([(
        String::from("v"),
        Condition {
            name: "Verb",
            is_dictionary_form: false,
            i18n: Some(&[RuleI18n {
                language: "ja",
                name: "動詞",
            }]),
            sub_conditions: Some(&["v1", "v5", "vk", "vs", "vz"]),
        },
    )])
});
/// a smaller version of japanese transformmap for testing.
pub static TEST_TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    HashMap::from([(
        "-ya",
        Transform {
            name: "-ya",
            description: Some("Contraction of -ba.".into()),
            i18n: Some(vec![TransformI18n {
                language: "ja",
                name: "～ゃ",
                description: Some("仮定形の縮約系"),
            }]),
            rules: vec![
                suffix_inflection("けりゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("きゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("や", "えば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("きゃ", "けば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("ぎゃ", "げば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("しゃ", "せば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("ちゃ", "てば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("にゃ", "ねば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("びゃ", "べば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("みゃ", "めば", vec!["-ya"], vec!["-ba"]),
                suffix_inflection("りゃ", "れば", vec!["-ya"], vec!["-ba"]),
            ],
        },
    )])
});

/// a smaller version of japanese trabsforms for testing.
pub static TEST_JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: String::from("ja"),
        conditions: CONDITIONS.clone(),
        transforms: TRANSFORMS.clone(),
    });

pub static CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
    HashMap::from([
        (
            String::from("v"),
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
            String::from("v1"),
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
            String::from("v1d"),
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
            String::from("v1p"),
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
            String::from("v5"),
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
            String::from("v5d"),
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
            String::from("v5m"),
            Condition {
                name: "Godan verb, polite (masu) form",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            String::from("vk"),
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
            String::from("vs"),
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
            String::from("vz"),
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
            String::from("adj-i"),
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
            String::from("-te"),
            Condition {
                name: "Intermediate -te endings for progressive or perfect tense",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            String::from("-ba"),
            Condition {
                name: "Intermediate -ba endings for conditional contraction",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            String::from("adv"),
            Condition {
                name: "Intermediate -ku endings for adverbs",
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            String::from("past"),
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
    HashMap::from([
        (
            "-ba",
            Transform {
                name: "-ba",
                description: Some(
                    "1. Conditional form; shows that the previous stated condition's establishment is the condition \
                    for the latter stated condition to occur.\n2. Shows a trigger for a latter stated perception or \
                    judgment.\nUsage: Attach ば to the hypothetical/realis form (kateikei/izenkei) of verbs and i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ば",
                    description: Some("仮定形"),
                }]),
                rules: vec![
                    suffix_inflection("ければ", "い", vec!["-ba"], vec!["adj-i"]),
                    suffix_inflection("えば", "う", vec!["-ba"], vec!["v5"]),
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
                description: Some("Contraction of -ba.".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ゃ",
                    description: Some("仮定形の縮約系"),
                }]),
                rules: vec![
                    suffix_inflection("けりゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("きゃ", "ければ", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("や", "えば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("きゃ", "けば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("ぎゃ", "げば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("しゃ", "せば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("ちゃ", "てば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("にゃ", "ねば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("びゃ", "べば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("みゃ", "めば", vec!["-ya"], vec!["-ba"]),
                    suffix_inflection("りゃ", "れば", vec!["-ya"], vec!["-ba"]),
                ],
            },
        ),
        (
            "-cha",
            Transform {
                name: "-cha",
                description: Some(
                    "Contraction of ～ては.\n1. Explains how something always happens under the condition that it marks.\n\
                    2. Expresses the repetition (of a series of) actions.\n3. Indicates a hypothetical situation in \
                    which the speaker gives a (negative) evaluation about the other party's intentions.\n\
                    4. Used in `Must Not` patterns like ～てはいけない.\nUsage: Attach は after the te-form of verbs, \
                    contract ては into ちゃ.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ちゃ",
                    description: Some("「～ては」の縮約系"),
                }]),
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
            },
        ),
        (
            "-chau",
            Transform {
                name: "-chau",
                description: Some(format!("Contraction of -shimau.\n{SHIMAU_ENGLISH_DESCRIPTION}Usage: Attach しまう after the te-form of verbs, contract てしまう into ちゃう.")),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ちゃう",
                    description: Some("「～てしまう」のややくだけた口頭語的表現"),
                }]),
                rules: vec![
                    suffix_inflection("ちゃう",   "る", vec!["v5"], vec!["v1"]),
                    suffix_inflection("いじゃう", "ぐ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("いちゃう", "く", vec!["v5"], vec!["v5"]),
                    suffix_inflection("しちゃう", "す", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちゃう", "う", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちゃう", "く", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちゃう", "つ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちゃう", "る", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじゃう", "ぬ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじゃう", "ぶ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじゃう", "む", vec!["v5"], vec!["v5"]),
                    suffix_inflection("じちゃう", "ずる", vec!["v5"], vec!["vz"]),
                    suffix_inflection("しちゃう", "する", vec!["v5"], vec!["vs"]),
                    suffix_inflection("為ちゃう", "為る", vec!["v5"], vec!["vs"]),
                    suffix_inflection("きちゃう", "くる", vec!["v5"], vec!["vk"]),
                    suffix_inflection("来ちゃう", "来る", vec!["v5"], vec!["vk"]),
                    suffix_inflection("來ちゃう", "來る", vec!["v5"], vec!["vk"]),
                ],
            }
        ),
        (
            "-chimau",
            Transform {
                name: "-chimau",
                description: Some(format!(
                    "Contraction of -shimau.\n{SHIMAU_ENGLISH_DESCRIPTION}Usage: 
                    Attach しまう after the te-form of verbs, contract てしまう into ちまう."
                )),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ちまう",
                    description: Some("「～てしまう」の音変化"),
                }]),
                rules: vec![
                    suffix_inflection("ちまう", "る", vec!["v5"], vec!["v1"]),
                    suffix_inflection("いじまう", "ぐ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("いちまう", "く", vec!["v5"], vec!["v5"]),
                    suffix_inflection("しちまう", "す", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちまう", "う", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちまう", "く", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちまう", "つ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っちまう", "る", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじまう", "ぬ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじまう", "ぶ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んじまう", "む", vec!["v5"], vec!["v5"]),
                    suffix_inflection("じちまう", "ずる", vec!["v5"], vec!["vz"]),
                    suffix_inflection("しちまう", "する", vec!["v5"], vec!["vs"]),
                    suffix_inflection("為ちまう", "為る", vec!["v5"], vec!["vs"]),
                    suffix_inflection("きちまう", "くる", vec!["v5"], vec!["vk"]),
                    suffix_inflection("来ちまう", "来る", vec!["v5"], vec!["vk"]),
                    suffix_inflection("來ちまう", "來る", vec!["v5"], vec!["vk"]),
                ],
            }
        ),
        (
            "-shimau",
            Transform {
                name: "-shimau",
                description: Some(format!(
                    "{SHIMAU_ENGLISH_DESCRIPTION}Usage: Attach しまう after the te-form of verbs."
                )),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～しまう",
                    description: Some("その動作がすっかり終わる、その状態が完成することを表す。
                    終わったことを強調したり、不本意である、困ったことになった、などの気持ちを添えたりすることもある。"),
                }]),
                rules: vec![
                    suffix_inflection("てしまう", "て", vec!["v5"], vec!["-te"]),
                    suffix_inflection("でしまう", "で", vec!["v5"], vec!["-te"]),
                ],
            }
        ),
        (
            "-nasai",
            Transform {
                name: "-nasai",
                description: Some(
                    "Polite imperative suffix.\nUsage: Attach なさい after the continuative form (renyoukei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～なさい",
                    description: Some("動詞「なさる」の命令形"),
                }]),
                rules: vec![
                    suffix_inflection("なさい", "る", vec!["-nasai"], vec!["v1"]),
                    suffix_inflection("いなさい", "う", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("きなさい", "く", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("ぎなさい", "ぐ", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("しなさい", "す", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("ちなさい", "つ", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("になさい", "ぬ", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("びなさい", "ぶ", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("みなさい", "む", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("りなさい", "る", vec!["-nasai"], vec!["v5"]),
                    suffix_inflection("じなさい", "ずる", vec!["-nasai"], vec!["vz"]),
                    suffix_inflection("しなさい", "する", vec!["-nasai"], vec!["vs"]),
                    suffix_inflection("為なさい", "為る", vec!["-nasai"], vec!["vs"]),
                    suffix_inflection("きなさい", "くる", vec!["-nasai"], vec!["vk"]),
                    suffix_inflection("来なさい", "来る", vec!["-nasai"], vec!["vk"]),
                    suffix_inflection("來なさい", "來る", vec!["-nasai"], vec!["vk"]),
                ],
            }
        ),
        (
            "-sou",
            Transform {
                name: "-sou",
                description: Some(
                    "Appearing that; looking like.\n
                     Usage: Attach そう to the continuative form (renyoukei) of verbs, or to the stem of adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～そう",
                    description: Some("そういう様子だ、そうなる様子だということ、すなわち様態を表す助動詞。"),
                }]),
                rules: vec![
                    suffix_inflection("そう", "い", vec![], vec!["adj-i"]),
                    suffix_inflection("そう", "る", vec![], vec!["v1"]),
                    suffix_inflection("いそう", "う", vec![], vec!["v5"]),
                    suffix_inflection("きそう", "く", vec![], vec!["v5"]),
                    suffix_inflection("ぎそう", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("しそう", "す", vec![], vec!["v5"]),
                    suffix_inflection("ちそう", "つ", vec![], vec!["v5"]),
                    suffix_inflection("にそう", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("びそう", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("みそう", "む", vec![], vec!["v5"]),
                    suffix_inflection("りそう", "る", vec![], vec!["v5"]),
                    suffix_inflection("じそう", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("しそう", "する", vec![], vec!["vs"]),
                    suffix_inflection("為そう", "為る", vec![], vec!["vs"]),
                    suffix_inflection("きそう", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来そう", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來そう", "來る", vec![], vec!["vk"]),
                ],
            }
        ),
        (
            "-sugiru",
            Transform {
                name: "-sugiru",
                description: Some(
                    "Shows something \"is too...\" or someone is doing something \"too much\".\n
                    Usage: Attach すぎる to the continuative form (renyoukei) of verbs, or to the stem of adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～すぎる",
                    description: Some("程度や限度を超える"),
                }]),
                rules: vec![
                    suffix_inflection("すぎる", "い", vec!["v1"], vec!["adj-i"]),
                    suffix_inflection("すぎる", "る", vec!["v1"], vec!["v1"]),
                    suffix_inflection("いすぎる", "う", vec!["v1"], vec!["v5"]),
                    suffix_inflection("きすぎる", "く", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ぎすぎる", "ぐ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("しすぎる", "す", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ちすぎる", "つ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("にすぎる", "ぬ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("びすぎる", "ぶ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("みすぎる", "む", vec!["v1"], vec!["v5"]),
                    suffix_inflection("りすぎる", "る", vec!["v1"], vec!["v5"]),
                    suffix_inflection("じすぎる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("しすぎる", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("為すぎる", "為る", vec!["v1"], vec!["vs"]),
                    suffix_inflection("きすぎる", "くる", vec!["v1"], vec!["vk"]),
                    suffix_inflection("来すぎる", "来る", vec!["v1"], vec!["vk"]),
                    suffix_inflection("來すぎる", "來る", vec!["v1"], vec!["vk"]),
                ],
            }
        ),
        (
            "-tai",
            Transform {
                name: "-tai",
                description: Some(
                    "1. Expresses the feeling of desire or hope.\n
                    2. Used in ...たいと思います, an indirect way of saying what the speaker intends to do.\n
                    Usage: Attach たい to the continuative form (renyoukei) of verbs. たい itself conjugates as i-adjective.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～たい",
                    description: Some("することをのぞんでいる、という、希望や願望の気持ちをあらわす。"),
                }]),
                rules: vec![
                    suffix_inflection("たい", "る", vec!["adj-i"], vec!["v1"]),
                    suffix_inflection("いたい", "う", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("きたい", "く", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("ぎたい", "ぐ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("したい", "す", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("ちたい", "つ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("にたい", "ぬ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("びたい", "ぶ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("みたい", "む", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("りたい", "る", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("じたい", "ずる", vec!["adj-i"], vec!["vz"]),
                    suffix_inflection("したい", "する", vec!["adj-i"], vec!["vs"]),
                    suffix_inflection("為たい", "為る", vec!["adj-i"], vec!["vs"]),
                    suffix_inflection("きたい", "くる", vec!["adj-i"], vec!["vk"]),
                    suffix_inflection("来たい", "来る", vec!["adj-i"], vec!["vk"]),
                    suffix_inflection("來たい", "來る", vec!["adj-i"], vec!["vk"]),
                ],
            }
        ),
        (
            "-tara",
            Transform {
                name: "-tara",
                description: Some(
                    "1. Denotes the latter stated event is a continuation of the previous stated event.\n
                        2. Assumes that a matter has been completed or concluded.\n
                        Usage: Attach たら to the continuative form (renyoukei) 
                        of verbs after euphonic change form, かったら to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～たら",
                    description: Some("仮定をあらわす・…すると・したあとに"),
                }]),
                rules: vec![
                    suffix_inflection("かったら", "い", vec![], vec!["adj-i"]),
                    suffix_inflection("たら", "る", vec![], vec!["v1"]),
                    suffix_inflection("いたら", "く", vec![], vec!["v5"]),
                    suffix_inflection("いだら", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("したら", "す", vec![], vec!["v5"]),
                    suffix_inflection("ったら", "う", vec![], vec!["v5"]),
                    suffix_inflection("ったら", "つ", vec![], vec!["v5"]),
                    suffix_inflection("ったら", "る", vec![], vec!["v5"]),
                    suffix_inflection("んだら", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("んだら", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("んだら", "む", vec![], vec!["v5"]),
                    suffix_inflection("じたら", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("したら", "する", vec![], vec!["vs"]),
                    suffix_inflection("為たら", "為る", vec![], vec!["vs"]),
                    suffix_inflection("きたら", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来たら", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來たら", "來る", vec![], vec!["vk"]),
                    suffix_inflection("いったら", "いく", vec![], vec!["v5"]),
                    suffix_inflection("おうたら", "おう", vec![], vec!["v5"]),
                    suffix_inflection("こうたら", "こう", vec![], vec!["v5"]),
                    suffix_inflection("そうたら", "そう", vec![], vec!["v5"]),
                    suffix_inflection("とうたら", "とう", vec![], vec!["v5"]),
                    suffix_inflection("行ったら", "行く", vec![], vec!["v5"]),
                    suffix_inflection("逝ったら", "逝く", vec![], vec!["v5"]),
                    suffix_inflection("往ったら", "往く", vec![], vec!["v5"]),
                    suffix_inflection("請うたら", "請う", vec![], vec!["v5"]),
                    suffix_inflection("乞うたら", "乞う", vec![], vec!["v5"]),
                    suffix_inflection("恋うたら", "恋う", vec![], vec!["v5"]),
                    suffix_inflection("問うたら", "問う", vec![], vec!["v5"]),
                    suffix_inflection("負うたら", "負う", vec![], vec!["v5"]),
                    suffix_inflection("沿うたら", "沿う", vec![], vec!["v5"]),
                    suffix_inflection("添うたら", "添う", vec![], vec!["v5"]),
                    suffix_inflection("副うたら", "副う", vec![], vec!["v5"]),
                    suffix_inflection("厭うたら", "厭う", vec![], vec!["v5"]),
                    suffix_inflection("のたもうたら", "のたまう", vec![], vec!["v5"]),
                ],
            }
        ),
        (
            "-tari",
            Transform {
                name: "-tari",
                description: Some(
                    "1. Shows two actions occurring back and forth (when used with two verbs).\n
                    2. Shows examples of actions and states (when used with multiple verbs and adjectives).\n
                    Usage: Attach たり to the continuative form (renyoukei) 
                    of verbs after euphonic change form, かったり to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～たり",
                    description: Some("ある動作を例示的にあげることを表わす。"),
                }]),
                rules: vec![
                    suffix_inflection("かったり", "い", vec![], vec!["adj-i"]),
                    suffix_inflection("たり", "る", vec![], vec!["v1"]),
                    suffix_inflection("いたり", "く", vec![], vec!["v5"]),
                    suffix_inflection("いだり", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("したり", "す", vec![], vec!["v5"]),
                    suffix_inflection("ったり", "う", vec![], vec!["v5"]),
                    suffix_inflection("ったり", "つ", vec![], vec!["v5"]),
                    suffix_inflection("ったり", "る", vec![], vec!["v5"]),
                    suffix_inflection("んだり", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("んだり", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("んだり", "む", vec![], vec!["v5"]),
                    suffix_inflection("じたり", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("したり", "する", vec![], vec!["vs"]),
                    suffix_inflection("為たり", "為る", vec![], vec!["vs"]),
                    suffix_inflection("きたり", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来たり", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來たり", "來る", vec![], vec!["vk"]),
                    suffix_inflection("いったり", "いく", vec![], vec!["v5"]),
                    suffix_inflection("おうたり", "おう", vec![], vec!["v5"]),
                    suffix_inflection("こうたり", "こう", vec![], vec!["v5"]),
                    suffix_inflection("そうたり", "そう", vec![], vec!["v5"]),
                    suffix_inflection("とうたり", "とう", vec![], vec!["v5"]),
                    suffix_inflection("行ったり", "行く", vec![], vec!["v5"]),
                    suffix_inflection("逝ったり", "逝く", vec![], vec!["v5"]),
                    suffix_inflection("往ったり", "往く", vec![], vec!["v5"]),
                    suffix_inflection("請うたり", "請う", vec![], vec!["v5"]),
                    suffix_inflection("乞うたり", "乞う", vec![], vec!["v5"]),
                    suffix_inflection("恋うたり", "恋う", vec![], vec!["v5"]),
                    suffix_inflection("問うたり", "問う", vec![], vec!["v5"]),
                    suffix_inflection("負うたり", "負う", vec![], vec!["v5"]),
                    suffix_inflection("沿うたり", "沿う", vec![], vec!["v5"]),
                    suffix_inflection("添うたり", "添う", vec![], vec!["v5"]),
                    suffix_inflection("副うたり", "副う", vec![], vec!["v5"]),
                    suffix_inflection("厭うたり", "厭う", vec![], vec!["v5"]),
                    suffix_inflection("のたもうたり", "のたまう", vec![], vec!["v5"]),
                ],
            }
        ),
        (
            "-te",
            Transform {
                name: "-te",
                description: Some(
                    "te-form.\nIt has a myriad of meanings. Primarily, it is a conjunctive particle that connects two clauses together.\n
                    Usage: Attach て to the continuative form (renyoukei) 
                    of verbs after euphonic change form, くて to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～て",
                    description: Some("て（で）形"),
                }]),
                rules: vec![
                    suffix_inflection("くて", "い", vec!["-te"], vec!["adj-i"]),
                    suffix_inflection("て", "る", vec!["-te"], vec!["v1"]),
                    suffix_inflection("いて", "く", vec!["-te"], vec!["v5"]),
                    suffix_inflection("いで", "ぐ", vec!["-te"], vec!["v5"]),
                    suffix_inflection("して", "す", vec!["-te"], vec!["v5"]),
                    suffix_inflection("って", "う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("って", "つ", vec!["-te"], vec!["v5"]),
                    suffix_inflection("って", "る", vec!["-te"], vec!["v5"]),
                    suffix_inflection("んで", "ぬ", vec!["-te"], vec!["v5"]),
                    suffix_inflection("んで", "ぶ", vec!["-te"], vec!["v5"]),
                    suffix_inflection("んで", "む", vec!["-te"], vec!["v5"]),
                    suffix_inflection("じて", "ずる", vec!["-te"], vec!["vz"]),
                    suffix_inflection("して", "する", vec!["-te"], vec!["vs"]),
                    suffix_inflection("為て", "為る", vec!["-te"], vec!["vs"]),
                    suffix_inflection("きて", "くる", vec!["-te"], vec!["vk"]),
                    suffix_inflection("来て", "来る", vec!["-te"], vec!["vk"]),
                    suffix_inflection("來て", "來る", vec!["-te"], vec!["vk"]),
                    suffix_inflection("いって", "いく", vec!["-te"], vec!["v5"]),
                    suffix_inflection("おうて", "おう", vec!["-te"], vec!["v5"]),
                    suffix_inflection("こうて", "こう", vec!["-te"], vec!["v5"]),
                    suffix_inflection("そうて", "そう", vec!["-te"], vec!["v5"]),
                    suffix_inflection("とうて", "とう", vec!["-te"], vec!["v5"]),
                    suffix_inflection("行って", "行く", vec!["-te"], vec!["v5"]),
                    suffix_inflection("逝って", "逝く", vec!["-te"], vec!["v5"]),
                    suffix_inflection("往って", "往く", vec!["-te"], vec!["v5"]),
                    suffix_inflection("請うて", "請う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("乞うて", "乞う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("恋うて", "恋う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("問うて", "問う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("負うて", "負う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("沿うて", "沿う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("添うて", "添う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("副うて", "副う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("厭うて", "厭う", vec!["-te"], vec!["v5"]),
                    suffix_inflection("のたもうて", "のたまう", vec!["-te"], vec!["v5"]),
                    suffix_inflection("まして", "ます", vec![], vec!["v"]),
                ],
            }
        ),
        (
            "-zu",
            Transform {
                name: "-zu",
                description: Some(
                    "1. Negative form of verbs.\n
                        2. Continuative form (renyoukei) of the particle ぬ (nu).\n
                        Usage: Attach ず to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ず",
                    description: Some("口語の否定の助動詞「ぬ」の連用形"),
                }]),
                rules: vec![
                    suffix_inflection("ず", "る", vec![], vec!["v1"]),
                    suffix_inflection("かず", "く", vec![], vec!["v5"]),
                    suffix_inflection("がず", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("さず", "す", vec![], vec!["v5"]),
                    suffix_inflection("たず", "つ", vec![], vec!["v5"]),
                    suffix_inflection("なず", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ばず", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("まず", "む", vec![], vec!["v5"]),
                    suffix_inflection("らず", "る", vec![], vec!["v5"]),
                    suffix_inflection("わず", "う", vec![], vec!["v5"]),
                    suffix_inflection("ぜず", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("せず", "する", vec![], vec!["vs"]),
                    suffix_inflection("為ず", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こず", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来ず", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來ず", "來る", vec![], vec!["vk"]),
                ],
            }
        ),
        (
            "-nu",
            Transform {
                name: "-nu",
                description: Some(
                    "Negative form of verbs.\nUsage: Attach ぬ to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ぬ",
                    description: Some("動作・状態などを「…ない」と否定することを表わす。"),
                }]),
                rules: vec![
                    suffix_inflection("ぬ", "る", vec![], vec!["v1"]),
                    suffix_inflection("かぬ", "く", vec![], vec!["v5"]),
                    suffix_inflection("がぬ", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("さぬ", "す", vec![], vec!["v5"]),
                    suffix_inflection("たぬ", "つ", vec![], vec!["v5"]),
                    suffix_inflection("なぬ", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ばぬ", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("まぬ", "む", vec![], vec!["v5"]),
                    suffix_inflection("らぬ", "る", vec![], vec!["v5"]),
                    suffix_inflection("わぬ", "う", vec![], vec!["v5"]),
                    suffix_inflection("ぜぬ", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("せぬ", "する", vec![], vec!["vs"]),
                    suffix_inflection("為ぬ", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こぬ", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来ぬ", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來ぬ", "來る", vec![], vec!["vk"]),
                ],
            }
        ),
        (
            "-n",
            Transform {
                name: "-n",
                description: Some(
                    "1. Negative form of verbs; a sound change of ぬ.\n
                        2. (As …んばかり) Shows an action or condition is on the verge of occurring, or an excessive/extreme degree.\n
                        Usage: Attach ん to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja",
                    name: "～ん",
                    description: Some("〔否定の助動詞〕…ない"),
                }]),
                rules: vec![
                    suffix_inflection("ん", "る", vec![], vec!["v1"]),
                    suffix_inflection("かん", "く", vec![], vec!["v5"]),
                    suffix_inflection("がん", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("さん", "す", vec![], vec!["v5"]),
                    suffix_inflection("たん", "つ", vec![], vec!["v5"]),
                    suffix_inflection("なん", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ばん", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("まん", "む", vec![], vec!["v5"]),
                    suffix_inflection("らん", "る", vec![], vec!["v5"]),
                    suffix_inflection("わん", "う", vec![], vec!["v5"]),
                    suffix_inflection("ぜん", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("せん", "する", vec![], vec!["vs"]),
                    suffix_inflection("為ん", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こん", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来ん", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來ん", "來る", vec![], vec!["vk"]),
                ],
            }
        ),
        (
            "-mu",
            Transform {
                name: "-mu",
                description: Some(
                    "Archaic.\n
                    Shows an inference of a certain matter.\n 
                    Shows speaker's intention.\n
                    Usage: Attach む to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～む",
                        description: Some("…だろう"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("む", "る", vec![], vec!["v1"]),
                    suffix_inflection("かむ", "く", vec![], vec!["v5"]),
                    suffix_inflection("がむ", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("さむ", "す", vec![], vec!["v5"]),
                    suffix_inflection("たむ", "つ", vec![], vec!["v5"]),
                    suffix_inflection("なむ", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ばむ", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("まむ", "む", vec![], vec!["v5"]),
                    suffix_inflection("らむ", "る", vec![], vec!["v5"]),
                    suffix_inflection("わむ", "う", vec![], vec!["v5"]),
                    suffix_inflection("ぜむ", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("せむ", "する", vec![], vec!["vs"]),
                    suffix_inflection("為む", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こむ", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来む", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來む", "來る", vec![], vec!["vk"]),
                ],
            },
        ),
        (
            "-zaru",
            Transform {
                name: "-zaru",
                description: Some(
                    "Negative form of verbs.\n
                    Usage: Attach ざる to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～ざる",
                        description: Some("…ない…"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ざる", "る", vec![], vec!["v1"]),
                    suffix_inflection("かざる", "く", vec![], vec!["v5"]),
                    suffix_inflection("がざる", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("さざる", "す", vec![], vec!["v5"]),
                    suffix_inflection("たざる", "つ", vec![], vec!["v5"]),
                    suffix_inflection("なざる", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ばざる", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("まざる", "む", vec![], vec!["v5"]),
                    suffix_inflection("らざる", "る", vec![], vec!["v5"]),
                    suffix_inflection("わざる", "う", vec![], vec!["v5"]),
                    suffix_inflection("ぜざる", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("せざる", "する", vec![], vec!["vs"]),
                    suffix_inflection("為ざる", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こざる", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来ざる", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來ざる", "來る", vec![], vec!["vk"]),
                ],
            },
        ),
        (
            "-neba",
            Transform {
                name: "-neba",
                description: Some(
                    "1. Shows a hypothetical negation; if not ...\n. 
                    Shows a must. Used with or without ならぬ.\n
                    Usage: Attach ねば to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～ねば",
                        description: Some("もし…ないなら。…なければならない。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ねば", "る", vec!["-ba"], vec!["v1"]),
                    suffix_inflection("かねば", "く", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("がねば", "ぐ", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("さねば", "す", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("たねば", "つ", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("なねば", "ぬ", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("ばねば", "ぶ", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("まねば", "む", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("らねば", "る", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("わねば", "う", vec!["-ba"], vec!["v5"]),
                    suffix_inflection("ぜねば", "ずる", vec!["-ba"], vec!["vz"]),
                    suffix_inflection("せねば", "する", vec!["-ba"], vec!["vs"]),
                    suffix_inflection("為ねば", "為る", vec!["-ba"], vec!["vs"]),
                    suffix_inflection("こねば", "くる", vec!["-ba"], vec!["vk"]),
                    suffix_inflection("来ねば", "来る", vec!["-ba"], vec!["vk"]),
                    suffix_inflection("來ねば", "來る", vec!["-ba"], vec!["vk"]),
                ],
            },
        ),
        (
            "-ku",
            Transform {
                name: "-ku",
                description: Some(
                    "Adverbial form of i-adjectives.\n".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "連用形",
                        description: Some("〔形容詞で〕用言へ続く。例、「大きく育つ」の「大きく」。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("く", "い", vec!["adv"], vec!["adj-i"]),
                ],
            },
        ),
        (
            "causative",
            Transform {
                name: "causative",
                description: Some(
                    "Describes the intention to make someone do something.\n
                    Usage: Attach させる to the irrealis form (mizenkei) of ichidan verbs and くる.\n
                    Attach せる to the irrealis form (mizenkei) of godan verbs and する.\n
                    It itself conjugates as an ichidan verb.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "使役形",
                        description: Some("だれかにある行為をさせる意を表わす時の言い方。例、「行かせる」の「せる」。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("させる", "る", vec!["v1"], vec!["v1"]),
                    suffix_inflection("かせる", "く", vec!["v1"], vec!["v5"]),
                    suffix_inflection("がせる", "ぐ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("させる", "す", vec!["v1"], vec!["v5"]),
                    suffix_inflection("たせる", "つ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("なせる", "ぬ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ばせる", "ぶ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ませる", "む", vec!["v1"], vec!["v5"]),
                    suffix_inflection("らせる", "る", vec!["v1"], vec!["v5"]),
                    suffix_inflection("わせる", "う", vec!["v1"], vec!["v5"]),
                    suffix_inflection("じさせる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("ぜさせる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("させる", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("為せる", "為る", vec!["v1"], vec!["vs"]),
                    suffix_inflection("せさせる", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("為させる", "為る", vec!["v1"], vec!["vs"]),
                    suffix_inflection("こさせる", "くる", vec!["v1"], vec!["vk"]),
                    suffix_inflection("来させる", "来る", vec!["v1"], vec!["vk"]),
                    suffix_inflection("來させる", "來る", vec!["v1"], vec!["vk"]),
                ],
            },
        ),
        (
            "imperative",
            Transform {
                name: "imperative",
                description: Some(
                    "1. To give orders.\n
                    2. (As あれ) Represents the fact that it will never change no matter the circumstances.\n
                    3. Express a feeling of hope.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "命令形",
                        description: Some("命令の意味を表わすときの形。例、「行け」。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ろ", "る", vec![], vec!["v1"]),
                    suffix_inflection("よ", "る", vec![], vec!["v1"]),
                    suffix_inflection("え", "う", vec![], vec!["v5"]),
                    suffix_inflection("け", "く", vec![], vec!["v5"]),
                    suffix_inflection("げ", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("せ", "す", vec![], vec!["v5"]),
                    suffix_inflection("て", "つ", vec![], vec!["v5"]),
                    suffix_inflection("ね", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("べ", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("め", "む", vec![], vec!["v5"]),
                    suffix_inflection("れ", "る", vec![], vec!["v5"]),
                    suffix_inflection("じろ", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("ぜよ", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("しろ", "する", vec![], vec!["vs"]),
                    suffix_inflection("せよ", "する", vec![], vec!["vs"]),
                    suffix_inflection("為ろ", "為る", vec![], vec!["vs"]),
                    suffix_inflection("為よ", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こい", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来い", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來い", "來る", vec![], vec!["vk"]),
                ],
            },
        ),
        (
            "imperative negative",
            Transform {
                name: "imperative negative",
                description: None,
                i18n: None,
                rules: vec![
                    suffix_inflection("な", "", vec!["-na"], vec!["v"]),
                ],
            },
        ),
        (
            "continuative",
            Transform {
                name: "continuative",
                description: Some(
                    "Used to indicate actions that are (being) carried out.\n
                    Refers to the renyoukei, the part of the verb after conjugating with -masu and dropping masu.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "連用形",
                        description: Some(
                            "〔動詞などで〕「ます」などに続く。例、「バスを降りて歩きます」の「降り」「歩き」。"
                        ),
                    },
                ]),
                rules: vec![
                    suffix_inflection("い", "いる", vec![], vec!["v1d"]),
                    suffix_inflection("え", "える", vec![], vec!["v1d"]),
                    suffix_inflection("き", "きる", vec![], vec!["v1d"]),
                    suffix_inflection("ぎ", "ぎる", vec![], vec!["v1d"]),
                    suffix_inflection("け", "ける", vec![], vec!["v1d"]),
                    suffix_inflection("げ", "げる", vec![], vec!["v1d"]),
                    suffix_inflection("じ", "じる", vec![], vec!["v1d"]),
                    suffix_inflection("せ", "せる", vec![], vec!["v1d"]),
                    suffix_inflection("ぜ", "ぜる", vec![], vec!["v1d"]),
                    suffix_inflection("ち", "ちる", vec![], vec!["v1d"]),
                    suffix_inflection("て", "てる", vec![], vec!["v1d"]),
                    suffix_inflection("で", "でる", vec![], vec!["v1d"]),
                    suffix_inflection("に", "にる", vec![], vec!["v1d"]),
                    suffix_inflection("ね", "ねる", vec![], vec!["v1d"]),
                    suffix_inflection("ひ", "ひる", vec![], vec!["v1d"]),
                    suffix_inflection("び", "びる", vec![], vec!["v1d"]),
                    suffix_inflection("へ", "へる", vec![], vec!["v1d"]),
                    suffix_inflection("べ", "べる", vec![], vec!["v1d"]),
                    suffix_inflection("み", "みる", vec![], vec!["v1d"]),
                    suffix_inflection("め", "める", vec![], vec!["v1d"]),
                    suffix_inflection("り", "りる", vec![], vec!["v1d"]),
                    suffix_inflection("れ", "れる", vec![], vec!["v1d"]),
                    suffix_inflection("い", "う", vec![], vec!["v5"]),
                    suffix_inflection("き", "く", vec![], vec!["v5"]),
                    suffix_inflection("ぎ", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("し", "す", vec![], vec!["v5"]),
                    suffix_inflection("ち", "つ", vec![], vec!["v5"]),
                    suffix_inflection("に", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("び", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("み", "む", vec![], vec!["v5"]),
                    suffix_inflection("り", "る", vec![], vec!["v5"]),
                    suffix_inflection("き", "くる", vec![], vec!["vk"]),
                    suffix_inflection("し", "する", vec![], vec!["vs"]),
                    suffix_inflection("来", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來", "來る", vec![], vec!["vk"]),
                ],
            },
        ),
        (
            "negative",
            Transform {
                name: "negative",
                description: Some(
                    "1. Negative form of verbs.\n
                    2. Expresses a feeling of solicitation to the other party.\n
                    Usage: Attach ない to the irrealis form (mizenkei) of verbs, くない to the stem of i-adjectives. ない itself conjugates as i-adjective. ます becomes ません.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～ない",
                        description: Some(
                            "その動作・作用・状態の成立を否定することを表わす。"
                        ),
                    },
                ]),
                rules: vec![
                    suffix_inflection("くない", "い", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("ない", "る", vec!["adj-i"], vec!["v1"]),
                    suffix_inflection("かない", "く", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("がない", "ぐ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("さない", "す", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("たない", "つ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("なない", "ぬ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("ばない", "ぶ", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("まない", "む", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("らない", "る", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("わない", "う", vec!["adj-i"], vec!["v5"]),
                    suffix_inflection("じない", "ずる", vec!["adj-i"], vec!["vz"]),
                    suffix_inflection("しない", "する", vec!["adj-i"], vec!["vs"]),
                    suffix_inflection("為ない", "為る", vec!["adj-i"], vec!["vs"]),
                    suffix_inflection("こない", "くる", vec!["adj-i"], vec!["vk"]),
                    suffix_inflection("来ない", "来る", vec!["adj-i"], vec!["vk"]),
                    suffix_inflection("來ない", "來る", vec!["adj-i"], vec!["vk"]),
                    suffix_inflection("ません", "ます", vec!["v"], vec!["v"]),
                ],
            },
        ),
        (
            "-sa",
            Transform {
                name: "-sa",
                description: Some(
                    "Nominalizing suffix of i-adjectives indicating nature, state, mind or degree.\n
                    Usage: Attach さ to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～さ",
                        description: Some("こと。程度。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("さ", "い", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "passive",
            Transform {
                name: "passive",
                description: Some(format!("{PASSIVE_ENGLISH_DESCRIPTION} 
                Usage: Attach れる to the irrealis form (mizenkei) of godan verbs.")),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "受身形",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かれる", "く", vec!["v1"], vec!["v5"]),
                    suffix_inflection("がれる", "ぐ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("される", "す", vec!["v1"], vec!["v5"]),
                    suffix_inflection("たれる", "つ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("なれる", "ぬ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ばれる", "ぶ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("まれる", "む", vec!["v1"], vec!["v5"]),
                    suffix_inflection("われる", "う", vec!["v1"], vec!["v5"]),
                    suffix_inflection("られる", "る", vec!["v1"], vec!["v5"]),
                    suffix_inflection("じされる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("ぜされる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("される", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("為れる", "為る", vec!["v1"], vec!["vs"]),
                    suffix_inflection("こられる", "くる", vec!["v1"], vec!["vk"]),
                    suffix_inflection("来られる", "来る", vec!["v1"], vec!["vk"]),
                    suffix_inflection("來られる", "來る", vec!["v1"], vec!["vk"]),
                ],
            },
        ),
        (
            "-ta",
            Transform {
                name: "-ta",
                description: Some(
                    "1. Indicates a reality that has happened in the past.\n
                    2. Indicates the completion of an action.\n
                    3. Indicates the confirmation of a matter.\n
                    4. Indicates the speaker's confidence that the action will definitely be fulfilled.\n
                    5. Indicates the events that occur before the main clause are represented as relative past.\n
                    6. Indicates a mild imperative/command.\n
                    Usage: Attach た to the continuative form (renyoukei) of verbs after euphonic change form, 
                    かった to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～た・かった形",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かった", "い", vec!["past"], vec!["adj-i"]),
                    suffix_inflection("た", "る", vec!["past"], vec!["v1"]),
                    suffix_inflection("いた", "く", vec!["past"], vec!["v5"]),
                    suffix_inflection("いだ", "ぐ", vec!["past"], vec!["v5"]),
                    suffix_inflection("した", "す", vec!["past"], vec!["v5"]),
                    suffix_inflection("った", "う", vec!["past"], vec!["v5"]),
                    suffix_inflection("った", "つ", vec!["past"], vec!["v5"]),
                    suffix_inflection("った", "る", vec!["past"], vec!["v5"]),
                    suffix_inflection("んだ", "ぬ", vec!["past"], vec!["v5"]),
                    suffix_inflection("んだ", "ぶ", vec!["past"], vec!["v5"]),
                    suffix_inflection("んだ", "む", vec!["past"], vec!["v5"]),
                    suffix_inflection("じた", "ずる", vec!["past"], vec!["vz"]),
                    suffix_inflection("した", "する", vec!["past"], vec!["vs"]),
                    suffix_inflection("為た", "為る", vec!["past"], vec!["vs"]),
                    suffix_inflection("きた", "くる", vec!["past"], vec!["vk"]),
                    suffix_inflection("来た", "来る", vec!["past"], vec!["vk"]),
                    suffix_inflection("來た", "來る", vec!["past"], vec!["vk"]),
                    suffix_inflection("いった", "いく", vec!["past"], vec!["v5"]),
                    suffix_inflection("おうた", "おう", vec!["past"], vec!["v5"]),
                    suffix_inflection("こうた", "こう", vec!["past"], vec!["v5"]),
                    suffix_inflection("そうた", "そう", vec!["past"], vec!["v5"]),
                    suffix_inflection("とうた", "とう", vec!["past"], vec!["v5"]),
                    suffix_inflection("行った", "行く", vec!["past"], vec!["v5"]),
                    suffix_inflection("逝った", "逝く", vec!["past"], vec!["v5"]),
                    suffix_inflection("往った", "往く", vec!["past"], vec!["v5"]),
                    suffix_inflection("請うた", "請う", vec!["past"], vec!["v5"]),
                    suffix_inflection("乞うた", "乞う", vec!["past"], vec!["v5"]),
                    suffix_inflection("恋うた", "恋う", vec!["past"], vec!["v5"]),
                    suffix_inflection("問うた", "問う", vec!["past"], vec!["v5"]),
                    suffix_inflection("負うた", "負う", vec!["past"], vec!["v5"]),
                    suffix_inflection("沿うた", "沿う", vec!["past"], vec!["v5"]),
                    suffix_inflection("添うた", "添う", vec!["past"], vec!["v5"]),
                    suffix_inflection("副うた", "副う", vec!["past"], vec!["v5"]),
                    suffix_inflection("厭うた", "厭う", vec!["past"], vec!["v5"]),
                    suffix_inflection("のたもうた", "のたまう", vec!["past"], vec!["v5"]),
                    suffix_inflection("ました", "ます", vec!["past"], vec!["v"]),
                    suffix_inflection("ませんでした", "ません", vec!["past"], vec!["v"]),
                ],
            },
        ),
        (
            "-masu",
            Transform {
                name: "-masu",
                description: Some(
                    "Polite conjugation of verbs and adjectives.\n\
                    Usage: Attach ます to the continuative form (renyoukei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～ます",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("ます", "る", vec!["v1"], vec!["v1"]),
                    suffix_inflection("います", "う", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("きます", "く", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("ぎます", "ぐ", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("します", "す", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("ちます", "つ", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("にます", "ぬ", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("びます", "ぶ", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("みます", "む", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("ります", "る", vec!["v5m"], vec!["v5d"]),
                    suffix_inflection("じます", "ずる", vec!["vz"], vec!["vz"]),
                    suffix_inflection("します", "する", vec!["vs"], vec!["vs"]),
                    suffix_inflection("為ます", "為る", vec!["vs"], vec!["vs"]),
                    suffix_inflection("きます", "くる", vec!["vk"], vec!["vk"]),
                    suffix_inflection("来ます", "来る", vec!["vk"], vec!["vk"]),
                    suffix_inflection("來ます", "來る", vec!["vk"], vec!["vk"]),
                    suffix_inflection("くあります", "い", vec!["v"], vec!["adj-i"]),
                ],
            },
        ),
        (
            "potential",
            Transform {
                name: "potential",
                description: Some(
                    "Indicates a state of being (naturally) capable of doing an action.\n\
                    Usage: Attach (ら)れる to the irrealis form (mizenkei) of ichidan verbs.\n\
                    Attach る to the imperative form (meireikei) of godan verbs.\n\
                    する becomes できる, くる becomes こ(ら)れる".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "可能",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("れる", "る", vec!["v1"], vec!["v1", "v5d"]),
                    suffix_inflection("える", "う", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("ける", "く", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("げる", "ぐ", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("せる", "す", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("てる", "つ", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("ねる", "ぬ", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("べる", "ぶ", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("める", "む", vec!["v1"], vec!["v5d"]),
                    suffix_inflection("できる", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("出来る", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("これる", "くる", vec!["v1"], vec!["vk"]),
                    suffix_inflection("来れる", "来る", vec!["v1"], vec!["vk"]),
                    suffix_inflection("來れる", "來る", vec!["v1"], vec!["vk"]),
                ],
            },
        ),
        (
            "potential or passive",
            Transform {
                name: "potential or passive",
                description: Some(
                    "Usage: Attach られる to the irrealis form (mizenkei) of ichidan verbs.\n\
                    する becomes せられる, くる becomes こられる\n\
                    3. Indicates a state of being (naturally) capable of doing an action.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "受身・自発・可能・尊敬",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("られる", "る", vec!["v1"], vec!["v1"]),
                    suffix_inflection("ざれる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("ぜられる", "ずる", vec!["v1"], vec!["vz"]),
                    suffix_inflection("せられる", "する", vec!["v1"], vec!["vs"]),
                    suffix_inflection("為られる", "為る", vec!["v1"], vec!["vs"]),
                    suffix_inflection("こられる", "くる", vec!["v1"], vec!["vk"]),
                    suffix_inflection("来られる", "来る", vec!["v1"], vec!["vk"]),
                    suffix_inflection("來られる", "來る", vec!["v1"], vec!["vk"]),
                ],
            },
        ),
        (
            "volitional",
            Transform {
                name: "volitional",
                description: Some(
                    "1. Expresses speaker's will or intention; volitional form.\n
                    2. Expresses an invitation to the other party.\n
                    3. (Used in …ようとする) Indicates being on the verge of initiating an action or transforming a state.\n
                    4. Indicates an inference of a matter.\n
                    Usage: Attach よう to the irrealis form (mizenkei) of ichidan verbs.\n
                    Attach う to the irrealis form (mizenkei) of godan verbs after -o euphonic change form.\n
                    Attach かろう to the stem of i-adjectives (4th meaning only).".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～う形",
                        description: Some("主体の意志を表わす"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("よう", "る", vec![], vec!["v1"]),
                    suffix_inflection("おう", "う", vec![], vec!["v5"]),
                    suffix_inflection("こう", "く", vec![], vec!["v5"]),
                    suffix_inflection("ごう", "ぐ", vec![], vec!["v5"]),
                    suffix_inflection("そう", "す", vec![], vec!["v5"]),
                    suffix_inflection("とう", "つ", vec![], vec!["v5"]),
                    suffix_inflection("のう", "ぬ", vec![], vec!["v5"]),
                    suffix_inflection("ぼう", "ぶ", vec![], vec!["v5"]),
                    suffix_inflection("もう", "む", vec![], vec!["v5"]),
                    suffix_inflection("ろう", "る", vec![], vec!["v5"]),
                    suffix_inflection("じよう", "ずる", vec![], vec!["vz"]),
                    suffix_inflection("しよう", "する", vec![], vec!["vs"]),
                    suffix_inflection("為よう", "為る", vec![], vec!["vs"]),
                    suffix_inflection("こよう", "くる", vec![], vec!["vk"]),
                    suffix_inflection("来よう", "来る", vec![], vec!["vk"]),
                    suffix_inflection("來よう", "來る", vec![], vec!["vk"]),
                    suffix_inflection("ましょう", "ます", vec![], vec!["v"]),
                    suffix_inflection("かろう", "い", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "causative-passive",
            Transform {
                name: "causative-passive",
                description: Some(
                    "Contraction of the passive of the causative form of verbs.\n\
                    Someone was made to do something by someone else.\n\
                    Usage: ～せられる becomes ~される (only for godan verbs)".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "使役受け身形",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かされる", "く", vec!["v1"], vec!["v5"]),
                    suffix_inflection("がされる", "ぐ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("たされる", "つ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("なされる", "ぬ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("ばされる", "ぶ", vec!["v1"], vec!["v5"]),
                    suffix_inflection("まされる", "む", vec!["v1"], vec!["v5"]),
                    suffix_inflection("らされる", "る", vec!["v1"], vec!["v5"]),
                    suffix_inflection("わされる", "う", vec!["v1"], vec!["v5"]),
                ],
            },
        ),
        (
            "-toku",
            Transform {
                name: "-toku",
                description: Some(
                    "Contraction of -teoku.\n\
                    To do certain things in advance in preparation (or in anticipation) of latter needs.\n\
                    Usage: Attach おく to the te-form of verbs, then contract ておく into とく.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～とく",
                        description: Some("「～テオク」の縮約系"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("とく", "る", vec!["v5"], vec!["v1"]),
                    suffix_inflection("いとく", "く", vec!["v5"], vec!["v5"]),
                    suffix_inflection("いどく", "ぐ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("しとく", "す", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っとく", "う", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っとく", "つ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("っとく", "る", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んどく", "ぬ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んどく", "ぶ", vec!["v5"], vec!["v5"]),
                    suffix_inflection("んどく", "む", vec!["v5"], vec!["v5"]),
                    suffix_inflection("じとく", "ずる", vec!["v5"], vec!["vz"]),
                    suffix_inflection("しとく", "する", vec!["v5"], vec!["vs"]),
                    suffix_inflection("為とく", "為る", vec!["v5"], vec!["vs"]),
                    suffix_inflection("きとく", "くる", vec!["v5"], vec!["vk"]),
                    suffix_inflection("来とく", "来る", vec!["v5"], vec!["vk"]),
                    suffix_inflection("來とく", "來る", vec!["v5"], vec!["vk"]),
                ],
            },
        ),
        (
            "-teiru",
            Transform {
                name: "-teiru",
                description: Some(
                    "1. Indicates an action continues or progresses to a point in time.\n\
                    2. Indicates an action is completed and remains as is.\n\
                    3. Indicates a state or condition that can be taken to be the result of undergoing some change.\n\
                    Usage: Attach いる to the te-form of verbs. い can be dropped in speech.\n\
                    (Slang) Attach おる to the te-form of verbs. Contracts to とる・でる in speech.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～ている",
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("ている", "て", vec!["v1"], vec!["-te"]),
                    suffix_inflection("ておる", "て", vec!["v5"], vec!["-te"]),
                    suffix_inflection("てる", "て", vec!["v1p"], vec!["-te"]),
                    suffix_inflection("でいる", "で", vec!["v1"], vec!["-te"]),
                    suffix_inflection("でおる", "で", vec!["v5"], vec!["-te"]),
                    suffix_inflection("でる", "で", vec!["v1p"], vec!["-te"]),
                    suffix_inflection("とる", "て", vec!["v5"], vec!["-te"]),
                    suffix_inflection("ないでいる", "ない", vec!["v1"], vec!["adj-i"]),
                ],
            },
        ),
        (
            "-ki",
            Transform {
                name: "-ki",
                description: Some(
                    "Attributive form (rentaikei) of i-adjectives. An archaic form that remains in modern Japanese.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～き",
                        description: Some("連体形"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("き", "い", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "-ge",
            Transform {
                name: "-ge",
                description: Some(
                    "Describes a person's appearance. Shows feelings of the person.\n\
                    Usage: Attach げ or 気 to the stem of i-adjectives".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～げ",
                        description: Some("…でありそうな様子。いかにも…らしいさま。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("げ", "い", vec![], vec!["adj-i"]),
                    suffix_inflection("気", "い", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "-garu",
            Transform {
                name: "-garu",
                description: Some(
                    "1. Shows subject’s feelings contrast with what is thought/known about them.\n\
                    2. Indicates subject's behavior (stands out).\n\
                    Usage: Attach がる to the stem of i-adjectives. It itself conjugates as a godan verb.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～がる",
                        description: Some("いかにもその状態にあるという印象を相手に与えるような言動をする。"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("がる", "い", vec!["v5"], vec!["adj-i"]),
                ],
            },
        ),
        (
            "-e",
            Transform {
                name: "-e",
                description: Some(
                    "Slang. A sound change of i-adjectives.\n\
                    ai：やばい → やべぇ\n\
                    ui：さむい → さみぃ/さめぇ\n\
                    oi：すごい → すげぇ".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja",
                        name: "～え",
                        description: Some("方言。例、「ない→ねえ」"),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ねえ", "ない", vec![], vec!["adj-i"]),
                    suffix_inflection("めえ", "むい", vec![], vec!["adj-i"]),
                    suffix_inflection("みい", "むい", vec![], vec!["adj-i"]),
                    suffix_inflection("ちぇえ", "つい", vec![], vec!["adj-i"]),
                    suffix_inflection("ちい", "つい", vec![], vec!["adj-i"]),
                    suffix_inflection("せえ", "すい", vec![], vec!["adj-i"]),
                    suffix_inflection("ええ", "いい", vec![], vec!["adj-i"]),
                    suffix_inflection("ええ", "わい", vec![], vec!["adj-i"]),
                    suffix_inflection("ええ", "よい", vec![], vec!["adj-i"]),
                    suffix_inflection("いぇえ", "よい", vec![], vec!["adj-i"]),
                    suffix_inflection("うぇえ", "わい", vec![], vec!["adj-i"]),
                    suffix_inflection("けえ", "かい", vec![], vec!["adj-i"]),
                    suffix_inflection("げえ", "がい", vec![], vec!["adj-i"]),
                    suffix_inflection("げえ", "ごい", vec![], vec!["adj-i"]),
                    suffix_inflection("せえ", "さい", vec![], vec!["adj-i"]),
                    suffix_inflection("めえ", "まい", vec![], vec!["adj-i"]),
                    suffix_inflection("ぜえ", "ずい", vec![], vec!["adj-i"]),
                    suffix_inflection("っぜえ", "ずい", vec![], vec!["adj-i"]),
                    suffix_inflection("れえ", "らい", vec![], vec!["adj-i"]),
                    suffix_inflection("ちぇえ", "ちゃい", vec![], vec!["adj-i"]),
                    suffix_inflection("でえ", "どい", vec![], vec!["adj-i"]),
                    suffix_inflection("れえ", "れい", vec![], vec!["adj-i"]),
                    suffix_inflection("べえ", "ばい", vec![], vec!["adj-i"]),
                    suffix_inflection("てえ", "たい", vec![], vec!["adj-i"]),
                    suffix_inflection("ねぇ", "ない", vec![], vec!["adj-i"]),
                    suffix_inflection("めぇ", "むい", vec![], vec!["adj-i"]),
                    suffix_inflection("みぃ", "むい", vec![], vec!["adj-i"]),
                    suffix_inflection("ちぃ", "つい", vec![], vec!["adj-i"]),
                    suffix_inflection("せぇ", "すい", vec![], vec!["adj-i"]),
                    suffix_inflection("けぇ", "かい", vec![], vec!["adj-i"]),
                    suffix_inflection("げぇ", "がい", vec![], vec!["adj-i"]),
                    suffix_inflection("げぇ", "ごい", vec![], vec!["adj-i"]),
                    suffix_inflection("せぇ", "さい", vec![], vec!["adj-i"]),
                    suffix_inflection("めぇ", "まい", vec![], vec!["adj-i"]),
                    suffix_inflection("ぜぇ", "ずい", vec![], vec!["adj-i"]),
                    suffix_inflection("っぜぇ", "ずい", vec![], vec!["adj-i"]),
                    suffix_inflection("れぇ", "らい", vec![], vec!["adj-i"]),
                    suffix_inflection("でぇ", "どい", vec![], vec!["adj-i"]),
                    suffix_inflection("れぇ", "れい", vec![], vec!["adj-i"]),
                    suffix_inflection("べぇ", "ばい", vec![], vec!["adj-i"]),
                    suffix_inflection("てぇ", "たい", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "slang",
            Transform {
                name: "slang",
                description: None,
                i18n: None,
                rules: vec![
                    suffix_inflection("てぇてぇ", "とうとい", vec![], vec!["adj-i"]),
                    suffix_inflection("てぇてぇ", "尊い", vec![], vec!["adj-i"]),
                    suffix_inflection("おなしゃす", "おねがいします", vec![], vec!["v5"]),
                    suffix_inflection("おなしゃす", "お願いします", vec![], vec!["v5"]),
                    suffix_inflection("あざす", "ありがとうございます", vec![], vec!["v5"]),
                    suffix_inflection("さーせん", "すみません", vec![], vec!["v5"]),
                    suffix_inflection("神ってる", "神がかっている", vec![], vec!["v1p"]),
                    suffix_inflection("じわる", "じわじわ来る", vec![], vec!["vk"]),
                    suffix_inflection("おさしみ", "おやすみ", vec![], vec![]),
                    suffix_inflection("おやさい", "おやすみ", vec![], vec![]),
                ],
            },
        ),
        (
            "n-slang",
            Transform {
                name: "n-slang",
                description: Some("Slang sound change of r-column syllables to n (when before an n-sound, usually の or な)".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("んな", "るな", vec![], vec!["-na"]),
                    suffix_inflection("んなさい", "りなさい", vec![], vec!["-nasai"]),
                    suffix_inflection("らんない", "られない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("んない", "らない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("んなきゃ", "らなきゃ", vec![], vec!["-ya"]),
                    suffix_inflection("んなきゃ", "れなきゃ", vec![], vec!["-ya"]),
                ],
            },
        ),
        (
            "kansai-ben negative",
            Transform {
                name: "kansai-ben",
                description: Some("Negative form of kansai-ben verbs".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("へん", "ない", vec![], vec!["adj-i"]),
                    suffix_inflection("ひん", "ない", vec![], vec!["adj-i"]),
                    suffix_inflection("せえへん", "しない", vec![], vec!["adj-i"]),
                    suffix_inflection("へんかった", "なかった", vec!["past"], vec!["past"]),
                    suffix_inflection("ひんかった", "なかった", vec!["past"], vec!["past"]),
                    suffix_inflection("うてへん", "ってない", vec![], vec!["adj-i"]),
                ],
            },
        ),
        (
            "kansai-ben -te",
            Transform {
                name: "kansai-ben",
                description: Some("-te form of kansai-ben verbs".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うて", "って", vec!["-te"], vec!["-te"]),
                    suffix_inflection("おうて", "あって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("こうて", "かって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ごうて", "がって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("そうて", "さって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ぞうて", "ざって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("とうて", "たって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("どうて", "だって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("のうて", "なって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ほうて", "はって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ぼうて", "ばって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("もうて", "まって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ろうて", "らって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ようて", "やって", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ゆうて", "いって", vec!["-te"], vec!["-te"]),
                ],
            },
        ),
        (
            "kansai-ben past",
            Transform {
                name: "kansai-ben",
                description: Some("Past form of kansai-ben terms".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うた", "った", vec!["past"], vec!["past"]),
                    suffix_inflection("おうた", "あった", vec!["past"], vec!["past"]),
                    suffix_inflection("こうた", "かった", vec!["past"], vec!["past"]),
                    suffix_inflection("ごうた", "がった", vec!["past"], vec!["past"]),
                    suffix_inflection("そうた", "さった", vec!["past"], vec!["past"]),
                    suffix_inflection("ぞうた", "ざった", vec!["past"], vec!["past"]),
                    suffix_inflection("とうた", "たった", vec!["past"], vec!["past"]),
                    suffix_inflection("どうた", "だった", vec!["past"], vec!["past"]),
                    suffix_inflection("のうた", "なった", vec!["past"], vec!["past"]),
                    suffix_inflection("ほうた", "はった", vec!["past"], vec!["past"]),
                    suffix_inflection("ぼうた", "ばった", vec!["past"], vec!["past"]),
                    suffix_inflection("もうた", "まった", vec!["past"], vec!["past"]),
                    suffix_inflection("ろうた", "らった", vec!["past"], vec!["past"]),
                    suffix_inflection("ようた", "やった", vec!["past"], vec!["past"]),
                    suffix_inflection("ゆうた", "いった", vec!["past"], vec!["past"]),
                ],
            },
        ),
        (
            "kansai-ben -tara",
            Transform {
                name: "kansai-ben",
                description: Some("-tara form of kansai-ben terms".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うたら", "ったら", vec![], vec![]),
                    suffix_inflection("おうたら", "あったら", vec![], vec![]),
                    suffix_inflection("こうたら", "かったら", vec![], vec![]),
                    suffix_inflection("ごうたら", "がったら", vec![], vec![]),
                    suffix_inflection("そうたら", "さったら", vec![], vec![]),
                    suffix_inflection("ぞうたら", "ざったら", vec![], vec![]),
                    suffix_inflection("とうたら", "たったら", vec![], vec![]),
                    suffix_inflection("どうたら", "だったら", vec![], vec![]),
                    suffix_inflection("のうたら", "なったら", vec![], vec![]),
                    suffix_inflection("ほうたら", "はったら", vec![], vec![]),
                    suffix_inflection("ぼうたら", "ばったら", vec![], vec![]),
                    suffix_inflection("もうたら", "まったら", vec![], vec![]),
                    suffix_inflection("ろうたら", "らったら", vec![], vec![]),
                    suffix_inflection("ようたら", "やったら", vec![], vec![]),
                    suffix_inflection("ゆうたら", "いったら", vec![], vec![]),
                ],
            },
        ),
        (
            "kansai-ben -ku",
            Transform {
                name: "kansai-ben",
                description: Some("-ku stem of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("う", "く", vec![], vec!["adv"]),
                    suffix_inflection("こう", "かく", vec![], vec!["adv"]),
                    suffix_inflection("ごう", "がく", vec![], vec!["adv"]),
                    suffix_inflection("そう", "さく", vec![], vec!["adv"]),
                    suffix_inflection("とう", "たく", vec![], vec!["adv"]),
                    suffix_inflection("のう", "なく", vec![], vec!["adv"]),
                    suffix_inflection("ぼう", "ばく", vec![], vec!["adv"]),
                    suffix_inflection("もう", "まく", vec![], vec!["adv"]),
                    suffix_inflection("ろう", "らく", vec![], vec!["adv"]),
                    suffix_inflection("よう", "よく", vec![], vec!["adv"]),
                    suffix_inflection("しゅう", "しく", vec![], vec!["adv"]),
                ],
            },
        ),
        (
            "kansai-ben adjective -te",
            Transform {
                name: "kansai-ben",
                description: Some("-te form of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うて", "くて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("こうて", "かくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ごうて", "がくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("そうて", "さくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("とうて", "たくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("のうて", "なくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ぼうて", "ばくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("もうて", "まくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ろうて", "らくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("ようて", "よくて", vec!["-te"], vec!["-te"]),
                    suffix_inflection("しゅうて", "しくて", vec!["-te"], vec!["-te"]),
                ],
            },
        ),
        (
            "kansai-ben adjective negative",
            Transform {
                name: "kansai-ben",
                description: Some("Negative form of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うない", "くない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("こうない", "かくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("ごうない", "がくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("そうない", "さくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("とうない", "たくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("のうない", "なくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("ぼうない", "ばくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("もうない", "まくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("ろうない", "らくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("ようない", "よくない", vec!["adj-i"], vec!["adj-i"]),
                    suffix_inflection("しゅうない", "しくない", vec!["adj-i"], vec!["adj-i"]),
                ],
            },
        ),
    ])
});

pub static JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: String::from("ja"),
        conditions: CONDITIONS.clone(),
        transforms: TRANSFORMS.clone(),
    });
