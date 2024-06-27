use crate::database::DatabaseTermEntry;
use crate::dictionary_data::TermGlossaryImage;
use crate::errors::ImportError;
use crate::structured_content::ImageElement;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

