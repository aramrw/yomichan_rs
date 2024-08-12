use regex::Regex;

use crate::language::transformer_d::RuleType;

use super::transformer_d::{Condition, Rule, SuffixRule};

pub fn suffix_inflection<'a>(
    inflected_suffix: &'a str,
    deinflected_suffix: &'a str,
    conditions_in: Vec<&'a str>,
    conditions_out: Vec<&'a str>,
) -> SuffixRule<'a> {
    let suffix_regex = Regex::new(format!("{}$", inflected_suffix).as_str()).unwrap();
    SuffixRule {
        rule_type: RuleType::Suffix,
        is_inflected: suffix_regex,
        deinflected: deinflected_suffix,
        conditions_in,
        conditions_out,
    }
}

pub fn prefix_inflection<T: AsRef<str>>(
    inflected_prefix: T,
    deinflected_prefix: T,
    conditions_in: Vec<String>,
    conditions_out: Vec<String>,
) -> Rule<impl Fn(&str, &str, &str) -> String> {
    let prefix_reg_exp = Regex::new(format!("^{}", inflected_prefix.as_ref()).as_str()).unwrap();
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

pub fn whole_word_inflection<T: AsRef<str>>(
    inflected_word: T,
    deinflected_word: T,
    conditions_in: Vec<String>,
    conditions_out: Vec<String>,
) -> Rule<impl Fn(&str, &str, &str) -> String> {
    let regex = Regex::new(format!("^{}$", inflected_word.as_ref()).as_str()).unwrap();

    let deinflected = deinflected_word.as_ref().to_owned();

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
