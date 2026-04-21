pub mod dictionary_database;
pub mod dictionary_importer;

pub use dictionary_database::{
    DatabaseKanjiEntry, DatabaseKanjiMeta, DatabaseMetaFrequency, DatabaseMetaMatchType,
    DatabaseMetaPhonetic, DatabaseMetaPitch, DatabaseTag, DatabaseTermEntry, DatabaseTermMeta,
    DictionaryDatabase, DictionaryDatabaseError, GenericQueryRequest, QueryRequestError,
    QueryRequestMatchType, QueryType, TermExactQueryRequest,
};
pub use dictionary_importer::DictionarySummary;

pub trait DictionaryService: Send + Sync {
    fn get_settings(&self) -> Result<Option<Vec<u8>>, Box<DictionaryDatabaseError>>;
    fn get_dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>>;
    fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, Box<DictionaryDatabaseError>>;
    fn find_term_meta_bulk(
        &self,
        keys: &indexmap::IndexSet<String>,
        enabled_dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
    ) -> Result<Vec<DatabaseTermMeta>, Box<DictionaryDatabaseError>>;
    fn find_terms_exact_bulk(
        &self,
        terms: &[TermExactQueryRequest],
        enabled_dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;
    fn find_terms_by_sequence_bulk(
        &self,
        queries: Vec<GenericQueryRequest>,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;
    fn find_terms_bulk(
        &self,
        term_list: &[String],
        dictionaries: &dyn crate::database::dictionary_database::DictionarySet,
        match_type: yomichan_importer::dictionary_database::TermSourceMatchType,
    ) -> Result<Vec<yomichan_importer::dictionary_database::TermEntry>, Box<DictionaryDatabaseError>>;
}
