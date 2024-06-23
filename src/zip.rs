use crate::dictionary::TermHeadword;
use crate::errors;
//use redb::Database;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
//use std::collections::HashMap;
use crate::try_with_line;
use tempfile::tempdir;

use std::fs;
use std::io::{self, BufReader, Error as StdIOError, ErrorKind as StdIOErrorKind};

pub type Entries = Vec<Vec<EntryItem>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryItem {
    Str(String),
    Int(i64),
    ContentBlock(Vec<serde_json::Value>),
}

