use crate::database::dictionary_database::{
    DatabaseTag, DatabaseTermMeta, DictionaryDatabase, GenericQueryRequest,
};
use crate::errors::DBError;
use crate::translation::TermEnabledDictionaryMap;
use importer::dictionary_database::{TermEntry, TermSourceMatchType};

impl crate::database::DictionaryService for DictionaryDatabase<'_> {
    fn find_tag_meta_bulk(
        &self,
        queries: &[GenericQueryRequest],
    ) -> Result<Vec<Option<DatabaseTag>>, DBError> {
        self.find_tag_meta_bulk(queries)
            .map_err(|e| DBError::DictionaryDatabase(e))
    }

    fn find_term_meta_bulk(
        &self,
        term_list: &indexmap::IndexSet<String>,
        enabled_dictionaries: &TermEnabledDictionaryMap,
    ) -> Result<Vec<DatabaseTermMeta>, DBError> {
        self.find_term_meta_bulk(term_list, enabled_dictionaries)
            .map_err(|e| DBError::DictionaryDatabase(e))
    }

    fn find_terms_exact_bulk(
        &self,
        terms: &[crate::database::dictionary_database::TermExactQueryRequest],
        enabled_dictionaries: &TermEnabledDictionaryMap,
    ) -> Result<Vec<TermEntry>, DBError> {
        self.find_terms_exact_bulk(terms, enabled_dictionaries)
            .map_err(|e| DBError::DictionaryDatabase(e))
    }

    fn find_terms_by_sequence_bulk(
        &self,
        queries: Vec<GenericQueryRequest>,
    ) -> Result<Vec<TermEntry>, DBError> {
        self.find_terms_by_sequence_bulk(queries)
            .map_err(|e| DBError::DictionaryDatabase(e))
    }

    fn find_terms_bulk(
        &self,
        term_list_input: &[String],
        dictionaries: &TermEnabledDictionaryMap,
        match_type: TermSourceMatchType,
    ) -> Result<Vec<TermEntry>, DBError> {
        self.find_terms_bulk(term_list_input, dictionaries, match_type)
            .map_err(|e| DBError::DictionaryDatabase(e))
    }
}
