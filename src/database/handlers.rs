use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFreqData, Tag as DictDataTag, TermGlossary, TermGlossaryContent, TermMetaDataMatchType,
    TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPhoneticData,
    TermMetaPitch, TermMetaPitchData,
};

    pub fn lookup_exact<Q: AsRef<str> + Debug>(&self, query: Q) -> Result<VecDBTermEntry, DBError> {
        let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        let rtx = db.r_transaction()?;

        let mut exps = query_sw(&rtx, DatabaseTermEntryKey::expression, query.as_ref())?;
        let readings = query_sw(&rtx, DatabaseTermEntryKey::reading, query.as_ref())?;

        if exps.is_empty() && readings.is_empty() {
            return Err(DBError::Query(format!(
                "no entries found for: {}",
                query.as_ref()
            )));
        }

        exps.extend(readings);

        Ok(exps)
    }

