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
