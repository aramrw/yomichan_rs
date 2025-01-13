use std::sync::Arc;

use regex::Regex;

use crate::language::transformer_d::RuleType;

use super::transformer_d::{Condition, Rule, SuffixRule};

pub fn suffix_inflection<'a>(
    inflected_suffix: &'a str,
    deinflected_suffix: &'a str,
    conditions_in: Vec<String>,
    conditions_out: Vec<String>,
) -> SuffixRule {
    let inflected_suffix = inflected_suffix.to_string();
    let deinflected_suffix = deinflected_suffix.to_string();
    let deinflected_suffix_2 = deinflected_suffix.to_string();
    let suffix_regex = Regex::new(format!("{}$", inflected_suffix).as_str()).unwrap();
    let deinflect = Arc::new(move |text: String| -> String {
        let base = &text[..text.len() - inflected_suffix.len()];
        format!("{}{}", base, deinflected_suffix)
    });
    SuffixRule {
        rule_type: RuleType::Suffix,
        is_inflected: suffix_regex,
        deinflected: deinflected_suffix_2,
        deinflect,
        conditions_in,
        conditions_out,
    }
}

pub fn prefix_inflection<'a>(
    inflected_prefix: &'a str,
    deinflected_prefix: &'a str,
    conditions_in: Vec<&'a str>,
    conditions_out: Vec<&'a str>,
) -> Rule<'a, impl Fn(&str, &str, &str) -> String> {
    let prefix_reg_exp = Regex::new(format!("^{}", inflected_prefix).as_str()).unwrap();
    let deinflect = move |text: &str, inflected_prefix: &str, deinflected_prefix: &str| -> String {
        format!("{}{}", deinflected_prefix, &text[inflected_prefix.len()..])
    };

    Rule {
        rule_type: RuleType::Prefix,
        is_inflected: prefix_reg_exp,
        deinflect,
        conditions_in,
        conditions_out,
    }
}

pub fn whole_word_inflection<'a>(
    inflected_word: &'a str,
    deinflected_word: &'a str,
    conditions_in: Vec<&'a str>,
    conditions_out: Vec<&'a str>,
) -> Rule<'a, impl Fn(&str, &str, &str) -> String> {
    let regex = Regex::new(format!("^{}$", inflected_word).as_str()).unwrap();

    let deinflected = deinflected_word.to_owned();

    let deinflect = move |_text: &str,
                          _inflected_suffix: &str,
                          _deinflected_suffix: &str|
          -> String { deinflected.clone() };

    Rule {
        rule_type: RuleType::WholeWord,
        is_inflected: regex,
        deinflect,
        conditions_in,
        conditions_out,
    }
}
