use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
use crate::dictionary_data::{
    GenericFreqData, Tag as DictDataTag, TermGlossary, TermGlossaryContent, TermMetaDataMatchType,
    TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPhoneticData,
    TermMetaPitch, TermMetaPitchData,
};
use pretty_assertions::assert_eq;

use crate::database::dictionary_importer::{prepare_dictionary, Summary, TermMetaBank};
use crate::dictionary_data::KANA_MAP;
use crate::errors::{DBError, ImportError};
use crate::settings::{DictionaryOptions, Options, Profile};
use crate::Yomichan;

use bincode::Error;

//use lindera::{LinderaError, Token, Tokenizer};

use db_type::{KeyOptions, ToKeyDefinition};
use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
use native_model::{native_model, Model};

use rayon::collections::hash_set;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer as JsonDeserializer;

use transaction::RTransaction;
//use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use uuid::Uuid;

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::{fs, marker};

use super::dictionary_database::{
    DBMetaType, DatabaseMeta, DatabaseMetaFrequency, DatabaseMetaFrequencyKey,
    DatabaseMetaPhoneticKey, DatabaseMetaPitchKey, DatabaseTermEntry, DatabaseTermEntryKey,
    HasExpression, Queries, TermEntry, VecDBMetaFreq, VecDBTermEntry, VecDBTermMeta, VecTermEntry,
    DB_MODELS,
};

impl Yomichan {
    /// Queries multiple terms via text.
    /// Matches the query to either an `expression` or a `reading`.
    ///
    /// `exact` functions do not tokenize the query;
    /// meaning sentences or queries containing particles and grammar
    /// should not be passed.
    ///
    /// if you need to lookup a sentence, see: [`Self::lookup_tokens`]
    pub fn lookup_exact<Q: AsRef<str> + Debug>(&self, query: Q) -> Result<VecDBTermEntry, DBError> {
        //let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        let db = &self.db;
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
        //let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
        let db = &self.db;
        let rtx = db.r_transaction()?;

        let (exps, readings) = handle_term_query(&queries, &rtx)?;

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

fn construct_term_entries(
    entries: Vec<DatabaseTermEntry>,
    match_source: TermSourceMatchSource,
    match_type: TermSourceMatchType,
) -> Result<Vec<TermEntry>, DBError> {
    let mut grouped_indices: Vec<(usize, usize)> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let mut found = false;
        for (mut idx, (term, reading)) in grouped_indices.iter_mut().enumerate() {
            if entry.expression == entries[*term].expression
                && entry.reading == entries[*reading].reading
            {
                found = true;
                idx = i;
                break;
            }
        }
        if !found {
            grouped_indices.push((i, i));
        }
    }

    let result: Vec<TermEntry> = grouped_indices
        .iter()
        .enumerate()
        .map(|(i, (start, end))| {
            let mut term_tags: HashSet<String> = HashSet::new();
            let mut rules: HashSet<String> = HashSet::new();
            let mut definitions: Vec<TermGlossary> = Vec::new();

            entries
                .iter()
                .take(*end + 1)
                .skip(*start)
                .for_each(|entry| {
                    if let Some(t_tags) = entry.term_tags.clone() {
                        term_tags.insert(t_tags);
                    }
                    rules.insert(entry.rules.clone());
                    definitions.push(entry.glossary.clone());
                });

            TermEntry {
                id: Uuid::new_v4().to_string(),
                index: u32::try_from(i).unwrap_or_default(),
                term: entries[i].expression.clone(),
                reading: entries[i].reading.clone(),
                match_type: match_type.clone(),
                match_source: match_source.clone(),
                definition_tags: entries[i].definition_tags.clone(),
                term_tags: Some(term_tags.into_iter().collect()),
                rules: rules.into_iter().collect(),
                definitions,
                score: entries[i].score,
                dictionary: entries[i].dictionary.clone(),
                sequence: entries[i].sequence,
            }
        })
        .collect();
    Ok(result)
}

fn handle_term_query<Q: AsRef<str> + Debug>(
    queries: &Queries<Q>,
    rtx: &RTransaction,
) -> Result<(VecDBTermEntry, VecDBTermEntry), DBError> {
    let (exps, readings): (VecDBTermEntry, VecDBTermEntry) = match queries {
        Queries::Exact(queries) => {
            let exps = queries
                .iter()
                .filter_map(|q| {
                    if !is_kana(q.as_ref()) {
                        return Some(query_exact(
                            rtx,
                            DatabaseTermEntryKey::expression,
                            q.as_ref(),
                        ));
                    }
                    None
                })
                .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
                .into_iter()
                .flatten()
                .collect::<VecDBTermEntry>();
            let readings = queries
                .iter()
                .filter_map(|q| {
                    if is_kana(q.as_ref()) {
                        return Some(query_exact(rtx, DatabaseTermEntryKey::reading, q.as_ref()));
                    }
                    None
                })
                .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
                .into_iter()
                .flatten()
                .collect::<VecDBTermEntry>();

            (exps, readings)
        }
        Queries::StartWith(queries) => {
            let exps = queries
                .iter()
                .filter_map(|q| {
                    if !is_kana(q.as_ref()) {
                        return Some(query_sw(rtx, DatabaseTermEntryKey::expression, q.as_ref()));
                    }
                    None
                })
                .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
                .into_iter()
                .flatten()
                .collect::<VecDBTermEntry>();
            let readings = queries
                .iter()
                .filter_map(|q| {
                    if is_kana(q.as_ref()) {
                        return Some(query_sw(rtx, DatabaseTermEntryKey::reading, q.as_ref()));
                    }
                    None
                })
                .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
                .into_iter()
                .flatten()
                .collect::<VecDBTermEntry>();

            (exps, readings)
        }
    };
    Ok((exps, readings))
}

fn query_sw<'a, R: native_db::ToInput>(
    rtx: &RTransaction,
    key: impl ToKeyDefinition<KeyOptions>,
    query: impl ToKey + 'a,
) -> Result<Vec<R>, DBError> {
    Ok(rtx
        .scan()
        .secondary(key)?
        .start_with(query)
        .collect::<Result<Vec<R>, db_type::Error>>()?)
}

fn query_exact<R>(
    rtx: &RTransaction,
    key: impl ToKeyDefinition<KeyOptions>,
    query: &str,
) -> Result<Vec<R>, DBError>
where
    R: HasExpression + native_db::ToInput,
{
    let results: Result<Vec<R>, db_type::Error> = rtx
        .scan()
        .secondary(key)?
        .start_with(query)
        .take_while(|e: &Result<R, db_type::Error>| match e {
            Ok(entry) => entry.expression() == query,
            Err(_) => false,
        })
        .collect();

    Ok(results?)
}

pub fn is_kana(str: &str) -> bool {
    for c in str.chars() {
        let mut tmp = [0u8; 4];
        let char = c.encode_utf8(&mut tmp);
        if KANA_MAP.get_by_left(char).is_none() && KANA_MAP.get_by_right(char).is_none() {
            return false;
        }
    }

    true
}

fn query_all_freq_meta(
    query: &str,
    rtx: &RTransaction,
) -> Result<Vec<DatabaseMetaFrequency>, DBError> {
    let results: Result<Vec<DatabaseMetaFrequency>, db_type::Error> = rtx
        .scan()
        .primary()?
        .all()
        .filter_map(|e: Result<DatabaseMetaFrequency, db_type::Error>| match e {
            Ok(entry) => {
                if entry.mode == TermMetaModeType::Freq {
                    if query == entry.expression {
                        return Some(Ok(entry));
                    }

                    match &entry.data {
                        TermMetaFreqDataMatchType::Generic(gd) => {
                            if gd.try_get_reading().is_some_and(|r| query == r) {
                                return Some(Ok(entry));
                            }
                        }
                        TermMetaFreqDataMatchType::WithReading(wr) => {
                            if query == wr.reading {
                                return Some(Ok(entry));
                            }

                            if wr.frequency.try_get_reading().is_some_and(|r| query == r) {
                                return Some(Ok(entry));
                            }
                        }
                    }
                }
                None
            }
            Err(err) => Some(Err(err)),
        })
        .collect();

    Ok(results?)
}

#[cfg(test)]
mod db_tests {
    use crate::{database::dictionary_database::Queries, Yomichan};
    use pretty_assertions::assert_eq;

    #[test]
    fn lookup_exact() {
        let ycd = Yomichan::new("./a/yomichan/data.yc").unwrap();
        let res = ycd.lookup_exact("日本語").unwrap();
        let first = res.first().unwrap();
        assert_eq!(
            ("日本語", "にほんご"),
            (&*first.expression, &*first.reading)
        )
    }

    #[test]
    fn bulk_lookup_term() {
        let ycd = Yomichan::new("./a/yomichan/data.yc").unwrap();
        let res = ycd.bulk_lookup_term(Queries::Exact(&["日本語"])).unwrap();
        let first = res.first().unwrap();
        assert_eq!(("日本語", "にほんご"), (&*first.term, &*first.reading))
    }
}

// fn handle_meta_freq_query<Q: AsRef<str> + Debug>(
//     queries: &Queries<Q>,
//     rtx: &RTransaction,
// ) -> Result<(VecDBMetaFreq, VecDBMetaFreq), DBError> {
//     match queries {
//         Queries::Exact(queries) => {
//             let exps = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if !is_kana(q.as_ref()) {
//                         Some(query_exact::<DatabaseMetaFrequency>(
//                             rtx,
//                             DatabaseMetaFrequencyKey::expression,
//                             q.as_ref(),
//                         ))
//                     } else {
//                         None
//                     }
//                 })
//                 .collect::<Result<Vec<VecDBMetaFreq>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBMetaFreq>();
//
//             Ok((exps, readings))
//         }
//         Queries::StartWith(queries) => {
//             let exps = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if !is_kana(q.as_ref()) {
//                         Some(query_exact::<DatabaseMetaFrequency>(
//                             rtx,
//                             DatabaseMetaFrequencyKey::expression,
//                             q.as_ref(),
//                         ))
//                     } else {
//                         None
//                     }
//                 })
//                 .collect::<Result<Vec<VecDBMetaFreq>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBMetaFreq>();
//             Ok((exps, readings))
//         }
//     }
// }

// fn handle_meta_query<Q: AsRef<str> + Debug>(
//     queries: &Queries<Q>,
//     rtx: &RTransaction,
// ) -> Result<(VecDBTermMeta, VecDBTermMeta), DBError> {
//     let (exps, readings): (VecDBTermMeta, VecDBTermMeta) = match queries {
//         Queries::Exact(queries) => {
//             let exps = queries.iter().filter_map(|q| {
//             if !is_kana(q.as_ref()) {
//                 let frequency = query_exact(rtx, DatabaseMetaFrequencyKey::expression, q.as_ref());
//                 let pitch = query_exact(rtx, DatabaseMetaPitchKey::expression, q.as_ref());
//                 let phonetic = query_exact(rtx, DatabaseMetaPhoneticKey::expression, q.as_ref());
//
//                 return Some(DatabaseMeta {
//                         frequency,
//                         pitch,
//                         phonetic
//                     });
//             }
//             None
//         }).collect();
//
//         }
//         Queries::StartWith(queries) => {
//     (exps, readings),
// }
//     };
//     Ok((exps, readings))
// }
