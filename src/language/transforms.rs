// use std::sync::Arc;
//
// use regex::Regex;
//
// // use crate::language::transformer_d::RuleType;
//
// use super::{
//     ja::transforms::{FU_VERB_TE_CONJUGATIONS, GODAN_U_SPECIAL_VERBS, IKU_VERBS},
//     // transformer_d::{Condition, Rule, SuffixRule},
// };
//
// pub fn suffix_inflection(
//     inflected_suffix: &'static str,
//     deinflected_suffix: &'static str,
//     conditions_in: &'static [&'static str],
//     conditions_out: &'static [&'static str],
// ) -> SuffixRule {
//     let reg_str = format!("{}$", inflected_suffix);
//     let suffix_regex = Regex::new(&reg_str).unwrap();
//     SuffixRule {
//         rule_type: RuleType::Suffix,
//         is_inflected: suffix_regex,
//         deinflected: deinflected_suffix,
//         conditions_in,
//         conditions_out,
//     }
// }
//
// pub fn prefix_inflection<'a>(
//     inflected_prefix: &'a str,
//     deinflected_prefix: &'a str,
//     conditions_in: Vec<&'a str>,
//     conditions_out: Vec<&'a str>,
// ) -> Rule<'a, impl Fn(&str, &str, &str) -> String> {
//     let prefix_reg_exp = Regex::new(format!("^{}", inflected_prefix).as_str()).unwrap();
//     let deinflect = move |text: &str, inflected_prefix: &str, deinflected_prefix: &str| -> String {
//         format!("{}{}", deinflected_prefix, &text[inflected_prefix.len()..])
//     };
//
//     Rule {
//         rule_type: RuleType::Prefix,
//         is_inflected: prefix_reg_exp,
//         deinflect,
//         conditions_in,
//         conditions_out,
//     }
// }
//
// #[derive(Debug)]
// pub(crate) enum IrregularVerbSuffix {
//     て,
//     た,
//     たら,
//     たり,
// }
//
// impl std::fmt::Display for IrregularVerbSuffix {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{self:?}")
//     }
// }
//
// pub fn irregular_verb_suffix_inflections(
//     suffix: IrregularVerbSuffix,
//     conditions_in: &'static [&'static str],
//     conditions_out: &'static [&'static str],
// ) -> Vec<SuffixRule> {
//     let suffix_str = suffix.to_string();
//
//     let iku_inflections = IKU_VERBS.iter().map(|verb| {
//         let first_char = verb.chars().next().unwrap();
//         let transformed: &'static str = format!("{}っ{}", first_char, suffix_str).leak();
//         suffix_inflection(transformed, verb, conditions_in, conditions_out)
//     });
//
//     let godan_inflections = GODAN_U_SPECIAL_VERBS.iter().map(|verb| {
//         let transformed: &'static str = format!("{}{}", verb, suffix_str).leak();
//         suffix_inflection(transformed, verb, conditions_in, conditions_out)
//     });
//
//     let fu_inflections = FU_VERB_TE_CONJUGATIONS.iter().map(|[verb, te_root]| {
//         let transformed: &'static str = format!("{}{}", te_root, suffix_str).leak();
//         suffix_inflection(transformed, verb, conditions_in, conditions_out)
//     });
//
//     iku_inflections
//         .chain(godan_inflections)
//         .chain(fu_inflections)
//         .collect()
// }
//
// pub fn whole_word_inflection<'a>(
//     inflected_word: &'a str,
//     deinflected_word: &'a str,
//     conditions_in: Vec<&'a str>,
//     conditions_out: Vec<&'a str>,
// ) -> Rule<'a, impl Fn(&str, &str, &str) -> String> {
//     let regex = Regex::new(format!("^{}$", inflected_word).as_str()).unwrap();
//
//     let deinflected = deinflected_word.to_owned();
//
//     let deinflect = move |_text: &str,
//                           _inflected_suffix: &str,
//                           _deinflected_suffix: &str|
//           -> String { deinflected.clone() };
//
//     Rule {
//         rule_type: RuleType::WholeWord,
//         is_inflected: regex,
//         deinflect,
//         conditions_in,
//         conditions_out,
//     }
// }
//
// #[cfg(test)]
// mod inflection_tests {
//     use std::sync::Arc;
//
//     use pretty_assertions::assert_eq;
//     use regex::Regex;
//
//     use crate::language::{
//         transformer_d::{DeinflectFnTrait, RuleType, SuffixRule},
//         transforms::suffix_inflection,
//     };
//
//     use super::irregular_verb_suffix_inflections;
//
//     #[test]
//     pub fn irregular_verb_suffix() {
//         #[rustfmt::skip]
//         let te_test = [SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("いって$").unwrap(), deinflected: "いく", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("行って$").unwrap(), deinflected: "行く", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("逝って$").unwrap(), deinflected: "逝く", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("往って$").unwrap(), deinflected: "往く", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("こうて$").unwrap(), deinflected: "こう", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("とうて$").unwrap(), deinflected: "とう", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("請うて$").unwrap(), deinflected: "請う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("乞うて$").unwrap(), deinflected: "乞う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("恋うて$").unwrap(), deinflected: "恋う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("問うて$").unwrap(), deinflected: "問う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("訪うて$").unwrap(), deinflected: "訪う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("宣うて$").unwrap(), deinflected: "宣う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("曰うて$").unwrap(), deinflected: "曰う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("給うて$").unwrap(), deinflected: "給う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("賜うて$").unwrap(), deinflected: "賜う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("揺蕩うて$").unwrap(), deinflected: "揺蕩う", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("のたもうて$").unwrap(), deinflected: "のたまう", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("たもうて$").unwrap(), deinflected: "たまう", conditions_in: &["-て"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("たゆとうて$").unwrap(), deinflected: "たゆたう", conditions_in: &["-て"], conditions_out: &["v5"] }];
//         #[rustfmt::skip]
//         let ta_test = [SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("いった$").unwrap(), deinflected: "いく", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("行った$").unwrap(), deinflected: "行く", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("逝った$").unwrap(), deinflected: "逝く", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("往った$").unwrap(), deinflected: "往く", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("こうた$").unwrap(), deinflected: "こう", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("とうた$").unwrap(), deinflected: "とう", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("請うた$").unwrap(), deinflected: "請う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("乞うた$").unwrap(), deinflected: "乞う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("恋うた$").unwrap(), deinflected: "恋う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("問うた$").unwrap(), deinflected: "問う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("訪うた$").unwrap(), deinflected: "訪う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("宣うた$").unwrap(), deinflected: "宣う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("曰うた$").unwrap(), deinflected: "曰う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("給うた$").unwrap(), deinflected: "給う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("賜うた$").unwrap(), deinflected: "賜う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("揺蕩うた$").unwrap(), deinflected: "揺蕩う", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("のたもうた$").unwrap(), deinflected: "のたまう", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("たもうた$").unwrap(), deinflected: "たまう", conditions_in: &["-た"], conditions_out: &["v5"] }, SuffixRule { rule_type: RuleType::Suffix, is_inflected: Regex::new("たゆとうた$").unwrap(), deinflected: "たゆたう", conditions_in: &["-た"], conditions_out: &["v5"] }];
//         let て =
//             irregular_verb_suffix_inflections(super::IrregularVerbSuffix::て, &["-て"], &["v5"]);
//         assert_eq!(て, te_test);
//         let た =
//             irregular_verb_suffix_inflections(super::IrregularVerbSuffix::た, &["-た"], &["v5"]);
//         assert_eq!(た, ta_test);
//     }
//
//     #[test]
//     pub fn suffix() {
//         let test = SuffixRule {
//             rule_type: RuleType::Suffix,
//             is_inflected: Regex::new("ければ$").unwrap(),
//             deinflected: "い",
//             conditions_in: &["-ば"],
//             conditions_out: &["adj-i"],
//         };
//         let sr = suffix_inflection("ければ", "い", &["-ば"], &["adj-i"]);
//         assert_eq!(sr, test);
//         assert_eq!(sr.deinflect("食べれば"), test.deinflect("食べれば"));
//     }
// }
