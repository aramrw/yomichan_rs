use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFrequencyData, TermGlossary, TermMetaFrequencyDataType, TermMetaModeType,
    TermMetaPhoneticData, TermMetaPitchData,
};
use crate::errors;
use crate::Yomichan;

//use redb::TableDefinition;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

