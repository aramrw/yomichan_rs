use std::{cmp::Ordering, collections::VecDeque, path::Path, sync::Arc};

use anki_direct::AnkiClient;
use deinflector::{
    ja::japanese::{distribute_furigana_inflected, is_code_point_japanese, FuriganaSegment},
    language_d::FindTermsTextReplacement,
};
use fancy_regex::Regex;
use indexmap::{IndexMap, IndexSet};
use native_db::transaction::RwTransaction;
use serde::{Deserialize, Serialize};

#[cfg(feature = "anki")]
use crate::anki::DisplayAnki;
use crate::{
    database::{
        dictionary_database::{DictionaryDatabase, DictionaryDatabaseError},
        dictionary_importer::DictionarySummary,
    },
    dictionary::{TermDictionaryEntry, TermSource, TermSourceMatchSource, TermSourceMatchType},
    environment::{EnvironmentInfo, CACHED_ENVIRONMENT_INFO},
    errors::{DBError, YomichanError},
    settings::{
        DictionaryOptions, GeneralOptions, ProfileError, ProfileOptions, ProfileResult,
        ScanningOptions, SearchResolution, TranslationOptions, TranslationTextReplacementGroup,
        TranslationTextReplacementOptions, YomichanOptions,
    },
    text_scanner::{TermSearchResults, TextScanner},
    translation::{
        FindTermDictionary, FindTermsMatchType, FindTermsOptions, TermEnabledDictionaryMap,
    },
    translator::{EnabledDictionaryMapType, FindTermsMode, FindTermsResult, Translator},
    Ptr, Yomichan,
};

/// `yomichan_rs` private engine
pub struct Backend<'a> {
    pub environment: EnvironmentInfo,
    #[cfg(feature = "anki")]
    pub anki: DisplayAnki,
    pub text_scanner: TextScanner<'a>,
    pub db: Arc<DictionaryDatabase<'a>>,
    pub options: Ptr<YomichanOptions>,
}

impl<'a> Backend<'a> {
    #[cfg(not(feature = "anki"))]
    pub fn new(db: Arc<DictionaryDatabase<'a>>) -> Result<Self, Box<native_db::db_type::Error>> {
        let rtx = db.r_transaction()?;
        let opts: Option<YomichanOptions> = rtx.get().primary("global_user_options")?;
        let options = match opts {
            Some(opts) => opts,
            None => YomichanOptions::new(),
        };
        let backend = Self {
            environment: EnvironmentInfo::default(),
            text_scanner: TextScanner::new(&db),
            db,
            options: Ptr::new(options),
        };
        Ok(backend)
    }
}

impl<'a> Backend<'a> {
    /// The internal impl to write global options to the database.
    /// Takes a [Option<RwTransaction>] so rwtx's can be reused if necessary.
    ///
    /// # Returns
    ///
    /// * Option<YomichanOptions>: The previous [YomichanOptions] found in the db.
    ///   [None] should only ever happen after [Yomichan] creates a brand new `ycd.db`.
    fn _update_options_internal(
        &self,
        rwtx: Option<RwTransaction>,
    ) -> Result<Option<YomichanOptions>, Box<native_db::db_type::Error>> {
        let rwtx = rwtx.unwrap_or(self.db.rw_transaction()?);
        let clone = self.options.write().clone();
        let stale: Option<YomichanOptions> = rwtx.upsert(clone)?;
        rwtx.commit()?;
        Ok(stale)
    }
}

impl<'a> Yomichan<'a> {
    // The user facing api to update their options.
    /// Saves global options for all profiles to the database;
    /// Meant to be called after you mutate a profile (ie. via [Self::mod_options_mut().get_current_profile_mut])
    pub fn update_options(&self) -> Result<(), Box<native_db::db_type::Error>> {
        self.backend._update_options_internal(None);
        Ok(())
    }

    /// Sets the current profile's main language.
    ///
    /// Only updates the language in memory;
    /// To save the set language to the db, call [Self::update_options] after.
    ///
    /// # Example
    ///
    /// Returns [ProfileError::SelectedOutOfBounds] if the language len is out of bounds.
    ///
    /// ```no_run
    /// fn persist_language() -> Option<()> {
    ///    set_language("es").ok()
    /// }
    ///
    /// ```
    pub fn set_language(&mut self, language_iso: &str) -> ProfileResult<()> {
        let res: ProfileResult<()> = self.backend.options.with_ptr(|global| {
            let mut current_profile = global.get_current_profile()?;
            current_profile.write().set_language(language_iso);
            Ok(())
        });
        res
    }

    /// Deletes dictionaries from the database and options by name, in memory only.
    pub fn delete_dictionaries_by_names_in_memory(
        &mut self,
        names: &[impl AsRef<str>],
    ) -> Result<(), DBError> {
        let current_profile = self.backend.get_current_profile()?;
        current_profile.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for name in names {
                let name = name.as_ref();
                dictionaries.swap_remove(name);
            }
        });
        self.update_options()?;
        Ok(())
    }

    pub fn delete_dictionaries_by_indexes(&mut self, indexes: &[usize]) -> Result<(), DBError> {
        let current_profile = self.backend.get_current_profile()?;
        current_profile.with_ptr_mut(|prof| {
            let dictionaries = prof.dictionaries_mut();
            for i in indexes {
                dictionaries.swap_remove_index(*i);
            }
        });
        self.update_options()?;

        Ok(())
    }

    /// Gets all dictionary summaries from the database.
    /// [DictionarySummary] is different from [DictionaryOptions]
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
    use std::{cell::RefCell, fs::File, io::BufReader};

    use crate::{
        database::dictionary_database::{DatabaseMetaFrequency, DatabaseTermEntry},
        dictionary_data::{GenericFreqData, TermMetaFreqDataMatchType, TermMetaModeType},
        test_utils::{self, TEST_PATHS},
        Yomichan,
    };

    use super::{Backend, FindTermsDetails};

    #[test]
    fn rmp_serde_item_debug() {
        use rmp_serde::{Deserializer, Serializer};
        use serde::{Deserialize, Serialize};
        use std::fs::File;
        use std::io::BufReader;

        // This is the generic Value type from the correct crate
        use rmpv::Value;

        // --- SETUP: Create the MessagePack bytes from your JSON ---
        let path = &test_utils::TEST_PATHS.tests_dir;
        let file = File::open(path.join("自業自得_rust.json")).unwrap();
        let reader = BufReader::new(file);

        // This part uses your full DatabaseTermEntry struct, which is correct
        let items: Vec<DatabaseTermEntry> = serde_json::from_reader(reader).unwrap();

        let mut buf = Vec::new();
        items.serialize(&mut Serializer::new(&mut buf)).unwrap();

        // --- THE DEBUGGING STEP ---
        // Deserialize the raw bytes into the generic `rmpv::Value`
        let decoded_value: Value =
            rmp_serde::from_slice(&buf).expect("Failed to decode MessagePack into generic Value");

        // Print the decoded structure. This is the crucial output!
        // It will show you exactly what is being stored.
        println!("--- DECODED RMPV::VALUE ---");
        println!("{decoded_value:#?}");
        println!("--- END DECODED RMPV::VALUE ---");

        // We still expect the final deserialization to fail, which is what we're debugging.
        let result: Result<Vec<DatabaseTermEntry>, _> = rmp_serde::from_slice(&buf);
        result.unwrap();
    }

    #[test]
    fn rmp_serde_meta() {
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
