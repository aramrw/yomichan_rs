use crate::dictionary_importer::ImportDetails;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictionaryMessageActionMatchType {
    ImportDictionary,
    DeleteDictionary,
    GetDictionaryCounts,
    GetImageDetailsResponse,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDictionaryMessage {
    /// This is of type [`DictionaryMessageActionMatchType::ImportDictionary`]
    action: DictionaryMessageActionMatchType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDictionaryMessageParams {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetDictionaryCountsMessageParams {
    dictionary_names: Vec<String>,
    get_total: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetImageDetailsResponseMessage {
    /// This is of type [`DictionaryMessageActionMatchType::GetImageDetailsResponse`]
    action: DictionaryMessageActionMatchType,
    params: DictionaryWorkerMediaLoaderHandleMessageParams,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryWorkerMediaLoaderHandleMessageParams {
    // Define the fields of this struct based on the actual fields of the JavaScript object
}
