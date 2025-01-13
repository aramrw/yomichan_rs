use serde_json::from_reader;

use crate::language::transformer_d::{
    Condition, ConditionMap, LanguageTransformDescriptor, RuleI18n, Transform, TransformI18n,
    TransformMap,
};
use crate::language::transforms::suffix_inflection;

use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, LazyLock};

/// TODO: instead of hand writing the transforms, deserialize them from js
/// and then put the deserialized version in the lazylock;
///
/// serializes both javascript & rust conditions to see if they are
/// an exact match
#[test]
fn compare_json() {
    let file =
        std::fs::File::open("C:/Users/arami/Desktop/pretty_REAL_JP_TRANSFORMS.json").unwrap();
    let reader = std::io::BufReader::new(file);
    let transforms: LanguageTransformDescriptor = from_reader(reader).unwrap();
}

pub static TEST_DESC: LazyLock<LanguageTransformDescriptor> = LazyLock::new(|| {
    let file =
        std::fs::File::open("C:/Users/arami/Desktop/pretty_REAL_JP_TRANSFORMS.json").unwrap();
    let reader = std::io::BufReader::new(file);
    let d: LanguageTransformDescriptor = from_reader(reader).unwrap();
    d
});

const SHIMAU_ENGLISH_DESCRIPTION: &str = "1. Shows a sense of regret/surprise when you did have volition in doing something, but it turned out to be bad to do.\n2. Shows perfective/punctual achievement. This shows that an action has been completed.\n 3. Shows unintentional action–“accidentally”.\n";

const PASSIVE_ENGLISH_DESCRIPTION: &str = "1. Indicates an action received from an action performer.\n2. Expresses respect for the subject of action performer.\n";

/// a smaller version of japanese transformmap for testing.
pub static TEST_TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    TransformMap(HashMap::from([(
            "-ば".into(),
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
                    suffix_inflection("ければ", "い", vec!["-ba".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("えば", "う", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("けば", "く", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("げば", "ぐ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("せば", "す", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("てば", "つ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ねば", "ぬ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("べば", "ぶ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("めば", "む", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("れば", "る", vec!["-ba".to_string()], vec!["v1".to_string(), "v5".to_string(), "vk".to_string(), "vs".to_string(), "vz".to_string()]),
                    suffix_inflection("れば", "", vec!["-ば".to_string()], vec!["-ます".to_string()]),
                ],
            },
        )]))
});

/// a smaller version of japanese transforms for testing.
pub static TEST_JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: "ja".to_string(),
        conditions: CONDITIONS.clone(),
        transforms: TEST_TRANSFORMS.clone(),
    });

pub static CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
    ConditionMap(HashMap::from([
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
            "v1p".to_string(),
            Condition {
                name: "Ichidan verb, progressive or perfect form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "一段動詞、進行形または完了形".to_string(),
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
                sub_conditions: Some(vec!["v5d".to_string(), "v5m".to_string()]),
            },
        ),
        (
            "v5d".to_string(),
            Condition {
                name: "Godan verb, dictionary form".to_string(),
                is_dictionary_form: false,
                i18n: Some(vec![RuleI18n {
                    language: "ja".to_string(),
                    name: "五段動詞、辞書形".to_string(),
                }]),
                sub_conditions: None,
            },
        ),
        (
            "v5m".to_string(),
            Condition {
                name: "Godan verb, polite (masu) form".to_string(),
                is_dictionary_form: false,
                i18n: None,
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
            "-te".to_string(),
            Condition {
                name: "Intermediate -te endings for progressive or perfect tense".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "-ba".to_string(),
            Condition {
                name: "Intermediate -ba endings for conditional contraction".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "adv".to_string(),
            Condition {
                name: "Intermediate -ku endings for adverbs".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
        (
            "past".to_string(),
            Condition {
                name: "-ta past form ending".to_string(),
                is_dictionary_form: false,
                i18n: None,
                sub_conditions: None,
            },
        ),
    ]))
});

pub static TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    TransformMap(HashMap::from([
        (
            "-ba".to_string(),
            Transform {
                name: "-ba".to_string(),
                description: Some(
                    "(1) Conditional form; shows that the previous stated condition's establishment is the condition for the latter stated condition to occur. (2) Shows a trigger for a latter stated perception or judgment. Usage: Attach ば to the hypothetical/realis form (kateikei/izenkei) of verbs and i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ば".to_string(),
                    description: Some("仮定形".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ければ", "い", vec!["-ba".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("えば", "う", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("けば", "く", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("げば", "ぐ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("せば", "す", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("てば", "つ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ねば", "ぬ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("べば", "ぶ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("めば", "む", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("れば", "る", vec!["-ba".to_string()], vec!["v1".to_string(), "v5".to_string(), "vk".to_string(), "vs".to_string(), "vz".to_string()]),
                    suffix_inflection("れば", "", vec!["-ば".to_string()], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-ya".to_string(),
            Transform {
                name: "-ya".to_string(),
                description: Some("Contraction of -ba.".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ゃ".to_string(),
                    description: Some("仮定形の縮約系".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("けりゃ", "ければ", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("きゃ", "ければ", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("や", "えば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("きゃ", "けば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("ぎゃ", "げば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("しゃ", "せば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("ちゃ", "てば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("にゃ", "ねば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("びゃ", "べば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("みゃ", "めば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                    suffix_inflection("りゃ", "れば", vec!["-ya".to_string()], vec!["-ba".to_string()]),
                ],
            },
        ),
        (            "-cha".to_string(),
            Transform {
                name: "-cha".to_string(),
                description: Some(
                    "Contraction of ～ては.\n1. Explains how something always happens under the condition that it marks.\n\
                    2. Expresses the repetition (of a series of) actions.\n3. Indicates a hypothetical situation in \
                    which the speaker gives a (negative) evaluation about the other party's intentions.\n\
                    4. Used in `Must Not` patterns like ～てはいけない.\nUsage: Attach は after the te-form of verbs, \
                    contract ては into ちゃ.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちゃ".to_string(),
                    description: Some("「～ては」の縮約系".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ちゃ", "る", vec!["v5".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いじゃ", "ぐ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いちゃ", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しちゃ", "す", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃ", "う", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃ", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃ", "つ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃ", "る", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃ", "ぬ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃ", "ぶ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃ", "む", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じちゃ", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しちゃ", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ちゃ", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きちゃ", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ちゃ", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ちゃ", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-chau".to_string(),
            Transform {
                name: "-chau".to_string(),
                description: Some(format!("Contraction of -shimau.\n{SHIMAU_ENGLISH_DESCRIPTION}Usage: Attach しまう after the te-form of verbs, contract てしまう into ちゃう.")),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちゃう".to_string(),
                    description: Some("「～てしまう」のややくだけた口頭語的表現".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ちゃう",   "る", vec!["v5".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いじゃう", "ぐ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いちゃう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しちゃう", "す", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "う", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "つ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "る", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃう", "ぬ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃう", "ぶ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃう", "む", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じちゃう", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しちゃう", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ちゃう", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きちゃう", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ちゃう", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ちゃう", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-chimau".to_string(),
            Transform {
                name: "-chimau".to_string(),
                description: Some(format!(
                    "Contraction of -shimau.\n{SHIMAU_ENGLISH_DESCRIPTION}Usage: 
                    Attach しまう after the te-form of verbs, contract てしまう into ちまう."
                )),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちまう".to_string(),
                    description: Some("「～てしまう」の音変化".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ちまう", "る", vec!["v5".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いじまう", "ぐ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いちまう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しちまう", "す", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちまう", "う", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちまう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちまう", "つ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちまう", "る", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじまう", "ぬ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじまう", "ぶ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじまう", "む", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じちまう", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しちまう", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ちまう", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きちまう", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ちまう", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ちまう", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-shimau".to_string(),
            Transform {
                name: "-shimau".to_string(),
                description: Some(format!(
                    "{SHIMAU_ENGLISH_DESCRIPTION}Usage: Attach しまう after the te-form of verbs."
                )),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～しまう".to_string(),
                    description: Some("その動作がすっかり終わる、その状態が完成することを表す。
                    終わったことを強調したり、不本意である、困ったことになった、などの気持ちを添えたりすることもある。".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("てしまう", "て", vec!["v5".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("でしまう", "で", vec!["v5".to_string()], vec!["-te".to_string()]),
                ],
            }
        ),
        (            "-nasai".to_string(),
            Transform {
                name: "-nasai".to_string(),
                description: Some(
                    "Polite imperative suffix.\nUsage: Attach なさい after the continuative form (renyoukei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～なさい".to_string(),
                    description: Some("動詞「なさる」の命令形".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("なさい", "る", vec!["-nasai".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いなさい", "う", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("きなさい", "く", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぎなさい", "ぐ", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しなさい", "す", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ちなさい", "つ", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("になさい", "ぬ", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("びなさい", "ぶ", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("みなさい", "む", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("りなさい", "る", vec!["-nasai".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じなさい", "ずる", vec!["-nasai".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しなさい", "する", vec!["-nasai".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為なさい", "為る", vec!["-nasai".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きなさい", "くる", vec!["-nasai".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来なさい", "来る", vec!["-nasai".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來なさい", "來る", vec!["-nasai".to_string()], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-sou".to_string(),
            Transform {
                name: "-sou".to_string(),
                description: Some(
                    "Appearing that; looking like.\n
                     Usage: Attach そう to the continuative form (renyoukei) of verbs, or to the stem of adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～そう".to_string(),
                    description: Some("そういう様子だ、そうなる様子だということ、すなわち様態を表す助動詞。".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("そう", "い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("そう", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("いそう", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("きそう", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぎそう", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("しそう", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ちそう", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("にそう", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("びそう", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("みそう", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("りそう", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じそう", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("しそう", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為そう", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("きそう", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来そう", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來そう", "來る", vec![], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-sugiru".to_string(),
            Transform {
                name: "-sugiru".to_string(),
                description: Some(
                    "Shows something \"is too...\" or someone is doing something \"too much\".\n
                    Usage: Attach すぎる to the continuative form (renyoukei) of verbs, or to the stem of adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～すぎる".to_string(),
                    description: Some("程度や限度を超える".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("すぎる", "い", vec!["v1".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("すぎる", "る", vec!["v1".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いすぎる", "う", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("きすぎる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぎすぎる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しすぎる", "す", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ちすぎる", "つ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("にすぎる", "ぬ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("びすぎる", "ぶ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("みすぎる", "む", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("りすぎる", "る", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じすぎる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しすぎる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為すぎる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きすぎる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来すぎる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來すぎる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-tai".to_string(),
            Transform {
                name: "-tai".to_string(),
                description: Some(
                    "1. Expresses the feeling of desire or hope.\n
                    2. Used in ...たいと思います, an indirect way of saying what the speaker intends to do.\n
                    Usage: Attach たい to the continuative form (renyoukei) of verbs. たい itself conjugates as i-adjective.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たい".to_string(),
                    description: Some("することをのぞんでいる、という、希望や願望の気持ちをあらわす。".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("たい", "る", vec!["adj-i".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いたい", "う", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("きたい", "く", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぎたい", "ぐ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("したい", "す", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ちたい", "つ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("にたい", "ぬ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("びたい", "ぶ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("みたい", "む", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("りたい", "る", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じたい", "ずる", vec!["adj-i".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("したい", "する", vec!["adj-i".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為たい", "為る", vec!["adj-i".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きたい", "くる", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来たい", "来る", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來たい", "來る", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-tara".to_string(),
            Transform {
                name: "-tara".to_string(),
                description: Some(
                    "1. Denotes the latter stated event is a continuation of the previous stated event.\n
                        2. Assumes that a matter has been completed or concluded.\n
                        Usage: Attach たら to the continuative form (renyoukei) 
                        of verbs after euphonic change form, かったら to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たら".to_string(),
                    description: Some("仮定をあらわす・…すると・したあとに".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("かったら", "い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("たら", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("いたら", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("いだら", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("したら", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったら", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったら", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったら", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだら", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだら", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだら", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じたら", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("したら", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為たら", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("きたら", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来たら", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來たら", "來る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("いったら", "いく", vec![], vec!["v5".to_string()]),
                    suffix_inflection("おうたら", "おう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("こうたら", "こう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("そうたら", "そう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("とうたら", "とう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("行ったら", "行く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("逝ったら", "逝く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("往ったら", "往く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("請うたら", "請う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("乞うたら", "乞う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("恋うたら", "恋う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("問うたら", "問う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("負うたら", "負う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("沿うたら", "沿う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("添うたら", "添う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("副うたら", "副う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("厭うたら", "厭う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("のたもうたら", "のたまう", vec![], vec!["v5".to_string()]),
                ],
            }
        ),
        (            "-tari".to_string(),
            Transform {
                name: "-tari".to_string(),
                description: Some(
                    "1. Shows two actions occurring back and forth (when used with two verbs).\n
                    2. Shows examples of actions and states (when used with multiple verbs and adjectives).\n
                    Usage: Attach たり to the continuative form (renyoukei) 
                    of verbs after euphonic change form, かったり to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たり".to_string(),
                    description: Some("ある動作を例示的にあげることを表わす。".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("かったり", "い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("たり", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("いたり", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("いだり", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("したり", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったり", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったり", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ったり", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだり", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだり", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("んだり", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じたり", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("したり", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為たり", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("きたり", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来たり", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來たり", "來る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("いったり", "いく", vec![], vec!["v5".to_string()]),
                    suffix_inflection("おうたり", "おう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("こうたり", "こう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("そうたり", "そう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("とうたり", "とう", vec![], vec!["v5".to_string()]),
                    suffix_inflection("行ったり", "行く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("逝ったり", "逝く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("往ったり", "往く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("請うたり", "請う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("乞うたり", "乞う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("恋うたり", "恋う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("問うたり", "問う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("負うたり", "負う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("沿うたり", "沿う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("添うたり", "添う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("副うたり", "副う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("厭うたり", "厭う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("のたもうたり", "のたまう", vec![], vec!["v5".to_string()]),
                ],
            }
        ),
        (            "-te".to_string(),
            Transform {
                name: "-te".to_string(),
                description: Some(
                    "te-form.\nIt has a myriad of meanings. Primarily, it is a conjunctive particle that connects two clauses together.\n
                    Usage: Attach て to the continuative form (renyoukei) 
                    of verbs after euphonic change form, くて to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～て".to_string(),
                    description: Some("て（で）形".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("くて", "い", vec!["-te".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("て", "る", vec!["-te".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いて", "く", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いで", "ぐ", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("して", "す", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "つ", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "る", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "ぬ", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "ぶ", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "む", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じて", "ずる", vec!["-te".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("して", "する", vec!["-te".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為て", "為る", vec!["-te".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きて", "くる", vec!["-te".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来て", "来る", vec!["-te".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來て", "來る", vec!["-te".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("いって", "いく", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("おうて", "おう", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("こうて", "こう", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("そうて", "そう", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("とうて", "とう", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("行って", "行く", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("逝って", "逝く", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("往って", "往く", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("請うて", "請う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("乞うて", "乞う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("恋うて", "恋う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("問うて", "問う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("負うて", "負う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("沿うて", "沿う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("添うて", "添う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("副うて", "副う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("厭うて", "厭う", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("のたもうて", "のたまう", vec!["-te".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まして", "ます", vec![], vec!["v".to_string()]),
                ],
            }
        ),
        (            "-zu".to_string(),
            Transform {
                name: "-zu".to_string(),
                description: Some(
                    "1. Negative form of verbs.\n
                        2. Continuative form (renyoukei) of the particle ぬ (nu).\n
                        Usage: Attach ず to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ず".to_string(),
                    description: Some("口語の否定の助動詞「ぬ」の連用形".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ず", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かず", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がず", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さず", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たず", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なず", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばず", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まず", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らず", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わず", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜず", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せず", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為ず", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こず", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来ず", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來ず", "來る", vec![], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-nu".to_string(),
            Transform {
                name: "-nu".to_string(),
                description: Some(
                    "Negative form of verbs.\nUsage: Attach ぬ to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ぬ".to_string(),
                    description: Some("動作・状態などを「…ない」と否定することを表わす。".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ぬ", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かぬ", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がぬ", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さぬ", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たぬ", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なぬ", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばぬ", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まぬ", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らぬ", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わぬ", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜぬ", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せぬ", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為ぬ", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こぬ", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来ぬ", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來ぬ", "來る", vec![], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-n".to_string(),
            Transform {
                name: "-n".to_string(),
                description: Some(
                    "1. Negative form of verbs; a sound change of ぬ.\n
                        2. (As …んばかり) Shows an action or condition is on the verge of occurring, or an excessive/extreme degree.\n
                        Usage: Attach ん to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ん".to_string(),
                    description: Some("〔否定の助動詞〕…ない".to_string()),
                }]),
                rules: vec![
                    suffix_inflection("ん", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かん", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がん", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さん", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たん", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なん", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばん", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まん", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らん", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わん", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜん", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せん", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為ん", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こん", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来ん", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來ん", "來る", vec![], vec!["vk".to_string()]),
                ],
            }
        ),
        (            "-mu".to_string(),
            Transform {
                name: "-mu".to_string(),
                description: Some(
                    "Archaic.\n
                    Shows an inference of a certain matter.\n 
                    Shows speaker's intention.\n
                    Usage: Attach む to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～む".to_string(),
                        description: Some("…だろう".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("む", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かむ", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がむ", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さむ", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たむ", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なむ", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばむ", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まむ", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らむ", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わむ", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜむ", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せむ", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為む", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こむ", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来む", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來む", "來る", vec![], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-zaru".to_string(),
            Transform {
                name: "-zaru".to_string(),
                description: Some(
                    "Negative form of verbs.\n
                    Usage: Attach ざる to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～ざる".to_string(),
                        description: Some("…ない…".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ざる", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かざる", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がざる", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さざる", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たざる", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なざる", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばざる", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まざる", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らざる", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わざる", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜざる", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せざる", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為ざる", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こざる", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来ざる", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來ざる", "來る", vec![], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-neba".to_string(),
            Transform {
                name: "-neba".to_string(),
                description: Some(
                    "1. Shows a hypothetical negation; if not ...\n. 
                    Shows a must. Used with or without ならぬ.\n
                    Usage: Attach ねば to the irrealis form (mizenkei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～ねば".to_string(),
                        description: Some("もし…ないなら。…なければならない。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ねば", "る", vec!["-ba".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かねば", "く", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がねば", "ぐ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さねば", "す", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たねば", "つ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なねば", "ぬ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばねば", "ぶ", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まねば", "む", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らねば", "る", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わねば", "う", vec!["-ba".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぜねば", "ずる", vec!["-ba".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("せねば", "する", vec!["-ba".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ねば", "為る", vec!["-ba".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こねば", "くる", vec!["-ba".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ねば", "来る", vec!["-ba".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ねば", "來る", vec!["-ba".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-ku".to_string(),
            Transform {
                name: "-ku".to_string(),
                description: Some(
                    "Adverbial form of i-adjectives.\n".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "連用形".to_string(),
                        description: Some("〔形容詞で〕用言へ続く。例、「大きく育つ」の「大きく」。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("く", "い", vec!["adv".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "causative".to_string(),
            Transform {
                name: "causative".to_string(),
                description: Some(
                    "Describes the intention to make someone do something.\n
                    Usage: Attach させる to the irrealis form (mizenkei) of ichidan verbs and くる.\n
                    Attach せる to the irrealis form (mizenkei) of godan verbs and する.\n
                    It itself conjugates as an ichidan verb.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "使役形".to_string(),
                        description: Some("だれかにある行為をさせる意を表わす時の言い方。例、「行かせる」の「せる」。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("させる", "る", vec!["v1".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かせる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がせる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("させる", "す", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たせる", "つ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なせる", "ぬ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばせる", "ぶ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ませる", "む", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らせる", "る", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わせる", "う", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じさせる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("ぜさせる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("させる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為せる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("せさせる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為させる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こさせる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来させる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來させる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "imperative".to_string(),
            Transform {
                name: "imperative".to_string(),
                description: Some(
                    "1. To give orders.\n
                    2. (As あれ) Represents the fact that it will never change no matter the circumstances.\n
                    3. Express a feeling of hope.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "命令形".to_string(),
                        description: Some("命令の意味を表わすときの形。例、「行け」。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ろ", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("よ", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("え", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("け", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("げ", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("せ", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("て", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ね", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("べ", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("め", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("れ", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じろ", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("ぜよ", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("しろ", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("せよ", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為ろ", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為よ", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こい", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来い", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來い", "來る", vec![], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "imperative negative".to_string(),
            Transform {
                name: "imperative negative".to_string(),
                description: None,
                i18n: None,
                rules: vec![
                    suffix_inflection("な", "", vec!["-na".to_string()], vec!["v".to_string()]),
                ],
            },
        ),
        (            "continuative".to_string(),
            Transform {
                name: "continuative".to_string(),
                description: Some(
                    "Used to indicate actions that are (being) carried out.\n
                    Refers to the renyoukei, the part of the verb after conjugating with -masu and dropping masu.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "連用形".to_string(),
                        description: Some(
                            "〔動詞などで〕「ます」などに続く。例、「バスを降りて歩きます」の「降り」「歩き」。".to_string()
                        ),
                    },
                ]),
                rules: vec![
                    suffix_inflection("い", "いる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("え", "える", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("き", "きる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("ぎ", "ぎる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("け", "ける", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("げ", "げる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("じ", "じる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("せ", "せる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("ぜ", "ぜる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("ち", "ちる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("て", "てる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("で", "でる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("に", "にる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("ね", "ねる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("ひ", "ひる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("び", "びる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("へ", "へる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("べ", "べる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("み", "みる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("め", "める", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("り", "りる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("れ", "れる", vec![], vec!["v1d".to_string()]),
                    suffix_inflection("い", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("き", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぎ", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("し", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ち", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("に", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("び", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("み", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("り", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("き", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("し", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("来", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來", "來る", vec![], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "negative".to_string(),
            Transform {
                name: "negative".to_string(),
                description: Some(
                    "1. Negative form of verbs.\n
                    2. Expresses a feeling of solicitation to the other party.\n
                    Usage: Attach ない to the irrealis form (mizenkei) of verbs, くない to the stem of i-adjectives. ない itself conjugates as i-adjective. ます becomes ません.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～ない".to_string(),
                        description: Some(
                            "その動作・作用・状態の成立を否定することを表わす。".to_string()
                        ),
                    },
                ]),
                rules: vec![
                    suffix_inflection("くない", "い", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ない", "る", vec!["adj-i".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かない", "く", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がない", "ぐ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さない", "す", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たない", "つ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なない", "ぬ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばない", "ぶ", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まない", "む", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らない", "る", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わない", "う", vec!["adj-i".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じない", "ずる", vec!["adj-i".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しない", "する", vec!["adj-i".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ない", "為る", vec!["adj-i".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こない", "くる", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ない", "来る", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ない", "來る", vec!["adj-i".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("ません", "ます", vec!["v".to_string()], vec!["v".to_string()]),
                ],
            },
        ),
        (            "-sa".to_string(),
            Transform {
                name: "-sa".to_string(),
                description: Some(
                    "Nominalizing suffix of i-adjectives indicating nature, state, mind or degree.\n
                    Usage: Attach さ to the stem of i-adjectives.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～さ".to_string(),
                        description: Some("こと。程度。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("さ", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "passive".to_string(),
            Transform {
                name: "passive".to_string(),
                description: Some(format!("{PASSIVE_ENGLISH_DESCRIPTION} 
                Usage: Attach れる to the irrealis form (mizenkei) of godan verbs.")),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "受身形".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かれる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がれる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("される", "す", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たれる", "つ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なれる", "ぬ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばれる", "ぶ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まれる", "む", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("われる", "う", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("られる", "る", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じされる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("ぜされる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("される", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為れる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こられる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来られる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來られる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-ta".to_string(),
            Transform {
                name: "-ta".to_string(),
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
                        language: "ja".to_string(),
                        name: "～た・かった形".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かった", "い", vec!["past".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("た", "る", vec!["past".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いた", "く", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いだ", "ぐ", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("した", "す", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "つ", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "る", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "ぬ", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "ぶ", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "む", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じた", "ずる", vec!["past".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("した", "する", vec!["past".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為た", "為る", vec!["past".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きた", "くる", vec!["past".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来た", "来る", vec!["past".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來た", "來る", vec!["past".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("いった", "いく", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("おうた", "おう", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("こうた", "こう", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("そうた", "そう", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("とうた", "とう", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("行った", "行く", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("逝った", "逝く", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("往った", "往く", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("請うた", "請う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("乞うた", "乞う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("恋うた", "恋う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("問うた", "問う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("負うた", "負う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("沿うた", "沿う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("添うた", "添う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("副うた", "副う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("厭うた", "厭う", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("のたもうた", "のたまう", vec!["past".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ました", "ます", vec!["past".to_string()], vec!["v".to_string()]),
                    suffix_inflection("ませんでした", "ません", vec!["past".to_string()], vec!["v".to_string()]),
                ],
            },
        ),
        (            "-masu".to_string(),
            Transform {
                name: "-masu".to_string(),
                description: Some(
                    "Polite conjugation of verbs and adjectives.\n\
                    Usage: Attach ます to the continuative form (renyoukei) of verbs.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～ます".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("ます", "る", vec!["v1".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("います", "う", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("きます", "く", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ぎます", "ぐ", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("します", "す", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ちます", "つ", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("にます", "ぬ", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("びます", "ぶ", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("みます", "む", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ります", "る", vec!["v5m".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("じます", "ずる", vec!["vz".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("します", "する", vec!["vs".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ます", "為る", vec!["vs".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きます", "くる", vec!["vk".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ます", "来る", vec!["vk".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ます", "來る", vec!["vk".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("くあります", "い", vec!["v".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "potential".to_string(),
            Transform {
                name: "potential".to_string(),
                description: Some(
                    "Indicates a state of being (naturally) capable of doing an action.\n\
                    Usage: Attach (ら)れる to the irrealis form (mizenkei) of ichidan verbs.\n\
                    Attach る to the imperative form (meireikei) of godan verbs.\n\
                    する becomes できる, くる becomes こ(ら)れる".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "可能".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("れる", "る", vec!["v1".to_string()], vec!["v1".to_string(), "v5d".to_string()]),
                    suffix_inflection("える", "う", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ける", "く", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("げる", "ぐ", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("せる", "す", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("てる", "つ", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ねる", "ぬ", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("べる", "ぶ", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("める", "む", vec!["v1".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("できる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("出来る", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("これる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来れる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來れる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "potential or passive".to_string(),
            Transform {
                name: "potential or passive".to_string(),
                description: Some(
                    "Usage: Attach られる to the irrealis form (mizenkei) of ichidan verbs.\n\
                    する becomes せられる, くる becomes こられる\n\
                    3. Indicates a state of being (naturally) capable of doing an action.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "受身・自発・可能・尊敬".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("られる", "る", vec!["v1".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("ざれる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("ぜられる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("せられる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為られる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こられる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来られる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來られる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "volitional".to_string(),
            Transform {
                name: "volitional".to_string(),
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
                        language: "ja".to_string(),
                        name: "～う形".to_string(),
                        description: Some("主体の意志を表わす".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("よう", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("おう", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("こう", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ごう", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("そう", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("とう", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("のう", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぼう", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("もう", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ろう", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じよう", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("しよう", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為よう", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こよう", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来よう", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來よう", "來る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("ましょう", "ます", vec![], vec!["v".to_string()]),
                    suffix_inflection("かろう", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "causative-passive".to_string(),
            Transform {
                name: "causative-passive".to_string(),
                description: Some(
                    "Contraction of the passive of the causative form of verbs.\n\
                    Someone was made to do something by someone else.\n\
                    Usage: ～せられる becomes ~される (only for godan verbs)".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "使役受け身形".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("かされる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がされる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たされる", "つ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なされる", "ぬ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばされる", "ぶ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まされる", "む", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らされる", "る", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わされる", "う", vec!["v1".to_string()], vec!["v5".to_string()]),
                ],
            },
        ),
        (            "-toku".to_string(),
            Transform {
                name: "-toku".to_string(),
                description: Some(
                    "Contraction of -teoku.\n\
                    To do certain things in advance in preparation (or in anticipation) of latter needs.\n\
                    Usage: Attach おく to the te-form of verbs, then contract ておく into とく.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～とく".to_string(),
                        description: Some("「～テオク」の縮約系".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("とく", "る", vec!["v5".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いとく", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いどく", "ぐ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しとく", "す", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っとく", "う", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っとく", "つ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っとく", "る", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んどく", "ぬ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んどく", "ぶ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んどく", "む", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じとく", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しとく", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為とく", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きとく", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来とく", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來とく", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (            "-teiru".to_string(),
            Transform {
                name: "-teiru".to_string(),
                description: Some(
                    "1. Indicates an action continues or progresses to a point in time.\n\
                    2. Indicates an action is completed and remains as is.\n\
                    3. Indicates a state or condition that can be taken to be the result of undergoing some change.\n\
                    Usage: Attach いる to the te-form of verbs. い can be dropped in speech.\n\
                    (Slang) Attach おる to the te-form of verbs. Contracts to とる・でる in speech.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～ている".to_string(),
                        description: None,
                    },
                ]),
                rules: vec![
                    suffix_inflection("ている", "て", vec!["v1".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ておる", "て", vec!["v5".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("てる", "て", vec!["v1p".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("でいる", "で", vec!["v1".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("でおる", "で", vec!["v5".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("でる", "で", vec!["v1p".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("とる", "て", vec!["v5".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ないでいる", "ない", vec!["v1".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "-ki".to_string(),
            Transform {
                name: "-ki".to_string(),
                description: Some(
                    "Attributive form (rentaikei) of i-adjectives. An archaic form that remains in modern Japanese.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～き".to_string(),
                        description: Some("連体形".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("き", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "-ge".to_string(),
            Transform {
                name: "-ge".to_string(),
                description: Some(
                    "Describes a person's appearance. Shows feelings of the person.\n\
                    Usage: Attach げ or 気 to the stem of i-adjectives".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～げ".to_string(),
                        description: Some("…でありそうな様子。いかにも…らしいさま。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("げ", "い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("気", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "-garu".to_string(),
            Transform {
                name: "-garu".to_string(),
                description: Some(
                    "1. Shows subject’s feelings contrast with what is thought/known about them.\n\
                    2. Indicates subject's behavior (stands out).\n\
                    Usage: Attach がる to the stem of i-adjectives. It itself conjugates as a godan verb.".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～がる".to_string(),
                        description: Some("いかにもその状態にあるという印象を相手に与えるような言動をする。".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("がる", "い", vec!["v5".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "-e".to_string(),
            Transform {
                name: "-e".to_string(),
                description: Some(
                    "Slang. A sound change of i-adjectives.\n\
                    ai：やばい → やべぇ\n\
                    ui：さむい → さみぃ/さめぇ\n\
                    oi：すごい → すげぇ".into()
                ),
                i18n: Some(vec![
                    TransformI18n {
                        language: "ja".to_string(),
                        name: "～え".to_string(),
                        description: Some("方言。例、「ない→ねえ」".to_string()),
                    },
                ]),
                rules: vec![
                    suffix_inflection("ねえ", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("めえ", "むい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("みい", "むい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ちぇえ", "つい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ちい", "つい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せえ", "すい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ええ", "いい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ええ", "わい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ええ", "よい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("いぇえ", "よい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("うぇえ", "わい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("けえ", "かい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("げえ", "がい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("げえ", "ごい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せえ", "さい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("めえ", "まい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ぜえ", "ずい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("っぜえ", "ずい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("れえ", "らい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ちぇえ", "ちゃい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("でえ", "どい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("れえ", "れい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("べえ", "ばい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("てえ", "たい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ねぇ", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("めぇ", "むい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("みぃ", "むい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ちぃ", "つい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せぇ", "すい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("けぇ", "かい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("げぇ", "がい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("げぇ", "ごい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せぇ", "さい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("めぇ", "まい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ぜぇ", "ずい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("っぜぇ", "ずい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("れぇ", "らい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("でぇ", "どい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("れぇ", "れい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("べぇ", "ばい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("てぇ", "たい", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "slang".to_string(),
            Transform {
                name: "slang".to_string(),
                description: None,
                i18n: None,
                rules: vec![
                    suffix_inflection("てぇてぇ", "とうとい", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("てぇてぇ", "尊い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("おなしゃす", "おねがいします", vec![], vec!["v5".to_string()]),
                    suffix_inflection("おなしゃす", "お願いします", vec![], vec!["v5".to_string()]),
                    suffix_inflection("あざす", "ありがとうございます", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さーせん", "すみません", vec![], vec!["v5".to_string()]),
                    suffix_inflection("神ってる", "神がかっている", vec![], vec!["v1p".to_string()]),
                    suffix_inflection("じわる", "じわじわ来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("おさしみ", "おやすみ", vec![], vec![]),
                    suffix_inflection("おやさい", "おやすみ", vec![], vec![]),
                ],
            },
        ),
        (            "n-slang".to_string(),
            Transform {
                name: "n-slang".to_string(),
                description: Some("Slang sound change of r-column syllables to n (when before an n-sound, usually の or な)".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("んな", "るな", vec![], vec!["-na".to_string()]),
                    suffix_inflection("んなさい", "りなさい", vec![], vec!["-nasai".to_string()]),
                    suffix_inflection("らんない", "られない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("んない", "らない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("んなきゃ", "らなきゃ", vec![], vec!["-ya".to_string()]),
                    suffix_inflection("んなきゃ", "れなきゃ", vec![], vec!["-ya".to_string()]),
                ],
            },
        ),
        (            "kansai-ben negative".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("Negative form of kansai-ben verbs".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("へん", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ひん", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せえへん", "しない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("へんかった", "なかった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ひんかった", "なかった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("うてへん", "ってない", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (            "kansai-ben -te".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("-te form of kansai-ben verbs".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うて", "って", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("おうて", "あって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("こうて", "かって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ごうて", "がって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("そうて", "さって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ぞうて", "ざって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("とうて", "たって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("どうて", "だって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("のうて", "なって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ほうて", "はって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ぼうて", "ばって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("もうて", "まって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ろうて", "らって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ようて", "やって", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ゆうて", "いって", vec!["-te".to_string()], vec!["-te".to_string()]),
                ],
            },
        ),
        (            "kansai-ben past".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("Past form of kansai-ben terms".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うた", "った", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("おうた", "あった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("こうた", "かった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ごうた", "がった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("そうた", "さった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ぞうた", "ざった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("とうた", "たった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("どうた", "だった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("のうた", "なった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ほうた", "はった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ぼうた", "ばった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("もうた", "まった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ろうた", "らった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ようた", "やった", vec!["past".to_string()], vec!["past".to_string()]),
                    suffix_inflection("ゆうた", "いった", vec!["past".to_string()], vec!["past".to_string()]),
                ],
            },
        ),
        (            "kansai-ben -tara".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
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
        (            "kansai-ben -ku".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("-ku stem of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("う", "く", vec![], vec!["adv".to_string()]),
                    suffix_inflection("こう", "かく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("ごう", "がく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("そう", "さく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("とう", "たく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("のう", "なく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("ぼう", "ばく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("もう", "まく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("ろう", "らく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("よう", "よく", vec![], vec!["adv".to_string()]),
                    suffix_inflection("しゅう", "しく", vec![], vec!["adv".to_string()]),
                ],
            },
        ),
        (            "kansai-ben adjective -te".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("-te form of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うて", "くて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("こうて", "かくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ごうて", "がくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("そうて", "さくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("とうて", "たくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("のうて", "なくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ぼうて", "ばくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("もうて", "まくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ろうて", "らくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("ようて", "よくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                    suffix_inflection("しゅうて", "しくて", vec!["-te".to_string()], vec!["-te".to_string()]),
                ],
            },
        ),
        (            "kansai-ben adjective negative".to_string(),
            Transform {
                name: "kansai-ben".to_string(),
                description: Some("Negative form of kansai-ben adjectives".into()),
                i18n: None,
                rules: vec![
                    suffix_inflection("うない", "くない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("こうない", "かくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ごうない", "がくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("そうない", "さくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("とうない", "たくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("のうない", "なくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ぼうない", "ばくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("もうない", "まくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ろうない", "らくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ようない", "よくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("しゅうない", "しくない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
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
