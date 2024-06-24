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

// ! Javascript's `ArrayBuffer` most likely fundamentally differs from any Rust ds'
// ! Should look up how to map these before continuing
// ! So media can be displayed (or at least stored) within the lib
// TODO https://github.com/themoeway/yomitan/blob/8eea3661714c64857aa32a8662f7cca6674dd3a4/types/ext/dictionary-database.d.ts#L41
#[derive(Serialize, Deserialize, Debug)]
pub struct MediaDataBase<TContentType> {
    dictionary: String,
    path: String,
    media_type: String,
    width: u16,
    height: u16,
    content: TContentType,
}

