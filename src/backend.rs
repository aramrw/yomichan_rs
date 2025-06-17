use std::{cmp::Ordering, collections::VecDeque, path::Path, sync::Arc};

use anki_direct::AnkiClient;
use fancy_regex::Regex;
use indexmap::{IndexMap, IndexSet};
use language_transformer::{
    ja::japanese::{distribute_furigana_inflected, is_code_point_japanese, FuriganaSegment},
    language_d::FindTermsTextReplacement,
};
use native_db::transaction::RwTransaction;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        dictionary_database::{DictionaryDatabase, DictionaryDatabaseError},
        dictionary_importer::DictionarySummary,
    },
    dictionary::{TermDictionaryEntry, TermSource, TermSourceMatchSource, TermSourceMatchType},
    environment::{EnvironmentInfo, CACHED_ENVIRONMENT_INFO},
    settings::{
        DictionaryOptions, GeneralOptions, Options, ProfileOptions, ScanningOptions,
        SearchResolution, TranslationOptions, TranslationTextReplacementGroup,
        TranslationTextReplacementOptions,
    },
    text_scanner::{TermSearchResults, TextScanner},
    translation::{
        FindTermDictionary, FindTermsMatchType, FindTermsOptions, TermEnabledDictionaryMap,
    },
    translator::{EnabledDictionaryMapType, FindTermsMode, FindTermsResult, Translator},
    Yomichan,
};

pub struct Backend<'a> {
    pub environment: EnvironmentInfo,
    pub anki: AnkiClient,
    pub text_scanner: TextScanner<'a>,
    pub db: Arc<DictionaryDatabase<'a>>,
    pub options: Options,
}

impl<'a> Backend<'a> {
    pub fn new(db: Arc<DictionaryDatabase<'a>>) -> Result<Self, Box<native_db::db_type::Error>> {
        let rtx = db.r_transaction()?;
        let opts: Option<Options> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => Options::new(),
        };
        let backend = Self {
            environment: EnvironmentInfo::default(),
            anki: AnkiClient::default(),
            text_scanner: TextScanner::new(db.clone()),
            db,
            options,
        };
        Ok(backend)
    }
}

impl<'a> Backend<'a> {
    /// The internal impl to write global options to the database.
    /// This takes a [Option<RwTransaction>] so rwtx's can be reused if necessary.
    fn _update_options_internal(
        &self,
        rwtx: Option<RwTransaction>,
    ) -> Result<(), Box<native_db::db_type::Error>> {
        let rwtx = match rwtx {
            Some(rwtx) => Ok(rwtx),
            None => self.db.rw_transaction(),
        }?;
        rwtx.insert(self.options.clone())?;
        Ok(())
    }
}

impl Yomichan<'_> {
    /// Sets the current profile's main language.
    ///
    /// Only updates the language in Yomichan's memory (ie. does not persist);
    /// To save the set language to the db, call [Self::update_options] after.
    pub fn set_language(&mut self, language_iso: &str) {
        self.backend
            .options
            .get_current_profile_mut()
            .options
            .general
            .language = language_iso.to_string();
    }

    // The user facing api to update their options.
    /// Saves global options for all profiles to the database;
    /// Meant to be called after you mutate a profile (ie. via [Self::get_current_profile_mut])
    pub fn update_options(&self) -> Result<(), Box<native_db::db_type::Error>> {
        self.backend._update_options_internal(None);
        Ok(())
    }

    /// Deletes dictionaries from the database and options by name.
    /// This function automatically saves, so no need to call `update_options`.
    pub fn delete_dictionaries(
        &mut self,
        names: &[impl AsRef<str>],
    ) -> Result<(), Box<native_db::db_type::Error>> {
        let current_profile = self.backend.options.get_current_profile_mut();
        let dictionaries = &mut current_profile.options.dictionaries;
        for name in names {
            let name = name.as_ref();
            dictionaries.swap_remove(name);
        }
        self.update_options()?;

        Ok(())
    }

    /// Gets all dictionary summaries from the database.
    /// [DictionarySummary] is different from [DictionaryOptions]
    /// If you need [DictionaryOptions], use `dictionary_summaries`
    pub fn dictionary_summaries(
        &self,
    ) -> Result<Vec<DictionarySummary>, Box<DictionaryDatabaseError>> {
        self.db.get_dictionary_summaries()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FindTermsDetails {
    pub match_type: Option<FindTermsMatchType>,
    pub deinflect: Option<bool>,
    pub primary_reading: Option<String>,
}
impl Default for FindTermsDetails {
    fn default() -> Self {
        Self {
            match_type: Some(FindTermsMatchType::Exact),
            deinflect: Some(true),
            primary_reading: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParseTextSegment {
    text: String,
    reading: String,
}
impl From<FuriganaSegment> for ParseTextSegment {
    fn from(value: FuriganaSegment) -> Self {
        Self {
            text: value.text,
            reading: value.reading.unwrap_or_default(),
        }
    }
}
type ParseTextLine = Vec<ParseTextSegment>;

mod ycd_tests {
    use std::cell::RefCell;

    use crate::{
        database::dictionary_database::DatabaseMetaFrequency,
        dictionary_data::{GenericFreqData, TermMetaFreqDataMatchType, TermMetaModeType},
        test_utils::TEST_PATHS,
        Yomichan,
    };

    use super::{Backend, FindTermsDetails};

    #[test]
    fn rmp_serde() {
        use rmp_serde::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};

        let db_meta_frequency = DatabaseMetaFrequency {
            id: "01974e4d-fced-7e10-8a42-831546fbde45".to_string(),
            freq_expression: "自業自得".to_string(),
            mode: TermMetaModeType::Freq,
            data: TermMetaFreqDataMatchType::Generic(GenericFreqData::Integer(8455)),
            dictionary: "Anime & J-drama".to_string(),
        };

        // Serialize to MessagePack
        let mut buf = Vec::new();
        db_meta_frequency
            .serialize(&mut Serializer::new(&mut buf))
            .unwrap();

        // Deserialize from MessagePack
        let mut deserializer = Deserializer::new(&buf[..]);
        let deserialized: DatabaseMetaFrequency =
            Deserialize::deserialize(&mut deserializer).unwrap();

        assert_eq!(db_meta_frequency, deserialized);
        println!("Original: {db_meta_frequency:?}");
        println!("Deserialized: {deserialized:?}");
    }
}
