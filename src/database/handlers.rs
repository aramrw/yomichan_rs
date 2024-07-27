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

    /// Queries terms via a query.
    /// Matches the query to both an `expression` and `reading`.
    ///
    /// _Does not_ tokenize the query;
    /// meaning sentences, or queries containing particles and grammar
    /// should not be passed.
    ///
    /// If you need to lookup a sentence, see: [`bulk_lookup_tokens`]
    pub fn bulk_lookup_term<Q: AsRef<str> + Debug>(
        &self,
        queries: Queries<Q>,
    ) -> Result<VecTermEntry, DBError> {
        let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        let rtx = db.r_transaction()?;

        let (mut exps, readings) = handle_term_query(&queries, &rtx)?;

        if exps.is_empty() && readings.is_empty() {
            return Err(DBError::NoneFound(format!(
                "no entries found for: {:?}",
                queries
            )));
        }

        let exp_terms = construct_term_entries(
            exps,
            TermSourceMatchSource::Term,
            TermSourceMatchType::Exact,
        )?;

        let reading_terms = construct_term_entries(
            readings,
            TermSourceMatchSource::Reading,
            TermSourceMatchType::Exact,
        )?;

        let seqs: HashSet<&i128> = exp_terms
            .iter()
            .filter_map(|e| e.sequence.as_ref())
            .chain(reading_terms.iter().filter_map(|e| e.sequence.as_ref()))
            .collect();

        let seqs = self.lookup_seqs(seqs, Some(&db))?;
        let seq_terms = construct_term_entries(
            seqs,
            TermSourceMatchSource::Sequence,
            TermSourceMatchType::Exact,
        )?;

        let terms: VecTermEntry = exp_terms
            .into_iter()
            .chain(reading_terms)
            .chain(seq_terms)
            .collect();

        let mut ids: HashSet<String> = HashSet::new();

        let terms = terms
            .into_iter()
            .filter(|t| ids.insert(t.id.clone()))
            .collect::<VecTermEntry>();

        Ok(terms)
    }

