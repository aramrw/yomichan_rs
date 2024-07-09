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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDictionaryMessage {
    /// This is of type [`DictionaryMessageActionMatchType::DeleteDictionary`]
    action: DictionaryMessageActionMatchType,
    params: DeleteDictionaryMessageParams,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDictionaryMessageParams {
    dictionary_title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetDictionaryCountsMessage {
    /// This is of type [`DictionaryMessageActionMatchType::GetDictionaryCounts`]
    action: DictionaryMessageActionMatchType,
    params: GetDictionaryCountsMessageParams,
}

