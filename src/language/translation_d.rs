use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::{dictionary::TermSourceMatchType, settings::SearchResolution};

pub type KanjiEnabledDictionaryMap<'a> = HashMap<&'a str, FindKanjiDictionary>;
pub type TermEnabledDictionaryMap<'a> = HashMap<&'a str, FindTermDictionary>;
