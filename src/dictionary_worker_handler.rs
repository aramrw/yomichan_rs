use crate::dictionary_importer::ImportDetails;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictionaryMessageActionMatchType {
    ImportDictionary,
    DeleteDictionary,
    GetDictionaryCounts,

}


