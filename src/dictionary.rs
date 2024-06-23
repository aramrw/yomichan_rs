use crate::dictionary_data::TermGlossaryContent;
use crate::errors;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;
use std::io::{self, BufRead, BufReader};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumOrStr {
    Num(u64),
    Str(String),
}

