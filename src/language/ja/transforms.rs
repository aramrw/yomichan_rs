/// https://raw.githubusercontent.com/yomidevs/yomitan/c3bec65bc44a33b1b1686e5d81a6910e42889174/ext/js/language/ja/japanese-transforms.js
use indexmap::IndexMap;
use serde_json::from_reader;

use crate::language::transformer_d::{
    Condition, ConditionMap, LanguageTransformDescriptor, LanguageTransformer, RuleI18n, Transform,
    TransformI18n, TransformMap,
};
use crate::language::transforms::suffix_inflection;

use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, LazyLock};

const SHIMAU_ENGLISH_DESCRIPTION: &str = "1. Shows a sense of regret/surprise when you did have volition in doing something, but it turned out to be bad to do.\n2. Shows perfective/punctual achievement. This shows that an action has been completed.\n 3. Shows unintentional action–“accidentally”.\n";
const PASSIVE_ENGLISH_DESCRIPTION: &str = "1. Indicates an action received from an action performer.\n2. Expresses respect for the subject of action performer.\n";

pub static JAPANESE_TRANSFORMS: LazyLock<LanguageTransformDescriptor> =
    LazyLock::new(|| LanguageTransformDescriptor {
        language: "ja".to_string(),
        conditions: &CONDITIONS,
        transforms: &TRANSFORMS,
    });

#[derive(Debug, thiserror::Error)]
enum TransformTestError<'a> {
    #[error("{term} should have term candidate {src} with rule {rule} with reasons {reasons:?}")]
    MissingTransformation {
        src: &'static str,
        term: &'static str,
        rule: &'static str,
        reasons: &'a [&'static str],
    },
}

pub(crate) struct TransformTest {
    pub(crate) term: &'static str,
    pub(crate) sources: Vec<LanguageTransformerTestCase>,
}

pub(crate) struct LanguageTransformerTestCase {
    inner: &'static str,
    rule: &'static str,
    reasons: Vec<&'static str>,
}

#[derive(Debug)]
pub(crate) struct HasTermReasons {
    pub(crate) reasons: Vec<String>,
    pub(crate) rules: usize,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum HasTermReasonsError {
    #[error("No transformation from '{src}' to '{term}' with rule '{rule}'.\nRejected candidates:\n{}", .rejected.join("\n"))]
    NoMatch {
        src: String,
        term: String,
        rule: String,
        rejected: Vec<String>,
    },
    #[error("Trace length mismatch: expected {expected}, found {found}")]
    TraceLengthMismatch { expected: usize, found: usize },
    #[error("Reason {index}: expected '{expected}', found '{found}'")]
    ReasonMismatch {
        index: usize,
        expected: String,
        found: String,
    },
}

pub(crate) fn has_term_reasons(
    lt: &LanguageTransformer,
    source: &str,
    expected_term: &str,
    expected_condition_name: Option<&str>,
    expected_reasons: Option<&[&str]>,
) -> Result<HasTermReasons, HasTermReasonsError> {
    let results = lt.transform(source).unwrap();
    let rule = expected_condition_name.unwrap_or("");
    let mut rejected = Vec::new();

    for result in results {
        let mut rejection_reasons = Vec::new();

        // Check term match
        if result.text != expected_term {
            rejection_reasons.push(format!(
                "Term mismatch: expected '{}', got '{}'",
                expected_term, result.text
            ));
        }

        // Check rule match if term matched
        if result.text == expected_term {
            if let Some(expected_name) = expected_condition_name {
                let expected_conditions =
                    lt.get_condition_flags_from_single_condition_type(expected_name);
                if !LanguageTransformer::conditions_match(
                    result.conditions,
                    expected_conditions as u32,
                ) {
                    rejection_reasons.push(format!(
                        "Condition mismatch: expected {}({:b}), got {:b}",
                        expected_name, expected_conditions, result.conditions
                    ));
                }
            }
        }

        // If we had any rejection reasons, log and continue
        if !rejection_reasons.is_empty() {
            rejected.push(format!(
                "Candidate '{}' [conditions {:b}] rejected because:\n  {}",
                result.text,
                result.conditions,
                rejection_reasons.join("\n  ")
            ));
            continue;
        }

        // Now check trace reasons if we got this far
        if let Some(expected) = expected_reasons {
            // Check trace length
            if result.trace.len() != expected.len() {
                return Err(HasTermReasonsError::TraceLengthMismatch {
                    expected: expected.len(),
                    found: result.trace.len(),
                });
            }

            // Check individual reasons
            for (i, (actual, expected)) in result.trace.iter().zip(expected.iter()).enumerate() {
                if &actual.transform != expected {
                    return Err(HasTermReasonsError::ReasonMismatch {
                        index: i,
                        expected: (*expected).to_string(),
                        found: actual.transform.clone(),
                    });
                }
            }
        }

        // Success case
        return Ok(HasTermReasons {
            reasons: result.trace.iter().map(|f| f.transform.clone()).collect(),
            rules: result.conditions as usize,
        });
    }

    // No matches found - return all rejection reasons
    Err(HasTermReasonsError::NoMatch {
        src: source.to_string(),
        term: expected_term.to_string(),
        rule: rule.to_string(),
        rejected,
    })
}

pub(crate) static TRANSFORM_TESTS: LazyLock<[&TransformTest; 1]> =
    LazyLock::new(|| [&*JP_ADJ_TESTS]);
pub(crate) static JP_ADJ_TESTS: LazyLock<TransformTest> = LazyLock::new(|| TransformTest {
    term: "愛しい",
    sources: vec![
        LanguageTransformerTestCase {
            inner: "愛しそう",
            rule: "adj-i",
            reasons: vec!["-そう"],
        },
        LanguageTransformerTestCase {
            inner: "愛しすぎる",
            rule: "adj-i",
            reasons: vec!["-すぎる"],
        },
        LanguageTransformerTestCase {
            inner: "愛し過ぎる",
            rule: "adj-i",
            reasons: vec!["-過ぎる"],
        },
        LanguageTransformerTestCase {
            inner: "愛しかったら",
            rule: "adj-i",
            reasons: vec!["-たら"],
        },
        LanguageTransformerTestCase {
            inner: "愛しかったり",
            rule: "adj-i",
            reasons: vec!["-たり"],
        },
        LanguageTransformerTestCase {
            inner: "愛しくて",
            rule: "adj-i",
            reasons: vec!["-て"],
        },
        LanguageTransformerTestCase {
            inner: "愛しく",
            rule: "adj-i",
            reasons: vec!["-く"],
        },
        LanguageTransformerTestCase {
            inner: "愛しくない",
            rule: "adj-i",
            reasons: vec!["negative"],
        },
        LanguageTransformerTestCase {
            inner: "愛しさ",
            rule: "adj-i",
            reasons: vec!["-さ"],
        },
        LanguageTransformerTestCase {
            inner: "愛しかった",
            rule: "adj-i",
            reasons: vec!["-た"],
        },
        LanguageTransformerTestCase {
            inner: "愛しくありません",
            rule: "adj-i",
            reasons: vec!["-ます", "negative"],
        },
        LanguageTransformerTestCase {
            inner: "愛しくありませんでした",
            rule: "adj-i",
            reasons: vec!["-ます", "negative", "-た"],
        },
        LanguageTransformerTestCase {
            inner: "愛しき",
            rule: "adj-i",
            reasons: vec!["-き"],
        },
        LanguageTransformerTestCase {
            inner: "愛しげ",
            rule: "adj-i",
            reasons: vec!["-げ"],
        },
        LanguageTransformerTestCase {
            inner: "愛し気",
            rule: "adj-i",
            reasons: vec!["-げ"],
        },
        LanguageTransformerTestCase {
            inner: "愛しがる",
            rule: "adj-i",
            reasons: vec!["-がる"],
        },
    ],
});

pub static TRANSFORMS: LazyLock<TransformMap> = LazyLock::new(|| {
    let t = TransformMap(IndexMap::from([
        (
            "-ば".to_string(),
            Transform {
                name: "-ば".to_string(),
                description: Some(
                    "1. Conditional form; shows that the previous stated condition's establishment is the condition for the latter stated condition to occur.\n2. Shows a trigger for a latter stated perception or judgment.\nUsage: Attach ば to the hypothetical form (仮定形) of verbs and i-adjectives.".into(),
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
        (
            "-ゃ".to_string(),
            Transform {
                name: "-ゃ".to_string(),
                description: Some("Contraction of -ば.".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ゃ".to_string(),
                    description: Some("「～ば」の短縮".into()),
                }]),
                rules: vec![
                    suffix_inflection("けりゃ", "ければ", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("きゃ", "ければ", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("や", "えば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("きゃ", "けば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("ぎゃ", "げば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("しゃ", "せば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("ちゃ", "てば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("にゃ", "ねば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("びゃ", "べば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("みゃ", "めば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                    suffix_inflection("りゃ", "れば", vec!["-ゃ".to_string()], vec!["-ば".to_string()]),
                ],
            },
        ),
        (
            "-ちゃ".to_string(),
            Transform {
                name: "-ちゃ".to_string(),
                description: Some(
                    "Contraction of ～ては.\n1. Explains how something always happens under the condition that it marks.\n2. Expresses the repetition (of a series of) actions.\n3. Indicates a hypothetical situation in which the speaker gives a (negative) evaluation about the other party's intentions.\n4. Used in \"Must Not\" patterns like ～てはいけない.\nUsage: Attach は after the て-form of verbs, contract ては into ちゃ.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちゃ".to_string(),
                    description: Some("「～ては」の短縮".into()),
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
                    suffix_inflection("じちゃ", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しちゃ", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ちゃ", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きちゃ", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ちゃ", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ちゃ", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-ちゃう".to_string(),
            Transform {
                name: "-ちゃう".to_string(),
                description: Some(
                    "Contraction of -しまう.\nShows completion of an action with regret or accidental completion.\nUsage: Attach しまう after the て-form of verbs, contract てしまう into ちゃう.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちゃう".to_string(),
                    description: Some("「～てしまう」のややくだけた口頭語的表現".into()),
                }]),
                rules: vec![
                    suffix_inflection("ちゃう", "る", vec!["v5".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いじゃう", "ぐ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いちゃう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しちゃう", "す", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "う", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "く", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "つ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("っちゃう", "る", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃう", "ぬ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んじゃう", "ぶ", vec!["v5".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じちゃう", "ずる", vec!["v5".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しちゃう", "する", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ちゃう", "為る", vec!["v5".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きちゃう", "くる", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ちゃう", "来る", vec!["v5".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ちゃう", "來る", vec!["v5".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-ちまう".to_string(),
            Transform {
                name: "-ちまう".to_string(),
                description: Some(
                    "Contraction of -しまう.\nShows completion of an action with regret or accidental completion.\nUsage: Attach しまう after the て-form of verbs, contract てしまう into ちまう.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ちまう".to_string(),
                    description: Some("「～てしまう」の音変化".into()),
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
            },
        ),
        (
            "-しまう".to_string(),
            Transform {
                name: "-しまう".to_string(),
                description: Some(
                    "Shows completion of an action with regret or accidental completion.\nUsage: Attach しまう after the て-form of verbs.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～しまう".to_string(),
                    description: Some(
                        "その動作がすっかり終わる、その状態が完成することを表す。終わったことを強調したり、不本意である、困ったことになった、などの気持ちを添えたりすることもある。".into(),
                    ),
                }]),
                rules: vec![
                    suffix_inflection("てしまう", "て", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("でしまう", "で", vec!["v5".to_string()], vec!["-て".to_string()]),
                ],
            },
        ),
        (
            "-なさい".to_string(),
            Transform {
                name: "-なさい".to_string(),
                description: Some(
                    "Polite imperative suffix.\nUsage: Attach なさい after the continuative form (連用形) of verbs.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～なさい".to_string(),
                    description: Some("動詞「なさる」の命令形".into()),
                }]),
                rules: vec![
                    suffix_inflection("なさい", "る", vec!["-なさい".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いなさい", "う", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("きなさい", "く", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぎなさい", "ぐ", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("しなさい", "す", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ちなさい", "つ", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("になさい", "ぬ", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("びなさい", "ぶ", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("みなさい", "む", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("りなさい", "る", vec!["-なさい".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じなさい", "ずる", vec!["-なさい".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("しなさい", "する", vec!["-なさい".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為なさい", "為る", vec!["-なさい".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きなさい", "くる", vec!["-なさい".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来なさい", "来る", vec!["-なさい".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來なさい", "來る", vec!["-なさい".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-そう".to_string(),
            Transform {
                name: "-そう".to_string(),
                description: Some(
                    "Appearing that; looking like.\nUsage: Attach そう to the continuative form (連用形) of verbs, or to the stem of adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～そう".to_string(),
                    description: Some("そういう様子だ、そうなる様子だということ、すなわち様態を表す助動詞。".into()),
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
            },
        ),
        (
            "-すぎる".to_string(),
            Transform {
                name: "-すぎる".to_string(),
                description: Some(
                    "Shows something \"is too...\" or someone is doing something \"too much\".\nUsage: Attach すぎる to the continuative form (連用形) of verbs, or to the stem of adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～すぎる".to_string(),
                    description: Some("程度や限度を超える".into()),
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
            },
        ),
        (
            "-過ぎる".to_string(),
            Transform {
                name: "-過ぎる".to_string(),
                description: Some(
                    "Shows something \"is too...\" or someone is doing something \"too much\".\nUsage: Attach 過ぎる to the continuative form (連用形) of verbs, or to the stem of adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～過ぎる".to_string(),
                    description: Some("程度や限度を超える".into()),
                }]),
                rules: vec![
                    suffix_inflection("過ぎる", "い", vec!["v1".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("過ぎる", "る", vec!["v1".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("い過ぎる", "う", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("き過ぎる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぎ過ぎる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("し過ぎる", "す", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ち過ぎる", "つ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("に過ぎる", "ぬ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("び過ぎる", "ぶ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("み過ぎる", "む", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("り過ぎる", "る", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じ過ぎる", "ずる", vec!["v1".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("し過ぎる", "する", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為過ぎる", "為る", vec!["v1".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("き過ぎる", "くる", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来過ぎる", "来る", vec!["v1".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來過ぎる", "來る", vec!["v1".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-たい".to_string(),
            Transform {
                name: "-たい".to_string(),
                description: Some(
                    "1. Expresses the feeling of desire or hope.\n2. Used in ...たいと思います, an indirect way of saying what the speaker intends to do.\nUsage: Attach たい to the continuative form (連用形) of verbs. たい itself conjugates as i-adjective.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たい".to_string(),
                    description: Some("することをのぞんでいる、という、希望や願望の気持ちをあらわす。".into()),
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
            },
        ),
        (
            "-たら".to_string(),
            Transform {
                name: "-たら".to_string(),
                description: Some(
                    "1. Denotes the latter stated event is a continuation of the previous stated event.\n2. Assumes that a matter has been completed or concluded.\nUsage: Attach たら to the continuative form (連用形) of verbs after euphonic change form, かったら to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たら".to_string(),
                    description: Some("仮定をあらわす・…すると・したあとに".into()),
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
                    suffix_inflection("ましたら", "ます", vec![], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-たり".to_string(),
            Transform {
                name: "-たり".to_string(),
                description: Some(
                    "1. Shows two actions occurring back and forth (when used with two verbs).\n2. Shows examples of actions and states (when used with multiple verbs and adjectives).\nUsage: Attach たり to the continuative form (連用形) of verbs after euphonic change form, かったり to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～たり".to_string(),
                    description: Some("ある動作を例示的にあげることを表わす。".into()),
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
                ],
            },
        ),
        (
            "-て".to_string(),
            Transform {
                name: "-て".to_string(),
                description: Some(
                    "て-form.\nIt has a myriad of meanings. Primarily, it is a conjunctive particle that connects two clauses together.\nUsage: Attach て to the continuative form (連用形) of verbs after euphonic change form, くて to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～て".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("くて", "い", vec!["-て".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("て", "る", vec!["-て".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いて", "く", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いで", "ぐ", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("して", "す", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "う", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "つ", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("って", "る", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "ぬ", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "ぶ", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んで", "む", vec!["-て".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じて", "ずる", vec!["-て".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("して", "する", vec!["-て".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為て", "為る", vec!["-て".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きて", "くる", vec!["-て".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来て", "来る", vec!["-て".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來て", "來る", vec!["-て".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("まして", "ます", vec![], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-ず".to_string(),
            Transform {
                name: "-ず".to_string(),
                description: Some(
                    "1. Negative form of verbs.\n2. Continuative form (連用形) of the particle ぬ (nu).\nUsage: Attach ず to the irrealis form (未然形) of verbs.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ず".to_string(),
                    description: Some("～ない".into()),
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
            },
        ),
        (
            "-ぬ".to_string(),
            Transform {
                name: "-ぬ".to_string(),
                description: Some(
                    "Negative form of verbs.\nUsage: Attach ぬ to the irrealis form (未然形) of verbs.\nする becomes せぬ".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ぬ".to_string(),
                    description: Some("～ない".into()),
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
            },
        ),
        (
            "-ん".to_string(),
            Transform {
                name: "-ん".to_string(),
                description: Some(
                    "Negative form of verbs; a sound change of ぬ.\nUsage: Attach ん to the irrealis form (未然形) of verbs.\nする becomes せん".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ん".to_string(),
                    description: Some("～ない".into()),
                }]),
                rules: vec![
                    suffix_inflection("ん", "る", vec!["-ん".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かん", "く", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がん", "ぐ", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さん", "す", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たん", "つ", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なん", "ぬ", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばん", "ぶ", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まん", "む", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らん", "る", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わん", "う", vec!["-ん".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぜん", "ずる", vec!["-ん".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("せん", "する", vec!["-ん".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ん", "為る", vec!["-ん".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こん", "くる", vec!["-ん".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ん", "来る", vec!["-ん".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ん", "來る", vec!["-ん".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-んばかり".to_string(),
            Transform {
                name: "-んばかり".to_string(),
                description: Some(
                    "Shows an action or condition is on the verge of occurring, or an excessive/extreme degree.\nUsage: Attach んばかり to the irrealis form (未然形) of verbs.\nする becomes せんばかり".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～んばかり".to_string(),
                    description: Some("今にもそうなりそうな、しかし辛うじてそうなっていないようなさまを指す表現".into()),
                }]),
                rules: vec![
                    suffix_inflection("んばかり", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("かんばかり", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("がんばかり", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("さんばかり", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("たんばかり", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("なんばかり", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ばんばかり", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("まんばかり", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("らんばかり", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("わんばかり", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぜんばかり", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("せんばかり", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為んばかり", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こんばかり", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来んばかり", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來んばかり", "來る", vec![], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-んとする".to_string(),
            Transform {
                name: "-んとする".to_string(),
                description: Some(
                    "1. Shows the speaker's will or intention.\n2. Shows an action or condition is on the verge of occurring.\nUsage: Attach んとする to the irrealis form (未然形) of verbs.\nする becomes せんとする".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～んとする".to_string(),
                    description: Some("…しようとする、…しようとしている".into()),
                }]),
                rules: vec![
                    suffix_inflection("んとする", "る", vec!["vs".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かんとする", "く", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がんとする", "ぐ", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さんとする", "す", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たんとする", "つ", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なんとする", "ぬ", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばんとする", "ぶ", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まんとする", "む", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らんとする", "る", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わんとする", "う", vec!["vs".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぜんとする", "ずる", vec!["vs".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("せんとする", "する", vec!["vs".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為んとする", "為る", vec!["vs".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こんとする", "くる", vec!["vs".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来んとする", "来る", vec!["vs".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來んとする", "來る", vec!["vs".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-む".to_string(),
            Transform {
                name: "-む".to_string(),
                description: Some(
                    "Archaic.\n1. Shows an inference of a certain matter.\n2. Shows speaker's intention.\nUsage: Attach む to the irrealis form (未然形) of verbs.\nする becomes せむ".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～む".to_string(),
                    description: Some("…だろう".into()),
                }]),
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
        (
            "-ざる".to_string(),
            Transform {
                name: "-ざる".to_string(),
                description: Some(
                    "Negative form of verbs.\nUsage: Attach ざる to the irrealis form (未然形) of verbs.\nする becomes せざる".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ざる".to_string(),
                    description: Some("…ない…".into()),
                }]),
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
        (
            "-ねば".to_string(),
            Transform {
                name: "-ねば".to_string(),
                description: Some(
                    "1. Shows a hypothetical negation; if not ...\n2. Shows a must. Used with or without ならぬ.\nUsage: Attach ねば to the irrealis form (未然形) of verbs.\nする becomes せねば".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ねば".to_string(),
                    description: Some("もし…ないなら。…なければならない。".into()),
                }]),
                rules: vec![
                    suffix_inflection("ねば", "る", vec!["-ば".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かねば", "く", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がねば", "ぐ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さねば", "す", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たねば", "つ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なねば", "ぬ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばねば", "ぶ", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("まねば", "む", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らねば", "る", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わねば", "う", vec!["-ば".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ぜねば", "ずる", vec!["-ば".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("せねば", "する", vec!["-ば".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ねば", "為る", vec!["-ば".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こねば", "くる", vec!["-ば".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ねば", "来る", vec!["-ば".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ねば", "來る", vec!["-ば".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "-く".to_string(),
            Transform {
                name: "-く".to_string(),
                description: Some(
                    "Adverbial form of i-adjectives.\n".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～く".to_string(),
                    description: Some("〔形容詞で〕用言へ続く。例、「大きく育つ」の「大きく」。".into()),
                }]),
                rules: vec![
                    suffix_inflection("く", "い", vec!["-く".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "causative".to_string(),
            Transform {
                name: "causative".to_string(),
                description: Some(
                    "Describes the intention to make someone do something.\nUsage: Attach させる to the irrealis form (未然形) of ichidan verbs and くる.\nAttach せる to the irrealis form (未然形) of godan verbs and する.\nIt itself conjugates as an ichidan verb.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～せる・させる".to_string(),
                    description: Some("だれかにある行為をさせる意を表わす時の言い方。例、「行かせる」の「せる」。".into()),
                }]),
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
        (
            "short causative".to_string(),
            Transform {
                name: "short causative".to_string(),
                description: Some(
                    "Contraction of the causative form.\nDescribes the intention to make someone do something.\nUsage: Attach す to the irrealis form (未然形) of godan verbs.\nAttach さす to the dictionary form (終止形) of ichidan verbs.\nする becomes さす, くる becomes こさす.\nIt itself conjugates as an godan verb.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～す・さす".to_string(),
                    description: Some("だれかにある行為をさせる意を表わす時の言い方。例、「食べさす」の「さす」。".into()),
                }]),
                rules: vec![
                    suffix_inflection("さす", "る", vec!["v5ss".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("かす", "く", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がす", "ぐ", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("さす", "す", vec!["v5ss".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("たす", "つ", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("なす", "ぬ", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ばす", "ぶ", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("ます", "む", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("らす", "る", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("わす", "う", vec!["v5sp".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じさす", "ずる", vec!["v5ss".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("ぜさす", "ずる", vec!["v5ss".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("さす", "する", vec!["v5ss".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為す", "為る", vec!["v5ss".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("こさす", "くる", vec!["v5ss".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来さす", "来る", vec!["v5ss".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來さす", "來る", vec!["v5ss".to_string()], vec!["vk".to_string()]),
                ],
            },
        ),
        (
            "imperative".to_string(),
            Transform {
                name: "imperative".to_string(),
                description: Some(
                    "1. To give orders.\n2. (As あれ) Represents the fact that it will never change no matter the circumstances.\n3. Express a feeling of hope.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "命令形".to_string(),
                    description: Some("命令の意味を表わすときの形。例、「行け」。".into()),
                }]),
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
        (
            "continuative".to_string(),
            Transform {
                name: "continuative".to_string(),
                description: Some(
                    "Used to indicate actions that are (being) carried out.\nRefers to 連用形, the part of the verb after conjugating with -ます and dropping ます.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "連用形".to_string(),
                    description: Some("〔動詞などで〕「ます」などに続く。例、「バスを降りて歩きます」の「降り」「歩き」。".into()),
                }]),
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
        (
            "negative".to_string(),
            Transform {
                name: "negative".to_string(),
                description: Some(
                    "1. Negative form of verbs.\n2. Expresses a feeling of solicitation to the other party.\nUsage: Attach ない to the irrealis form (未然形) of verbs, くない to the stem of i-adjectives. ない itself conjugates as i-adjective. ます becomes ません.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ない".to_string(),
                    description: Some("その動作・作用・状態の成立を否定することを表わす。".into()),
                }]),
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
                    suffix_inflection("ません", "ます", vec!["-ません".to_string()], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-さ".to_string(),
            Transform {
                name: "-さ".to_string(),
                description: Some(
                    "Nominalizing suffix of i-adjectives indicating nature, state, mind or degree.\nUsage: Attach さ to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～さ".to_string(),
                    description: Some("こと。程度。".into()),
                }]),
                rules: vec![
                    suffix_inflection("さ", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "passive".to_string(),
            Transform {
                name: "passive".to_string(),
                description: Some(
                    "Indicates that the subject is affected by the action of the verb.\nUsage: Attach れる to the irrealis form (未然形) of godan verbs.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～れる".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("かれる", "く", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("がれる", "ぐ", vec!["v1".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("される", "す", vec!["v1".to_string()], vec!["v5d".to_string(), "v5sp".to_string()]),
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
        (
            "-た".to_string(),
            Transform {
                name: "-た".to_string(),
                description: Some(
                    "1. Indicates a reality that has happened in the past.\n2. Indicates the completion of an action.\n3. Indicates the confirmation of a matter.\n4. Indicates the speaker's confidence that the action will definitely be fulfilled.\n5. Indicates the events that occur before the main clause are represented as relative past.\n6. Indicates a mild imperative/command.\nUsage: Attach た to the continuative form (連用形) of verbs after euphonic change form, かった to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～た".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("かった", "い", vec!["-た".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("た", "る", vec!["-た".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("いた", "く", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("いだ", "ぐ", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("した", "す", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "う", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "つ", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("った", "る", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "ぬ", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "ぶ", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("んだ", "む", vec!["-た".to_string()], vec!["v5".to_string()]),
                    suffix_inflection("じた", "ずる", vec!["-た".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("した", "する", vec!["-た".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為た", "為る", vec!["-た".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きた", "くる", vec!["-た".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来た", "来る", vec!["-た".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來た", "來る", vec!["-た".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("ました", "ます", vec!["-た".to_string()], vec!["-ます".to_string()]),
                    suffix_inflection("でした", "", vec!["-た".to_string()], vec!["-ません".to_string()]),
                    suffix_inflection("かった", "", vec!["-た".to_string()], vec!["-ません".to_string(), "-ん".to_string()]),
                ],
            },
        ),
        (
            "-ます".to_string(),
            Transform {
                name: "-ます".to_string(),
                description: Some(
                    "Polite conjugation of verbs and adjectives.\nUsage: Attach ます to the continuative form (連用形) of verbs.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～ます".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("ます", "る", vec!["-ます".to_string()], vec!["v1".to_string()]),
                    suffix_inflection("います", "う", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("きます", "く", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ぎます", "ぐ", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("します", "す", vec!["-ます".to_string()], vec!["v5d".to_string(), "v5s".to_string()]),
                    suffix_inflection("ちます", "つ", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("にます", "ぬ", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("びます", "ぶ", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("みます", "む", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("ります", "る", vec!["-ます".to_string()], vec!["v5d".to_string()]),
                    suffix_inflection("じます", "ずる", vec!["-ます".to_string()], vec!["vz".to_string()]),
                    suffix_inflection("します", "する", vec!["-ます".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("為ます", "為る", vec!["-ます".to_string()], vec!["vs".to_string()]),
                    suffix_inflection("きます", "くる", vec!["-ます".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("来ます", "来る", vec!["-ます".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("來ます", "來る", vec!["-ます".to_string()], vec!["vk".to_string()]),
                    suffix_inflection("くあります", "い", vec!["-ます".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "potential".to_string(),
            Transform {
                name: "potential".to_string(),
                description: Some(
                    "Indicates a state of being (naturally) capable of doing an action.\nUsage: Attach (ら)れる to the irrealis form (未然形) of ichidan verbs.\nAttach る to the imperative form (命令形) of godan verbs.\nする becomes できる, くる becomes こ(ら)れる.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～(ら)れる".to_string(),
                    description: None,
                }]),
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
        (
            "potential or passive".to_string(),
            Transform {
                name: "potential or passive".to_string(),
                description: Some(
                    "Indicates that the subject is affected by the action of the verb.\n3. Indicates a state of being (naturally) capable of doing an action.\nUsage: Attach られる to the irrealis form (未然形) of ichidan verbs.\nする becomes せられる, くる becomes こられる.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～られる".to_string(),
                    description: None,
                }]),
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
        (
            "volitional".to_string(),
            Transform {
                name: "volitional".to_string(),
                description: Some(
                    "1. Expresses speaker's will or intention.\n2. Expresses an invitation to the other party.\n3. (Used in …ようとする) Indicates being on the verge of initiating an action or transforming a state.\n4. Indicates an inference of a matter.\nUsage: Attach よう to the irrealis form (未然形) of ichidan verbs.\nAttach う to the irrealis form (未然形) of godan verbs after -o euphonic change form.\nAttach かろう to the stem of i-adjectives (4th meaning only).".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～う・よう".to_string(),
                    description: Some("主体の意志を表わす".into()),
                }]),
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
                    suffix_inflection("ましょう", "ます", vec![], vec!["-ます".to_string()]),
                    suffix_inflection("かろう", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "volitional slang".to_string(),
            Transform {
                name: "volitional slang".to_string(),
                description: Some(
                    "Contraction of volitional form + か\n1. Expresses speaker's will or intention.\n2. Expresses an invitation to the other party.\nUsage: Replace final う with っ of volitional form then add か.\nFor example: 行こうか -> 行こっか.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～っか・よっか".to_string(),
                    description: Some("「うか・ようか」の短縮".into()),
                }]),
                rules: vec![
                    suffix_inflection("よっか", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("おっか", "う", vec![], vec!["v5".to_string()]),
                    suffix_inflection("こっか", "く", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ごっか", "ぐ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("そっか", "す", vec![], vec!["v5".to_string()]),
                    suffix_inflection("とっか", "つ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("のっか", "ぬ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ぼっか", "ぶ", vec![], vec!["v5".to_string()]),
                    suffix_inflection("もっか", "む", vec![], vec!["v5".to_string()]),
                    suffix_inflection("ろっか", "る", vec![], vec!["v5".to_string()]),
                    suffix_inflection("じよっか", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("しよっか", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為よっか", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こよっか", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来よっか", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來よっか", "來る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("ましょっか", "ます", vec![], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-まい".to_string(),
            Transform {
                name: "-まい".to_string(),
                description: Some(
                    "Negative volitional form of verbs.\n1. Expresses speaker's assumption that something is likely not true.\n2. Expresses speaker's will or intention not to do something.\nUsage: Attach まい to the dictionary form (終止形) of verbs.\nAttach まい to the irrealis form (未然形) of ichidan verbs.\nする becomes しまい, くる becomes こまい.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～まい".to_string(),
                    description: Some(
                        "1. 打うち消けしの推量すいりょう 「～ないだろう」と想像する\n2. 打うち消けしの意志いし「～ないつもりだ」という気持ち".into(),
                    ),
                }]),
                rules: vec![
                    suffix_inflection("まい", "", vec![], vec!["v".to_string()]),
                    suffix_inflection("まい", "る", vec![], vec!["v1".to_string()]),
                    suffix_inflection("じまい", "ずる", vec![], vec!["vz".to_string()]),
                    suffix_inflection("しまい", "する", vec![], vec!["vs".to_string()]),
                    suffix_inflection("為まい", "為る", vec![], vec!["vs".to_string()]),
                    suffix_inflection("こまい", "くる", vec![], vec!["vk".to_string()]),
                    suffix_inflection("来まい", "来る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("來まい", "來る", vec![], vec!["vk".to_string()]),
                    suffix_inflection("まい", "", vec![], vec!["-ます".to_string()]),
                ],
            },
        ),
        (
            "-おく".to_string(),
            Transform {
                name: "-おく".to_string(),
                description: Some(
                    "To do certain things in advance in preparation (or in anticipation) of latter needs.\nUsage: Attach おく to the て-form of verbs.\nAttach でおく after ない negative form of verbs.\nContracts to とく・どく in speech.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～おく".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("ておく", "て", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("でおく", "で", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("とく", "て", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("どく", "で", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ないでおく", "ない", vec!["v5".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("ないどく", "ない", vec!["v5".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "-いる".to_string(),
            Transform {
                name: "-いる".to_string(),
                description: Some(
                    "1. Indicates an action continues or progresses to a point in time.\n2. Indicates an action is completed and remains as is.\n3. Indicates a state or condition that can be taken to be the result of undergoing some change.\nUsage: Attach いる to the て-form of verbs. い can be dropped in speech.\nAttach でいる after ない negative form of verbs.\n(Slang) Attach おる to the て-form of verbs. Contracts to とる・でる in speech.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～いる".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("ている", "て", vec!["v1".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ておる", "て", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("てる", "て", vec!["v1p".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("でいる", "で", vec!["v1".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("でおる", "で", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("でる", "で", vec!["v1p".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("とる", "て", vec!["v5".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ないでいる", "ない", vec!["v1".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "-き".to_string(),
            Transform {
                name: "-き".to_string(),
                description: Some(
                    "Attributive form (連体形) of i-adjectives. An archaic form that remains in modern Japanese.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～き".to_string(),
                    description: Some("連体形".into()),
                }]),
                rules: vec![
                    suffix_inflection("き", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "-げ".to_string(),
            Transform {
                name: "-げ".to_string(),
                description: Some(
                    "Describes a person's appearance. Shows feelings of the person.\nUsage: Attach げ or 気 to the stem of i-adjectives.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～げ".to_string(),
                    description: Some("…でありそうな様子。いかにも…らしいさま。".into()),
                }]),
                rules: vec![
                    suffix_inflection("げ", "い", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("気", "い", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "-がる".to_string(),
            Transform {
                name: "-がる".to_string(),
                description: Some(
                    "1. Shows subject’s feelings contrast with what is thought/known about them.\n2. Indicates subject's behavior (stands out).\nUsage: Attach がる to the stem of i-adjectives. It itself conjugates as a godan verb.".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～がる".to_string(),
                    description: Some("いかにもその状態にあるという印象を相手に与えるような言動をする。".into()),
                }]),
                rules: vec![
                    suffix_inflection("がる", "い", vec!["v5".to_string()], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "-え".to_string(),
            Transform {
                name: "-え".to_string(),
                description: Some(
                    "Slang. A sound change of i-adjectives.\nai：やばい → やべぇ\nui：さむい → さみぃ/さめぇ\noi：すごい → すげぇ".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～え".to_string(),
                    description: None,
                }]),
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
        (
            "n-slang".to_string(),
            Transform {
                name: "n-slang".to_string(),
                description: Some(
                    "Slang sound change of r-column syllables to n (when before an n-sound, usually の or な)".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～んな".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("んなさい", "りなさい", vec![], vec!["-なさい".to_string()]),
                    suffix_inflection("らんない", "られない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("んない", "らない", vec!["adj-i".to_string()], vec!["adj-i".to_string()]),
                    suffix_inflection("んなきゃ", "らなきゃ", vec![], vec!["-ゃ".to_string()]),
                    suffix_inflection("んなきゃ", "れなきゃ", vec![], vec!["-ゃ".to_string()]),
                ],
            },
        ),
        (
            "imperative negative slang".to_string(),
            Transform {
                name: "imperative negative slang".to_string(),
                description: None,
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "～んな".to_string(),
                    description: None,
                }]),
                rules: vec![
                    suffix_inflection("んな", "る", vec![], vec!["v".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben negative".to_string(),
            Transform {
                name: "kansai-ben negative".to_string(),
                description: Some(
                    "Negative form of kansai-ben verbs".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～ない (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("へん", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("ひん", "ない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("せえへん", "しない", vec![], vec!["adj-i".to_string()]),
                    suffix_inflection("へんかった", "なかった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ひんかった", "なかった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("うてへん", "ってない", vec![], vec!["adj-i".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben -て".to_string(),
            Transform {
                name: "kansai-ben -て".to_string(),
                description: Some(
                    "-て form of kansai-ben verbs".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～て (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("うて", "って", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("おうて", "あって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("こうて", "かって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ごうて", "がって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("そうて", "さって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ぞうて", "ざって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("とうて", "たって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("どうて", "だって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("のうて", "なって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ほうて", "はって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ぼうて", "ばって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("もうて", "まって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ろうて", "らって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ようて", "やって", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ゆうて", "いって", vec!["-て".to_string()], vec!["-て".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben -た".to_string(),
            Transform {
                name: "kansai-ben -た".to_string(),
                description: Some(
                    "-た form of kansai-ben terms".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～た (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("うた", "った", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("おうた", "あった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("こうた", "かった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ごうた", "がった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("そうた", "さった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ぞうた", "ざった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("とうた", "たった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("どうた", "だった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("のうた", "なった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ほうた", "はった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ぼうた", "ばった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("もうた", "まった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ろうた", "らった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ようた", "やった", vec!["-た".to_string()], vec!["-た".to_string()]),
                    suffix_inflection("ゆうた", "いった", vec!["-た".to_string()], vec!["-た".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben -たら".to_string(),
            Transform {
                name: "kansai-ben -たら".to_string(),
                description: Some(
                    "-たら form of kansai-ben terms".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～たら (関西弁)".into()),
                }]),
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
            "kansai-ben -たり".to_string(),
            Transform {
                name: "kansai-ben -たり".to_string(),
                description: Some(
                    "-たり form of kansai-ben terms".into(),
                ),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～たり (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("うたり", "ったり", vec![], vec![]),
                    suffix_inflection("おうたり", "あったり", vec![], vec![]),
                    suffix_inflection("こうたり", "かったり", vec![], vec![]),
                    suffix_inflection("ごうたり", "がったり", vec![], vec![]),
                    suffix_inflection("そうたり", "さったり", vec![], vec![]),
                    suffix_inflection("ぞうたり", "ざったり", vec![], vec![]),
                    suffix_inflection("とうたり", "たったり", vec![], vec![]),
                    suffix_inflection("どうたり", "だったり", vec![], vec![]),
                    suffix_inflection("のうたり", "なったり", vec![], vec![]),
                    suffix_inflection("ほうたり", "はったり", vec![], vec![]),
                    suffix_inflection("ぼうたり", "ばったり", vec![], vec![]),
                    suffix_inflection("もうたり", "まったり", vec![], vec![]),
                    suffix_inflection("ろうたり", "らったり", vec![], vec![]),
                    suffix_inflection("ようたり", "やったり", vec![], vec![]),
                    suffix_inflection("ゆうたり", "いったり", vec![], vec![]),
                ],
            },
        ),
        (
            "kansai-ben -く".to_string(),
            Transform {
                name: "kansai-ben -く".to_string(),
                description: Some("-く stem of kansai-ben adjectives".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("連用形 (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("う", "く", vec![], vec!["-く".to_string()]),
                    suffix_inflection("こう", "かく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("ごう", "がく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("そう", "さく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("とう", "たく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("のう", "なく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("ぼう", "ばく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("もう", "まく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("ろう", "らく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("よう", "よく", vec![], vec!["-く".to_string()]),
                    suffix_inflection("しゅう", "しく", vec![], vec!["-く".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben adjective -て".to_string(),
            Transform {
                name: "kansai-ben adjective -て".to_string(),
                description: Some("-て form of kansai-ben adjectives".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～て (関西弁)".into()),
                }]),
                rules: vec![
                    suffix_inflection("うて", "くて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("こうて", "かくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ごうて", "がくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("そうて", "さくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("とうて", "たくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("のうて", "なくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ぼうて", "ばくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("もうて", "まくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ろうて", "らくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("ようて", "よくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                    suffix_inflection("しゅうて", "しくて", vec!["-て".to_string()], vec!["-て".to_string()]),
                ],
            },
        ),
        (
            "kansai-ben adjective negative".to_string(),
            Transform {
                name: "kansai-ben adjective negative".to_string(),
                description: Some("Negative form of kansai-ben adjectives".into()),
                i18n: Some(vec![TransformI18n {
                    language: "ja".to_string(),
                    name: "関西弁".to_string(),
                    description: Some("～ない (関西弁)".into()),
                }]),
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
    ]));
    t
});

#[cfg(test)]
mod jp_transforms {
    use super::*;
    use crate::language::ja::transforms::JAPANESE_TRANSFORMS;

    #[test]
    fn len() {
        assert_eq!(JAPANESE_TRANSFORMS.transforms.len(), 53);
    }

    #[test]
    fn test_japanese_transformations() {
        let mut lt = LanguageTransformer::new();
        lt.add_descriptor(&*JAPANESE_TRANSFORMS);

        for (i, test) in TRANSFORM_TESTS.iter().enumerate() {
            let term = test.term;
            for case in &test.sources {
                let source = case.inner;
                let rule = case.rule;
                let expected_reasons = &case.reasons;

                let result =
                    has_term_reasons(&lt, source, term, Some(rule), Some(expected_reasons));
                if let Err(e) = result {
                    panic!("Failed: {}", e);
                }
            }
        }
    }
}

pub(crate) static CONDITIONS: LazyLock<ConditionMap> = LazyLock::new(|| {
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
