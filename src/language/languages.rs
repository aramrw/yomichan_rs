use super::{
    descriptors::{self, LANGUAGE_DESCRIPTORS_MAP},
    descriptors_d::LanguageDescriptor,
    language_d::{
        LanguageAndProcessors, LanguageAndReadingNormalizer, LanguageAndTransforms, LanguageSummary,
    },
};

pub fn get_language_summaries() -> Vec<LanguageSummary> {
    LANGUAGE_DESCRIPTORS_MAP
        .values()
        .map(|entry| LanguageSummary {
            name: entry.name.clone(),
            iso: entry.iso.clone(),
            iso639_3: entry.iso639_3.clone(),
            example_text: entry.example_text.clone(),
        })
        .collect::<Vec<LanguageSummary>>()
}

pub fn get_all_language_reading_normalizers() -> Vec<LanguageAndReadingNormalizer> {
    LANGUAGE_DESCRIPTORS_MAP
        .values()
        .filter_map(|entry| {
            if let Some(reading_normalizer) = entry.reading_normalizer {
                return Some(LanguageAndReadingNormalizer {
                    iso: entry.iso.clone(),
                    reading_normalizer,
                });
            };
            None
        })
        .collect::<Vec<LanguageAndReadingNormalizer>>()
}

// pub fn get_all_language_text_processors<'a, O, S>() -> Vec<LanguageAndProcessors<'a, O, S>> {
//     let results = Vec::new();
//     for entry in LANGUAGE_DESCRIPTORS_MAP.values() {
//         let pre_processors = Vec::new();
//
//         for processor in entry.text_processors {
//             pre_processors.push(processor);
//         }
//     }
// }

pub fn is_text_lookup_worthy(text: &str, language: &str) -> bool {
    if let Some(descriptor) = LANGUAGE_DESCRIPTORS_MAP.get(language) {
        if let Some(itlw_fn) = descriptor.is_text_lookup_worthy {
            return itlw_fn(text);
        }
    }
    false
}

pub fn get_all_language_transform_descriptors<'a>() -> Vec<LanguageAndTransforms<'a>> {
    let mut results: Vec<LanguageAndTransforms> = Vec::new();
    for entry in LANGUAGE_DESCRIPTORS_MAP.values() {
        if let Some(language_transforms) = entry.language_transforms {
            let item = LanguageAndTransforms {
                iso: entry.iso.clone(),
                language_transforms,
            };
            results.push(item);
        }
    }
    results
}
