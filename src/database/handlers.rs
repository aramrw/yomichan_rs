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

    /// Queries terms via their sequences.
    ///
    /// Unless there is a specific use case, its generally better
    /// to do lookup terms via `reading` or an `expression` using
    /// [`Self::bulk_lookup`] or [`Self::bulk_lookup_tokens`].
    /// Both of these functions use [`Self::lookup_seqs`] under the hood.
    ///
    /// # Example
    ///
    /// ```
    /// let ycd = Yomichan::new("./db").unwrap();
    ///
    ///
    /// // 504000000 matches all ありがとう terms in 大辞林第四版.
    /// let terms = ycd.lookup_seqs(&[504000000], None).unwrap();
    ///
    /// for t in terms {
    ///     assert!(t.reading == ありがとう);
    /// }
    /// ```
    pub fn lookup_seqs<'a, I>(
        &self,
        seqs: I,
        db: Option<&Database>,
    ) -> Result<VecDBTermEntry, DBError>
    where
        I: IntoIterator<Item = &'a i128> + Debug + Clone,
    {
        let db = match db {
            Some(db) => db,
            None => &DBBuilder::new().open(&DB_MODELS, &self.db_path)?,
        };
        let rtx = db.r_transaction()?;

        let entries: Result<VecDBTermEntry, DBError> = seqs
            .clone()
            .into_iter()
            .map(|seq| query_sw(&rtx, DatabaseTermEntryKey::sequence, *seq))
            .flat_map(|result| match result {
                Ok(vec) => vec.into_iter().map(Ok).collect(),
                Err(err) => vec![Err(err)],
            })
            .collect();

        let entries = entries?;

        if entries.is_empty() {
            return Err(DBError::NoneFound(format!(
                "no entries sequences matched for: {:?}",
                &seqs
            )));
        }

        Ok(entries)
    }

    pub fn lookup_seq(&self, seq: i128) -> Result<VecDBTermEntry, DBError> {
        let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        let rtx = db.r_transaction()?;

        let entries: VecDBTermEntry =
            query_sw(&rtx, DatabaseTermEntryKey::expression, seq).map_err(DBError::from)?;

        if entries.is_empty() {
            return Err(DBError::Query(format!("no entries found for: {:?}", seq)));
        }

        Ok(entries)
    }
}

