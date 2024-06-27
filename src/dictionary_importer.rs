use crate::database::DatabaseTermEntry;
use crate::dictionary_data::TermGlossaryImage;
use crate::errors::ImportError;
use crate::structured_content::ImageElement;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSteps {
    Uninitialized,
    ValidateIndex,
    ValidateSchema,
    FormatDictionary,
    ImportMedia,
    ImportData,
    Completed,
}

