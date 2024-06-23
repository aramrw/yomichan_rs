use crate::dictionary_data::TermGlossaryContent;
use crate::errors;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;
use std::io::{self, BufRead, BufReader};

