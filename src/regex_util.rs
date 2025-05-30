use fancy_regex::{Captures, Regex};
use std::sync::LazyLock;

static MATCH_REPLACEMENT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(?:\$|&|`|'|(\d\d?)|<([^>]*)>)").unwrap());

pub fn apply_text_replacement(
    text: &str,
    pattern: &Regex,
    replacement_pattern_str: &str,
    is_global: &bool,
) -> String {
    let mut current_text = text.to_string();
    let mut current_pos = 0;
    let mut first_match_done = false;

    loop {
        if !is_global && first_match_done {
            break;
        }

        let captures_opt = pattern.captures(&current_text[current_pos..]).unwrap();
        let captures = match captures_opt {
            Some(caps) => caps,
            None => break,
        };

        let match_text = captures.get(0).unwrap();
        let match_start_index = current_pos + match_text.start();
        let match_end_index = current_pos + match_text.end();

        let actual_replacement =
            apply_match_replacement(replacement_pattern_str, &captures, &current_text);

        let mut new_text = String::with_capacity(
            current_text.len() - match_text.as_str().len() + actual_replacement.len(),
        );
        new_text.push_str(&current_text[..match_start_index]);
        new_text.push_str(&actual_replacement);
        new_text.push_str(&current_text[match_end_index..]);
        current_text = new_text;

        first_match_done = true;

        current_pos = match_start_index + actual_replacement.len();

        if current_pos >= current_text.len() {
            break;
        }
    }

    current_text
}

pub fn apply_match_replacement(
    replacement_pattern_input: &str,
    outer_captures: &Captures,
    original_text_at_match_time: &str,
) -> String {
    MATCH_REPLACEMENT_PATTERN
        .replace_all(replacement_pattern_input, |inner_caps: &Captures| {
            let g0_match_str = inner_caps.get(0).unwrap().as_str();

            if let Some(g1_digit_match) = inner_caps.get(1) {
                let group_index_str = g1_digit_match.as_str();
                if let Ok(idx) = group_index_str.parse::<usize>() {
                    if idx > 0 && idx < outer_captures.len() {
                        return outer_captures
                            .get(idx)
                            .map_or("".to_string(), |m| m.as_str().to_string());
                    }
                }
                g0_match_str.to_string()
            } else if let Some(g2_name_match) = inner_caps.get(2) {
                let group_name = g2_name_match.as_str();
                if let Some(named_capture) = outer_captures.name(group_name) {
                    return named_capture.as_str().to_string();
                }
                g0_match_str.to_string()
            } else {
                let match_start = outer_captures.get(0).unwrap().start();

                match g0_match_str {
                    "$$" => "$".to_string(),
                    "$&" => outer_captures.get(0).unwrap().as_str().to_string(),
                    "$`" => original_text_at_match_time[..match_start].to_string(),
                    "$'" => {
                        let match_end = outer_captures.get(0).unwrap().end();
                        original_text_at_match_time[match_end..].to_string()
                    }
                    _ => g0_match_str.to_string(),
                }
            }
        })
        .into_owned()
}
