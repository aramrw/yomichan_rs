use std::str::FromStr;

use super::ja::japanese::is_string_partially_japanese;

/// Returns the language that the string might be by using some heuristic checks.
/// Values returned are ISO codes. `None` is returned if no language can be determined.
pub fn get_language_from_text<T: AsRef<str>>(text: T) -> Option<String> {
    if is_string_partially_japanese(text.as_ref()) {
        return Some(String::from("ja"));
    }
    None
}
