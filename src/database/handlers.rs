// use crate::dictionary::{TermSourceMatchSource, TermSourceMatchType};
// use crate::dictionary_data::{
//     DictionaryDataTag, GenericFreqData, TermGlossary, TermGlossaryContent, TermMetaDataMatchType,
//     TermMetaFreqDataMatchType, TermMetaFrequency, TermMetaModeType, TermMetaPhoneticData,
//     TermMetaPitch, TermMetaPitchData,
// };
// use indexmap::IndexSet;
// use pretty_assertions::assert_eq;
//
// use crate::database::dictionary_importer::{prepare_dictionary, Summary, TermMetaBank};
// use crate::dictionary_data::KANA_MAP;
// use crate::errors::{DBError, ImportError};
// use crate::settings::{DictionaryOptions, Options, Profile};
// use crate::Yomichan;
//
// //use lindera::{LinderaError, Token, Tokenizer};
//
// use db_type::{KeyOptions, ToKeyDefinition};
// use native_db::{transaction::query::PrimaryScan, Builder as DBBuilder, *};
// use native_model::{native_model, Model};
//
// use rayon::collections::hash_set;
// use serde::{Deserialize, Serialize};
// use serde_json::Deserializer as JsonDeserializer;
//
// use transaction::RTransaction;
// //use unicode_segmentation::{Graphemes, UnicodeSegmentation};
// use uuid::Uuid;
//
// use std::ffi::OsString;
// use std::fmt::{Debug, Display};
// use std::hash::Hash;
// use std::io::BufReader;
// use std::path::{Path, PathBuf};
// use std::{fs, marker, thread};
//
// use super::dictionary_database::{
//     DBMetaType, DatabaseMeta, DatabaseMetaFrequency, DatabaseMetaFrequencyKey,
//     DatabaseMetaPhoneticKey, DatabaseMetaPitchKey, DatabaseTermEntry, DatabaseTermEntryKey,
//     HasExpression, Queries, TermEntry, VecDBMetaFreq, VecDBTermEntry, VecDBTermMeta, VecTermEntry,
//     DB_MODELS,
// };
//
// impl Yomichan {
//     /// Queries multiple terms via text.
//     /// Matches the query to either an `expression` or a `reading`.
//     ///
//     /// `exact` functions do not tokenize the query;
//     /// meaning sentences or queries containing particles and grammar
//     /// should not be passed.
//     ///
//     /// if you need to lookup a sentence, see: [`Self::lookup_tokens`]
//     pub fn lookup_exact<Q: AsRef<str> + Debug>(&self, query: Q) -> Result<VecDBTermEntry, DBError> {
//         let query = query.as_ref();
//         //let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
//         let db = &self.db;
//         let rtx = db.r_transaction()?;
//
//         #[cfg(feature = "rayon")]
//         let (mut exps, readings) = {
//             let mut exps: VecDBTermEntry = Vec::new();
//             let mut readings: VecDBTermEntry = Vec::new();
//             rayon::join(
//                 || exps = query_sw(&rtx, DatabaseTermEntryKey::expression, query).unwrap(),
//                 || readings = query_sw(&rtx, DatabaseTermEntryKey::reading, query).unwrap(),
//             );
//             (exps, readings)
//         };
//
//         #[cfg(not(feature = "rayon"))]
//         let mut exps = query_sw(&rtx, DatabaseTermEntryKey::expression, query).unwrap();
//         let readings = query_sw(&rtx, DatabaseTermEntryKey::reading, query).unwrap();
//
//         if exps.is_empty() && readings.is_empty() {
//             return Err(DBError::Query(format!("no entries found for: {query}")));
//         }
//
//         exps.extend(readings);
//         Ok(exps)
//     }
//
//     /// Queries terms via a query.
//     /// Matches the query to both an `expression` and `reading`.
//     ///
//     /// _Does not_ tokenize the query;
//     /// meaning sentences, or queries containing particles and grammar
//     /// should not be passed.
//     ///
//     /// If you need to lookup a sentence, see: [`bulk_lookup_tokens`]
//     pub fn bulk_lookup_term<Q: AsRef<str> + Debug>(
//         &self,
//         queries: Queries<Q>,
//     ) -> Result<VecTermEntry, DBError> {
//         //let db = DBBuilder::new().open(&DB_MODELS, &self.db_path)?;
//         let db = &self.db;
//         let rtx = db.r_transaction()?;
//
//         let (exps, readings) = handle_term_query(&queries, &rtx)?;
//
//         if exps.is_empty() && readings.is_empty() {
//             return Err(DBError::NoneFound(format!(
//                 "no entries found for: {queries:?}"
//             )));
//         }
//
//         let exp_terms = construct_term_entries(
//             exps,
//             TermSourceMatchSource::Term,
//             TermSourceMatchType::Exact,
//         )?;
//
//         let reading_terms = construct_term_entries(
//             readings,
//             TermSourceMatchSource::Reading,
//             TermSourceMatchType::Exact,
//         )?;
//
//         let seqs: IndexSet<i128> = exp_terms
//             .iter()
//             .filter_map(|e| Some(e.sequence))
//             .chain(reading_terms.iter().filter_map(|e| Some(e.sequence)))
//             .collect();
//
//         let seqs = self.lookup_seqs(seqs)?;
//         let seq_terms = construct_term_entries(
//             seqs,
//             TermSourceMatchSource::Sequence,
//             TermSourceMatchType::Exact,
//         )?;
//
//         let terms: VecTermEntry = exp_terms
//             .into_iter()
//             .chain(reading_terms)
//             .chain(seq_terms)
//             .collect();
//
//         let mut ids: IndexSet<String> = IndexSet::new();
//
//         let terms = terms
//             .into_iter()
//             .filter(|t| ids.insert(t.id.clone()))
//             .collect::<VecTermEntry>();
//
//         Ok(terms)
//     }
//
//     /// Queries terms via their sequences.
//     ///
//     /// Unless there is a specific use case, its generally better
//     /// to do lookup terms via `reading` or an `expression` using
//     /// [`Self::bulk_lookup`] or [`Self::bulk_lookup_tokens`].
//     /// Both of these functions use [`Self::lookup_seqs`] under the hood.
//     ///
//     /// # Example
//     ///
//     /// ```
//     /// let ycd = Yomichan::new("./db").unwrap();
//     ///
//     ///
//     /// // 504000000 matches all ありがとう terms in 大辞林第四版.
//     /// let terms = ycd.lookup_seqs(&[504000000], None).unwrap();
//     ///
//     /// for t in terms {
//     ///     assert!(t.reading == ありがとう);
//     /// }
//     /// ```
//     pub fn lookup_seqs<I>(&self, seqs: I) -> Result<VecDBTermEntry, DBError>
//     where
//         I: IntoIterator<Item = i128> + Debug + Clone,
//     {
//         let db = &self.db;
//         let rtx = db.r_transaction()?;
//
//         let entries: Result<VecDBTermEntry, DBError> = seqs
//             .clone()
//             .into_iter()
//             .map(|seq| query_sw(&rtx, DatabaseTermEntryKey::sequence, seq))
//             .flat_map(|result| match result {
//                 Ok(vec) => vec.into_iter().map(Ok).collect(),
//                 Err(err) => vec![Err(err)],
//             })
//             .collect();
//
//         let entries = entries?;
//
//         if entries.is_empty() {
//             return Err(DBError::NoneFound(format!(
//                 "no entries sequences matched for: {:?}",
//                 &seqs
//             )));
//         }
//
//         Ok(entries)
//     }
//
//     pub fn lookup_seq(&self, seq: i128) -> Result<VecDBTermEntry, DBError> {
//         let db = &self.db;
//         let rtx = db.r_transaction()?;
//
//         let entries: VecDBTermEntry = query_sw(&rtx, DatabaseTermEntryKey::expression, seq)?;
//
//         if entries.is_empty() {
//             return Err(DBError::Query(format!("no entries found for: {seq:?}")));
//         }
//
//         Ok(entries)
//     }
// }
//
// fn construct_term_entries(
//     entries: Vec<DatabaseTermEntry>,
//     match_source: TermSourceMatchSource,
//     match_type: TermSourceMatchType,
// ) -> Result<Vec<TermEntry>, DBError> {
//     let mut grouped_indices: Vec<(usize, usize)> = Vec::new();
//
//     for (i, entry) in entries.iter().enumerate() {
//         let mut found = false;
//         for (mut idx, (term, reading)) in grouped_indices.iter_mut().enumerate() {
//             if entry.expression == entries[*term].expression
//                 && entry.reading == entries[*reading].reading
//             {
//                 found = true;
//                 idx = i;
//                 break;
//             }
//         }
//         if !found {
//             grouped_indices.push((i, i));
//         }
//     }
//
//     let result: Vec<TermEntry> = grouped_indices
//         .iter()
//         .enumerate()
//         .map(|(i, (start, end))| {
//             let mut term_tags: IndexSet<String> = IndexSet::new();
//             let mut rules: IndexSet<String> = IndexSet::new();
//             let mut definitions: Vec<TermGlossary> = Vec::new();
//
//             entries
//                 .iter()
//                 .take(*end + 1)
//                 .skip(*start)
//                 .for_each(|entry| {
//                     if let Some(t_tags) = entry.term_tags.clone() {
//                         term_tags.insert(t_tags);
//                     }
//                     rules.insert(entry.rules.clone());
//                     definitions.extend(entry.glossary.clone());
//                 });
//
//             let current = entries[i].clone();
//             current.into_term_entry_specific(match_source, match_type, i)
//         })
//         .collect();
//     Ok(result)
// }
//
// fn handle_term_query<Q: AsRef<str> + Debug>(
//     queries: &Queries<Q>,
//     rtx: &RTransaction,
// ) -> Result<(VecDBTermEntry, VecDBTermEntry), DBError> {
//     let (exps, readings): (VecDBTermEntry, VecDBTermEntry) = match queries {
//         Queries::Exact(queries) => {
//             let exps = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if !is_kana(q.as_ref()) {
//                         return Some(query_exact(
//                             rtx,
//                             DatabaseTermEntryKey::expression,
//                             q.as_ref(),
//                         ));
//                     }
//                     None
//                 })
//                 .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBTermEntry>();
//             let readings = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if is_kana(q.as_ref()) {
//                         return Some(query_exact(rtx, DatabaseTermEntryKey::reading, q.as_ref()));
//                     }
//                     None
//                 })
//                 .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBTermEntry>();
//
//             (exps, readings)
//         }
//         Queries::StartsWith(queries) => {
//             let exps = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if !is_kana(q.as_ref()) {
//                         return Some(query_sw(rtx, DatabaseTermEntryKey::expression, q.as_ref()));
//                     }
//                     None
//                 })
//                 .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBTermEntry>();
//             let readings = queries
//                 .iter()
//                 .filter_map(|q| {
//                     if is_kana(q.as_ref()) {
//                         return Some(query_sw(rtx, DatabaseTermEntryKey::reading, q.as_ref()));
//                     }
//                     None
//                 })
//                 .collect::<Result<Vec<VecDBTermEntry>, DBError>>()?
//                 .into_iter()
//                 .flatten()
//                 .collect::<VecDBTermEntry>();
//
//             (exps, readings)
//         }
//     };
//     Ok((exps, readings))
// }
//
// fn query_sw<'a, R: native_db::ToInput>(
//     rtx: &RTransaction,
//     key: impl ToKeyDefinition<KeyOptions>,
//     query: impl ToKey + 'a,
// ) -> Result<Vec<R>, DBError> {
//     Ok(rtx
//         .scan()
//         .secondary(key)?
//         .start_with(query)?
//         .collect::<Result<Vec<R>, db_type::Error>>()?)
// }
//
// fn query_exact<R>(
//     rtx: &RTransaction,
//     key: impl ToKeyDefinition<KeyOptions>,
//     query: &str,
// ) -> Result<Vec<R>, DBError>
// where
//     R: HasExpression + native_db::ToInput,
// {
//     let results: Result<Vec<R>, db_type::Error> = rtx
//         .scan()
//         .secondary(key)?
//         .start_with(query)?
//         .take_while(|e: &Result<R, db_type::Error>| match e {
//             Ok(entry) => entry.expression() == query,
//             Err(_) => false,
//         })
//         .collect();
//
//     Ok(results?)
// }
//
// pub fn is_kana(str: &str) -> bool {
//     for c in str.chars() {
//         let mut tmp = [0u8; 4];
//         let char = c.encode_utf8(&mut tmp);
//         if KANA_MAP.get_by_left(char).is_none() && KANA_MAP.get_by_right(char).is_none() {
//             return false;
//         }
//     }
//     true
// }
//
// fn query_all_freq_meta(
//     query: &str,
//     rtx: &RTransaction,
// ) -> Result<Vec<DatabaseMetaFrequency>, DBError> {
//     let results: Result<Vec<DatabaseMetaFrequency>, db_type::Error> = rtx
//         .scan()
//         .primary()?
//         .all()?
//         .filter_map(|e: Result<DatabaseMetaFrequency, db_type::Error>| match e {
//             Ok(entry) => {
//                 if entry.mode == TermMetaModeType::Freq {
//                     if query == entry.expression {
//                         return Some(Ok(entry));
//                     }
//
//                     match &entry.data {
//                         TermMetaFreqDataMatchType::Generic(gd) => {
//                             if gd.try_get_reading().is_some_and(|r| query == r) {
//                                 return Some(Ok(entry));
//                             }
//                         }
//                         TermMetaFreqDataMatchType::WithReading(wr) => {
//                             if query == wr.reading {
//                                 return Some(Ok(entry));
//                             }
//
//                             if wr.frequency.try_get_reading().is_some_and(|r| query == r) {
//                                 return Some(Ok(entry));
//                             }
//                         }
//                     }
//                 }
//                 None
//             }
//             Err(err) => Some(Err(err)),
//         })
//         .collect();
//
//     Ok(results?)
// }
//
// fn handle_meta_freq_query<Q: AsRef<str> + Debug>(
//     queries: &Queries<Q>,
//     rtx: &RTransaction,
// ) -> Result<VecDBMetaFreq, Box<DBError>> {
//     let qs = match queries {
//         Queries::Exact(qs) => qs,
//         Queries::StartsWith(qs) => qs,
//     };
//     let exps = qs
//         .iter()
//         .map(|q| match queries {
//             Queries::Exact(_) => query_exact::<DatabaseMetaFrequency>(
//                 rtx,
//                 DatabaseMetaFrequencyKey::expression,
//                 q.as_ref(),
//             ),
//             Queries::StartsWith(_) => query_sw::<DatabaseMetaFrequency>(
//                 rtx,
//                 DatabaseMetaFrequencyKey::expression,
//                 q.as_ref(),
//             ),
//         })
//         .collect::<Result<Vec<VecDBMetaFreq>, DBError>>()?
//         .into_iter()
//         .flatten()
//         .collect::<VecDBMetaFreq>();
//     Ok(exps)
// }
//
// #[cfg(test)]
// mod db_tests {
//     use crate::{
//         database::dictionary_database::{DatabaseMetaFrequency, Queries},
//         dictionary_data::{GenericFreqData, TermMetaFreqDataMatchType, TermMetaModeType},
//         yomichan_test_utils::{self, set_backtrace, BacktraceKind, TEST_PATHS},
//         Yomichan,
//     };
//     use pretty_assertions::assert_eq;
//     use tempfile::{tempdir, tempdir_in, tempfile, tempfile_in};
//
//     use super::handle_meta_freq_query;
//
//     #[cfg(test)]
//     mod jp_tests {
//         use crate::{
//             database::dictionary_database::{DatabaseMetaFrequency, Queries},
//             dictionary_data::{GenericFreqData, TermMetaFreqDataMatchType, TermMetaModeType},
//             yomichan_test_utils::{self, set_backtrace, BacktraceKind, TEST_PATHS},
//             Yomichan,
//         };
//         use pretty_assertions::assert_eq;
//         use tempfile::{tempdir, tempdir_in, tempfile, tempfile_in};
//
//         use super::handle_meta_freq_query;
//         #[test]
//         fn lookup_exact() {
//             let (f_path, handle) = yomichan_test_utils::copy_test_db();
//             let ycd = Yomichan::new(&f_path).unwrap();
//             let res = ycd.lookup_exact("日本語").unwrap();
//             let first = res.first().unwrap();
//             assert_eq!(
//                 (&*first.expression, &*first.reading),
//                 ("日本語", "にほんご")
//             );
//             dbg!(first);
//         }
//
//         #[test]
//         fn bulk_lookup_term() {
//             let (f_path, handle) = yomichan_test_utils::copy_test_db();
//             let ycd = Yomichan::new(&f_path).unwrap();
//             let res = ycd.bulk_lookup_term(Queries::Exact(&["日本語"])).unwrap();
//             let first = res.first().unwrap();
//             assert_eq!((&*first.term, &*first.reading), ("日本語", "にほんご"))
//         }
//
//         #[test]
//         fn h_meta_freq_query() {
//             set_backtrace(BacktraceKind::One);
//             let (f_path, handle) = yomichan_test_utils::copy_test_db();
//             let queries = Queries::Exact(&["日本語"]);
//             let ycd =
//                 Yomichan::new(&*yomichan_test_utils::TEST_PATHS.tests_yomichan_db_path).unwrap();
//             let rtx = ycd.db.r_transaction().unwrap();
//             let mut res = handle_meta_freq_query(&queries, &rtx).unwrap();
//             res[0].id = "test_id".to_string();
//             assert_eq!(
//                 res[0],
//                 DatabaseMetaFrequency {
//                     id: "test_id".into(),
//                     expression: "日本語".into(),
//                     mode: TermMetaModeType::Freq,
//                     data: TermMetaFreqDataMatchType::Generic(GenericFreqData::Integer(4887)),
//                     dictionary: "Anime & J-drama".into(),
//                 }
//             );
//         }
//     }
//
//     #[test]
//     #[ignore]
//     /// Initializes the repo's yomichan database with specified dicts.
//     fn init_db() {
//         let td = &*yomichan_test_utils::TEST_PATHS.tests_dir;
//         let tdcs = &*yomichan_test_utils::TEST_PATHS.test_dicts_dir;
//         let mut ycd = Yomichan::new(td).unwrap();
//         let paths = [tdcs.join("daijirin"), tdcs.join("ajdfreq")];
//         match ycd.import_dictionaries(&paths) {
//             Ok(_) => {}
//             Err(e) => {
//                 let db_path = td.join("db.ycd");
//                 if db_path.exists() && db_path.is_file() {
//                     if let Some(ext) = db_path.extension() {
//                         if ext == "ycd" {
//                             std::fs::remove_file(db_path).unwrap();
//                         }
//                     }
//                 }
//                 panic!("failed init_db test: {e}");
//             }
//         }
//     }
//
//     #[test]
//     #[ignore]
//     /// only debug print
//     fn __lookup__() {
//         //let (f_path, handle) = yomichan_test_utils::copy_test_db();
//         let f_path = &yomichan_test_utils::TEST_PATHS.tests_yomichan_db_path;
//         let start = std::time::Instant::now();
//         let ycd = Yomichan::new(f_path).unwrap();
//         let res = ycd.lookup_exact("有り付く").unwrap();
//         yomichan_test_utils::print_timer(start, "");
//     }
// }
