use crate::dictionary_importer::ImportDetails;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictionaryMessageActionMatchType {
    ImportDictionary,
    DeleteDictionary,
    GetDictionaryCounts,

}


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDictionaryMessageParams{
    details: ImportDetails,
    archive_content: Vec<u8>,
}
